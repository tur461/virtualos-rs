use clap::{Parser, Subcommand};
use engine::ContainerManager;
use std::{path::PathBuf, process};
use storage::Store;

#[derive(Parser, Debug)]
#[command(name = "virtualos_rs")]
struct Cli {
    #[arg(short, long, default_value = "/var/lib/docklet")]
    base_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
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
    },
    /// Start a created container
    Start { id: String },
    /// Stop a running container
    Stop { id: String },
    /// Remove a container
    Rm { id: String },
    /// List containers
    Ps,
    /// Run a container (create + start, optionally foreground)
    Run {
        #[arg(short, long)]
        detach: bool,

        #[arg(short, long)]
        id: Option<String>,

        image: String,
        command: String,

        #[arg(last = true)]
        args: Vec<String>,

        #[arg(short, long, default_value = "./data/store")]
        store_dir: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let mgr = ContainerManager::new(&cli.base_dir);
    match cli.command {
        Commands::Pull {
            reference,
            store_dir,
        } => {
            let store = Store::new(store_dir);
            if let Err(e) = engine::pull_image(&reference, &store) {
                eprintln!("Error pulling image: {:?}", e);
                process::exit(1);
            }
        }

        Commands::Create {
            id,
            image,
            command,
            args,
            store_dir,
        } => {
            let store = Store::new(store_dir);
            if let Err(e) = mgr.create(id, &image, &command, args, &store) {
                eprintln!("Create error: {:?}", e);
                process::exit(1);
            }
        }

        Commands::Start { id } => {
            if let Err(e) = mgr.start(&id) {
                eprintln!("Start error: {:?}", e);
                process::exit(1);
            }
        }

        Commands::Stop { id } => {
            if let Err(e) = mgr.stop(&id) {
                eprintln!("Stop error: {:?}", e);
                process::exit(1);
            }
        }

        Commands::Rm { id } => {
            if let Err(e) = mgr.delete(&id) {
                eprintln!("Remove error: {:?}", e);
                process::exit(1);
            }
        }

        Commands::Ps => match mgr.list() {
            Ok(containers) => {
                if containers.is_empty() {
                    println!("No containers found.");
                } else {
                    for c in containers {
                        println!("{:<12} {:<10?} {:<20} {:?}", c.id, c.status, c.image, c.pid);
                    }
                }
            }
            Err(e) => {
                eprintln!("List error: {:?}", e);
                process::exit(1);
            }
        },

        Commands::Run {
            detach,
            id,
            image,
            command,
            args,
            store_dir,
        } => {
            let store = Store::new(store_dir);
            // Create container
            let container = match mgr.create(id, &image, &command, args, &store) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Run create error: {:?}", e);
                    process::exit(1);
                }
            };
            // Start it
            if let Err(e) = mgr.start(&container.id) {
                eprintln!("Run start error: {:?}", e);
                // Cleanup the container we just created
                let _ = mgr.delete(&container.id);
                process::exit(1);
            }
            if !detach {
                // Foreground: attach to the container by waiting for its PID to exit
                if let Some(pid) = container.pid {
                    let pid = nix::unistd::Pid::from_raw(pid);
                    loop {
                        match nix::sys::wait::waitpid(
                            pid,
                            Some(nix::sys::wait::WaitPidFlag::WUNTRACED),
                        ) {
                            Ok(status) => {
                                eprintln!("Container init process exited with {:?}", status);
                                break;
                            }
                            Err(nix::Error::EINTR) => continue,
                            Err(e) => {
                                eprintln!("waitpid error: {}", e);
                                process::exit(1);
                            }
                        }
                    }
                }
            }
        }
    }
}
