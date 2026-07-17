#![allow(irrefutable_let_patterns)]

use anyhow::Result;
use nix::{
    libc,
    mount::{MntFlags, MsFlags, mount, umount2},
    sched::CloneFlags,
    unistd::{Pid, chdir, execvp, pivot_root, sethostname, setsid},
};
use std::{
    ffi::CString,
    fs::{self, OpenOptions},
    os::{
        fd::{BorrowedFd, RawFd},
        unix::fs::OpenOptionsExt,
    },
    path::{Path, PathBuf},
};

use crate::container::{
    helpers::clone_into_cgroup,
    types::{Child, ChildConfig},
};

impl Child {
    pub fn new(r: &str, c: &str, a: &[String], d: bool, rfd: RawFd) -> Self {
        Self {
            config: ChildConfig {
                rootfs: r.to_string(),
                command: c.to_string(),
                args: a.to_vec(),
                detach: d,
                ready_fd: Some(rfd),
            },
        }
    }

    pub fn run(&self, cg_path: &PathBuf) -> Result<Pid> {
        let flags = (CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWUTS
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWCGROUP)
            .bits() as u64;

        // Prepare the cgroup directory and get a fd
        // Open cgroup directory with O_DIRECTORY (required for CLONE_INTO_CGROUP)
        // eprintln!("creating cg_path: {cg_path:?}");

        fs::create_dir_all(cg_path)?;
        let cg_fd = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC)
            .open(cg_path)?;

        // use clone3 helper
        let child_pid = clone_into_cgroup(|| self.init(), flags, cg_fd)?;

        // Note: The child function must not return.
        Ok(child_pid)
    }

    // The child init function (previously child_func) runs inside new namespaces.
    pub fn init(&self) -> i32 {
        if let Err(e) = sethostname("my-container") {
            eprintln!("sethostname failed: {}", e);
            return 1;
        }

        if let Err(e) = mount::<str, str, str, str>(
            None,
            "/",
            None,
            MsFlags::MS_PRIVATE | MsFlags::MS_REC,
            None,
        ) {
            eprintln!("mount / failed: {}", e);
            return 1;
        }

        if let Err(e) = mount::<str, str, str, str>(
            Some(&self.config.rootfs),
            &self.config.rootfs,
            None,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None,
        ) {
            eprintln!("mount rootfs failed: {}", e);
            return 1;
        }

        let rootfs = Path::new(&self.config.rootfs);

        let rfs_proc_path = rootfs.join("proc");
        let rfs_sys_path = rootfs.join("sys");
        let rfs_sysfs_path = rootfs.join("sys/fs");
        let rfs_cg_path = rootfs.join("sys/fs/cgroup");
        let rfs_dev_path = rootfs.join("dev");
        let rfs_devpts_path = rootfs.join("dev/pts");
        let rfs_oldroot_path = rootfs.join(".old_root");

        let create_dirs = || -> Result<()> {
            fs::create_dir_all(rfs_proc_path)?;
            fs::create_dir_all(rfs_sys_path)?;
            fs::create_dir_all(rfs_sysfs_path)?;
            fs::create_dir_all(&rfs_cg_path)?;
            fs::create_dir_all(rfs_dev_path)?;
            fs::create_dir_all(rfs_devpts_path)?;
            fs::create_dir_all(&rfs_oldroot_path)?;
            Ok(())
        };

        if let Err(e) = create_dirs() {
            eprintln!("Error creating dirs: {e}");
            return 1;
        }

        if let Err(e) = pivot_root(rootfs, rfs_oldroot_path.to_str().unwrap()) {
            eprintln!("pivot_root failed: {}", e);
            return 1;
        }

        if let Err(e) = chdir("/") {
            eprintln!("chdir / failed: {}", e);
            return 1;
        }

        // Unmount old root
        if let Err(e) = umount2("/.old_root", MntFlags::MNT_DETACH) {
            eprintln!("mount2 .old_root failed: {e}");
            return 1;
        }

        let _ = fs::remove_dir("/.old_root");

        // Mount standard filesystems
        if let Err(e) =
            mount::<str, str, str, str>(Some("proc"), "/proc", Some("proc"), MsFlags::empty(), None)
        {
            eprintln!("mount proc failed: {e}");
            return 1;
        }

        if let Err(e) = mount::<str, str, str, str>(
            Some("sysfs"),
            "/sys",
            Some("sysfs"),
            MsFlags::empty(),
            None,
        ) {
            eprintln!("mount sysfs failed: {e}");
            return 1;
        }

        if let Err(e) = mount::<str, str, str, str>(
            Some("devtmpfs"),
            "/dev",
            Some("devtmpfs"),
            MsFlags::empty(),
            None,
        ) {
            eprintln!("mount devtmpfs failed: {e}");
            return 1;
        }

        if let Err(e) = mount::<str, str, str, str>(
            Some("devpts"),
            "/dev/pts",
            Some("devpts"),
            MsFlags::empty(),
            None,
        ) {
            eprintln!("mount devpts failed: {e}");
            return 1;
        }

        // Ensure cgroup mountpoint exists
        if let Err(e) = mount::<str, str, str, str>(
            Some("cgroup2"),
            "/sys/fs/cgroup",
            Some("cgroup2"),
            MsFlags::empty(),
            None::<&str>,
        ) {
            eprintln!("Error mounting cgroup2 {e}");
            return 1;
        }

        if let Some(fd) = self.config.ready_fd {
            let mut buf = [0u8; 1];

            // SAFETY: fd is valid for the duration of this scope.
            let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };

            let _ = nix::unistd::read(borrowed, &mut buf);
            let _ = nix::unistd::close(fd);
        }

        // If detach, create a new session to lose the controlling terminal
        if self.config.detach
            && let Err(e) = setsid()
        {
            eprintln!("setsid failed: {}", e);
            return 1;
        }

        // Execute the command
        let cmd_c = CString::new(self.config.command.as_bytes())
            .unwrap_or_else(|_| CString::new("/bin/sh").unwrap());

        let args_c: Vec<CString> = self
            .config
            .args
            .iter()
            .map(|a| CString::new(a.as_bytes()).unwrap())
            .collect();

        let exec_args: Vec<&CString> = if args_c.is_empty() {
            vec![&cmd_c]
        } else {
            std::iter::once(&cmd_c).chain(args_c.iter()).collect()
        };

        if let Err(e) = execvp(&cmd_c, &exec_args) {
            eprintln!("execvp failed: {}", e);
            return 1;
        }

        unreachable!();
    }
}
