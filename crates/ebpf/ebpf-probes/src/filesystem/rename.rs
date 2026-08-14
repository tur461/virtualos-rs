use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_current_uid_gid, bpf_probe_read_user_str_bytes,
    },
    macros::tracepoint,
    programs::TracePointContext,
};

use crate::{
    events::{EVENT_FS_RENAME, FsRenameEvent, TASK_COMM_LEN},
    maps::{EVENTS, FS_RENAME_SCRATCH},
};

#[tracepoint]
pub fn sys_enter_rename(ctx: TracePointContext) -> u32 {
    match try_rename(ctx, 24, 32, 0) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

#[tracepoint]
pub fn sys_enter_renameat(ctx: TracePointContext) -> u32 {
    match try_rename(ctx, 32, 40, 0) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

#[tracepoint]
pub fn sys_enter_renameat2(ctx: TracePointContext) -> u32 {
    match try_rename(ctx, 32, 40, 48) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_rename(
    ctx: TracePointContext,
    old_path_offset: usize,
    new_path_offset: usize,
    flags_offset: usize,
) -> Result<u32, i32> {
    let old_ptr = unsafe { ctx.read_at::<*const u8>(old_path_offset).map_err(|_| -1) }?;

    let new_ptr = unsafe { ctx.read_at::<*const u8>(new_path_offset).map_err(|_| -1) }?;

    if old_ptr.is_null() || new_ptr.is_null() {
        return Ok(0);
    }

    let flags = if flags_offset != 0 {
        unsafe { ctx.read_at::<u64>(flags_offset).map_err(|_| -1) }?
    } else {
        0
    };

    let pid_tgid = bpf_get_current_pid_tgid();

    let pid = pid_tgid as u32;
    let tgid = (pid_tgid >> 32) as u32;

    let uid_gid = bpf_get_current_uid_gid();

    let uid = uid_gid as u32;
    let gid = (uid_gid >> 32) as u32;

    let cgroup_id = unsafe { bpf_get_current_cgroup_id() };

    let comm = bpf_get_current_comm().unwrap_or([0u8; TASK_COMM_LEN]);

    let scratch = match FS_RENAME_SCRATCH.get_ptr_mut(0) {
        Some(ptr) => ptr,
        None => return Ok(0),
    };

    let event = unsafe { &mut *scratch };
    unsafe {
        (*event).event_type = EVENT_FS_RENAME;

        (*event).pid = pid;
        (*event).tgid = tgid;

        (*event).uid = uid;
        (*event).gid = gid;

        (*event).cgroup_id = cgroup_id;

        (*event).flags = flags;

        (*event).comm = comm;

        (*event).old_path_len = 0;
        (*event).new_path_len = 0;

        let old_bytes = bpf_probe_read_user_str_bytes(old_ptr, &mut (*event).old_path);

        let new_bytes = bpf_probe_read_user_str_bytes(new_ptr, &mut (*event).new_path);

        if let Ok(bytes) = old_bytes {
            (*event).old_path_len = bytes.len() as u32;
        }

        if let Ok(bytes) = new_bytes {
            (*event).new_path_len = bytes.len() as u32;
        }
    }

    let mut buf = match EVENTS.reserve::<FsRenameEvent>(0) {
        Some(buf) => buf,
        None => return Ok(0),
    };
    buf.write(*event);
    buf.submit(0);

    Ok(0)
}
