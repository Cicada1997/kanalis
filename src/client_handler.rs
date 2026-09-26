use crate::{
    protocol::{ self, UserDetails, ServerPacket, ClientPacket },
    intercom::{ self, ClientChannel },
    connection::{ ClientConnection },
};

pub struct ClientHandler<C: ClientConnection> {
    user: Option<UserDetails>,
    conn: C, //Box<dyn ClientConnection>,
    channel: ClientChannel,
}

impl<C: ClientConnection> ClientHandler<C> {
    #[must_use]
    pub const fn new(conn: C, channel: ClientChannel) -> Self {
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
                if let Some(user) = self.user.clone() {
                    // let (sender, response_ch) = oneshot::channel();

                    self.channel.send(intercom::Request {
                        packet: ClientPacket::JustConnected(self.channel.get_sender()),
                        user: user.clone(),
                    });

                    self.conn.send(ServerPacket::LoginSuccess { user });
                } else {
                    self.conn.send_error(protocol::Error::AuthFail, "invalid json in response from auth server.");
                }
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
                        println!("connection closed");
                        return
                    };

                    if let Some(user) = &self.user {
                        self.channel.send(intercom::Request {
                            packet,
                            user: user.clone(),
                        });

                    } else {
                        self.handle_unauthorized(packet).await;
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
