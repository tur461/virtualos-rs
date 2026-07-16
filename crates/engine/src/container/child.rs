#![allow(irrefutable_let_patterns)]

// ----------- Child Process Execution ----------

use anyhow::Result;
use nix::{
    mount::{MsFlags, mount},
    sched::{CloneCb, CloneFlags, clone},
    sys::signal::Signal,
    unistd::{Pid, chdir, execvp, pivot_root, sethostname, setsid},
};
use std::{ffi::CString, fs, path::Path};

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
    pub unsafe fn run_child(&self) -> Result<Pid> {
        let stack_size = 1024 * 1024;
        let mut stack = vec![0u8; stack_size];
        let flags = CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWNS;

        let mut rootfs = Some(self.rootfs.to_owned());
        let mut cmd = Some(self.command.to_owned());
        let mut args = Some(self.args.to_owned());

        // SAFETY: The stack we provided is valid and large enough.
        // The child function will not outlive the stack because the parent
        // waits for the child to terminate before dropping the stack.
        let child_pid = unsafe {
            let cb: CloneCb = Box::new(move || {
                let rootfs_clone = rootfs.take().expect("callback called more than once");
                let cmd_clone = cmd.take().expect("callback called more than once");
                let args_clone = args.take().expect("callback called more than once");
                Self::child_init(rootfs_clone, cmd_clone, args_clone, self.detach)
            });
            clone(cb, &mut stack, flags, Some(Signal::SIGCHLD as i32))?
        };

        // Note: The child function must not return.
        Ok(child_pid)
    }

    // The child init function (previously child_func) runs inside new namespaces.
    pub fn child_init(rootfs: String, cmd: String, args: Vec<String>, is_detach: bool) -> isize {
        // 1. Set hostname
        if let Err(e) = sethostname("my-container") {
            eprintln!("sethostname failed: {}", e);
            return 1;
        }

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

        // 3. Bind mount rootfs onto itself, pivot, etc. – same as Phase 4 but using config.rootfs

        // 3a. Bind mount rootfs
        if let Err(e) =
            mount::<str, str, str, str>(Some(&rootfs), &rootfs, None, MsFlags::MS_BIND, None)
        {
            eprintln!("bind mount rootfs failed: {}", e);
            return 1;
        }

        let rootfs = Path::new(&rootfs);
        let put_old = rootfs.join(".old_root");
        fs::create_dir_all(&put_old).unwrap();

        if let Err(e) = pivot_root(rootfs, &put_old) {
            eprintln!("pivot_root failed: {}", e);
            return 1;
        }

        if let Err(e) = chdir("/") {
            eprintln!("chdir / failed: {}", e);
            return 1;
        }

        // Unmount old root
        let _ = nix::mount::umount2("/.old_root", nix::mount::MntFlags::MNT_DETACH);
        let _ = fs::remove_dir("/.old_root");

        // Mount standard filesystems
        let _ = mount::<str, str, str, str>(
            Some("proc"),
            "/proc",
            Some("proc"),
            MsFlags::empty(),
            None,
        );
        let _ = mount::<str, str, str, str>(
            Some("sysfs"),
            "/sys",
            Some("sysfs"),
            MsFlags::empty(),
            None,
        );
        let _ = mount::<str, str, str, str>(
            Some("devtmpfs"),
            "/dev",
            Some("devtmpfs"),
            MsFlags::empty(),
            None,
        );
        let _ = mount::<str, str, str, str>(
            Some("devpts"),
            "/dev/pts",
            Some("devpts"),
            MsFlags::empty(),
            None,
        );

        // If detach, create a new session to lose the controlling terminal
        if is_detach {
            if let Err(e) = setsid() {
                eprintln!("setsid failed: {}", e);
                return 1;
            }
        }

        // Execute the command
        let cmd_c =
            CString::new(cmd.as_bytes()).unwrap_or_else(|_| CString::new("/bin/sh").unwrap());
        let args_c: Vec<CString> = args
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

    // /// Child function that is called after clone().
    // /// It runs in the new namespaces.
    // /// Returns isize (0 on success, exit code on error).
    // fn container(rootfs: String, cmd: String, args: Vec<String>) -> isize {
    //     // 1. Set hostname inside UTS namespace
    //     if let Err(e) = sethostname("my-container") {
    //         eprintln!("Failed to set hostname: {}", e);
    //         return 1;
    //     }
    //
    //     // 2. Make the current root mount private (recursive) to avoid host interference.
    //     //    This uses MS_PRIVATE | MS_REC (equivalent to mount --make-rprivate /)
    //     let mnt_flags = MsFlags::MS_PRIVATE | MsFlags::MS_REC;
    //     if let Err(e) = mount::<str, str, str, str>(None, "/", None, mnt_flags, None) {
    //         eprintln!("Failed to make / private: {}", e);
    //         return 1;
    //     }
    //
    //     // 3. Bind‑mount the rootfs onto itself to make it a mount point.
    //     //    pivot_root requires that new_root is a mount point.
    //     let rootfs_path = Path::new(&rootfs);
    //     if let Err(e) =
    //         mount::<str, str, str, str>(Some(&rootfs), &rootfs, None, MsFlags::MS_BIND, None)
    //     {
    //         eprintln!("Failed to bind mount rootfs: {}", e);
    //         return 1;
    //     }
    //
    //     // 4. Create a temporary directory to put the old root.
    //     //    We'll use a path inside the new root. After pivot, old root will be at this location.
    //     let put_old = rootfs_path.join(".old_root");
    //     // Make sure the directory exists (it should be inside the new rootfs)
    //     std::fs::create_dir_all(&put_old).unwrap(); // Ok to panic if rootfs is broken
    //
    //     // 5. Execute pivot_root(new_root, put_old)
    //     if let Err(e) = pivot_root(rootfs_path, &put_old) {
    //         eprintln!("pivot_root failed: {}", e);
    //         return 1;
    //     }
    //
    //     // 6. Change current directory to the new root.
    //     //    After pivot_root, the process's root and cwd are not automatically adjusted.
    //     //    chdir to "/" to be safe, then unmount old root.
    //     if let Err(e) = chdir("/") {
    //         eprintln!("chdir / failed: {}", e);
    //         return 1;
    //     }
    //
    //     // 7. Unmount the old root (now at /.old_root) lazily.
    //     //    Using MNT_DETACH avoids EBUSY if something still holds a reference.
    //     if let Err(e) = umount2("/.old_root", MntFlags::MNT_DETACH) {
    //         eprintln!("Failed to unmount old root: {}", e);
    //         // Not fatal, but we'll try to remove the directory
    //     }
    //     let _ = std::fs::remove_dir("/.old_root");
    //
    //     // 8. Mount essential filesystems: /proc, /sys, /dev, /dev/pts
    //     let mounts = [
    //         ("proc", "/proc", "proc", MsFlags::empty()),
    //         ("sysfs", "/sys", "sysfs", MsFlags::empty()),
    //         ("devtmpfs", "/dev", "devtmpfs", MsFlags::empty()),
    //     ];
    //     for (source, target, fstype, flags) in &mounts {
    //         if let Err(e) = mount(Some(*source), *target, Some(*fstype), *flags, None::<&str>) {
    //             eprintln!("Mounting {} failed: {}", *target, e);
    //             // continue; some may be optional
    //         }
    //     }
    //     // /dev/pts is often mounted separately
    //     let _ = mount::<str, str, str, str>(
    //         Some("devpts"),
    //         "/dev/pts",
    //         Some("devpts"),
    //         MsFlags::empty(),
    //         None,
    //     );
    //
    //     // 9. Execute the command
    //     let cmd_c = CString::new(cmd).unwrap_or_else(|_| CString::new("/bin/sh").unwrap());
    //     let args_c: Vec<CString> = args
    //         .iter()
    //         .map(|a| CString::new(a.as_bytes()).unwrap())
    //         .collect();
    //     let exec_args: Vec<&CString> = if args_c.is_empty() {
    //         vec![&cmd_c]
    //     } else {
    //         std::iter::once(&cmd_c).chain(args_c.iter()).collect()
    //     };
    //
    //     if let Err(e) = execvp(&cmd_c, &exec_args) {
    //         eprintln!("execvp failed: {}", e);
    //         return 1;
    //     }
    //     unreachable!();
    // }
}
