use aya_ebpf::{
    EbpfContext, helpers::bpf_ktime_get_ns, macros::tracepoint, programs::TracePointContext,
};

use crate::{
    maps::NW_UDP_SCRATCH,
    networking::{
        events::emit_sock_event,
        types::{AddressFamily, SocketEvent, SocketEventKind, TransportProtocol},
    },
};

#[tracepoint]
pub fn udp_bind(ctx: TracePointContext) -> u32 {
    if let Some(event) = udp_sock_tracepoint(ctx, SocketEventKind::UdpBind) {
        let _ = emit_sock_event(event);
        return 0;
    }

    1
}

#[tracepoint]
pub fn udp_connect(ctx: TracePointContext) -> u32 {
    if let Some(event) = udp_sock_tracepoint(ctx, SocketEventKind::UdpConnect) {
        let _ = emit_sock_event(event);
        return 0;
    }

    1
}

#[tracepoint]
pub fn udp_sendmsg(ctx: TracePointContext) -> u32 {
    if let Some(event) = udp_sock_tracepoint(ctx, SocketEventKind::UdpSend) {
        let _ = emit_sock_event(event);
        return 0;
    }

    1
}

#[tracepoint]
pub fn udp_recvmsg(ctx: TracePointContext) -> u32 {
    if let Some(event) = udp_sock_tracepoint(ctx, SocketEventKind::UdpReceive) {
        let _ = emit_sock_event(event);
        return 0;
    }

    1
}

#[inline]
fn udp_sock_tracepoint(ctx: TracePointContext, kind: SocketEventKind) -> Option<SocketEvent> {
    let scratch = match NW_UDP_SCRATCH.get_ptr_mut(0) {
        Some(ptr) => ptr,
        None => return None,
    };

    let event = unsafe { &mut *scratch };

    (*event).timestamp_ns = unsafe { bpf_ktime_get_ns() };

    (*event).pid = ctx.pid();
    (*event).tgid = ctx.tgid();

    (*event).uid = ctx.uid();
    (*event).gid = ctx.gid();

    (*event).family = AddressFamily::Unknown;
    (*event).protocol = TransportProtocol::Udp;
    (*event).kind = kind;

    (*event)._pad = [0u8; 2];

    Some(*event)
}
