#![allow(unused)]
pub mod server;
pub mod protocol;
pub mod intercom;
pub mod client_handler;
pub mod connection;
pub mod test;

/// Server Ports ///
pub mod tcp;

pub mod result {
    use anyhow;
    
    pub type Result<T> = std::result::Result<T, anyhow::Error>;
}

pub const ADDR: &'static str = "127.0.0.1:9090";

use crate::{
    result::Result,
    server::{ Server },
    tcp::{ TcpServerPort },
};

#[tokio::main]
async fn main() -> Result<()> {
    Server::new()
        .add_port::<TcpServerPort>()
        .serve()
}

