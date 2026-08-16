use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_current_uid_gid,
    },
    macros::tracepoint,
    programs::TracePointContext,
};

use crate::{
    events::{EVENT_FS_CLOSE, FsCloseEvent, TASK_COMM_LEN},
    maps::{EVENTS, FS_CLOSE_SCRATCH},
};

#[tracepoint]
pub fn sys_enter_close(ctx: TracePointContext) -> u32 {
    match try_sys_enter_close(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_sys_enter_close(ctx: TracePointContext) -> Result<u32, i32> {
    /*
     * sys_enter_close:
     *
     * args[0] = fd
     *
     * sys_enter tracepoint:
     *
     * offset 16 = syscall id
     * offset 24 = args[0]
     */

    let fd = unsafe { ctx.read_at::<i64>(24).map_err(|_| -1) }?;

    let pid_tgid = bpf_get_current_pid_tgid();

    let pid = pid_tgid as u32;
    let tgid = (pid_tgid >> 32) as u32;

    let uid_gid = bpf_get_current_uid_gid();

    let uid = uid_gid as u32;
    let gid = (uid_gid >> 32) as u32;

    let cgroup_id = unsafe { bpf_get_current_cgroup_id() };

    let comm = bpf_get_current_comm().unwrap_or([0u8; TASK_COMM_LEN]);

    let scratch = match FS_CLOSE_SCRATCH.get_ptr_mut(0) {
        Some(ptr) => ptr,
        None => return Ok(0),
    };

    let event = unsafe { &mut *scratch };

    (*event).event_type = EVENT_FS_CLOSE;

    (*event).pid = pid;
    (*event).tgid = tgid;

    (*event).uid = uid;
    (*event).gid = gid;

    (*event).cgroup_id = cgroup_id;

    (*event).comm = comm;

    (*event).fd = fd;

    let mut buf = match EVENTS.reserve::<FsCloseEvent>(0) {
        Some(buf) => buf,
        None => return Ok(0),
    };
    buf.write(*event);
    buf.submit(0);

    Ok(0)
}
