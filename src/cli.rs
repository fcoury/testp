use std::net::SocketAddr;

use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    /// The address to listen on
    pub listen_addr: SocketAddr,

    /// Main target address
    pub main_target_addr: SocketAddr,

    /// Additional target addresses
    pub secondary_target_addrs: Vec<SocketAddr>,

    /// Modifying script
    #[clap(short, long)]
    pub script: Option<String>,
}
