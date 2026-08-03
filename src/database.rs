use chrono::prelude::*;
use serde::{ Serialize, Deserialize };
use crate::{
    protocol::{ User, UserId, ServerPacket },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Message {
    user: User,
    content: String,
    sent_at: DateTime<Utc>,
}

impl From<Message> for ServerPacket {
    fn from(msg: Message) -> ServerPacket {
        ServerPacket::NewMessage { user: msg.user, content: msg.content }
    }
}

pub trait Database {
    fn get_messages_since(&self, /* user_id: UserId, */ datetime: DateTime<Utc>) -> Vec<Message>;
    fn get_recent_messages(&self, user_id: UserId) -> Vec<Message>;

}

pub struct ShitDb {
    messages: Vec<Message>,
}

impl ShitDb {
    pub fn new() -> Self {
        Self { messages: Vec::with_capacity(32) }
    }

    pub fn from(messages: Vec<Message>) -> Self {
        Self { messages }
    }
}

impl Database for ShitDb {
    // TODO: Make dependant on user access
    fn get_messages_since(&self, /* user_id: UserId, */ datetime: DateTime<Utc>) -> Vec<Message> {
        self.messages
            .iter()
            .filter(|msg| datetime < msg.sent_at )
            .map(|msg| msg.clone())
            .collect()
    }

    fn get_recent_messages(&self, user_id: UserId) -> Vec<Message> {
        if self.messages.len() > 20 {
            self.messages[..20].iter().map(|msg| msg.to_owned()).collect()
        } else {
            self.messages.clone()
        }
    }
}
