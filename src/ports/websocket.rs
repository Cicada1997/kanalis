use crate::{
    result::Result,
    connection::{ ClientConnection, PinBoxFuture },
    server::{ ServerPort },
    client_handler::ClientHandler,
    protocol::{ ClientPacket, ServerPacket },
    intercom::{ ClientChannel },
};

use anyhow::anyhow;
use std::net::SocketAddr;
use axum::{
    Router,
    routing::any,

    extract::{ State, ConnectInfo },
    extract::ws::{
        Message,
        WebSocket,
        WebSocketUpgrade,
    },
};

use futures_util::{
    sink::SinkExt,
    stream::{ StreamExt, SplitSink, SplitStream },
};

use tokio::{
    sync::{ broadcast, mpsc },
};

pub struct WsServerPort {
    client_channel: ClientChannel,
    addr: SocketAddr,
}

async fn websocket_handler(ws: WebSocketUpgrade, ConnectInfo(addr): ConnectInfo<SocketAddr>, State(channel): State<ClientChannel>) -> impl axum::response::IntoResponse {
    println!("New connection websocket: {addr}");
    ws.on_upgrade(move |socket: WebSocket| async move {
        let conn = ClientWsConnection::new(socket, addr);
        ClientHandler::new(conn, channel)
            .start()
            .await;
    })
}

impl ServerPort for WsServerPort {
    fn new(client_channel: ClientChannel) -> Result<Self> {
        let port: u16 = dotenv::var("KANALIS_WS_PORT")
            .map_err(|_e| anyhow!("environment variable 'KANALIS_WS_PORT' is not set"))?
            .parse()
            .map_err(|_e| anyhow!("environment variable 'KANALIS_WS_PORT' is not an valid port number (a 16 bit unsigned integer)"))?;

        let ip: std::net::IpAddr = dotenv::var("KANALIS_ADDR")
            .map_err(|_e| anyhow!("environment variable 'KANALIS_ADDR' is not set"))?
            .parse()
            .map_err(|_e| anyhow!("KANALIS_ADDR is not a valid IP address"))?;

        let addr = std::net::SocketAddr::new(ip, port);

        Ok(Self { client_channel, addr })
    }

    async fn listen(&self) -> Result<()> {
        println!("Starting {} on {}", Self::name(), self.addr);
        let app = Router::new()
            .route( "/ws", any(websocket_handler) )
            .with_state(self.client_channel.clone());

        let listener = tokio::net::TcpListener::bind(&self.addr).await?;

        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        ).await?;

        Ok(())
    }

    fn name() -> String {
        String::from("Websocket Server")
    }
}

pub struct ClientWsConnection {
    reader: broadcast::Receiver<Option<String>>,
    sender: mpsc::Sender<ServerPacket>,
    addr: SocketAddr,
}

impl ClientWsConnection {
    pub fn new(socket: WebSocket, addr: SocketAddr) -> Self {
        let (sender, recv) = mpsc::channel(128);
        let (send, reader) = broadcast::channel(100);

        let (tcp_tx, tcp_rx) = socket.split();

        tokio::spawn(async move { from_client(send, tcp_rx).await; });
        tokio::spawn(async move { to_client(recv, tcp_tx).await; });

        Self { reader, sender, addr}
    }
}

impl ClientConnection for ClientWsConnection {
    fn recv(&mut self) -> PinBoxFuture<'_, Option<ClientPacket>> {
        Box::pin(async move {
            let response = self.reader.recv().await;
            let json_str = match response {
                Ok(Some(json_str)) => json_str,
                Ok(None) => {
                    eprintln!("recieved empty packet from client.");
                    return None
                },
                Err(e) => {
                    eprintln!("recieved error from websocket client: {e:?}");
                    return None
                }
            };

            serde_json::from_str::<Option<ClientPacket>>(&json_str)
                .inspect_err(|e| eprintln!("malformed packet from client: {e:?}"))
                .ok()
                .flatten()
        })
    }

    fn sender(&self) -> mpsc::Sender<ServerPacket> {
        self.sender.clone()
    }

    fn send(&mut self, packet: ServerPacket) -> PinBoxFuture<'_, ()> {
        Box::pin(async move {
            let _ = self.sender.send(packet).await;
        })
    }

    fn client_id(&self) -> String {
        self.addr.to_string()
    }
}

async fn from_client(channel: broadcast::Sender<Option<String>>, mut reader: SplitStream<WebSocket>) {
    while let Some(Ok(Message::Text(msg))) = reader.next().await {
        if let Err(e) = channel.send(Some(msg.to_string())) {
            eprintln!("failed to forward message: {e}");
            return;
        }
    }

    let _ = channel.send(None);
}

async fn to_client(mut channel: mpsc::Receiver<ServerPacket>, mut writer: SplitSink<WebSocket, Message>) {
    while let Some(packet) = channel.recv().await {
        let Ok(json_str) = serde_json::to_string(&packet) else {
            eprintln!("Unable to serialize packet: {packet:?}");
            continue;
        };

        if let Err(e) = writer.send(Message::Text(json_str.into())).await {
            eprintln!("unable to send packet {packet:?}: {e}");
            break;
        }
    }
}
