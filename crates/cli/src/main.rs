mod container;

use clap::{Parser, Subcommand};
use engine::{prepare_rootfs, pull_image};
use std::{path::PathBuf, process};
use storage::Store;

use crate::container::Container;

#[derive(Parser, Debug)]
#[command(name = "virtualos_rs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a container from a local rootfs
    Run {
        #[arg(short, long, default_value = "./rootfs")]
        rootfs: String,
        #[arg(short, long, default_value = "/bin/sh")]
        command: String,
        #[arg(last = true)]
        args: Vec<String>,
        /// Use an image reference (e.g., alpine:latest)
        #[arg(short, long)]
        image: Option<String>,
        #[arg(short, long, default_value = "./store")]
        store_dir: PathBuf,
    },
    /// Pull a container image
    Pull {
        /// Image reference (e.g., alpine:latest)
        reference: String,
        /// Store directory for layers (default: ./store)
        #[arg(short, long, default_value = "./store")]
        store_dir: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run {
            rootfs,
            command,
            args,
            image,
            store_dir,
        } => {
            let (root_path, _mounted) = if let Some(img) = image {
                let store = Store::new(store_dir);
                let mounted = prepare_rootfs(&img, &store).unwrap_or_else(|e| {
                    eprintln!("Failed to prepare rootfs: {:?}", e);
                    process::exit(1);
                });
                let path = mounted.root_path.clone();
                (path, Some(mounted)) // keep alive until after run
            } else {
                if rootfs.is_empty() {
                    eprintln!("Either --rootfs or --image must be provided");
                    process::exit(1);
                }
                (PathBuf::from(&rootfs), None)
            };
            Container::run(root_path.to_str().unwrap(), &command, &args);
        }
        Commands::Pull {
            reference,
            store_dir,
        } => {
            let store = Store::new(store_dir);
            if let Err(e) = pull_image(&reference, &store) {
                eprintln!("Error pulling image: {:?}", e);
                process::exit(1);
            }
        }
    }
}
