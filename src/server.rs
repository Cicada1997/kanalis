use tokio::task::JoinHandle;
use std::net::ToSocketAddrs;

use crate::{
    result::Result,
    protocol::{ ClientPacket, ServerPacket, User },
    intercom::{ ServerChannel, ClientChannel },
    client_handler::{ ClientHandler },
    // database::{ self, Database },
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
        let port = P::new(client_channel);

        let handle = tokio::spawn(async move {
            port.listen().await;
        });

        self.serverports.push(handle);

        self
    }

    /// # Errors
    ///
    /// Dramatic exits are returned as errors. 
    pub fn serve(&mut self) -> Result<()> {
        loop {
            let Some(packet) = self.channel.recv() else { continue };
            dbg!(&packet);

            match packet {
                ClientPacket::Message { content, .. } => {
                    self.channel.send(ServerPacket::NewMessage {
                        user: User { name: String::from("Okänd Användare") },
                        content,
                    });
                }

                _ => { }
            }

        }

        Ok(())
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

pub trait ServerPort {
    fn new(client_channel: ClientChannel) -> Self;
    fn listen(&self) -> impl std::future::Future<Output = Result<()>> + Send;
}
