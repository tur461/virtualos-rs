use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_current_uid_gid,
    },
    macros::btf_tracepoint,
    programs::BtfTracePointContext,
};

use crate::{
    events::{EVENT_FORK, ForkEvent, TASK_COMM_LEN},
    maps::EVENTS,
};

#[btf_tracepoint(function = "sched_process_fork")]
pub fn sched_process_fork(ctx: BtfTracePointContext) -> u32 {
    match unsafe { try_sched_process_fork(ctx) } {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

unsafe fn try_sched_process_fork(_ctx: BtfTracePointContext) -> Result<u32, i32> {
    let pid_tgid = bpf_get_current_pid_tgid();

    let parent_pid = pid_tgid as u32;
    let parent_tgid = (pid_tgid >> 32) as u32;

    let uid_gid = bpf_get_current_uid_gid();

    let uid = uid_gid as u32;
    let gid = (uid_gid >> 32) as u32;

    let cgroup_id = bpf_get_current_cgroup_id();

    let comm = bpf_get_current_comm().unwrap_or([0u8; TASK_COMM_LEN]);

    /*
     * We initially use the current task identity.
     *
     * The BTF context is intentionally kept available here so
     * we can later read the kernel task_struct arguments and
     * obtain the exact child PID/TGID without tracepoint offsets.
     */
    let event = ForkEvent {
        event_type: EVENT_FORK,

        parent_pid,
        parent_tgid,

        child_pid: 0,
        child_tgid: 0,

        uid,
        gid,

        cgroup_id,

        comm,
    };

    let mut buf = match EVENTS.reserve::<ForkEvent>(0) {
        Some(buf) => buf,
        None => return Ok(0),
    };

    buf.write(event);
    buf.submit(0);

    Ok(0)
}
