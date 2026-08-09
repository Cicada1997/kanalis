use crate::{
    ADDR,
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
    ws.on_upgrade(move |socket: WebSocket| async move {
        let conn = ClientWsConnection::new(socket, addr);
        ClientHandler::new(Box::new(conn), channel)
            .start()
            .await;
    })
}

impl ServerPort for WsServerPort {
    fn new(client_channel: ClientChannel) -> Result<Self> {
        let port: u16 = dotenv::var("WS_PORT")
            .map_err(|_e| anyhow!("environment variable 'WS_PORT' is not set"))?
            .parse()
            .map_err(|_e| anyhow!("environment variable 'WS_PORT' is not an valid port number (a 16 bit unsigned integer)"))?;

        let mut addr: SocketAddr = ADDR.parse()
            .map_err(|_e| anyhow!("ADDR is not a valid socket address"))?;

        addr.set_port(port);

        Ok(Self { client_channel, addr })
    }

    async fn listen(&self) -> Result<()> {
        println!("Starting {} on {}", Self::name(), self.addr);
        let app = Router::new()
            .route( "/ws", any(websocket_handler) )
            .with_state(self.client_channel.clone());

        let listener = tokio::net::TcpListener::bind(&self.addr).await?;

        axum::serve(listener, app)
            .await?;

        Ok(())
    }

    fn name() -> String {
        String::from("Websocket Server")
    }
}

pub struct ClientWsConnection {
    reader: broadcast::Receiver<Option<String>>,
    sender: mpsc::UnboundedSender<ServerPacket>,
    addr: SocketAddr,
}

impl ClientWsConnection {
    pub fn new(socket: WebSocket, addr: SocketAddr) -> Self {
        let (sender, recv) = mpsc::unbounded_channel();
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
            let Ok(Some(json_str)) = self.reader.recv().await else { return None };
            serde_json::from_str::<Option<ClientPacket>>(&json_str).ok().flatten()
        })
    }

    fn send(&mut self, packet: ServerPacket) {
        let _ = self.sender.send(packet);
    }

    fn client_id(&self) -> String {
        self.addr.to_string()
    }
}

async fn from_client(channel: broadcast::Sender<Option<String>>, mut reader: SplitStream<WebSocket>) {
    while let Some(Ok(Message::Text(msg))) = reader.next().await {
        let _ = channel.send(Some(msg.to_string()))
            .inspect_err(|e| eprintln!("failed to forward message: {e}"));
    }

    let _ = channel.send(None);
}

async fn to_client(mut channel: mpsc::UnboundedReceiver<ServerPacket>, mut writer: SplitSink<WebSocket, Message>) {
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
