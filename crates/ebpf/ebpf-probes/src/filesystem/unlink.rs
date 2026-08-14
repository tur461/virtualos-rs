use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_current_uid_gid, bpf_probe_read_user_str_bytes,
    },
    macros::tracepoint,
    programs::TracePointContext,
};

use crate::{
    events::{EVENT_FS_UNLINK, FsUnlinkEvent, TASK_COMM_LEN},
    maps::{EVENTS, FS_UNLINK_SCRATCH},
};

#[tracepoint]
pub fn sys_enter_unlink(ctx: TracePointContext) -> u32 {
    match try_unlink(ctx, 24) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

#[tracepoint]
pub fn sys_enter_unlinkat(ctx: TracePointContext) -> u32 {
    match try_unlink(ctx, 32) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_unlink(ctx: TracePointContext, path_offset: usize) -> Result<u32, i32> {
    let path_ptr = unsafe { ctx.read_at::<*const u8>(path_offset).map_err(|_| -1) }?;

    if path_ptr.is_null() {
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

    let scratch = match FS_UNLINK_SCRATCH.get_ptr_mut(0) {
        Some(ptr) => ptr,
        None => return Ok(0),
    };

    let event = unsafe { &mut *scratch };

    unsafe {
        (*event).event_type = EVENT_FS_UNLINK;

        (*event).pid = pid;
        (*event).tgid = tgid;

        (*event).uid = uid;
        (*event).gid = gid;

        (*event).cgroup_id = cgroup_id;

        (*event).flags = 0;

        (*event).comm = comm;

        let bytes = bpf_probe_read_user_str_bytes(path_ptr, &mut (*event).path);

        if let Ok(bytes) = bytes {
            (*event).path_len = bytes.len() as u32;
        }
    }

    let mut buf = match EVENTS.reserve::<FsUnlinkEvent>(0) {
        Some(buf) => buf,
        None => return Ok(0),
    };

    buf.write(*event);
    buf.submit(0);

    Ok(0)
}
