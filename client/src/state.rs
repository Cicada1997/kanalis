use crate::protocol::{ MessageId, User, Member, Channel, ChannelId, UserDetails, ClientPacket, ServerPacket, Error };
use crate::ui::CreateChannelModal;
use crate::net::ConnEvent;

use std::collections::HashMap;
use tokio::sync::mpsc;
use serde::{ Serialize, Deserialize };
use chrono::naive::NaiveDateTime;

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

    pub fn extend(&mut self, messages: &[Message]) {
        self.messages.extend(messages.iter().cloned());
        self.sort();
    }

    pub fn push_unchecked(&mut self, message: Message) {
        self.sorted = false;
        self.messages.push(message);
    }

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
    pub fn apply_conn_event(&mut self, event: ConnEvent) {
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
                println!("exiting with reason: {reason:?}");
                std::process::exit(0)
            },
            ConnEvent::Packet(packet) => self.handle_server_packet(packet),
        }
    }

    fn handle_server_packet(&mut self, packet: ServerPacket) {
        dbg!(&packet);
        match packet {
            ServerPacket::ServerData { name, channels, members } => {
                println!("Server name is {name}");
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
                self.messages.push(Message { id, user, channel_id, content, timestamp });
            }
            ServerPacket::Error { code, reason } => {
                if matches!(code, Error::AuthFail) {
                    eprintln!("Invalid token. Restart with a working one.");
                } else {
                    eprintln!("Error: {code:?}: {reason}");
                }
            }
        }
    }

    fn fetch_messages(&mut self, channel_id: ChannelId) {
        if let Some(ref sender) = self.sender {
            let _ = sender.try_send(ClientPacket::GetChannelMessages {
                channel_id: channel_id,
                before: None,
            }).inspect_err( |e| eprintln!("{e:?}") );
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
                Err(e) => eprintln!("Failed to send message: {e}"),
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
            .json(&crate::protocol::kattauth::LoginDetails {
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
