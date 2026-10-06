use crate::{
    // result::Result,
    protocol::{ ServerPacket, MessageId, ChannelId, UserId, User },
};

use serde::{ Serialize, Deserialize };
use sqlx::FromRow;
use sqlx::types::chrono::NaiveDateTime;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub author_id: UserId,
    pub channel_id: ChannelId,
    pub content: String,
    pub timestamp: NaiveDateTime,
}

// #[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
// pub struct ServerData {
//     pub 
// }
//
// chat=# CREATE TABLE channels (
// chat(# id BIGSERIAL PRIMARY KEY NOT NULL,
// chat(# name TEXT NOT NULL,
// chat(# owner_id BIGINT NOT NULL,
// chat(# FOREIGN KEY(owner_id) REFERENCES users(username)
// chat(# );
// ERROR:  there is no unique constraint matching given keys for referenced table "users"



impl Message {
    #[must_use]
    pub fn to_response(self, user: User) -> ServerPacket {
        ServerPacket::NewMessage {
            id: self.id,
            user,
            channel_id: self.channel_id,
            timestamp: self.timestamp,
            content: self.content,
        }
    }
}

    
// pub struct Database {
//     pool: Pool<Postgres>,
// }
//
// impl Database {
//     pub async fn connect() -> Result<Self> {
//         let db_url = env::var("DATABASE_URL")?;
//
//         let pool = connect(&db_url).await;
//         Self {
//             pool,
//         }
//     }
//
//     pub async fn add_message(&self, msg: Message) -> Result<()> {
//
//     }
// }
