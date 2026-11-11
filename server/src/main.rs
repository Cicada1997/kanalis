pub mod server;
pub mod intercom;
pub mod client_handler;
pub mod connection;
pub mod packet_handling;
pub mod ports;
pub mod db;

pub mod error {
    use anyhow;
    pub type Error = anyhow::Error;
}
pub mod result {
    pub type Result<T> = std::result::Result<T, crate::error::Error>;
}

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
    if std::env::args().any(|x| x == "--protocol") {
        protocol::print_protocol();
        return Ok(());
    }

    dotenv::dotenv().ok();

    Server::new()
        .add_port::<TcpServerPort>()
        .add_port::<WsServerPort>()
        .serve()
        .await
}

