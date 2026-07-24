use crate::{
    protocol::{ ClientPacket, ServerPacket },
};

use tokio::{
    sync::{ broadcast, mpsc },
};

pub struct ServerChannel {
    pub receiver: mpsc::Receiver<ClientPacket>,
    pub sender:   broadcast::Sender<ServerPacket>,
}

pub struct ClientChannel {
    pub receiver: broadcast::Receiver<ServerPacket>,
    pub sender:   mpsc::Sender<ClientPacket>,
}

impl ClientChannel {
    pub fn split(self) -> (broadcast::Receiver<ServerPacket>, mpsc::Sender<ClientPacket>) {
        (self.receiver, self.sender)
    }
}

impl Clone for ClientChannel {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.resubscribe(),
        }
    }
}

pub trait ServerHandler { }

// pub trait TcpServer {
//     fn serve(self, addr: impl ToSocketAddrs) -> impl std::future::Future<Output = tokio::io::Result<impl ServerHandler>>;
// }
