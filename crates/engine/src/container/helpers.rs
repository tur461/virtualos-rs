use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use nix::libc;
use nix::sys::signal::Signal;
use nix::unistd::Pid;

use super::types::ResourceLimits;

const CLONE_INTO_CGROUP: u64 = 0x200000000; // from linux/sched.h

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

/// Locate a cgroup (v2) parent where the given controllers are enabled.
/// Starts from the cgroup of the current process and goes up.
pub fn _find_cgroup_parent(controllers: &[&str]) -> Result<PathBuf> {
    let my_pid = std::process::id();
    let cgroup_path = format!("/proc/{}/cgroup", my_pid);
    let content = fs::read_to_string(&cgroup_path).context("Failed to read own cgroup")?;

    // v2: line starting with "0::"
    let line = content
        .lines()
        .find(|l| l.starts_with("0::"))
        .context("No cgroup v2 entry")?;

    let rel_path = line.strip_prefix("0::").unwrap();
    let rel_trimmed = rel_path.trim_start_matches('/');
    let mut current = PathBuf::from("/sys/fs/cgroup").join(rel_trimmed);
    eprintln!("debug: starting cgroup walk at {}", current.display());

    loop {
        let subtree_control = current.join("cgroup.subtree_control");
        if subtree_control.exists() {
            let ctrls = fs::read_to_string(&subtree_control).unwrap_or_default();
            eprintln!(
                "debug: {} subtree_control = {:?}",
                current.display(),
                ctrls.trim()
            );
            if controllers.iter().all(|c| ctrls.contains(c)) {
                eprintln!("debug: found suitable parent: {}", current.display());
                return Ok(current);
            }
        } else {
            eprintln!("debug: {} has no subtree_control file", current.display());
        }
        // Move to parent, but stop only when we've tried the root and there's no parent left.
        let parent = current
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| current.clone());
        if parent == current {
            // We're at the filesystem root and haven't found anything – give up.
            break;
        }
        current = parent;
    }

    anyhow::bail!(
        "Could not find a cgroup with {} enabled in any ancestor",
        controllers.join(", ")
    );
}

/// Clone a child process directly into a cgroup using clone3 with CLONE_INTO_CGROUP.
/// Requires Linux 5.7+.
pub fn clone_into_cgroup<F>(mut child_fn: F, flags: u64, cgroup_fd: impl AsRawFd) -> Result<Pid>
where
    F: FnMut() -> i32,
{
    println!("clone_into_cgroup called");
    // clone3 syscall number on x86_64 is 435
    // const SYS_CLONE3: libc::c_long = 435;

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

    // let mut args = clone_args {
    //     flags: (flags.bits() | libc::CLONE_INTO_CGROUP) as u64,
    //     cgroup: cgroup_fd.as_raw_fd() as u64,
    //     stack: stack.as_ptr() as u64 + stack.len() as u64,
    //     stack_size: stack.len() as u64,
    //     exit_signal: Signal::SIGCHLD as u64,
    //     ..Default::default()
    // };

    // Set pidfd if you want, but not needed now.
    let ret = unsafe {
        libc::syscall(
            // SYS_CLONE3,
            libc::SYS_clone3,
            &mut args,
            std::mem::size_of::<CloneArgs>(),
        )
    };
    println!("RET_VAL from SYS_clone3 syscall: {ret}");
    if ret == 0 {
        // Child
        println!("calling child_fn..");
        let exit_code = child_fn();
        println!("EXIT_CODE from child_fn: {exit_code}");
        unsafe { libc::_exit(exit_code) };
    } else if ret > 0 {
        Ok(Pid::from_raw(ret as i32))
    } else {
        Err(nix::Error::last().into())
    }
}
