use std::env;
use tokio::task::JoinHandle;
use anyhow::{ anyhow, Context };
use sqlx::postgres::PgPoolOptions;

use crate::{
    result::Result,
    protocol::{ ClientPacket, ServerPacket, User, Channel, Member },
    intercom::{ ServerChannel, ClientChannel, Request },
    db::Message,
};

pub struct Server {
    channel: ServerChannel,
    // db: Box<dyn Database + Send + Sync>,
    serverports: Vec<JoinHandle<()>>,
}

impl Server {
    #[must_use]
    pub fn new() -> Self {
        let channel = ServerChannel::new();
        Self { channel, serverports: Vec::new() }
    }

    #[must_use]
    pub fn add_port<P>(mut self) -> Self
    where 
        P: ServerPort + Send + 'static,
    {
        let client_channel = self.channel.subscribe();
        let port = match P::new(client_channel) {
            Ok(port) => port,
            Err(e) => {
                eprintln!("Unable to initialize port {}: {}", P::name(), e);
                return self;
            }
        };

        println!("Starting port {}", P::name());
        let handle = tokio::spawn(async move {
            let _ = port
                .listen()
                .await
                .inspect_err(|e| eprintln!("Unable to start port {}: {}", P::name(), e));
            });

        self.serverports.push(handle);

        self
    }

    /// # Errors
    ///
    /// Dramatic exits are returned as errors. 
    pub async fn serve(mut self) -> Result<()> {
        let database_url = env::var("DATABASE_URL")
            .context("environment variable 'DATABASE_URL' is not defined.")?;

        let pool = PgPoolOptions::new()
            .connect(&database_url).await?;

        if self.serverports.is_empty() {
            return Err(anyhow!("No ports listening for clients, exiting..."));
        }

        loop {
            let Some(Request { packet, user }) = self.channel.recv().await else { continue };
            dbg!(&packet);

            match packet {
                ClientPacket::JustConnected(reply_tx) => {
                    let _ = sqlx::query!(
                        "
                            INSERT INTO users (user_id, username)
                            VALUES ($1, $2)
                            ON CONFLICT (user_id) DO NOTHING
                        ",
                        user.user_id,
                        user.username,
                    )
                        .execute(&pool)
                        .await?;

                    let channels = sqlx::query_as!(
                        Channel,
                        "
                            SELECT c.id, c.name
                            FROM channels c
                            INNER JOIN channel_members cm ON cm.channel_id = c.id
                            WHERE cm.user_id = $1
                            AND cm.access
                        ",
                        user.user_id
                    )
                        .fetch_all(&pool)
                        .await?;

                    let members = sqlx::query_as!(
                        Member,
                        "
                            SELECT users.user_id, users.username
                            FROM users
                        "
                    )
                        .fetch_all(&pool)
                        .await?;

                    let res = reply_tx.send(ServerPacket::ServerData {
                        name: String::from("Cicadas Server"),
                        channels,
                        members,
                    })
                        .await
                        .inspect_err(|e| eprintln!("Unable to send Server data to client {user:?}: {}", e));

                    if res.is_err() {
                        continue;
                    }


                        // id: MessageId,
                        // user: User,
                        // channel_id: ChannelId,
                        // timestamp: NaiveDateTime,
                        // content: String,

                    let mut rows = sqlx::query!(
                        "
                            SELECT 
                                m.id, 
                                u.username AS name, 
                                m.channel_id, 
                                m.timestamp, 
                                m.content
                            FROM messages m
                            INNER JOIN channel_members cm ON cm.channel_id = m.channel_id
                            INNER JOIN users u ON m.author_id = u.user_id
                            WHERE cm.user_id = $1
                            ORDER BY m.timestamp DESC
                            LIMIT 150
                        ",
                        user.user_id
                    )
                        .fetch_all(&pool)
                        .await?;

                    rows.reverse();
                    
                    for row in rows {
                        let packet = ServerPacket::NewMessage {
                            id: row.id,
                            user: User { name: row.name },
                            channel_id: row.channel_id,
                            timestamp: row.timestamp,
                            content: row.content,
                        }; 
                        if let Err(e) = reply_tx.send(packet).await {
                            eprintln!("Unable to send NewMessage data to client {user:?}: {}", e);
                            break;
                        }
                    }
                }

                ClientPacket::Message { channel_id, content, .. } => {
                    let msg = sqlx::query_as!(
                        Message,
                        "
                            INSERT INTO messages (author_id, channel_id, content)
                            VALUES ($1, $2, $3)
                            RETURNING *
                        ",
                        user.user_id,
                        channel_id,
                        content
                    )
                        .fetch_one(&pool)
                        .await?;

                    let server_packet = msg.to_response( User { name: user.username.clone() });
                    self.channel.send(server_packet);
                }
                ClientPacket::LastUpdated { .. } | ClientPacket::AuthToken(_) => { }
            }

        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ServerPort {
    /// # Errors
    ///
    /// Often return `Err` when the function is not able to get neccessary environment variables and
    /// more.
    fn new(client_channel: ClientChannel) -> Result<Self> where Self: Sized;
    fn listen(&self) -> impl std::future::Future<Output = Result<()>> + Send;
    fn name() -> String;
}
