use aya_ebpf::bindings::{
    BPF_SOCK_OPS_PASSIVE_ESTABLISHED_CB, BPF_SOCK_OPS_STATE_CB, BPF_SOCK_OPS_TCP_CONNECT_CB,
    BPF_SOCK_OPS_TCP_LISTEN_CB,
};

use crate::{
    maps::EVENTS,
    networking::types::{SocketEvent, SocketEventKind, SocketMetadata},
};

#[inline(always)]
pub fn classify_sock_op(op: u32) -> SocketEventKind {
    match op {
        BPF_SOCK_OPS_TCP_CONNECT_CB => SocketEventKind::TcpConnect,

        BPF_SOCK_OPS_TCP_LISTEN_CB => SocketEventKind::TcpListen,

        BPF_SOCK_OPS_PASSIVE_ESTABLISHED_CB => SocketEventKind::TcpPassiveEstablished,

        BPF_SOCK_OPS_STATE_CB => SocketEventKind::TcpStateChange,

        _ => SocketEventKind::Unknown,
    }
}

#[inline(always)]
fn emit_event(event: &SocketEvent) -> Result<(), i64> {
    /*
     * Connect this to the RingBuf/perf-event
     * infrastructure from the previous batches.
     */

    let mut buf = match EVENTS.reserve::<SocketEvent>(0) {
        Some(buf) => buf,
        None => return Ok(()),
    };
    buf.write(*event);
    buf.submit(0);

    Ok(())
}

#[inline(always)]
pub fn emit_sock_meta_event(metadata: &SocketMetadata) -> Result<(), i64> {
    /*
     * Event transport will be wired to the project's
     * existing RingBuf/PerfEventArray implementation.
     *
     * Keeping this function isolated means the socket
     * extraction logic remains independent of the event
     * transport.
     */

    let mut buf = match EVENTS.reserve::<SocketMetadata>(0) {
        Some(buf) => buf,
        None => return Ok(()),
    };
    buf.write(*metadata);
    buf.submit(0);

    Ok(())
}

#[inline(always)]
pub fn emit_sock_event(event: SocketEvent) -> Result<(), i64> {
    emit_event(&event)
}
