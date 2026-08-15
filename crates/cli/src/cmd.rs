use anyhow::{Context, Result};
use daemon::client::Client;
use ebpf::EbpfManager;
use engine::{ContainerManager, ResourceLimits};
use std::{net::SocketAddr, process};
use storage::Store;
use virtualization::{Vm, VmConfig};

use crate::{
    helpers::parse_memory,
    types::{Cli, EbpfCmd},
};

use super::types::Commands;

pub async fn run_with_client(cli: Cli, client: &mut Client) -> Result<()> {
    match cli.command {
        Commands::Pull {
            reference,
            store_dir,
        } => {
            client
                .pull(&reference, &store_dir.to_string_lossy())
                .await?;
            println!("Pull succeeded.");
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
            let mem = memory.and_then(|s| parse_memory(&s).ok());
            let id = client
                .create(
                    id.as_deref(),
                    &image,
                    &command,
                    args.iter().map(|s| s.as_str()).collect(),
                    &store_dir.to_string_lossy(),
                    mem,
                    cpus,
                )
                .await?;
            println!("Container {} created.", id);
        }
        Commands::Start { id, detach: _ } => {
            client.start(&id).await?;
            println!("Container {} started.", id);
        }
        Commands::Stop { id } => {
            client.stop(&id).await?;
            println!("Container {} stopped.", id);
        }
        Commands::Rm { id, force } => {
            client.delete(&id, force).await?;
            println!("Container {} removed.", id);
        }
        Commands::Ps => {
            let containers = client.list().await?;
            if containers.is_empty() {
                println!("No containers.");
            } else {
                for c in containers {
                    println!(
                        "{:<12} {:<10} {:<20} PID={:?} IP={}",
                        c.id, c.status, c.image, c.pid, c.network_ip
                    );
                }
            }
        }
        Commands::Logs { id } => {
            // gRPC logs would require streaming; placeholder
            println!("Logs for {}: (not implemented via daemon)", id);
        }
        Commands::Run {
            detach,
            rm,
            id: _,
            interactive: _,
            tty: _,
            image,
            command,
            args,
            store_dir,
            memory,
            cpus,
            ..
        } => {
            if !detach {
                anyhow::bail!(
                    "Foreground run is not supported via daemon. Please run directly (unset daemon socket)."
                );
            }
            let mem_limit = memory.and_then(|s| parse_memory(&s).ok());
            let id = client
                .run(
                    None,
                    &image,
                    &command,
                    args.iter().map(|s| s.as_str()).collect(),
                    &store_dir.to_string_lossy(),
                    mem_limit,
                    cpus,
                    true, // detach
                    rm,
                )
                .await?;
            println!("{}", id);
        }
        Commands::NetworkInit => {
            anyhow::bail!("network-init must be executed locally (as root).");
        }
        // Commands::Ebpf {
        //     cmd: EbpfCmd::Trace,
        // } => {
        //     let mut manager = EbpfManager::load()?;
        //     let mut rx = manager.start_exec_tracing().await?;
        //     println!("Tracing execve... Press Ctrl-C to stop.");
        //     while let Some(event) = rx.recv().await {
        //         println!("PID {} exec: {}", event.pid, event.filename);
        //     }
        // }
        _ => run_local(cli)?,
    }
    Ok(())
}

pub fn run_local(cli: Cli) -> Result<()> {
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

        Commands::Logs { id: _ } => {
            // let container = mgr.load_container(&id)?;
            // println!("Container {} ({:?})", container.id, container.status);
            println!("Logs not yet implemented.");
        }

        Commands::Rm { id, force } => {
            // Stop if running, then delete
            if force && mgr.is_container_running(&id) {
                mgr.stop(&id)?;
            }

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
            rm,
            id,
            interactive: _, // ignore for now
            tty: _,
            detach,
            image,
            command,
            args,
            store_dir,
            memory,
            cpus,
            vm,
            kernel,
            rootfs_image,
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
            let container = match mgr.create(id, &image, &command, args.clone(), &store, limits) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Run create error: {:?}", e);
                    process::exit(1);
                }
            };
            if vm {
                let config = VmConfig {
                    kernel_path: kernel,
                    initramfs_path: Some(rootfs_image),
                    command: command.clone(),
                    args: args.clone(),
                    memory_mb: 128, // default
                    vcpu_count: 1,  // default
                };
                let mut vm = Vm::new(config)?;
                let exit_code = vm.run()?;
                println!("VM exited with code {}", exit_code);
            } else {
                // normal container run
                // Start it
                if let Err(e) = mgr.start(&container.id, detach) {
                    eprintln!("Run start error: {:?}", e);
                }

                if !detach {
                    if rm {
                        let _ = mgr.delete(&container.id);
                    } else {
                        // Mark as stopped (the foreground run already did that if it succeeded)
                        // If error, still try to set stopped state
                    }
                }
            }
        }
        Commands::NetworkInit => {
            network::init_network().context("Network init failed")?;
            eprintln!("Bridge virtualos0 created and NAT rule added.");
        }

        Commands::Monitor { port } => {
            let addr = SocketAddr::new(std::net::Ipv4Addr::UNSPECIFIED.into(), port);
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async { monitoring::serve_metrics(addr).await })?;
        }

        Commands::Ebpf {
            cmd: EbpfCmd::Trace,
        } => {
            let mut manager = EbpfManager::load()?;
            manager.start_filesystem_tracing()?;
            // let rt = tokio::runtime::Runtime::new()?;
            // rt.block_on(async {
            //     let mut rx = manager.start_filesystem_tracing()?;
            //     println!("Tracing execve... Press Ctrl-C to stop.");
            //     while let Some(event) = rx.recv().await {
            //         println!("PID {} exec: {}", event.pid, event.filename);
            //     }
            //     Ok::<_, anyhow::Error>(())
            // })?
        }
    }
    Ok(())
}
