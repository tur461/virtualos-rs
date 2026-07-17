#![allow(irrefutable_let_patterns)]

// ----------- Child Process Execution ----------

use anyhow::Result;
use nix::{
    libc,
    mount::{MsFlags, mount},
    sched::CloneFlags,
    unistd::{Pid, chdir, execvp, pivot_root, sethostname, setsid},
};
use std::{
    ffi::CString,
    fs::{self, OpenOptions},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

use crate::container::helpers::clone_into_cgroup;

pub struct ChildConfig {
    rootfs: String,
    command: String,
    args: Vec<String>,
    detach: bool,
}

impl ChildConfig {
    pub fn new(r: &str, c: &str, a: &[String], d: bool) -> Self {
        Self {
            rootfs: r.to_string(),
            command: c.to_string(),
            args: a.to_vec(),
            detach: d,
        }
    }

    /// # Safety
    /// The caller must ensure the stack remains valid for the lifetime of the child.
    pub fn run_child(&self, cg_path: &PathBuf) -> Result<Pid> {
        // let stack_size = 1024 * 1024;
        // let mut stack = vec![0u8; stack_size];
        let flags = (CloneFlags::CLONE_NEWPID
            | CloneFlags::CLONE_NEWUTS
            | CloneFlags::CLONE_NEWNS
            | CloneFlags::CLONE_NEWCGROUP)
            .bits() as u64;

        let mut rootfs = Some(self.rootfs.to_owned());
        let mut cmd = Some(self.command.to_owned());
        let mut args = Some(self.args.to_owned());

        // Prepare the cgroup directory and get a fd
        // Open cgroup directory with O_DIRECTORY (required for CLONE_INTO_CGROUP)
        fs::create_dir_all(cg_path)?;
        let cg_fd = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_CLOEXEC)
            .open(cg_path)?;

        // SAFETY: The stack we provided is valid and large enough.
        // The child function will not outlive the stack because the parent
        // waits for the child to terminate before dropping the stack.
        let cb = || {
            let rootfs_clone = rootfs.take().expect("callback called more than once");
            let cmd_clone = cmd.take().expect("callback called more than once");
            let args_clone = args.take().expect("callback called more than once");
            Self::child_init(rootfs_clone, cmd_clone, args_clone, self.detach)
        };
        let child_pid = clone_into_cgroup(cb, flags, cg_fd)?;
        // clone(cb, &mut stack, flags, Some(Signal::SIGCHLD as i32))?

        // Note: The child function must not return.
        Ok(child_pid)
    }

    // The child init function (previously child_func) runs inside new namespaces.
    pub fn child_init(rootfs: String, cmd: String, args: Vec<String>, is_detach: bool) -> i32 {
        // 1. Set hostname
        if let Err(e) = sethostname("my-container") {
            eprintln!("sethostname failed: {}", e);
            return 1;
        }
        eprintln!("after sethostname");

        // 2. Make / private
        if let Err(e) = mount::<str, str, str, str>(
            None,
            "/",
            None,
            MsFlags::MS_PRIVATE | MsFlags::MS_REC,
            None,
        ) {
            eprintln!("mount --make-rprivate / failed: {}", e);
            return 1;
        }
        eprintln!("after mount-(--make-rprivate)");

        // 3. Bind mount rootfs onto itself, pivot, etc. – same as Phase 4 but using config.rootfs

        // 3a. Bind mount rootfs
        if let Err(e) = mount::<str, str, str, str>(
            Some(&rootfs),
            &rootfs,
            None,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None,
        ) {
            eprintln!("bind mount rootfs failed: {}", e);
            return 1;
        }
        eprintln!("after bind-mount-rootfs");
        let rootfs = Path::new(&rootfs);

        let _ = fs::create_dir_all(rootfs.join("proc"));
        let _ = fs::create_dir_all(rootfs.join("sys"));
        let _ = fs::create_dir_all(rootfs.join("sys/fs"));
        let _ = fs::create_dir_all(rootfs.join("sys/fs/cgroup"));
        let _ = fs::create_dir_all(rootfs.join("dev"));
        let _ = fs::create_dir_all(rootfs.join("dev/pts"));
        let _ = fs::create_dir_all(rootfs.join(".old_root"));

        if let Err(e) = mount::<str, str, str, str>(
            Some("/sys/fs/cgroup"), // host source
            rootfs.join("sys/fs/cgroup").to_str().unwrap(),
            None,
            MsFlags::MS_BIND,
            None::<&str>,
        ) {
            eprintln!("Warning: failed to bind-mount cgroup: {}", e);
        }

        eprintln!("after mount-cgroup");

        if let Err(e) = pivot_root(rootfs, rootfs.join(".old_root").to_str().unwrap()) {
            eprintln!("pivot_root failed: {}", e);
            return 1;
        }
        eprintln!("after pivot_root");

        if let Err(e) = chdir("/") {
            eprintln!("chdir / failed: {}", e);
            return 1;
        }
        eprintln!("after chdir");

        // Unmount old root
        let _ = nix::mount::umount2("/.old_root", nix::mount::MntFlags::MNT_DETACH);
        let _ = fs::remove_dir("/.old_root");
        eprintln!("after umount-old_root");

        // Mount standard filesystems
        let _ = mount::<str, str, str, str>(
            Some("proc"),
            "/proc",
            Some("proc"),
            MsFlags::empty(),
            None,
        );
        eprintln!("after mount-proc");
        let _ = mount::<str, str, str, str>(
            Some("sysfs"),
            "/sys",
            Some("sysfs"),
            MsFlags::empty(),
            None,
        );
        eprintln!("after mount-sysfs");
        let _ = mount::<str, str, str, str>(
            Some("devtmpfs"),
            "/dev",
            Some("devtmpfs"),
            MsFlags::empty(),
            None,
        );
        eprintln!("after mount-devtmpfs");
        let _ = mount::<str, str, str, str>(
            Some("devpts"),
            "/dev/pts",
            Some("devpts"),
            MsFlags::empty(),
            None,
        );
        eprintln!("after mount-devpts");
        // Ensure cgroup mountpoint exists
        // let _ = std::fs::create_dir_all("/sys/fs/cgroup");
        // let _ = mount::<str, str, str, str>(
        //     Some("cgroup2"),
        //     "/sys/fs/cgroup",
        //     Some("cgroup2"),
        //     MsFlags::empty(),
        //     None::<&str>,
        // );
        // eprintln!("after mount-cgroup2");
        // Bind-mount the host's cgroup tree into the new rootfs so that
        // /sys/fs/cgroup is visible inside the container.

        // If detach, create a new session to lose the controlling terminal
        if is_detach && let Err(e) = setsid() {
            eprintln!("setsid failed: {}", e);
            return 1;
        }
        eprintln!("after is_detach (check): {is_detach}");

        // Execute the command
        let cmd_c =
            CString::new(cmd.as_bytes()).unwrap_or_else(|_| CString::new("/bin/sh").unwrap());
        eprintln!("Running inside container: {cmd}");
        let args_c: Vec<CString> = args
            .iter()
            .map(|a| CString::new(a.as_bytes()).unwrap())
            .collect();
        eprintln!("after argc: {args_c:?}");
        let exec_args: Vec<&CString> = if args_c.is_empty() {
            vec![&cmd_c]
        } else {
            std::iter::once(&cmd_c).chain(args_c.iter()).collect()
        };

        if let Err(e) = execvp(&cmd_c, &exec_args) {
            eprintln!("execvp failed: {}", e);
            return 1;
        }
        eprintln!("after execvp");
        unreachable!();
    }
}
