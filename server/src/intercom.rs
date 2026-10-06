use crate::{
    result::Result,
    protocol::{ ClientPacket, ServerPacket, UserDetails },
};

use tokio::{
    sync::{ broadcast, mpsc },
};

pub struct Request {
    pub user: UserDetails,
    pub packet: ClientPacket,
}

pub struct ServerChannel {
    receiver: mpsc::Receiver<Request>,
    sender_clone: mpsc::Sender<Request>,
    sender: broadcast::Sender<ServerPacket>,
}

impl ServerChannel {
    #[must_use]
    pub fn new() -> Self {
        let (sender_clone, receiver) = mpsc::channel(128);
        let (sender, _) = broadcast::channel(100);

        Self { receiver, sender_clone, sender }
    }

    pub fn subscribe(&mut self) -> ClientChannel {
        let receiver = self.sender.subscribe();
        let sender = self.sender_clone.clone();
        ClientChannel::new(receiver, sender)
    }

    #[must_use]
    pub async fn recv(&mut self) -> Option<Request> {
        self.receiver.recv().await
    }

    pub fn send(&mut self, packet: ServerPacket) {
        let _ = self.sender.send(packet);
    }
}

impl Default for ServerChannel {
    fn default() -> Self { Self::new() }
}

pub struct ClientChannel {
    pub receiver: broadcast::Receiver<ServerPacket>,
    pub sender:   mpsc::Sender<Request>,
    pub replies:  mpsc::Receiver<ServerPacket>,

    reply_sender_model: mpsc::Sender<ServerPacket>,
}

impl ClientChannel {
    #[must_use]
    pub fn new(receiver: broadcast::Receiver<ServerPacket>, sender: mpsc::Sender<Request>) -> Self {
        let (reply_sender_model, replies) = mpsc::channel(128);

        Self { receiver, sender, replies, reply_sender_model }
    }

    #[must_use]
    pub fn split(self) -> (broadcast::Receiver<ServerPacket>, mpsc::Sender<Request>) {
        (self.receiver, self.sender)
    }

    pub async fn send(&mut self, packet: Request) {
        let _ = self.sender.send(packet).await;
    }

    /// # Errors
    ///
    /// Forwarded errors from `tokio::sync::broadcast::Receiver`.
    pub async fn recv(&mut self) -> Result<ServerPacket> {
        loop {
            tokio::select! {
                packet = self.receiver.recv() => {
                    return Ok(packet?);
                }

                packet = self.replies.recv() => {
                    if let Some(packet) = packet {
                        return Ok(packet);
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn get_sender(&self) -> mpsc::Sender<ServerPacket> {
        self.reply_sender_model.clone()
    }
}

impl Clone for ClientChannel {
    fn clone(&self) -> Self {
        let sender = self.sender.clone();
        let receiver = self.receiver.resubscribe();
        Self::new(receiver, sender)
    }
}
