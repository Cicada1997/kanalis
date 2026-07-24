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

#[tokio::test(name = "client")]
async fn test() {
    use tokio::{
        net::{ TcpStream },
        io::{ AsyncWriteExt, AsyncReadExt },
    };

    use crate::{
        protocol::{ ServerPacket, ClientPacket },
    };

    let mut socket = TcpStream::connect(ADDR).await.expect("Could not instantiate socket.");
    
    let msg_content = "meddelande till allmänheten!".to_string();
    let packet = ClientPacket::Message {
        token: "tokeneroune".to_string(),
        content: msg_content.clone(),
    };

    let json = serde_json::to_string(&packet).expect("malformed component, unable to serialize to json.");
    socket.write_all((json + "\n").as_bytes()).await.expect("Failed to send packet.");

    let mut buffer = [0u8; 1024];

    // loop {
    let n = socket.read(&mut buffer).await.unwrap();
    let payload = str::from_utf8(&buffer[..n]).unwrap();

    let msg: ServerPacket = serde_json::from_str(payload).unwrap();

    let ServerPacket::NewMessage { content, .. } = msg;
    assert_eq!(msg_content, content);
}
