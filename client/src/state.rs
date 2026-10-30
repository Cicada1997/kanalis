use crate::ui::{ self, CreateChannelModal };
use crate::ui::auth::{ AuthComponent };
use crate::net::ConnEvent;

use protocol::{ MessageId, User, Member, Channel, ChannelId, kattauth::UserDetails, ClientPacket, ServerPacket };

use std::collections::{ HashMap };

use tokio::sync::{ mpsc };
use serde::{ Serialize, Deserialize };
use chrono::naive::NaiveDateTime;
use regex::Regex;
 
use iced::widget::{ image, operation };
use iced::Task;

use std::sync::LazyLock;

pub static IMAGE_TAG: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"<image:([^>]+)>")
        .unwrap()
);
                
#[derive(Debug, Default)]
pub enum AuthStatus {
    LoggedIn,
    Waiting,
    #[default]
    LoggedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: MessageId,
    pub user: User,
    pub channel_id: ChannelId,
    pub content: String,
    pub timestamp: NaiveDateTime,
}

use crate::message_history::*;

#[derive(Default)]
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
    pub auth: AuthComponent,

    // Connection
    pub sender: Option<mpsc::Sender<ClientPacket>>,

    // Auth
    pub current_token: Option<String>,
    pub auth_status: AuthStatus,
    pub username_input_field: String,
    pub password_input_field: String,
}

impl State {
    pub fn new() -> (Self, Task<ui::Message>) {
        let state = Self {
            current_token: dotenv::var("TOKEN").ok(),
            ..Default::default()
        };
        // Only focus the login field if we actually show the login screen
        let task = if state.current_token.is_none() {
            operation::focus("username")
        } else {
            Task::none()
        };
        (state, task)
    }

    pub fn apply_conn_event(&mut self, event: ConnEvent) -> Task<ui::Message> {
        match event {
            ConnEvent::Connecting => {
                self.auth_status = AuthStatus::Waiting;
                Task::none()
            }
            ConnEvent::Connected(sender) => {
                self.sender = Some(sender);
                Task::none()
            }
            ConnEvent::Disconnected => {
                println!("Disconnected from server");
                self.sender = None;
                Task::none()
            }
            ConnEvent::Exit(reason) => {
                println!("exiting with reason: {:?}", reason);
                std::process::exit(0)
            },
            ConnEvent::Packet(packet) => self.handle_server_packet(packet)
        }
    }

    fn handle_server_packet(&mut self, packet: ServerPacket) -> iced::Task<ui::Message> {
        match packet {
            ServerPacket::ServerData { name, channels, members } => {
                println!("Server name is {}", name);
                if let Some(first_channel) = channels.first() {
                    self.switch_channel(first_channel.id);
                }
                self.channels = channels;
                self.members = members;

                for member in self.members.iter() {
                    dbg!(member);
                }

                Task::none()
            }

            ServerPacket::LoginSuccess { user } => {
                self.auth_status = AuthStatus::LoggedIn;
                self.user = Some(user);

                Task::none()
            }

            ServerPacket::NewMessage { id, user, channel_id, timestamp, content } => {
                // Scanning for images
                let mut tasks = vec![];

                // loading found image tags asynchronously
                for caps in IMAGE_TAG.captures_iter(&content) {
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
                self.messages.push(ChatMessage { id, user, channel_id, content, timestamp });
                iced::Task::batch(tasks)
            }
            ServerPacket::Error { code, reason } => {
                if matches!(code, protocol::Error::AuthFail) {
                    eprintln!("Invalid token. Restart with a working one.");
                } else {
                    eprintln!("Error: {:?}: {}", code, reason);
                }
                Task::none()
            }
        }
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

    pub fn channel_next(&mut self) {
        self.move_selected_channel(false);
    }

    pub fn channel_previous(&mut self) {
        self.move_selected_channel(true);
    }

    fn move_selected_channel(&mut self, backwards: bool) {
        let len = self.channels.len();
        if len <= 1 {
            return;
        }

        let Some(current) = self
            .channels
            .iter()
            .position(|ch| ch.id == self.current_channel)
        else {
            return;
        };

        let next = if backwards {
            (current + len - 1) % len
        } else {
            (current + 1) % len
        };

        let id = self.channels[next].id;
        self.switch_channel(id);
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

    
}
