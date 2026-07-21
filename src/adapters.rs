use crate::{
    protocol::{ ClientPacket, ServerPacket },
};

use tokio::{
    task,
    net::{ ToSocketAddrs },
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
    fn split(self) -> (broadcast::Receiver<ServerPacket>, mpsc::Sender<ClientPacket>) {
        (self.receiver, self.sender)
    }
}

pub struct ServerHandle {
    pub channel: ServerChannel,
    pub tcp_server_thread: task::JoinHandle<tokio::io::Result<()>>,
}


pub trait Server {
    fn serve(self, addr: impl ToSocketAddrs) -> impl std::future::Future<Output = tokio::io::Result<ServerHandle>>;
}
