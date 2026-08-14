#![no_std]
#![no_main]

use aya_ebpf::{
    bindings::pt_regs,
    helpers::{bpf_get_current_pid_tgid, bpf_probe_read_user_str_bytes},
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};

// Ring buffer to send events to userspace
#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(1024 * 256, 0);

#[tracepoint]
pub fn trace_execve(ctx: TracePointContext) -> u32 {
    // Safety: ctx is provided by the kernel
    unsafe {
        if let Ok(regs) = ctx.read_at::<*const pt_regs>(1) {
            let regs = *regs;
            let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
            let filename_ptr = ctx.read_at::<*const u8>(0).unwrap_or(core::ptr::null());
            let _comm_ptr = regs.rcx as *const u8; // approximate: regs.cx often holds comm for syscalls

            // We'll send a simple event struct
            #[repr(C)]
            struct ExecEvent {
                pid: u32,
                filename: [u8; 256],
            }

            let mut event = ExecEvent {
                pid,
                filename: [0; 256],
            };

            // Copy filename safely (best effort)
            if !filename_ptr.is_null() {
                let _bytes =
                    bpf_probe_read_user_str_bytes(filename_ptr, &mut event.filename).unwrap_or(&[]);
            }

            if let Some(mut buf) = EVENTS.reserve::<ExecEvent>(0) {
                buf.write(event);
                buf.submit(0);
            }
        }
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// For cgroup/socket filter we'll define a separate program later
// We'll add a placeholder now.
