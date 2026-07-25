use serde::{ Serialize, Deserialize };

pub type UserId = u64;
pub type ChannelId = u64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UserDetails {
    pub user_id:    i32,
    pub username:   String,
    pub admin:      Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientPacket {
    AuthToken(String),
    Message {
        user_id: UserId,
        channel_id: ChannelId,
        content: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct User {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerPacket {
    NewMessage {
        user: User,
        content: String,
    }
}

