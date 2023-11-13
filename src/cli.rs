use std::net::SocketAddr;

use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    /// The address to listen on
    pub from_addr: SocketAddr,

    /// The address to forward to
    pub to_addr: Vec<SocketAddr>,

    /// Modifying script
    #[clap(short, long)]
    pub script: Option<String>,
}
