use std::env;
use tokio::task::JoinHandle;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use anyhow::{ anyhow, Context };
use sqlx::postgres::{ PgPool, PgPoolOptions };

use crate::{
    result::Result,
    protocol::{ ClientPacket, ServerPacket, UserDetails, UserId, User, Channel, Member },
    intercom::{ ServerChannel, ClientChannel, Request },
    db::Message,
};

pub struct Server {
    channel: ServerChannel,
    // db: Box<dyn Database + Send + Sync>,
    serverports: Vec<JoinHandle<()>>,
    active_users: Arc<RwLock<HashMap<UserId, tokio::sync::mpsc::Sender<ServerPacket>>>>,
}

/// # Errors
/// Accumulates the errors from database and client interactions when building and sending a
/// `ServerData` packet.
pub async fn just_connected(user: &UserDetails, output: tokio::sync::mpsc::Sender<ServerPacket>, pool: PgPool) -> Result<()> {
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

    let res = output.send(ServerPacket::ServerData {
        name: String::from("Cicadas Server"),
        channels,
        members,
    })
        .await;
        // .inspect_err(|e| eprintln!("Unable to send Server data to client {user:?}: {e}"));

    if let Err(e) = res {
        eprintln!("An error occured while transmitting data internally to the client handler: {e} (recipient: {user:?})");
        return Ok(())
    }

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
        if let Err(e) = output.send(packet).await {
            eprintln!("Unable to send NewMessage data to client {user:?}: {e}");
            break;
        }
    }
    
    Ok(())
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
                                if let Err(e) = just_connected(&user, reply_tx, pool).await {
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
