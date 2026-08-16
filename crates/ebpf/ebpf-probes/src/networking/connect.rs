use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_current_uid_gid,
    },
    macros::tracepoint,
    programs::TracePointContext,
};

use crate::{
    events::{EVENT_NET_CONNECT, NET_COMM_LEN, NetSocketEvent},
    maps::{EVENTS, NW_CONNECT_SCRATCH},
};

#[tracepoint]
pub fn sys_enter_connect(ctx: TracePointContext) -> u32 {
    match try_connect(ctx) {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

fn try_connect(ctx: TracePointContext) -> Result<u32, i32> {
    /*
     * connect(int fd, const struct sockaddr *addr, socklen_t len)
     *
     * args[0] = fd
     *
     * sys_enter tracepoint:
     *
     * args[0] starts at offset 24.
     */

    let fd = unsafe { ctx.read_at::<i32>(24).map_err(|_| -1) }?;

    let pid_tgid = bpf_get_current_pid_tgid();

    let pid = pid_tgid as u32;
    let tgid = (pid_tgid >> 32) as u32;

    let uid_gid = bpf_get_current_uid_gid();

    let uid = uid_gid as u32;
    let gid = (uid_gid >> 32) as u32;

    let cgroup_id = unsafe { bpf_get_current_cgroup_id() };

    let comm = bpf_get_current_comm().unwrap_or([0u8; NET_COMM_LEN]);

    let scratch = match NW_CONNECT_SCRATCH.get_ptr_mut(0) {
        Some(ptr) => ptr,
        None => return Ok(0),
    };

    let event = unsafe { &mut *scratch };

    (*event).event_type = EVENT_NET_CONNECT;

    (*event).pid = pid;
    (*event).tgid = tgid;

    (*event).uid = uid;
    (*event).gid = gid;

    (*event).cgroup_id = cgroup_id;

    (*event).comm = comm;

    (*event).fd = fd;
    (*event).family = 0;
    (*event).socket_type = 0;
    (*event).protocol = 0;
    (*event)._pad = 0;

    let mut buf = match EVENTS.reserve::<NetSocketEvent>(0) {
        Some(buf) => buf,
        None => return Ok(0),
    };
    buf.write(*event);
    buf.submit(0);

    Ok(0)
}
