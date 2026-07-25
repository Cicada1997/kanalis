pub mod adapters;
pub mod protocol;
pub mod client_handler;
pub mod server;

mod result {
    pub type Result<T> = tokio::io::Result<T>;
}

use crate::{
    result::Result,
    server::{ TcpServerHandle },
};

static ADDR: &'static str = "localhost:9090";

#[tokio::main]
async fn main() -> Result<()> {
    let mut server = TcpServerHandle::new()?;
    server.start(ADDR).await?;

    Ok(())
}

#[tokio::test]
async fn test() {
    use std::{str, time::Duration};
    use tokio::{
        net::TcpStream,
        io::{AsyncWriteExt, AsyncReadExt},
    };

    use crate::{
        protocol::{ServerPacket, ClientPacket},
        server::TcpServerHandle,
    };

    // 1. Spawn the server in the background so the socket has something to connect to
    tokio::spawn(async {
        let mut server = TcpServerHandle::new().expect("Failed to create server.");
        server.start(ADDR).await.expect("Server failed to start.");
    });

    // Allow the server a brief moment to bind to the port
    tokio::time::sleep(Duration::from_millis(100)).await;

    let mut socket = TcpStream::connect(ADDR).await.expect("Could not instantiate socket.");
    
    // Authenticate
    let packet = ClientPacket::AuthToken("1:UoSAmGCOqe16VCMc3woY8qSSqAskBhSH".to_string());
    let json = serde_json::to_string(&packet).expect("malformed component, unable to serialize to json.");
    socket.write_all((json + "\n").as_bytes()).await.expect("Failed to send packet.");

    tokio::time::sleep(Duration::from_millis(2_000)).await;

    // Send message (Fixed syntax error: replaced invalid field with user_id and channel_id u64 integers)
    let msg_content = "meddelande till allmänheten!".to_string();
    let packet = ClientPacket::Message {
        user_id: 1,
        channel_id: 1,
        content: msg_content.clone(),
    };

    let json = serde_json::to_string(&packet).expect("malformed component, unable to serialize to json.");
    socket.write_all((json + "\n").as_bytes()).await.expect("Failed to send packet.");

    let mut buffer = [0u8; 1024];
    let n = socket.read(&mut buffer).await.unwrap();
    let payload = str::from_utf8(&buffer[..n]).unwrap();

    let msg: ServerPacket = serde_json::from_str(payload).unwrap();

    let ServerPacket::NewMessage { content, .. } = msg;
    assert_eq!(msg_content, content);
}
