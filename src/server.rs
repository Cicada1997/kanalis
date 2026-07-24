use crate::{
    result::Result,
    adapters::{ ServerHandler, ClientChannel, ServerChannel },
    protocol::{ ClientPacket, ServerPacket, User },
    client_handler::{ ClientHandler },
};

use tokio::{
    net::{ TcpListener },
    sync::{ broadcast, mpsc },
};

pub struct TcpServerHandle {
    channel: ServerChannel,
    client_channel_model: ClientChannel,
}

async fn client_dispatch(addr: &'static str, client_channel_model: ClientChannel) {
    let listener = TcpListener::bind(addr).await.unwrap();
    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        println!("New connection from '{}'", &addr);
        let mut client = ClientHandler::new(socket, client_channel_model.clone());
        tokio::spawn(async move { client.start().await });
    }
}

impl TcpServerHandle {
    async fn recv_from_client(&mut self) -> Option<ClientPacket> {
        self.channel.receiver.recv().await
    }

    fn broadcast(&self, packet: ServerPacket) {
        self.channel.sender.send(packet.clone()).expect(&format!("Failed to broadcast message {:?}", packet));
    }

    pub fn new() -> Result<Self> {
        let (to_server, from_clients) = mpsc::channel::<ClientPacket>(100);
        let (to_clients, from_server) = broadcast::channel::<ServerPacket>(100);

        let client_ch = ClientChannel { sender: to_server, receiver: from_server };
        let server_ch = ServerChannel { sender: to_clients, receiver: from_clients };

        Ok(Self {
            channel: server_ch,
            client_channel_model: client_ch,
        })
    }

    pub async fn start(&mut self, addr: &'static str) -> Result<()> {
        let client_channel = self.client_channel_model.clone();
        tokio::spawn(async move {
            client_dispatch(addr, client_channel).await;
        });

        loop {
            let packet = self.recv_from_client().await;
            let Some(packet) = packet else {
                println!("Client fucked up channel communication. closing drastically...");
                continue;
            };

            self.handle_packet(packet).await?; // .expect("ClientPacket handeler crashed!");
        }
    }

    pub async fn handle_packet(&self, packet: ClientPacket) -> Result<()> {
        match packet {
            ClientPacket::Message { content, .. } => {
                self.broadcast(ServerPacket::NewMessage {
                    user: User { name: String::from("Okänd Användare") },
                    content,
                });
            }


        }

        Ok(())
    }
}

impl ServerHandler for TcpServerHandle { }
