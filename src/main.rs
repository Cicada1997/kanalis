pub mod adapters;
pub mod protocol;
pub mod client_handler;
pub mod server;

use crate::{
    adapters::{ Server },
    server::TcpServer,
};

static ADDR: &'static str = "localhost:9090";

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let mut server_handle = TcpServer::new()?
        .serve(ADDR).await?;

    loop {
        tokio::select! {
            packet = server_handle.channel.receiver.recv() => {
                let Some(packet) = packet else {
                    println!("Client fucked up channel communication. closing drastically...");
                    break;
                };
            }
        }
    }

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
    let packet = ClientPacket::Message {
        token: "tokeneroune".to_string(),
        content: "meddelande till allmänheten!".to_string(),
    };
    let json = serde_json::to_string(&packet).expect("malformed component, unable to serialize to json.");
    socket.write_all((json + "\n").as_bytes()).await.expect("Failed to send packet.");

    let mut buffer = [0u8; 1024];

    loop {
        let n = socket.read(&mut buffer).await.unwrap();
        dbg!(&n);
        let payload = match str::from_utf8(&buffer[..n]) {
            Ok(payload) => payload,
            Err(e) => {
                eprintln!("Invalid utf_8: {e}.");
                eprintln!("Buffer dump:\n {buffer:?}");
                continue;
            }
        };

        let msg: ServerPacket = match serde_json::from_str(payload) {
            Ok(payload) => payload,
            Err(e) => {
                eprintln!("Invalid json in payload: {e}.");
                eprintln!("Payload dump:\n {payload:?}");
                continue;
            }
        };

        dbg!(&msg);
    }
}
