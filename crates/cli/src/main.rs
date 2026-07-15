// cli/src/main.rs
use nix::mount::{MsFlags, mount};
use nix::sched::{CloneCb, CloneFlags, clone};
use nix::sys::signal::Signal;
use nix::sys::wait::waitpid;
use nix::unistd::{Pid, execvp, sethostname};
use std::ffi::CString;
use std::process;

/// Child function that is called after clone().
/// It runs in the new namespaces.
/// Returns isize (0 on success, exit code on error).
fn child_func() -> isize {
    // 1. Set hostname inside UTS namespace
    if let Err(e) = sethostname("my-container") {
        eprintln!("Failed to set hostname: {}", e);
        return 1;
    }

    // 2. Mount /proc so that tools like ps work correctly.
    //    We need a new mount namespace (CLONE_NEWNS) for this to be private.
    if let Err(e) = mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        // MsFlags::empty(),
        None::<&str>,
    ) {
        eprintln!("Failed to mount /proc: {}", e);
        return 1;
    }

    // 3. Execute an interactive shell.
    let cmd = CString::new("/bin/sh").unwrap();
    let args = [CString::new("sh").unwrap(), CString::new("-i").unwrap()];
    if let Err(e) = execvp(&cmd, &args) {
        eprintln!("Failed to exec /bin/sh: {}", e);
        return 1;
    }

    // execvp never returns on success
    unreachable!();
}

fn main() {
    // Allocate a stack for the child process.
    // clone() requires a pointer to the top of a stack for the child.
    let stack_size = 1024 * 1024; // 1 MB
    let mut stack: Vec<u8> = vec![0u8; stack_size];

    // clone() in nix takes a mutable reference to the child function
    // and the stack slice. The closure is just to avoid a separate function pointer.
    let flags = CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWNS;

    // SAFETY: The stack we provided is valid and large enough.
    // The child function will not outlive the stack because the parent
    // waits for the child to terminate before dropping the stack.
    let child_pid = unsafe {
        let cb: CloneCb<'static> = Box::new(child_func);
        clone(cb, &mut stack, flags, Some(Signal::SIGCHLD as i32)).expect("clone failed")
    };

    // Parent process waits for the shell to exit.
    match waitpid(child_pid, None) {
        Ok(status) => {
            eprintln!("Child exited with status: {:?}", status);
        }
        Err(e) => {
            eprintln!("waitpid failed: {}", e);
            process::exit(1);
        }
    }
}
