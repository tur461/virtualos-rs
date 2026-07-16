mod child;
use child::ChildConfig;
use nix::{
    sys::{
        signal::{Signal, kill},
        wait::{WaitPidFlag, waitpid},
    },
    unistd::Pid,
};

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use storage::Store;
use uuid::Uuid;

use crate::prepare_rootfs;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ContainerStatus {
    Created,
    Running,
    Stopped,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Container {
    pub id: String,
    pub image: String,
    pub command: String,
    pub args: Vec<String>,
    pub status: ContainerStatus,
    pub pid: Option<i32>,
    pub created_at: String,
    // Paths needed for cleanup
    pub rootfs_path: Option<PathBuf>, // merged mount point
    pub temp_dir: Option<PathBuf>,    // overlay upper/work directory parent
}

pub struct ContainerManager {
    base_dir: PathBuf,
}

impl ContainerManager {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        ContainerManager {
            base_dir: base_dir.into(),
        }
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
        };

        // Save state
        let json = serde_json::to_string_pretty(&container)?;
        fs::write(self.state_path(&id), json)?;

        println!("Container {} created", id);
        Ok(container)
    }

    /// Start a container that is in Created state.
    pub fn start(&self, id: &str) -> Result<()> {
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

        // Build the child configuration

        let child_cfg = ChildConfig::new(&rootfs, &cmd, &args, true);

        let child_pid =
            unsafe { child_cfg.run_child() }.context("Failed to start container process")?;

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
