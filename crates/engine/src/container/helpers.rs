use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use anyhow::Result;
use nix::libc;
use nix::sys::signal::Signal;
use nix::unistd::Pid;

use super::types::ResourceLimits;

pub fn apply_cgroup_limits(
    parent: &Path,
    container_id: &str,
    limits: &ResourceLimits,
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

    // cgroup.add_process(pid.as_raw() as u32)?;
    Ok(cgroup.path().to_path_buf())
}

/// Clone a child process directly into a cgroup using clone3 with CLONE_INTO_CGROUP.
/// Requires Linux 5.7+.
pub fn clone_into_cgroup<F>(mut child_fn: F, flags: u64, cgroup_fd: impl AsRawFd) -> Result<Pid>
where
    F: FnMut() -> i32,
{
    const CLONE_INTO_CGROUP: u64 = 0x200000000; // from linux/sched.h
    // The libc crate might not have CLONE_INTO_CGROUP, define our own.
    #[repr(C)]
    #[derive(Default)]
    struct CloneArgs {
        flags: u64,
        pidfd: u64,
        child_tid: u64,
        parent_tid: u64,
        exit_signal: u64,
        stack: u64,
        stack_size: u64,
        tls: u64,
        set_tid: u64,
        set_tid_size: u64,
        cgroup: u64,
    }

    let mut args = CloneArgs {
        flags: flags | CLONE_INTO_CGROUP,
        exit_signal: Signal::SIGCHLD as u64,
        cgroup: cgroup_fd.as_raw_fd() as u64,
        // All other fields zero – we are not using CLONE_VM, so no stack needed.
        ..Default::default()
    };

    let ret = unsafe {
        libc::syscall(
            libc::SYS_clone3,
            &mut args,
            std::mem::size_of::<CloneArgs>(),
        )
    };

    if ret == 0 {
        // Child
        let exit_code = child_fn();
        unsafe { libc::_exit(exit_code) };
    } else if ret > 0 {
        Ok(Pid::from_raw(ret as i32))
    } else {
        Err(nix::Error::last().into())
    }
}
