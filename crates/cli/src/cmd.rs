use anyhow::{Context, Result};
use engine::{ContainerManager, ResourceLimits};
use std::process;
use storage::Store;

use crate::helpers::parse_memory;

use super::types::Commands;

pub fn handle_cmd(cmd: Commands, mgr: ContainerManager) -> Result<()> {
    match cmd {
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
            memory,
            cpus,
        } => {
            let store = Store::new(store_dir);
            let mem_limit = match memory {
                Some(s) => Some(parse_memory(&s).context("invalid memory value")?),
                None => None,
            };
            let limits = ResourceLimits {
                memory: mem_limit,
                cpus,
            };
            if let Err(e) = mgr.create(id, &image, &command, args, &store, limits) {
                eprintln!("Create error: {:?}", e);
                process::exit(1);
            }
        }

        Commands::Start { id, detach } => {
            if let Err(e) = mgr.start(&id, detach) {
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
            memory,
            cpus,
        } => {
            let store = Store::new(store_dir);
            let mem_limit = match memory {
                Some(s) => Some(parse_memory(&s).context("invalid memory value")?),
                None => None,
            };
            let limits = ResourceLimits {
                memory: mem_limit,
                cpus,
            };

            // Create container
            let container = match mgr.create(id, &image, &command, args, &store, limits) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Run create error: {:?}", e);
                    process::exit(1);
                }
            };
            // Start it
            if let Err(e) = mgr.start(&container.id, detach) {
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
        Commands::NetworkInit => {
            network::init_network().context("Network init failed")?;
            eprintln!("Bridge docklet0 created and NAT rule added.");
        }
    }
    Ok(())
}
