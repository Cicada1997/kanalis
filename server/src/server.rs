use std::env;
use tokio::task::JoinHandle;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use anyhow::{ anyhow, Context };
use sqlx::postgres::{ PgPoolOptions };

use crate::{
    result::Result,
    protocol::{ ClientPacket, ServerPacket, UserId, User },
    intercom::{ ServerChannel, ClientChannel, Request },
    packet_handling::{ just_connected, send_messages_before },
    db::Message,
};

pub struct Server {
    channel: ServerChannel,
    // db: Box<dyn Database + Send + Sync>,
    serverports: Vec<JoinHandle<()>>,
    active_users: Arc<RwLock<HashMap<UserId, tokio::sync::mpsc::Sender<ServerPacket>>>>,
}

impl Server {
    #[must_use]
    pub fn new() -> Self {
        let channel = ServerChannel::new();
        let serverports = Vec::new();
        let active_users = Arc::new(RwLock::new(HashMap::new()));
        Self { channel, serverports, active_users }
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
    #[warn(clippy::too_many_lines)]
    pub async fn serve(mut self) -> Result<()> {
        let database_url = env::var("DATABASE_URL")
            .context("environment variable 'DATABASE_URL' is not defined.")?;

        let pool = PgPoolOptions::new()
            .connect(&database_url).await?;
        
        if self.serverports.is_empty() {
            return Err(anyhow!("No ports listening for clients, exiting..."));
        }

        let (err_sender, mut err_receiver) = tokio::sync::mpsc::channel(128);

        loop {
            tokio::select! {
                request = self.channel.recv() => {
                    let Some(Request { packet, user }) = request else { continue };
                    dbg!(&packet);

                    match packet {
                        ClientPacket::JustConnected(reply_tx) => {
                            self.active_users.write().await.insert(user.user_id, reply_tx.clone());

                            let pool = pool.clone();
                            let err_sender = err_sender.clone();
                            tokio::spawn(async move {
                                if let Err(e) = just_connected(pool, reply_tx, &user).await {
                                    let _ = err_sender.send(e).await;
                                }
                            });
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
                        ClientPacket::GetChannelMessages { channel_id, before }  => {
                            // TODO: implement cache
                            let pool = pool.clone();
                            let active_users = self.active_users.clone();
                            tokio::spawn(async move {
                                if let Some(reply_tx) = active_users.read().await.get(&user.user_id) {
                                    send_messages_before(pool, reply_tx.clone(), channel_id, before, &user).await;
                                } else {
                                    eprintln!("Unable to find user channel in active_users: {user:?}");
                                }
                            });
                        }

                        ClientPacket::Disconnected => {
                            self.active_users.write().await.remove(&user.user_id);
                            println!("User Disconnected: {}", user.username);
                        }

                        // ClientPacket::DeleteChannel { channel_id } => {
                        //     if !user.admin {
                        //         continue;
                        //     }
                        //
                        //     let mut tx = pool.begin().await?;
                        //
                        //     sqlx::query!("DELETE FROM messages WHERE channel_id = $1", channel_id)
                        //         .execute(&mut *tx)
                        //         .await?;
                        //
                        //     sqlx::query!("DELETE FROM channel_members WHERE channel_id = $1", channel_id)
                        //         .execute(&mut *tx)
                        //         .await?;
                        //
                        //     sqlx::query!("DELETE FROM channels WHERE id = $1", channel_id)
                        //         .execute(&mut *tx)
                        //         .await?;
                        //
                        //     tx.commit().await?;
                        // }

                        ClientPacket::CreateChannel { name, private } => {
                            if !user.admin {
                                continue;
                            }

                            // TODO: handle name conflicts

                            sqlx::query!(
                                "
                                    INSERT INTO channels (
                                        name,
                                        private
                                    )
                                    VALUES ($1, $2)
                                    ON CONFLICT (name) DO NOTHING
                                ",
                                name,
                                private,
                            )
                                .execute(&pool)
                                .await?;
                        }

                        ClientPacket::LastUpdated { .. } | ClientPacket::AuthToken(_) => { }
                    }

                }

                err = err_receiver.recv() => {
                    if let Some(err) = err {
                        eprintln!("A critical error occured on a dispatched thread: {err}");
                        return Err(err);
                    }
                }
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
