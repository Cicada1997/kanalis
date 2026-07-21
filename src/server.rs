use crate::{
    adapters::{ Server, ServerHandle, ClientChannel, ServerChannel },
    protocol::{ ClientPacket, ServerPacket },
    client_handler::ClientHandler,
};

use tokio::{
    net::{ TcpListener, ToSocketAddrs },
    sync::{ broadcast, mpsc },
};

pub struct TcpServer {
    channel: ServerChannel,
    channel_model: ClientChannel,
}

impl Clone for ClientChannel {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: self.receiver.resubscribe(),
        }
    }
}

impl TcpServer {
    pub fn new() -> tokio::io::Result<Self> {
        let (to_server, from_clients) = mpsc::channel::<ClientPacket>(100);
        let (to_clients, from_server) = broadcast::channel::<ServerPacket>(100);

        let client_ch = ClientChannel { sender: to_server, receiver: from_server };
        let server_ch = ServerChannel { sender: to_clients, receiver: from_clients };

        Ok(Self {
            channel: server_ch,
            channel_model: client_ch,
        })
    }
}

impl Server for TcpServer {
    async fn serve(self, addr: impl ToSocketAddrs) -> tokio::io::Result<ServerHandle> {
        let listener = TcpListener::bind(addr).await?;
        let client_ch = self.channel_model.clone();

        let listener_t = tokio::spawn(async move {
            loop {
                let (socket, addr) = listener.accept().await.unwrap();
                println!("New connection from '{}'", &addr);

                let mut client = ClientHandler::new(socket, client_ch.clone());
                tokio::spawn(async move {
                    client.start().await
                });
            }
        });
        
        Ok(ServerHandle {
            channel: self.channel,
            tcp_server_thread: listener_t,
        })
    }
}
