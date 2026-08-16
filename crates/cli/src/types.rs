use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "virtualos_rs")]
pub struct Cli {
    #[arg(short, long, default_value = "/var/lib/virtualos")]
    pub base_dir: PathBuf,

    #[command(subcommand)]
    pub command: Commands,

    /// Enable structured log output to a file (rotating)
    #[arg(long)]
    pub log_file: Option<PathBuf>,
    /// Send structured logs to a TCP endpoint (e.g., 127.0.0.1:5140)
    #[arg(long)]
    pub log_network: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Pull a container image
    Pull {
        reference: String,
        #[arg(short, long, default_value = "./data/store")]
        store_dir: PathBuf,
    },
    /// Create a container (without starting it)
    Create {
        #[arg(short, long)]
        id: Option<String>,

        image: String,
        command: String,

        #[arg(last = true)]
        args: Vec<String>,

        #[arg(short, long, default_value = "./data/store")]
        store_dir: PathBuf,

        #[arg(long)]
        memory: Option<String>,
        #[arg(long)]
        cpus: Option<f64>,
    },
    /// Start a created container
    Start {
        id: String,
        #[arg(short, long, default_value_t = false)]
        detach: bool,
    },
    /// Stop a running container
    Stop { id: String },
    /// get logs from running container
    Logs { id: String },
    /// Remove a container (use -f to force)
    #[command(alias = "rm")]
    Remove {
        id: String,
        /// Force removal (stop first if running)
        #[arg(short, long)]
        force: bool,
    },
    /// List containers
    Ps,
    /// Run a container (create + start, optionally foreground)
    Run {
        #[arg(short, long, default_value_t = false)]
        detach: bool,

        #[arg(short, long)]
        id: Option<String>,

        image: String,
        command: String,

        #[arg(last = true)]
        args: Vec<String>,

        #[arg(short, long, default_value = "./data/store")]
        store_dir: PathBuf,

        #[arg(long)]
        memory: Option<String>,
        #[arg(long)]
        cpus: Option<f64>,
        /// Automatically remove the container when it exits (foreground only)
        #[arg(long)]
        rm: bool,
        #[arg(short = 'i', long)]
        interactive: bool, // not yet fully used; for future PTY
        #[arg(short = 't', long)]
        tty: bool,
        #[arg(long)]
        vm: bool,
        /// Path to guest kernel image (required if --vm)
        #[arg(long, default_value = "/usr/share/virtualos/vmlinux.bin")]
        kernel: PathBuf,
        /// Path to rootfs image (squashfs/ext4)
        #[arg(long, default_value = "/usr/share/virtualos/rootfs.sqsh")]
        rootfs_image: PathBuf,
    },
    /// Initialise host bridge and NAT (run once)
    NetworkInit,
    /// Start a standalone metrics server (for local mode)
    Monitor {
        #[arg(long, default_value_t = 9090)]
        port: u16,
    },

    /// eBPF operations
    Ebpf {
        #[command(subcommand)]
        cmd: EbpfCmd,
    },
}

#[derive(Subcommand, Debug)]
pub enum EbpfCmd {
    /// Trace all execve calls system‑wide
    Trace,
}
