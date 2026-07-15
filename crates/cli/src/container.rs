#![allow(irrefutable_let_patterns)]
use nix::mount::{MntFlags, MsFlags, mount, umount2};
use nix::sched::{CloneCb, CloneFlags, clone};
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::{chdir, execvp, pivot_root, sethostname};
use std::ffi::CString;
use std::path::Path;
use std::process;

pub struct Container;

impl Container {
    pub fn run(rootfs: &str, cmd: &str, args: &[String]) {
        // Ensure rootfs exists and is a directory
        let rootfs_path = Path::new(rootfs);
        if !rootfs_path.exists() || !rootfs_path.is_dir() {
            eprintln!(
                "Root filesystem not found at '{}'. Provide an Alpine miniroot directory.",
                rootfs_path.display()
            );
            process::exit(1);
        }

        // Stack for clone
        let stack_size = 1024 * 1024;
        let mut stack: Vec<u8> = vec![0u8; stack_size];
        // clone() in nix takes a mutable reference to the child function
        // and the stack slice. The closure is just to avoid a separate function pointer.
        let flags = CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWNS;

        let mut rootfs = Some(rootfs.to_owned());
        let mut cmd = Some(cmd.to_owned());
        let mut args = Some(args.to_owned());

        // SAFETY: The stack we provided is valid and large enough.
        // The child function will not outlive the stack because the parent
        // waits for the child to terminate before dropping the stack.
        let child_pid = unsafe {
            let cb: CloneCb<'static> = Box::new(move || {
                let rootfs_clone = rootfs.take().expect("callback called more than once");
                let cmd_clone = cmd.take().expect("callback called more than once");
                let args_clone = args.take().expect("callback called more than once");
                Self::container(rootfs_clone, cmd_clone, args_clone)
            });
            clone(cb, &mut stack, flags, Some(Signal::SIGCHLD as i32)).expect("clone failed")
        };

        // Parent process waits for the shell to exit.
        match waitpid(child_pid, None) {
            Ok(status) => {
                eprintln!("Container exited with status: {:?}", status);
            }
            Err(e) => {
                eprintln!("waitpid failed: {}", e);
                process::exit(1);
            }
        }
    }
    /// Child function that is called after clone().
    /// It runs in the new namespaces.
    /// Returns isize (0 on success, exit code on error).
    fn container(rootfs: String, cmd: String, args: Vec<String>) -> isize {
        // 1. Set hostname inside UTS namespace
        if let Err(e) = sethostname("my-container") {
            eprintln!("Failed to set hostname: {}", e);
            return 1;
        }

        // 2. Make the current root mount private (recursive) to avoid host interference.
        //    This uses MS_PRIVATE | MS_REC (equivalent to mount --make-rprivate /)
        let mnt_flags = MsFlags::MS_PRIVATE | MsFlags::MS_REC;
        if let Err(e) = mount::<str, str, str, str>(None, "/", None, mnt_flags, None) {
            eprintln!("Failed to make / private: {}", e);
            return 1;
        }

        // 3. Bind‑mount the rootfs onto itself to make it a mount point.
        //    pivot_root requires that new_root is a mount point.
        let rootfs_path = Path::new(&rootfs);
        if let Err(e) =
            mount::<str, str, str, str>(Some(&rootfs), &rootfs, None, MsFlags::MS_BIND, None)
        {
            eprintln!("Failed to bind mount rootfs: {}", e);
            return 1;
        }

        // 4. Create a temporary directory to put the old root.
        //    We'll use a path inside the new root. After pivot, old root will be at this location.
        let put_old = rootfs_path.join(".old_root");
        // Make sure the directory exists (it should be inside the new rootfs)
        std::fs::create_dir_all(&put_old).unwrap(); // Ok to panic if rootfs is broken

        // 5. Execute pivot_root(new_root, put_old)
        if let Err(e) = pivot_root(rootfs_path, &put_old) {
            eprintln!("pivot_root failed: {}", e);
            return 1;
        }

        // 6. Change current directory to the new root.
        //    After pivot_root, the process's root and cwd are not automatically adjusted.
        //    chdir to "/" to be safe, then unmount old root.
        if let Err(e) = chdir("/") {
            eprintln!("chdir / failed: {}", e);
            return 1;
        }

        // 7. Unmount the old root (now at /.old_root) lazily.
        //    Using MNT_DETACH avoids EBUSY if something still holds a reference.
        if let Err(e) = umount2("/.old_root", MntFlags::MNT_DETACH) {
            eprintln!("Failed to unmount old root: {}", e);
            // Not fatal, but we'll try to remove the directory
        }
        let _ = std::fs::remove_dir("/.old_root");

        // 8. Mount essential filesystems: /proc, /sys, /dev, /dev/pts
        let mounts = [
            ("proc", "/proc", "proc", MsFlags::empty()),
            ("sysfs", "/sys", "sysfs", MsFlags::empty()),
            ("devtmpfs", "/dev", "devtmpfs", MsFlags::empty()),
        ];
        for (source, target, fstype, flags) in &mounts {
            if let Err(e) = mount(Some(*source), *target, Some(*fstype), *flags, None::<&str>) {
                eprintln!("Mounting {} failed: {}", *target, e);
                // continue; some may be optional
            }
        }
        // /dev/pts is often mounted separately
        let _ = mount::<str, str, str, str>(
            Some("devpts"),
            "/dev/pts",
            Some("devpts"),
            MsFlags::empty(),
            None,
        );

        // 9. Execute the command
        let cmd_c = CString::new(cmd).unwrap_or_else(|_| CString::new("/bin/sh").unwrap());
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
}
