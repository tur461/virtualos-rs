use aya_ebpf::{
    EbpfContext, helpers::bpf_ktime_get_ns, macros::tracepoint, programs::TracePointContext,
};

use crate::{
    events::{NetworkEvent, emit},
    networking::types::{AddressFamily, SocketEvent, SocketEventKind, TransportProtocol},
};

#[tracepoint]
pub fn udp_sendmsg(ctx: TracePointContext) -> u32 {
    let mut event = SocketEvent::empty();

    event.timestamp_ns = unsafe { bpf_ktime_get_ns() };

    event.pid = ctx.pid();
    event.tgid = ctx.tgid();

    event.family = AddressFamily::Unknown;
    event.protocol = TransportProtocol::Udp;
    event.kind = SocketEventKind::UdpSend;

    let network_event = NetworkEvent::socket(event);

    let _ = emit(network_event);

    0
}

#[tracepoint]
pub fn udp_recvmsg(ctx: TracePointContext) -> u32 {
    let mut event = SocketEvent::empty();

    event.timestamp_ns = unsafe { bpf_ktime_get_ns() };

    event.pid = ctx.pid();
    event.tgid = ctx.tgid();

    event.family = AddressFamily::Unknown;
    event.protocol = TransportProtocol::Udp;
    event.kind = SocketEventKind::UdpReceive;

    let network_event = NetworkEvent::socket(event);

    let _ = emit(network_event);

    0
}
