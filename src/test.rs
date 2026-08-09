#[tokio::test]
async fn test() {
    use std::time::Duration;
    use std::io::Write;
    use tokio::{
        net::TcpStream,
        io::AsyncWriteExt,
    };

    use crate::{
        ADDR,
        result::Result,
        protocol::{ServerPacket, ClientPacket},
        server::{ Server },
        tcp::{ TcpServerPort },
    };

    let mut token = String::new();
    print!("Enter kattmys token to proceed: ");
    std::io::stdout().flush().unwrap();
    std::io::stdin().read_line(&mut token).unwrap();
    token = token.trim().to_string();

    println!("Spawning server process...");
    // 1. Spawn the server in the background
    tokio::spawn(async {
        Server::new()
            .add_port::<TcpServerPort>()
            .serve()
    });
    println!("Done!");

    // Allow the server a brief moment to bind to the port
    // tokio::time::sleep(Duration::from_millis(500)).await;

    println!("Establishing connection...");
    let (reader, mut writer) = TcpStream::connect(ADDR)
        .await
        .map(|r| dbg!(r))
        .expect("Could not instantiate socket.")
        .into_split();
    println!("Done!");

    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("Spawning listener thread...");
    // 2. Spawn the background reader task
    let thread = tokio::spawn(async move {
        use tokio::io::{AsyncBufReadExt, BufReader};

        // CRITICAL FIX: Instantiate BufReader outside the loop!
        // Otherwise, any extra bytes read into the internal buffer are destroyed on the next iteration.
        let mut lines = BufReader::new(reader).lines();

        let mut login_success = false;

        loop {
            let payload = lines
                .next_line()
                .await
                .expect("Failed to read from socket")
                .expect("Server closed socket prematurely");

            let msg: ServerPacket = serde_json::from_str(&payload).expect("Invalid JSON received");
            // dbg!(&payload);
            println!("{}", serde_json::to_string_pretty(&msg).unwrap());

            match msg {
                ServerPacket::Error { code, reason } => {
                    panic!("Server returned error {code}: {reason}");
                },
                ServerPacket::LoginSuccess => {
                    login_success = true;
                }
                ServerPacket::NewMessage { .. } => {
                    if !login_success {
                        panic!("did not receive authentication response but received the msg.");
                    }

                    break msg;
                }
            }
        }
    });

    println!("Done!");

    // 3. Authenticate
    println!("authenticating...");
    let auth_packet = ClientPacket::AuthToken(token.to_string());
    let json = serde_json::to_string(&auth_packet).expect("Failed to serialize auth packet");
    writer.write_all((json + "\n").as_bytes()).await.expect("Failed to send auth packet");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // 4. Send message
    let msg_content = "meddelande till allmänheten!".to_string();
    let msg_packet = ClientPacket::Message {
        user_id: 1,
        channel_id: 1,
        content: msg_content.clone(),
    };

    println!("sending packet...\ncontents: {msg_packet:?}");
    let json = serde_json::to_string(&msg_packet).expect("Failed to serialize message packet");
    writer.write_all((json + "\n").as_bytes()).await.expect("Failed to send message packet");

    println!("listening for echo of message...");

    // 5. Await response with a fail-fast timeout
    let msg = tokio::time::timeout(Duration::from_secs(3), thread)
        .await
        .expect("Test timed out! The server never echoed the message (likely auth rejection).")
        .expect("The background reader task panicked!");

    let ServerPacket::NewMessage { content, .. } = msg else { 
        panic!("Response was not of enum 'NewMessage'") 
    };
    assert_eq!(msg_content, content);
}
