use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "virtualos_rs")]
pub struct Cli {
    #[arg(short, long, default_value = "/var/lib/docklet")]
    pub base_dir: PathBuf,

    #[command(subcommand)]
    pub command: Commands,
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
    /// Remove a container
    Rm { id: String },
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
    },
}
