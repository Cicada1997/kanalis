use crate::{
    result::Result,
    protocol::{ ClientPacket, ServerPacket },
};

use tokio::{
    sync::{ broadcast, mpsc },
};

pub struct ServerChannel {
    receiver: mpsc::UnboundedReceiver<ClientPacket>,
    sender_clone: mpsc::UnboundedSender<ClientPacket>,
    sender: broadcast::Sender<ServerPacket>,
}

impl ServerChannel {
    #[must_use]
    pub fn new() -> Self {
        let (sender_clone, receiver) = mpsc::unbounded_channel();
        let (sender, _) = broadcast::channel(100);

        Self { receiver, sender_clone, sender }
    }

    pub fn subscribe(&mut self) -> ClientChannel {
        let receiver = self.sender.subscribe();
        let sender = self.sender_clone.clone();
        ClientChannel { receiver, sender }
    }

    #[must_use]
    pub async fn recv(&mut self) -> Option<ClientPacket> {
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
    pub sender:   mpsc::UnboundedSender<ClientPacket>,
}

impl ClientChannel {
    #[must_use]
    pub fn split(self) -> (broadcast::Receiver<ServerPacket>, mpsc::UnboundedSender<ClientPacket>) {
        (self.receiver, self.sender)
    }

    pub fn send(&mut self, packet: ClientPacket) {
        let _ = self.sender.send(packet);
    }

    /// # Errors
    ///
    /// Forwarded errors from `tokio::sync::broadcast::Receiver`.
    pub async fn recv(&mut self) -> Result<ServerPacket> {
        Ok(self.receiver.recv().await?)
    }
}

impl Clone for ClientChannel {
    fn clone(&self) -> Self {
        let sender = self.sender.clone();
        let receiver = self.receiver.resubscribe();
        Self { receiver, sender }
    }
}
