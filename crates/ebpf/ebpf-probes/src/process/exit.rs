use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_current_uid_gid,
    },
    macros::tracepoint,
    programs::TracePointContext,
};

use crate::{
    events::{EVENT_EXIT, ExitEvent, TASK_COMM_LEN},
    maps::{EVENTS, PROC_EXIT_SCRATCH},
};

#[tracepoint]
pub fn sched_process_exit(ctx: TracePointContext) -> u32 {
    match try_sched_process_exit(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_sched_process_exit(_ctx: TracePointContext) -> Result<u32, i32> {
    /*
     * bpf_get_current_pid_tgid():
     *
     * upper 32 bits = TGID
     * lower 32 bits = PID
     */
    let pid_tgid = bpf_get_current_pid_tgid();

    let pid = pid_tgid as u32;
    let tgid = (pid_tgid >> 32) as u32;

    /*
     * bpf_get_current_uid_gid():
     *
     * lower 32 bits = UID
     * upper 32 bits = GID
     */
    let uid_gid = bpf_get_current_uid_gid();

    let uid = uid_gid as u32;
    let gid = (uid_gid >> 32) as u32;

    let cgroup_id = unsafe { bpf_get_current_cgroup_id() };

    let comm = bpf_get_current_comm().unwrap_or([0u8; TASK_COMM_LEN]);

    /*
     * sched_process_exit fires when the current task is
     * exiting, so pid/tgid/comm describe the task that is
     * actually terminating.
     *
     * We currently don't extract the kernel exit_code from
     * task_struct. Keep this zero until we add a BTF-safe
     * task_struct accessor.
     */
    let scratch = match PROC_EXIT_SCRATCH.get_ptr_mut(0) {
        Some(ptr) => ptr,
        None => return Err(-1),
    };

    let event = unsafe { &mut *scratch };

    (*event).event_type = EVENT_EXIT;

    (*event).pid = pid;
    (*event).tgid = tgid;

    (*event).uid = uid;
    (*event).gid = gid;

    (*event).cgroup_id = cgroup_id;
    (*event).comm = comm;
    (*event).exit_code = 0;

    /*
     * Reserve space in the BPF ring buffer.
     *
     * If the ring buffer is full, simply drop this event.
     * Never block an exiting task.
     */
    let mut buf = match EVENTS.reserve::<ExitEvent>(0) {
        Some(buf) => buf,
        None => return Ok(0),
    };

    buf.write(*event);
    buf.submit(0);

    Ok(0)
}
