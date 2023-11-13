use clap::Parser;
use cli::Cli;
use testp::{start_proxy, Result};

mod cli;

fn main() -> Result<()> {
    let args = Cli::parse();
    let mut targets = vec![args.main_target_addr];
    targets.extend(args.secondary_target_addrs);

    start_proxy(args.listen_addr, targets)?;
    Ok(())
}
