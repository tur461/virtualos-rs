use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_current_uid_gid,
    },
    macros::tracepoint,
    programs::TracePointContext,
};

use crate::{
    events::{EVENT_EXEC, ExecEvent, FILENAME_LEN},
    maps::{EVENTS, PROC_EXEC_SCRATCH},
};

#[tracepoint]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    match try_sched_process_exec(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_sched_process_exec(_ctx: TracePointContext) -> Result<u32, i32> {
    let pid_tgid = bpf_get_current_pid_tgid();

    let pid = pid_tgid as u32;
    let tgid = (pid_tgid >> 32) as u32;

    let uid_gid = bpf_get_current_uid_gid();

    let uid = uid_gid as u32;
    let gid = (uid_gid >> 32) as u32;

    let cgroup_id = unsafe { bpf_get_current_cgroup_id() };

    let comm = match bpf_get_current_comm() {
        Ok(comm) => comm,
        Err(_) => [0u8; aya_ebpf::TASK_COMM_LEN],
    };

    let scratch = match PROC_EXEC_SCRATCH.get_ptr_mut(0) {
        Some(ptr) => ptr,
        None => return Err(-1),
    };

    let event = unsafe { &mut *scratch };

    (*event).event_type = EVENT_EXEC;

    (*event).pid = pid;
    (*event).tgid = tgid;

    (*event).uid = uid;
    (*event).gid = gid;

    (*event).cgroup_id = cgroup_id;
    (*event).comm = comm;
    (*event).filename = [0u8; FILENAME_LEN];

    /*
     * sched_process_exec is a kernel tracepoint and does not provide
     * the original userspace filename pointer in a portable way through
     * TracePointContext.
     *
     * The executable identity is therefore initially represented by
     * the task's comm.
     *
     * We keep filename in the ABI so that a future BTF/fentry implementation
     * can populate it without changing the userspace event structure.
     */

    (*event).filename[..comm.len()].copy_from_slice(&comm);

    let mut buf = match EVENTS.reserve::<ExecEvent>(0) {
        Some(buf) => buf,
        None => return Ok(0),
    };

    buf.write(*event);
    buf.submit(0);

    Ok(0)
}
