use super::{
    helpers::apply_cgroup_limits,
    types::{Container, ContainerManager, ContainerStatus, ResourceLimits},
};
use crate::mount::prepare_rootfs;

use super::types::Child;
use nix::{
    sys::{
        signal::{Signal, kill},
        wait::{WaitPidFlag, waitpid},
    },
    unistd::{Pid, pipe},
};

use std::{
    fs::{self},
    net::Ipv4Addr,
    os::fd::AsRawFd,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use storage::Store;
use uuid::Uuid;

impl ContainerManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        // Auto-detect a cgroup parent where memory and cpu are available.
        // let cgroup_parent = find_cgroup_parent(&["memory", "cpu"]).expect(
        //     "cgroup delegation missing: ensure cgroup v2 is available and controllers are enabled",
        // );
        // .unwrap_or_else(|_| PathBuf::from("/sys/fs/cgroup"));
        // Ensure the docklet subdirectory exists inside that parent.
        // let docklet_cgroup = cgroup_parent.join("docklet");

        let docklet_cgroup = PathBuf::from("/sys/fs/cgroup/docklet");
        // Ensure we have a fresh directory so that controllers are inherited
        // from the detected parent.
        // if docklet_cgroup.exists() {
        //     let _ = std::fs::remove_dir(&docklet_cgroup);
        // }
        std::fs::create_dir_all(&docklet_cgroup)
            .expect("Failed to create docklet cgroup directory");

        ContainerManager {
            base_dir,
            cgroup_parent: docklet_cgroup,
        }
    }

    /// Allocate the next available IP in the 10.0.0.0/24 subnet.
    /// This scans existing containers (via the manager) and picks the smallest free host part >= 2.
    pub fn allocate_ip(&self) -> Result<String> {
        let containers = self.list()?;
        let mut used: Vec<u8> = containers
            .iter()
            .filter_map(|c| c.network_ip.as_ref())
            .map(|ip| {
                ip.parse::<Ipv4Addr>()
                    .map(|addr| addr.octets()[3])
                    .unwrap_or(0)
            })
            .collect();
        used.sort_unstable();

        let mut next = 2u8;
        for u in used {
            if u == next {
                next += 1;
            } else if u > next {
                break;
            }
        }
        if next > 254 {
            anyhow::bail!("No free IP addresses in the container subnet");
        }
        Ok(format!("10.0.0.{}", next))
    }

    fn container_dir(&self, id: &str) -> PathBuf {
        self.base_dir.join("containers").join(id)
    }

    fn state_path(&self, id: &str) -> PathBuf {
        self.container_dir(id).join("state.json")
    }

    /// Create a new container: pull image, prepare overlay, save state as Created.
    pub fn create(
        &self,
        id: Option<String>,
        image: &str,
        command: &str,
        args: Vec<String>,
        store: &Store,
        limits: ResourceLimits,
    ) -> Result<Container> {
        let id = id.unwrap_or_else(|| {
            Uuid::new_v4()
                .to_string()
                .split('-')
                .next()
                .unwrap()
                .to_string()
        });
        let container_dir = self.container_dir(&id);
        if container_dir.exists() {
            anyhow::bail!("Container {} already exists", id);
        }
        fs::create_dir_all(&container_dir)?;

        let mounted = prepare_rootfs(image, store)?;
        // Detach to keep overlay alive after MountedRoot is dropped.
        let (rootfs_path, temp_dir) = mounted.detach();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        let ip = self.allocate_ip()?;
        let container = Container {
            id: id.clone(),
            image: image.to_string(),
            command: command.to_string(),
            args,
            status: ContainerStatus::Created,
            pid: None,
            created_at: now,
            rootfs_path: Some(rootfs_path),
            temp_dir: Some(temp_dir),
            memory_limit: limits.memory,
            cpu_limit: limits.cpus,
            cgroup_path: None,
            network_ip: Some(ip),
        };

        // Save state
        let json = serde_json::to_string_pretty(&container)?;
        fs::write(self.state_path(&id), json)?;

        println!("Container {} created", id);
        Ok(container)
    }

    /// Start a container that is in Created state.
    pub fn start(&self, id: &str, is_detach: bool) -> Result<()> {
        let mut container = self.load_container(id)?;
        if container.status != ContainerStatus::Created {
            anyhow::bail!("Container {} is not in Created state", id);
        }

        let rootfs = container
            .rootfs_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Missing rootfs path"))?
            .to_str()
            .unwrap()
            .to_owned();
        let cmd = container.command.clone();
        let args = container.args.clone();

        let (rx, tx) = pipe()?; // rx is read end, tx is write end
        let ready_fd = rx.as_raw_fd();
        // Build the child configuration

        let child = Child::new(&rootfs, &cmd, &args, is_detach, ready_fd);
        // Close our copy of the read end so that if we crash, child sees EOF
        drop(rx);
        // Apply cgroup limits if any are set
        if container.memory_limit.is_some() || container.cpu_limit.is_some() {
            let cg_path = apply_cgroup_limits(
                &self.cgroup_parent,
                &container.id,
                &ResourceLimits {
                    memory: container.memory_limit,
                    cpus: container.cpu_limit,
                },
            )?;

            container.cgroup_path = Some(cg_path);
        }
        let child_pid = child
            .run(&self.cgroup_parent.join(&container.id))
            .context("Failed to start container process")?;

        // eprintln!(
        //     "debug: child cgroup = {}",
        //     std::fs::read_to_string(format!("/proc/{}/cgroup", child_pid.as_raw()))
        //         .unwrap_or_default()
        // );
        // Set up network if the container has an IP
        if let Some(ref ip) = container.network_ip {
            network::setup_container_net(child_pid.as_raw() as u32, &container.id, ip)?;
        }

        // Signal child to proceed
        nix::unistd::write(&tx, &[0])?;
        // tx will be dropped, closing the write end.
        drop(tx);
        // Update container state
        container.status = ContainerStatus::Running;
        container.pid = Some(child_pid.as_raw());
        self.save_container(&container)?;

        println!("Container {} started with PID {}", id, child_pid);
        Ok(())
    }

    /// Stop a running container by sending SIGTERM, then SIGKILL after a grace period.
    pub fn stop(&self, id: &str) -> Result<()> {
        let mut container = self.load_container(id)?;
        if container.status != ContainerStatus::Running {
            anyhow::bail!("Container {} is not running", id);
        }
        let pid = container.pid.unwrap();
        let pid = Pid::from_raw(pid);

        // Send SIGTERM
        kill(pid, Signal::SIGTERM).context("Failed to send SIGTERM")?;
        // Wait up to 10 seconds for the process to exit
        let timeout = std::time::Duration::from_secs(10);
        let start = std::time::Instant::now();
        loop {
            match kill(pid, None) {
                Ok(_) => {
                    // Process still exists
                    if start.elapsed() > timeout {
                        // Send SIGKILL
                        kill(pid, Signal::SIGKILL).context("Failed to send SIGKILL")?;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(nix::Error::ESRCH) => {
                    // Process no longer exists
                    break;
                }
                Err(e) => anyhow::bail!("Error checking process: {}", e),
            }
        }

        // Reap the child (waitpid) to avoid zombies – we can just try waitpid with WNOHANG
        let _ = waitpid(pid, Some(WaitPidFlag::WNOHANG));

        container.status = ContainerStatus::Stopped;
        container.pid = None;
        if let Some(ref cg_path) = container.cgroup_path {
            if let Ok(cg) = cgroups::Cgroup::from_path(cg_path) {
                let _ = cg.delete();
            }
            container.cgroup_path = None;
        }

        if let Some(_ref_ip) = &container.network_ip {
            let _ = network::teardown_container_net(&container.id);
            // we could keep the IP, but it's freed by teardown; no need to mark as None.
        }

        self.save_container(&container)?;
        println!("Container {} stopped", id);
        Ok(())
    }

    /// Delete a container. Stop it first if running, then remove overlay and state.
    pub fn delete(&self, id: &str) -> Result<()> {
        let container = self.load_container(id)?;
        if container.status == ContainerStatus::Running {
            self.stop(id)?;
        }
        // Unmount rootfs if still mounted
        if let Some(ref rootfs) = container.rootfs_path
            && rootfs.exists()
        {
            // Try to unmount (ignore error if already unmounted)
            let _ = Store::unmount(rootfs);
        }
        // Remove temp directory (upper/work)
        if let Some(ref temp) = container.temp_dir
            && temp.exists()
        {
            fs::remove_dir_all(temp).ok();
        }

        if let Some(ref cg_path) = container.cgroup_path
            && let Ok(cg) = cgroups::Cgroup::from_path(cg_path)
        {
            let _ = cg.delete();
        }

        if let Some(_ref_ip) = &container.network_ip {
            let _ = network::teardown_container_net(&container.id);
            // we could keep the IP, but it's freed by teardown; no need to mark as None.
        }

        // Remove container directory
        let dir = self.container_dir(id);
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        println!("Container {} deleted", id);
        Ok(())
    }

    /// List all containers and their statuses.
    pub fn list(&self) -> Result<Vec<Container>> {
        let containers_dir = self.base_dir.join("containers");
        if !containers_dir.exists() {
            return Ok(Vec::new());
        }
        let mut containers = Vec::new();
        for entry in fs::read_dir(&containers_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let state_file = entry.path().join("state.json");
                if state_file.exists() {
                    let data = fs::read_to_string(&state_file)?;
                    let mut cont: Container = serde_json::from_str(&data)?;
                    // If status is Running, check if PID is still alive; if not, mark as Stopped.
                    if cont.status == ContainerStatus::Running
                        && let Some(pid) = cont.pid
                        && kill(Pid::from_raw(pid), None).is_err()
                    {
                        cont.status = ContainerStatus::Stopped;
                        cont.pid = None;
                        // Update state on disk
                        let json = serde_json::to_string_pretty(&cont)?;
                        fs::write(&state_file, json).ok();
                    }
                    containers.push(cont);
                }
            }
        }
        Ok(containers)
    }

    fn load_container(&self, id: &str) -> Result<Container> {
        let path = self.state_path(id);
        if !path.exists() {
            anyhow::bail!("Container {} not found", id);
        }
        let data = fs::read_to_string(&path)?;
        let cont: Container = serde_json::from_str(&data)?;
        Ok(cont)
    }

    fn save_container(&self, container: &Container) -> Result<()> {
        let path = self.state_path(&container.id);
        let json = serde_json::to_string_pretty(container)?;
        fs::write(path, json)?;
        Ok(())
    }
}
