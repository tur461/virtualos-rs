use std::{os::fd::RawFd, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ContainerStatus {
    Created,
    Running,
    Stopped,
}

pub struct ResourceLimits {
    pub memory: Option<u64>, // bytes
    pub cpus: Option<f64>,
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
    // cgroups
    pub memory_limit: Option<u64>,    // bytes
    pub cpu_limit: Option<f64>,       // CPUs (e.g., 1.5)
    pub cgroup_path: Option<PathBuf>, // path to cgroup dir
    // network
    pub network_ip: Option<String>,
}

pub struct ContainerManager {
    pub base_dir: PathBuf,
    pub cgroup_parent: PathBuf,
}

pub struct ChildConfig {
    pub rootfs: String,
    pub command: String,
    pub args: Vec<String>,
    pub detach: bool,
    pub ready_fd: Option<RawFd>,
}

pub struct Child {
    pub config: ChildConfig,
}
