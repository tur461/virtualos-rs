use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents a cgroup (v2) for a container.
pub struct Cgroup {
    path: PathBuf,
}

impl Cgroup {
    /// Create a new cgroup under the given parent path (e.g., "/sys/fs/cgroup/docklet").
    /// The container id is used as the cgroup name.
    pub fn new(parent: &Path, container_id: &str) -> Result<Self> {
        let path = parent.join(container_id);
        fs::create_dir_all(&path)
            .with_context(|| format!("Failed to create cgroup directory {}", path.display()))?;

        // Enable memory and cpu controllers in the parent if needed.
        // We attempt to write "+memory +cpu" to cgroup.subtree_control.
        // This may fail if already enabled or not writable, which is fine.
        let subtree_control = parent.join("cgroup.subtree_control");
        let _ = fs::write(&subtree_control, "+memory +cpu");

        Ok(Cgroup { path })
    }

    /// Open an existing cgroup by path.
    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        if !path.exists() {
            anyhow::bail!("Cgroup path {} does not exist", path.display());
        }
        Ok(Cgroup { path })
    }

    /// Set a memory limit in bytes. Pass 0 for unlimited.
    pub fn set_memory_limit(&self, limit_bytes: u64) -> Result<()> {
        let mem_max = self.path.join("memory.max");
        let value = if limit_bytes == 0 {
            "max".to_string()
        } else {
            limit_bytes.to_string()
        };
        fs::write(&mem_max, &value)
            .with_context(|| format!("Failed to set memory.max to {}", value))
    }

    /// Set a CPU bandwidth limit. `cpus` is a float (e.g., 1.5).
    /// We translate to cpu.max: quota = cpus * 100000, period = 100000.
    pub fn set_cpu_limit(&self, cpus: f64) -> Result<()> {
        if cpus <= 0.0 {
            anyhow::bail!("CPU limit must be positive");
        }
        let quota = (cpus * 100_000.0) as u64;
        let cpu_max = self.path.join("cpu.max");
        let value = format!("{} 100000", quota);
        fs::write(&cpu_max, &value).with_context(|| format!("Failed to set cpu.max to {}", value))
    }

    /// Set a CPU weight (shares). Default is 100. Range 1-10000.
    pub fn set_cpu_weight(&self, weight: u64) -> Result<()> {
        let cpu_weight = self.path.join("cpu.weight");
        fs::write(&cpu_weight, weight.to_string())
            .with_context(|| format!("Failed to set cpu.weight to {}", weight))
    }

    /// Add a process (by PID) to this cgroup.
    pub fn add_process(&self, pid: u32) -> Result<()> {
        let procs = self.path.join("cgroup.procs");
        fs::write(&procs, pid.to_string())
            .with_context(|| format!("Failed to add PID {} to cgroup.procs", pid))
    }

    /// Remove the cgroup directory. This will fail if there are still processes,
    /// but we call it after the container is stopped (empty).
    pub fn delete(self) -> Result<()> {
        fs::remove_dir(&self.path)
            .with_context(|| format!("Failed to remove cgroup {}", self.path.display()))
    }

    /// Return the path for debugging or storage.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
