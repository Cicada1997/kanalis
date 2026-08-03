use serde::{ Serialize, Deserialize };
use chrono::prelude::*;
// use crate::client_handler::ClientConn;

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
#[serde(untagged)]
pub enum ClientPacket {
    AuthToken(String),
    LastUpdated {
        datetime: DateTime<Utc>, 
        // #[serde(skip)]
        // resp: Option<ClientConn>,
    },
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
pub enum Error {
    ConnectionError,
    AuthFail,
    Unauthorized,
}

use std::fmt::Display;
impl Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(fmt, "{:?}", self)?;
        Ok(())
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerPacket {
    NewMessage {
        user: User,
        content: String,
    },

    // results //
    LoginSuccess,
    Error {
        code: Error,
        reason: String,
    },
}

