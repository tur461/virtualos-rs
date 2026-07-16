use std::path::{Path, PathBuf};

use anyhow::Result;
use nix::unistd::Pid;

use super::types::ResourceLimits;

pub fn apply_cgroup_limits(
    parent: &Path,
    container_id: &str,
    limits: &ResourceLimits,
    pid: &Pid,
) -> Result<PathBuf> {
    let cgroup = cgroups::Cgroup::new(parent, container_id)?;

    if let Some(mem) = limits.memory {
        cgroup.set_memory_limit(mem)?;
    }
    if let Some(cpus) = limits.cpus {
        cgroup.set_cpu_limit(cpus)?;
    } else {
        // Set a default weight to ensure this container’s cgroup exists, but no hard limit.
        // Actually, we need to ensure that the cgroup is not the root cgroup; we just created it.
        // Setting a weight is optional.
    }

    cgroup.add_process(pid.as_raw() as u32)?;
    Ok(cgroup.path().to_path_buf())
}
