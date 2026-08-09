use crate::{
    protocol::{ self, UserDetails, ServerPacket, ClientPacket },
    intercom::{ ClientChannel },
    connection::{ ClientConnection },
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

    async fn handle_unauthorized(&mut self, packet: ClientPacket) {
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
                            return
                        },
                    };

                if !resp.status().is_success() {
                    self.conn.send_error(protocol::Error::AuthFail, "Unable to authorize token.");
                    return
                }

                self.user = resp.json::<UserDetails>().await.ok();
                self.conn.send(ServerPacket::LoginSuccess);
            }

            _ => {
                self.conn.send_error(protocol::Error::Unauthorized, "Unauthorized.");
            }
        }
    }

    pub async fn start(&mut self) {
        loop {
            tokio::select! {
                packet = self.conn.recv() => {
                    let Some(packet) = packet else {
                        println!("conneciton closed");
                        return
                    };

                    if self.user.is_none() {
                        self.handle_unauthorized(packet).await;
                    } else {
                        self.channel.send(packet);
                    }
                }

                packet = self.channel.recv() => {
                    let packet = match packet {
                        Ok(packet) => packet,
                        Err(e) => {
                            eprintln!("receive error: {e}");
                            continue;
                        }
                    };

                    self.conn.send(packet);
                }
            }
        }
    }
}
