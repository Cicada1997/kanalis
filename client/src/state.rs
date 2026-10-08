use crate::ui::{ self, CreateChannelModal };
use crate::net::ConnEvent;

use protocol::{ MessageId, User, Member, Channel, ChannelId, kattauth::UserDetails, ClientPacket, ServerPacket };

use std::collections::{ HashMap };

use tokio::sync::{ mpsc };
use serde::{ Serialize, Deserialize };
use chrono::naive::NaiveDateTime;
use regex::Regex;
 
use iced::widget::image;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub user: User,
    pub channel_id: ChannelId,
    pub content: String,
    pub timestamp: NaiveDateTime,
}

struct ChannelMessages {
    messages: Vec<Message>,
    sorted: bool,
}

impl ChannelMessages {
    pub fn get(&self) -> &[Message] {
        return self.messages.as_slice();
    }

    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
        self.sort();
    }

    #[allow(unused)]
    pub fn extend(&mut self, messages: &[Message]) {
        self.messages.extend(messages.iter().cloned());
        self.sort();
    }

    pub fn push_unchecked(&mut self, message: Message) {
        self.sorted = false;
        self.messages.push(message);
    }

    #[allow(unused)]
    pub fn try_sort(&mut self) {
        if !self.sorted {
            self.sort();
        }
    }

    fn sort(&mut self) {
        self.messages.sort_by(|m1, m2| m1.timestamp.cmp(&m2.timestamp));
        self.sorted = true;
    }
}

#[derive(Default)]
pub struct MessageHistory {
    channels: HashMap<ChannelId, ChannelMessages>
}

impl MessageHistory {
    pub fn new() -> Self { Self::default() }

    pub fn get(&self, channel_id: ChannelId) -> Option<&[Message]> {
        self.channels.get(&channel_id).map(|ch| ch.get())
    }

    pub fn push(&mut self, message: Message) {
        self.channels.entry(message.channel_id)
            .or_insert_with(|| ChannelMessages { messages: vec![], sorted: true })
            .push(message);
    }

    pub fn push_unchecked(&mut self, message: Message) {
        if let Some(ch) = self.channels.get_mut(&message.channel_id) {
            ch.push_unchecked(message);
        } else {
            self.channels.insert(message.channel_id, ChannelMessages {
                messages: vec![message],
                sorted: true,
            });
        }
    }

    pub fn extend(&mut self, messages: &[Message]) {
        for message in messages {
            self.channels.entry(message.channel_id)
                .or_insert_with(|| ChannelMessages { messages: vec![], sorted: true })
                .push(message.clone());
        }
    }

    pub fn extend_unchecked(&mut self, messages: &[Message], id: ChannelId) {
        let ch = self.channels.entry(id)
            .or_insert_with(|| ChannelMessages { messages: vec![], sorted: true });
        
        ch.messages.extend(messages.iter().cloned());
        ch.sorted = false;
    }
}

pub struct State {
    // Chat Data
    pub channels: Vec<Channel>,
    pub members: Vec<Member>,
    pub messages: MessageHistory,
    pub user: Option<UserDetails>,
    pub current_channel: ChannelId,
    pub current_message: String,
    pub image_cache: HashMap<String, image::Handle>,

    // UI Components
    pub create_channel_modal: Option<CreateChannelModal>,

    // Connection
    pub sender: Option<mpsc::Sender<ClientPacket>>,

    // Auth
    pub current_token: Option<String>,
    pub auth_status: AuthStatus,
    pub username_input_field: String,
    pub password_input_field: String,
}

#[derive(Debug)]
pub enum AuthStatus {
    LoggedIn,
    Waiting,
    LoggedOut,
}

impl Default for State {
    fn default() -> Self {
        Self {
            channels: vec![],
            members: vec![],
            messages: MessageHistory::new(),
            user: None,
            current_channel: 0,
            current_message: String::new(),
            image_cache: HashMap::new(),

            create_channel_modal: None,

            sender: None,

            current_token: dotenv::var("TOKEN").ok(),
            auth_status: AuthStatus::LoggedOut,
            username_input_field: String::new(),
            password_input_field: String::new(),
        }
    }
}

impl State {
    pub fn apply_conn_event(&mut self, event: ConnEvent) -> iced::Task<ui::Message> {
        let mut tasks = vec![];

        match event {
            ConnEvent::Connecting => {
                self.auth_status = AuthStatus::Waiting;
            }
            ConnEvent::Connected(sender) => {
                self.sender = Some(sender);
            }
            ConnEvent::Disconnected => {
                println!("Disconnected from server");
                self.sender = None;
            }
            ConnEvent::Exit(reason) => {
                println!("exiting with reason: {:?}", reason);
                std::process::exit(0)
            },
            ConnEvent::Packet(packet) => {
                tasks.push(self.handle_server_packet(packet));
            }
        }

        iced::Task::batch(tasks)
    }

    fn handle_server_packet(&mut self, packet: ServerPacket) -> iced::Task<ui::Message> {
        let mut tasks = vec![];

        match packet {
            ServerPacket::ServerData { name, channels, members } => {
                println!("Server name is {}", name);
                if let Some(first_channel) = channels.first() {
                    self.switch_channel(first_channel.id);
                }
                self.channels = channels;
                self.members = members;
            }
            ServerPacket::LoginSuccess { user } => {
                self.auth_status = AuthStatus::LoggedIn;
                self.user = Some(user);
            }
            ServerPacket::NewMessage { id, user, channel_id, timestamp, content } => {
                let regex = Regex::new(r"<image:(.*)>").unwrap();
                for caps in regex.captures_iter(&content) {
                    if let Some(m) = caps.get(1) {
                        let url = m.as_str().to_string();
                        if !self.image_cache.contains_key(&url) {
                            let url_clone = url.clone();
                            tasks.push(iced::Task::perform(
                                async move {
                                    reqwest::get(&url.clone()).await.ok()?
                                        .bytes().await.ok()
                                        .map(|b| b.to_vec())
                                },
                                move |bytes| ui::Message::ImageLoaded(url_clone, bytes)
                            ));
                        }
                    }
                }
                self.messages.push(Message { id, user, channel_id, content, timestamp });
            }
            ServerPacket::Error { code, reason } => {
                if matches!(code, protocol::Error::AuthFail) {
                    eprintln!("Invalid token. Restart with a working one.");
                } else {
                    eprintln!("Error: {:?}: {}", code, reason);
                }
            }
        }

        iced::Task::batch(tasks)
    }

    fn fetch_messages(&mut self, channel_id: ChannelId) {
        if let Some(ref sender) = self.sender {
            let _ = sender.try_send(ClientPacket::GetChannelMessages {
                channel_id: channel_id,
                before: None,
            }).inspect_err( |e| eprintln!("{:?}", e) );
        } else { eprintln!("ERROR [fetch_messages()]: THIS FUNCTION IS UNREACHABLE AND SHOULD NOT BE USED WHEN A SENDER IS NOT SET") }
    }

    pub fn switch_channel(&mut self, channel_id: ChannelId) {
        self.current_channel = channel_id;
        if matches!(self.messages.get(channel_id), None) {
            self.fetch_messages(channel_id);
        }
    }

    pub fn send_current_message(&mut self) {
        if self.current_message.trim().is_empty() { return; }

        if let Some(ref mut sender) = self.sender {
            let packet = ClientPacket::Message {
                channel_id: self.current_channel,
                content: self.current_message.clone(),
            };

            match sender.try_send(packet) {
                Ok(()) => self.current_message.clear(),
                Err(e) => eprintln!("Failed to send message: {}", e),
            }
        } else {
            eprintln!("Unable to send message, sender is not yet defined.");
        }
    }

    pub fn submit_login(&mut self) {
        let username = self.username_input_field.clone();
        let hashword = self.password_input_field.clone();

        self.username_input_field.clear();
        self.password_input_field.clear();

        let client = reqwest::blocking::Client::new();
        let res = client.post("https://auth.kattmys.se/login")
            .json(&protocol::kattauth::LoginDetails {
                username,
                hashword,
            })
            .send();

        match res {
            Ok(resp) => {
                dbg!(&resp.status());
                if resp.status().is_success() {
                    let token: String = match resp.json() {
                        Ok(token) => token,
                        Err(_e) => { return }
                    };
                    self.current_token = Some(token);
                }

            }
            Err(e) => {
                dbg!(&e);
            }
        }
    }
}
