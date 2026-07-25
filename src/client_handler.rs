use crate::{
    result::Result,
    adapters::{ ClientChannel },
    protocol::{ ClientPacket, UserDetails },
};

use reqwest;
use tokio::{
    io::{ AsyncBufReadExt, BufReader, Lines },
    io::{ AsyncWriteExt },
    net::{ TcpStream },
    net::tcp::{ OwnedWriteHalf, OwnedReadHalf },
};

pub struct ClientHandler {
    reader: Lines<BufReader<OwnedReadHalf>>,
    writer: OwnedWriteHalf,
    channel: ClientChannel,
    user: Option<UserDetails>
}

impl ClientHandler {
    pub fn new( socket: TcpStream, channel: ClientChannel ) -> Self {
        let (r, w) = socket.into_split();

        Self {
            reader: BufReader::new(r).lines(),
            writer: w,
            channel,
            user: None,
        }
    }

    fn jsonify(payload: &str) -> std::result::Result<ClientPacket, String> {
        let msg: ClientPacket = match serde_json::from_str(payload) {
            Ok(json) => json,
            Err(e) => {
                eprintln!("Invalid json in payload: {e}");
                eprintln!("Payload dump:\n {payload:?}");
                return Err(e.to_string());
            }
        };

        Ok(msg)
    }

    async fn forward(&self, packet: ClientPacket) {
        self.channel.sender.send(packet.clone()).await.unwrap();
    }

    pub async fn handle_unauthorized(&self, packet: ClientPacket) -> Option<UserDetails> {
        match packet {
            ClientPacket::AuthToken( token ) => {
                let client = reqwest::Client::new();
                let resp = match client.post("https://auth.kattmys.se/token-login")
                    .json(&token)
                    .send()
                    .await {
                    Ok(resp) => resp,
                    Err(_e) => return None,
                };

                if resp.status().is_success() {
                    return resp.json().await.ok();
                }

                None
            }
            
            _ => None
        }
    }

    pub async fn start(&mut self) -> Result<()> {
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

                    let Ok(packet) = Self::jsonify(&line) else { continue; };

                    if self.user.is_none() {
                        self.user = self.handle_unauthorized(packet).await;
                        continue;
                    };

                    self.forward(packet).await;
                }

                msg = self.channel.receiver.recv() => {
                    let msg = msg.unwrap();
                    let json = serde_json::to_string(&msg).expect("Struct cant be serialized to json (???).") + "\n";
                    if let Err(e) = self.writer.write_all(json.as_bytes()).await {
                        eprintln!("failed to write to socket; err = {:?}", e);
                        return Err(e);
                    }
                }
            }

        }
    }
}
