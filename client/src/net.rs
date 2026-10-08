use protocol::{ ClientPacket, ServerPacket };

use tokio::sync::mpsc;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;

use iced::futures::sink::SinkExt;
use iced::stream;
use iced::Subscription;

#[derive(Debug, Clone)]
pub enum ExitType {
    SocketClosed,
    UndefinedError(String),
}

#[derive(Debug, Clone)]
pub enum ConnEvent {
    Connecting,
    Connected(mpsc::Sender<ClientPacket>),
    Disconnected,
    Packet(ServerPacket),
    Exit(ExitType),
}


pub async fn send_packet(writer: &mut OwnedWriteHalf, packet: &ClientPacket) {
    if let Ok(json_str) = serde_json::to_string(packet)
        .inspect_err(|_e| eprintln!("tried to send malformed ClientPacket: {packet:?}")) {
        let _ = writer.write_all((json_str + "\n").as_bytes())
            .await
            .inspect_err(|e| eprintln!("Error in writer: {e:?}"));
    }
}

pub fn connect(token: String) -> Subscription<crate::ui::Message> {
    Subscription::run_with(
        token,
        |token_ref: &String| {
            let addr = dotenv::var("HOST").expect("The environment variable 'HOST' must be set to an correct ip or url to connect to the chatserver");
            let token_owned = token_ref.clone();

            stream::channel(100, async move |mut output| {
                println!("Establishing connection to {addr}...");

                let stream = match TcpStream::connect(addr).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = output.send(ConnEvent::Exit(ExitType::UndefinedError(e.to_string()))).await;
                        return;
                    }
                };

                let (reader, mut writer) = stream.into_split();                  // tcp over network
                let (sender, mut receiver) = mpsc::channel::<ClientPacket>(100); // internal

                let _ = output.send(ConnEvent::Connected(sender)).await;

                send_packet(&mut writer, &ClientPacket::AuthToken(token_owned)).await;

                let mut reader = BufReader::new(reader).lines();

                loop {
                    tokio::select! {
                        line = reader.next_line() => {
                            if !handle_packet(&mut output, line).await {
                                break;
                            }
                        }

                        packet = receiver.recv() => {
                            let Some(packet) = packet else { break };
                            send_packet(&mut writer, &packet).await;
                        }
                    }
                }
            })
        }
    ).map(crate::ui::Message::ConnMessage)
}

pub async fn handle_packet(
    output: &mut iced::futures::channel::mpsc::Sender<ConnEvent>,
    line: std::io::Result<Option<String>>,
) -> bool {
    match line {
        Ok(Some(json_str)) => {
            if let Ok(packet) = serde_json::from_str::<ServerPacket>(&json_str) {
                let _ = output.send(ConnEvent::Packet(packet)).await;
            } else {
                eprintln!("Bad JSON: {json_str}");
            }
        }

        Ok(None) => {
            let _ = output.send(ConnEvent::Exit(ExitType::SocketClosed)).await;
            return false;
        }
        Err(e) => {
            let _ = output.send(ConnEvent::Exit(ExitType::UndefinedError(e.to_string()))).await;
            return false;
        }
    }

    true
}
