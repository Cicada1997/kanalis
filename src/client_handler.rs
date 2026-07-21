use crate::{
    adapters::{ ClientChannel },
    protocol::{ ClientPacket, /* ServerPacket */ },
};

use tokio::{
    io::{ AsyncBufReadExt, BufReader, Lines },
    net::{ TcpStream },
    net::tcp::{ OwnedWriteHalf, OwnedReadHalf },
};

pub struct ClientHandler {
    // socket: TcpStream,
    // reader: ,
    reader: Lines<BufReader<OwnedReadHalf>>,
    writer: OwnedWriteHalf,
    channel: ClientChannel,
}

// static BUFFER_SIZE: usize = 1024;

impl ClientHandler {
    pub fn new( socket: TcpStream, channel: ClientChannel ) -> Self {
        let (r, w) = socket.into_split();

        let reader = BufReader::new(r).lines();
        let writer = w;

        Self {
            reader,
            writer,
            channel,
        }
    }

    fn jsonify(payload: &str) -> std::result::Result<ClientPacket, String> {
        let msg: ClientPacket = match serde_json::from_str(payload) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Invalid json in payload: {e}.");
                eprintln!("Payload dump:\n {payload:?}");
                return Err(e.to_string());
            }
        };

        Ok(msg)
    }

    pub async fn start(&mut self) -> tokio::io::Result<()> {
        loop {
            tokio::select! {
                resp = self.reader.next_line() => {
                    let line = match resp {
                        Ok(Some(line)) => line,
                        Ok(None) => { // Disconnected
                            println!("Client disconnected");
                            return Ok(());
                        },
                        Err(e) => {
                            eprintln!("failed to read from socket; err = {:?}", e);
                            return Err(e);
                        }
                    };

                    let Ok(msg) = Self::jsonify(&line) else { continue; };

                    dbg!(&msg);
                    self.channel.sender.send(msg).await.expect("Broken client channel.");

                    // if let Err(e) = self.socket.write_all(&buf[0..n]).await {
                    //     eprintln!("failed to write to socket; err = {:?}", e);
                    //     return Err(e);
                    // }

                }
            }
        }
    }
}
