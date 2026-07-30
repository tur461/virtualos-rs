use clap::Parser;
use proto::virtualos::virtual_os_server::VirtualOsServer;
use std::path::PathBuf;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::Server;

use daemon::MyVirtualOs;

#[derive(Parser, Debug)]
#[command(name = "virtualos-daemon")]
struct DaemonArgs {
    /// Path to Unix socket
    #[arg(long, default_value = "/var/run/virtualos.sock")]
    socket: PathBuf,
    /// Base directory for container state
    #[arg(long, default_value = "/var/lib/virtualos")]
    base_dir: PathBuf,
    /// Cgroup parent
    #[arg(long, default_value = "/sys/fs/cgroup/virtualos")]
    cgroup_parent: PathBuf,
    /// Structured log file (rotating)
    #[arg(long)]
    log_file: Option<PathBuf>,
    /// TCP log sink address
    #[arg(long)]
    log_network: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = DaemonArgs::parse();
    logging::init_logging(true, args.log_file, args.log_network.as_deref())?;

    let socket_path = "/var/run/virtualos.sock";

    let _ = std::fs::remove_file(socket_path);

    let listener = UnixListener::bind(socket_path)?;
    let incoming = UnixListenerStream::new(listener);

    println!("Daemon listening on {}", socket_path);

    Server::builder()
        .add_service(VirtualOsServer::new(MyVirtualOs::default()))
        .serve_with_incoming_shutdown(incoming, shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.ok();
    println!("Shutting down");
}
