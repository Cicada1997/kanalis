use crate::{
    ADDR,
    result::Result,
    server::{ ServerPort },
    client_handler::ClientHandler,
    protocol::{ ClientPacket, ServerPacket },
    connection::{ ClientConnection, PinBoxFuture },
    intercom::{ ClientChannel },
};

use std::{
    pin::Pin,
    future::Future,
    sync::mpsc,
    net::SocketAddr,
};

use tokio::{
    io::{ AsyncBufReadExt, BufReader, Lines },
    io::{ AsyncWriteExt },
    net::{ TcpStream, TcpListener },
    net::tcp::{ OwnedWriteHalf, OwnedReadHalf },
    sync::{ broadcast }
};

pub struct TcpServerPort {
    client_channel: ClientChannel,
}

impl ServerPort for TcpServerPort {
    fn new(client_channel: ClientChannel) -> Self {
        Self { client_channel }
    }

    /// # Errors
    ///
    /// Will return `Err` if the address and port are occupied.
    async fn listen(&self) -> Result<()> {
        println!("Starting tcp port...");
        let mut listener = TcpListener::bind(ADDR).await?;
        println!("listening for raw tcp socket on {ADDR}...");

        loop {
            let Ok((socket, addr)) = listener
                .accept()
                .await
                .inspect_err(|e| eprintln!("failed to establish client connection: {e}")) 
                else { continue };

            println!("New connection tcp: {addr}");

            let conn = ClientTcpConnection::new(socket, addr);
            let channel = self.client_channel.clone();

            tokio::spawn(async move {
                let mut handler = ClientHandler::new(Box::new(conn), channel);
                handler.start().await;
            });
        }
    }
}

pub struct ClientTcpConnection {
    reader: broadcast::Receiver<Option<String>>,
    sender: mpsc::Sender<ServerPacket>,
    addr: SocketAddr,
}

impl ClientTcpConnection {
    pub fn new(socket: TcpStream, addr: SocketAddr) -> Self {
        let (sender, recv) = mpsc::channel();
        let (send, reader) = broadcast::channel(100);

        let (tcp_rx, tcp_tx) = socket.into_split();

        tokio::spawn(async move { from_client(send, tcp_rx).await; });
        tokio::spawn(async move { to_client(recv, tcp_tx).await; });

        Self { reader, sender, addr }
    }
}

impl ClientConnection for ClientTcpConnection {
    fn recv(&mut self) -> PinBoxFuture<'_, Option<ClientPacket>> {
        Box::pin(async move {
            let Ok(Some(json_str)) = self.reader.recv().await else { return None };
            serde_json::from_str::<Option<ClientPacket>>(&json_str).ok().flatten()
        })
    }

    fn send(&mut self, packet: ServerPacket) {
        self.sender.send(packet);
    }

    fn client_id(&self) -> String {
        self.addr.to_string()
    }
}

pub async fn from_client(channel: broadcast::Sender<Option<String>>, reader: OwnedReadHalf) {
    // todo!("handle incoming client messages")
    let mut reader = BufReader::new(reader).lines();

    loop {
        let json_str = match reader.next_line().await {
            Ok(str) => str,
            Err(e) => {
                eprintln!("Socket read error, closing connection: e");
                break;
            }
        };

        channel.send(json_str.clone())
            .inspect_err(|e| eprintln!("unable to forward string {json_str:?}: {e}"));
            // .is_err() { continue }
    }

    // TODO: ensure connection is closed
    let _ = channel.send(None);
}

pub async fn to_client(channel: mpsc::Receiver<ServerPacket>, mut writer: OwnedWriteHalf) {
    // todo!("handle scheduled messages to the client")
    loop {
        let Ok(packet) = channel.recv() else {
            // eprintln!("");
            continue
        };
        let Ok(json_str) = serde_json::to_string(&packet) else {
            eprintln!("Unable to serialize packet: {packet:?}");
            continue;
        };

        writer.write_all((json_str + "\n").as_bytes())
            .await
            .inspect_err(|e| eprintln!("unable to send packet {packet:?}: {e}"));
    }
}
