use aya_ebpf::{
    helpers::{
        bpf_get_current_cgroup_id, bpf_get_current_comm, bpf_get_current_pid_tgid,
        bpf_get_current_uid_gid,
    },
    macros::tracepoint,
    programs::TracePointContext,
};

use crate::{
    events::{EVENT_NET_BIND, NET_COMM_LEN, NetSocketEvent},
    maps::EVENTS,
};

#[tracepoint]
pub fn sys_enter_bind(ctx: TracePointContext) -> u32 {
    match unsafe { try_bind(ctx) } {
        Ok(ret) => ret,
        Err(_) => 0,
    }
}

unsafe fn try_bind(ctx: TracePointContext) -> Result<u32, i32> {
    /*
     * bind(int fd, const struct sockaddr *addr, socklen_t len)
     *
     * args[0] = fd
     */
    let fd = ctx.read_at::<i32>(24).map_err(|_| -1)?;

    let pid_tgid = bpf_get_current_pid_tgid();

    let pid = pid_tgid as u32;
    let tgid = (pid_tgid >> 32) as u32;

    let uid_gid = bpf_get_current_uid_gid();

    let uid = uid_gid as u32;
    let gid = (uid_gid >> 32) as u32;

    let cgroup_id = bpf_get_current_cgroup_id();

    let comm = bpf_get_current_comm().unwrap_or([0u8; NET_COMM_LEN]);

    let event = NetSocketEvent {
        event_type: EVENT_NET_BIND,

        pid,
        tgid,

        uid,
        gid,

        cgroup_id,

        fd,

        family: 0,
        socket_type: 0,
        protocol: 0,

        _pad: 0,

        comm,
    };

    let mut buf = match EVENTS.reserve::<NetSocketEvent>(0) {
        Some(buf) => buf,
        None => return Ok(0),
    };

    buf.write(event);
    buf.submit(0);

    Ok(0)
}
