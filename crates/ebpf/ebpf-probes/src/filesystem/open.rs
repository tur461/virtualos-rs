use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_current_uid_gid, bpf_probe_read_user_str_bytes,
    },
    macros::tracepoint,
    programs::TracePointContext,
};

use crate::{
    events::{EVENT_FS_OPEN, FsOpenEvent, TASK_COMM_LEN},
    maps::{EVENTS, FS_OPEN_SCRATCH},
};

const PATH_LEN: usize = 256;

#[tracepoint]
pub fn sys_enter_openat(ctx: TracePointContext) -> u32 {
    match try_sys_enter_openat(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_sys_enter_openat(ctx: TracePointContext) -> Result<u32, i32> {
    /*
     * sys_enter_openat tracepoint:
     *
     * struct trace_event_raw_sys_enter {
     *     struct trace_entry ent;
     *     long id;
     *     unsigned long args[6];
     * };
     *
     * For x86_64:
     *
     * args[0] = dfd
     * args[1] = filename
     * args[2] = flags
     * args[3] = mode
     *
     * The syscall tracepoint header is 16 bytes:
     *
     *   u16 type
     *   u8  flags
     *   u8  preempt_count
     *   s32 pid
     *
     * followed by:
     *
     *   long id
     *   unsigned long args[6]
     *
     * Therefore args[1] starts at offset 32.
     */

    let filename_ptr = unsafe { ctx.read_at::<*const u8>(32).map_err(|_| -1) }?;

    let flags = unsafe { ctx.read_at::<u64>(40).map_err(|_| -1) }?;

    if filename_ptr.is_null() {
        return Ok(0);
    }

    let pid_tgid = bpf_get_current_pid_tgid();

    let pid = pid_tgid as u32;
    let tgid = (pid_tgid >> 32) as u32;

    let uid_gid = bpf_get_current_uid_gid();

    let uid = uid_gid as u32;
    let gid = (uid_gid >> 32) as u32;

    let cgroup_id = unsafe { bpf_get_current_cgroup_id() };

    let comm = bpf_get_current_comm().unwrap_or([0u8; TASK_COMM_LEN]);

    let mut path = [0u8; PATH_LEN];

    /*
     * Read the userspace pathname safely.
     */
    let path_slice = match unsafe { bpf_probe_read_user_str_bytes(filename_ptr, &mut path) } {
        Ok(bytes) => bytes,
        Err(_) => return Ok(0),
    };

    let path_len = path_slice.len();

    let scratch = match FS_OPEN_SCRATCH.get_ptr_mut(0) {
        Some(ptr) => ptr,
        None => return Ok(0),
    };

    let event = unsafe { &mut *scratch };

    (*event).event_type = EVENT_FS_OPEN;

    (*event).pid = pid;
    (*event).tgid = tgid;

    (*event).uid = uid;
    (*event).gid = gid;

    (*event).cgroup_id = cgroup_id;

    (*event).flags = flags;

    (*event).comm = comm;

    (*event).path = path;
    (*event).path_len = path_len as u32;

    let mut buf = match EVENTS.reserve::<FsOpenEvent>(0) {
        Some(buf) => buf,
        None => return Ok(0),
    };
    buf.write(*event);
    buf.submit(0);

    Ok(0)
}
