use crate::{
    result::Result,
    server::{ ServerPort },
    client_handler::ClientHandler,
    protocol::{ ClientPacket, ServerPacket },
    connection::{ ClientConnection, PinBoxFuture },
    intercom::{ ClientChannel },
};

use anyhow::anyhow;
use std::{
    net::SocketAddr,
};

use tokio::{
    io::{ AsyncBufReadExt, BufReader },
    io::{ AsyncWriteExt },
    net::{ TcpStream, TcpListener },
    net::tcp::{ OwnedWriteHalf, OwnedReadHalf },
    sync::{ broadcast, mpsc },
};

pub struct TcpServerPort {
    client_channel: ClientChannel,
}

impl ServerPort for TcpServerPort {
    fn new(client_channel: ClientChannel) -> Result<Self> {
        Ok(Self { client_channel })
    }

    /// # Errors
    ///
    /// Will return `Err` if the address and port are occupied.
    async fn listen(&self) -> Result<()> {
        println!("Starting tcp port...");
        let port: u16 = dotenv::var("KANALIS_TCP_PORT")
            .map_err(|_e| anyhow!("environment variable 'KANALIS_TCP_PORT' is not set"))?
            .parse()
            .map_err(|_e| anyhow!("environment variable 'KANALIS_TCP_PORT' is not an valid port number (a 16 bit unsigned integer)"))?;

        let ip: std::net::IpAddr = dotenv::var("KANALIS_ADDR")
            .map_err(|_e| anyhow!("environment variable 'KANALIS_ADDR' is not set"))?
            .parse()
            .map_err(|_e| anyhow!("KANALIS_ADDR is not a valid IP address"))?;

        let addr = std::net::SocketAddr::new(ip, port);

        let listener = TcpListener::bind(addr).await?;
        println!("listening for raw tcp socket on {addr}...");

        loop {
            let Ok((socket, client_addr)) = listener
                .accept()
                .await
                .inspect_err(|e| eprintln!("failed to establish client connection: {e}")) 
                else { continue };

            println!("New connection tcp: {client_addr}");

            let conn = ClientTcpConnection::new(socket, client_addr);
            let channel = self.client_channel.clone();

            tokio::spawn(async move {
                let mut handler = ClientHandler::new(conn, channel);
                handler.start().await;
            });
        }
    }

    fn name() -> String {
        String::from("Tcp Server")
    }
}

pub struct ClientTcpConnection {
    reader: broadcast::Receiver<Option<String>>,
    sender: mpsc::UnboundedSender<ServerPacket>,
    addr: SocketAddr,
}

impl ClientTcpConnection {
    pub fn new(socket: TcpStream, addr: SocketAddr) -> Self {
        let (sender, recv) = mpsc::unbounded_channel();
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

    fn sender(&self) -> mpsc::UnboundedSender<ServerPacket> {
        self.sender.clone()
    }

    fn send(&mut self, packet: ServerPacket) {
        let _ = self.sender.send(packet);
    }

    fn client_id(&self) -> String {
        self.addr.to_string()
    }
}

async fn from_client(channel: broadcast::Sender<Option<String>>, reader: OwnedReadHalf) {
    // todo!("handle incoming client messages")
    let mut reader = BufReader::new(reader).lines();

    loop {
        let json_str = match reader.next_line().await {
            Ok(str) => str,
            Err(e) => {
                eprintln!("Socket read error, closing connection: {e}");
                break;
            }
        };

        if let Err(e) = channel.send(json_str.clone()) {
            eprintln!("unable to forward string {json_str:?}: {e}");
            break;
        }
    }

    // TODO: ensure connection is closed
    let _ = channel.send(None);
}

async fn to_client(mut channel: mpsc::UnboundedReceiver<ServerPacket>, mut writer: OwnedWriteHalf) {
    // todo!("handle scheduled messages to the client")
    loop {
        let Some(packet) = channel.recv().await else {
            // eprintln!("");
            continue
        };
        let Ok(json_str) = serde_json::to_string(&packet) else {
            eprintln!("Unable to serialize packet: {packet:?}");
            continue;
        };

        let _ = writer.write_all((json_str + "\n").as_bytes())
            .await
            .inspect_err(|e| eprintln!("unable to send packet {packet:?}: {e}"));
    }
}
