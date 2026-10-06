use serde::{ Serialize, Deserialize };
use tokio::sync::{ mpsc };
use chrono::{ NaiveDateTime };

pub mod kattauth {
    use serde::{ Serialize, Deserialize };
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub struct LoginDetails {
        pub username: String,
        pub hashword: String,
    }
}

pub type MessageId = i64;
pub type UserId = i64;
pub type ChannelId = i64;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UserDetails {
    pub user_id:    UserId,
    pub username:   String,
    pub admin:      bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientPacket {
    AuthToken(String),
    #[serde(skip)]
    JustConnected(mpsc::Sender<ServerPacket>),
    #[serde(skip)]
    Disconnected,
    GetChannelMessages {
        channel_id: ChannelId,
        before: Option<MessageId>,
    },
    CreateChannel {
        name: String,
        private: bool,
    },
    LastUpdated {
        datetime: String, // DateTime<Utc>, 
        // #[serde(skip)]
        // resp: Option<ClientConn>,
    },
    Message {
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
pub enum Error {
    ConnectionError,
    AuthFail,
    Unauthorized,
}

use std::fmt::Display;
impl Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "{self:?}")?;
        Ok(())
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub user_id: i64,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerPacket {
    ServerData {
        name: String,
        channels: Vec<Channel>, //TODO: Update to a struct from db (id, name)
        members: Vec<Member>
    },

    NewMessage {
        id: MessageId,
        user: User,
        channel_id: ChannelId,
        timestamp: NaiveDateTime,
        content: String,
    },


    // results //
    LoginSuccess {
        user: UserDetails,
    },

    Error {
        code: Error,
        reason: String,
    },
}
