use crate::{
    result::Result,
    adapters::{ ClientChannel },
    protocol::{ ServerPacket, ClientPacket, UserDetails, Error },
};

use reqwest;
use tokio::{
    io::{ AsyncBufReadExt, BufReader, Lines },
    io::{ AsyncWriteExt },
    net::{ TcpStream },
    net::tcp::{ OwnedWriteHalf, OwnedReadHalf },
    sync::mpsc::{ self },
};

#[derive(Debug, Clone)]
pub struct ClientConn {
    inner: mpsc::Sender<ServerPacket>,
}

impl ClientConn {
    pub fn new(mut writer: OwnedWriteHalf) -> Self {
        let (queue_writer, mut queue_reader) = mpsc::channel::<ServerPacket>(100);
        tokio::spawn(async move {
            loop {
                let packet = queue_reader.recv().await;
                let json = serde_json::to_string(&packet).expect("Struct cant be serialized to json (???).") + "\n";
                let err = writer.write_all(json.as_bytes()).await.err();
                eprintln!("{err:?}");
            }
        });

        Self {
            inner: queue_writer,
        }
    }

    pub fn send(&self, packet: ServerPacket) {
        let sender = self.inner.clone();
        tokio::task::spawn_blocking(async move || {
            sender.send(packet).await;
        });
    }
}

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

    pub async fn handle_unauthorized(&mut self, packet: ClientPacket) -> Result<()> {
        match packet {
            ClientPacket::AuthToken( token ) => {
                let client = reqwest::Client::new();
                let resp = match client.post("https://auth.kattmys.se/token-login")
                    .json(&token)
                    .send()
                    .await {
                    Ok(resp) => resp,
                    Err(_e) => {
                        self.send_error(Error::ConnectionError, "failed to contact auth-server.").await?;
                        return Ok(())
                    },
                };

                if resp.status().is_success() {
                    self.user = resp.json::<UserDetails>().await.ok();
                    self.send(ServerPacket::LoginSuccess).await?;
                } else {
                    self.send_error(Error::AuthFail, "Unable to authorize token.").await?;
                }
            }
            _ => {
                self.send_error(Error::Unauthorized, "Unauthorized.").await?;
            }
        }

        Ok(())
    }

    pub async fn send_error(&mut self, code: Error, reason: &str) -> Result<()> {
        self.send(ServerPacket::Error { code, reason: reason.to_string() }).await?;
        Ok(())
    }

    pub async fn send(&mut self, packet: ServerPacket) -> Result<()> {
        let json = serde_json::to_string(&packet).expect("Struct cant be serialized to json (???).") + "\n";
        if let Err(e) = self.writer.write_all(json.as_bytes()).await {
            eprintln!("failed to write to socket; err = {:?}", e);
            return Err(e);
        }

        Ok(())
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

                    match self.user {
                        Some(_) => self.forward(packet).await,
                        None => self.handle_unauthorized(packet).await?,
                    };

                }

                msg = self.channel.receiver.recv() => {
                    let msg = msg.unwrap();
                    if let Err(e) = self.send(msg).await {
                        eprintln!("failed to write broadcast to socket: {:?}", e);
                        return Err(e);
                    }
                }
            }

        }
    }
}
