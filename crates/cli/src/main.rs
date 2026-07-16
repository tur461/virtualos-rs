mod cmd;
mod helpers;
mod types;

use clap::Parser;
use engine::ContainerManager;

use types::Cli;

use crate::cmd::handle_cmd;

fn main() {
    let cli = Cli::parse();
    let mgr = ContainerManager::new(&cli.base_dir, &cli.cgroup_parent);
    if let Err(e) = handle_cmd(cli.command, mgr) {
        eprintln!("Error running the command: {e}");
    }
}
