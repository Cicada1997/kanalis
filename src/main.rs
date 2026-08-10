pub mod server;
pub mod protocol;
pub mod intercom;
pub mod client_handler;
pub mod connection;
pub mod test;
pub mod ports;

pub mod result {
    use anyhow;
    
    pub type Result<T> = std::result::Result<T, anyhow::Error>;
}

// pub const ADDR: &str = "127.0.0.1:9090";
pub const ADDR: &str = "0.0.0.0:9090";

use crate::{
    result::Result,
    server::{ Server },
    ports::{
        tcp::{ TcpServerPort },
        websocket::{ WsServerPort },
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    Server::new()
        .add_port::<TcpServerPort>()
        .add_port::<WsServerPort>()
        .serve()
        .await
}

