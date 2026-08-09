use std::sync::mpsc;
use std::net::SocketAddr;
use tokio::{
    net::TcpStream,
    sync::broadcast,
};
use crate::{
    protocol::{ self, User, UserDetails, ServerPacket, ClientPacket },
    intercom::{ ClientChannel },
    connection::{ PinBoxFuture, ClientConnection },
};

pub struct ClientHandler {
    user: Option<UserDetails>,
    conn: Box<dyn ClientConnection>,
    channel: ClientChannel,
}

impl ClientHandler {
    #[must_use]
    pub fn new(conn: Box<dyn ClientConnection>, channel: ClientChannel) -> Self {
        Self { user: None, conn, channel }
    }

    pub async fn start(&mut self) {
        'unauthorized: loop {
            let Some(packet) = self.conn.recv().await else {
                println!("conneciton closed");
                return
            };

            dbg!(&packet);
            
            match packet {
                ClientPacket::AuthToken( token ) => {
                    let client = reqwest::Client::new();
                    let resp = match client.post("https://auth.kattmys.se/token-login")
                        .json(&token)
                        .send()
                        .await {
                            Ok(resp) => resp,
                            Err(_e) => {
                                self.conn.send_error(protocol::Error::ConnectionError, "failed to contact auth-server.");
                                continue
                            },
                        };

                    if !resp.status().is_success() {
                        self.conn.send_error(protocol::Error::AuthFail, "Unable to authorize token.");
                        continue
                    }

                    self.user = resp.json::<UserDetails>().await.ok();
                    self.conn.send(ServerPacket::LoginSuccess);
                    break 'unauthorized
                }

                _ => {
                    self.conn.send_error(protocol::Error::Unauthorized, "Unauthorized.");
                }
            }
        }

        'authorized: loop {
            let Some(packet) = self.conn.recv().await else {
                println!("conneciton closed");
                return
            };

            // temproarily send all packets to the server without filter:
            self.channel.send(packet);
        }
    }
}
