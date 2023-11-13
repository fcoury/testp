use clap::Parser;
use cli::Cli;
use testp::{start_proxy, Result};

mod cli;

fn main() -> Result<()> {
    let args = Cli::parse();
    start_proxy(args.from_addr, args.to_addr)?;
    Ok(())
}
