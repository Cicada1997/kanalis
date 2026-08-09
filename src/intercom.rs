use crate::{
    protocol::{ ClientPacket, ServerPacket },
};

use std::sync::mpsc;
use tokio::{
    sync::{ broadcast },
};

pub struct ServerChannel {
    receiver: mpsc::Receiver<ClientPacket>,
    sender_clone: mpsc::Sender<ClientPacket>,
    sender: broadcast::Sender<ServerPacket>,
}

impl ServerChannel {
    #[must_use]
    pub fn new() -> Self {
        let (sender_clone, receiver) = mpsc::channel();
        let (sender, _) = broadcast::channel(100);

        Self { receiver, sender_clone, sender }
    }

    pub fn subscribe(&mut self) -> ClientChannel {
        let receiver = self.sender.subscribe();
        let sender = self.sender_clone.clone();
        ClientChannel { receiver, sender }
    }

    #[must_use]
    pub fn recv(&self) -> Option<ClientPacket> {
        self.receiver
            .recv()
            .inspect_err(|e| eprintln!("Broken pipe to client: {e}"))
            .ok()
    }

    pub fn send(&mut self, packet: ServerPacket) {
        self.sender.send(packet);
    }
}

impl Default for ServerChannel {
    fn default() -> Self { Self::new() }
}

pub struct ClientChannel {
    pub receiver: broadcast::Receiver<ServerPacket>,
    pub sender:   mpsc::Sender<ClientPacket>,
}

impl ClientChannel {
    #[must_use]
    pub fn split(self) -> (broadcast::Receiver<ServerPacket>, mpsc::Sender<ClientPacket>) {
        (self.receiver, self.sender)
    }

    pub fn send(&mut self, packet: ClientPacket) {
        self.sender.send(packet);
    }
}

impl Clone for ClientChannel {
    fn clone(&self) -> Self {
        let sender = self.sender.clone();
        let receiver = self.receiver.resubscribe();
        Self { receiver, sender }
    }
}
