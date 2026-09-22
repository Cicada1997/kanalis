use std::pin::Pin;
use std::future::Future;
use tokio::sync::{ mpsc };

use crate::{
    protocol::{ self, ClientPacket, ServerPacket },
};

pub type PinBoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// TODO: use fantom data instead of an interface.

pub trait ClientConnection: Send {
    fn recv(&mut self) -> PinBoxFuture<'_, Option<ClientPacket>>; // Pin<Box<dyn Future<Output = Option<ClientPacket>> + Send + '_>>;
    fn send(&mut self, packet: ServerPacket);
    fn sender(&self) -> mpsc::UnboundedSender<ServerPacket>;
    fn client_id(&self) -> String;
    
    fn send_error(&mut self, code: protocol::Error, reason: &str) {
        self.send(ServerPacket::Error { code, reason: reason.to_string() });
    }
}
