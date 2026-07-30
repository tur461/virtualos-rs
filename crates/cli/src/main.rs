mod cmd;
mod helpers;
mod types;

use anyhow::Result;
use clap::Parser;
use daemon::client::Client;

use types::Cli;

use crate::cmd::{run_local, run_with_client};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match Client::connect().await? {
        Some(mut client) => run_with_client(cli, &mut client).await,
        None => run_local(cli),
    }
}
