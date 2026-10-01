use serde::{ Serialize, Deserialize };
// use tokio::sync::{ mpsc, oneshot };
use tokio::sync::{ mpsc };
use sqlx::types::chrono::{ NaiveDateTime, NaiveDate };
// use chrono::prelude::*;

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

#[derive(sqlx::FromRow, Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub user_id: i64,
    pub username: String,
}

#[derive(sqlx::FromRow, Debug, Clone, Serialize, Deserialize)]
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


fn print_json<T: Serialize>(packet: T) {
    if let Ok(packet_str) = serde_json::to_string_pretty(&packet) {
        println!("{packet_str}");
    }
}

/// # Panics
///
/// Panics if anything goes wrong. It's really just a test so let it be.
pub fn print_protocol() {
    println!("// SERVER PACKET //");

    let Some(timestamp) = NaiveDate::from_ymd_opt(2016, 7, 8)
        .and_then(|d| d.and_hms_opt(9, 10, 11)) else {
            eprintln!("wtf jag kunde inte göra en timestamp????!!");
            return;
    };

    print_json(ServerPacket::NewMessage {
        id: 0,
        user: User { name: "string".to_string() },
        channel_id: 0,
        timestamp,
        content: "string".to_string(),
    });

    print_json(ServerPacket::LoginSuccess {
        user: UserDetails {
            user_id: 0,
            username: "string".to_string(),
            admin: false,
        },
    });

    print_json(ServerPacket::Error {
        code: Error::AuthFail,
        reason: "string".to_string(),
    });

    println!("// CLIENT PACKET //");

    print_json(ClientPacket::AuthToken("string".to_string()));

    print_json(ClientPacket::LastUpdated {
        datetime: "2024-01-01T00:00:00Z".to_string(),
    });

    print_json(ClientPacket::Message {
        // user_id: 0,
        channel_id: 0,
        content: "string".to_string(),
    });
}
