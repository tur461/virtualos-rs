use aya_ebpf::{EbpfContext, helpers::generated::bpf_ktime_get_ns, programs::SockOpsContext};

use crate::networking::{
    address::extract_address,
    events::classify_sock_op,
    types::{SocketEvent, SocketMetadata, TransportProtocol},
};

#[inline(always)]
pub fn socket_metadata(ctx: &SockOpsContext) -> SocketMetadata {
    let family = ctx.family();

    let local_addr = extract_address(family, ctx.local_ip4(), ctx.local_ip6());

    let remote_addr = extract_address(family, ctx.remote_ip4(), ctx.remote_ip6());

    let mut metadata = SocketMetadata::zeroed();

    metadata.family = local_addr.family;

    metadata.local_addr = local_addr.address;
    metadata.remote_addr = remote_addr.address;

    metadata.local_port = ctx.local_port() as u16;
    metadata.remote_port = ctx.remote_port() as u16;

    metadata
}

#[inline(always)]
pub fn socket_event(ctx: &SockOpsContext) -> SocketEvent {
    let mut event = SocketEvent::zeroed();

    let family = ctx.family();

    let local = extract_address(family, ctx.local_ip4(), ctx.local_ip6());

    let remote = extract_address(family, ctx.remote_ip4(), ctx.remote_ip6());

    event.timestamp_ns = unsafe { bpf_ktime_get_ns() };

    event.pid = ctx.pid();
    event.tgid = ctx.tgid();
    event.uid = ctx.uid();
    event.gid = ctx.gid();

    event.family = local.family;

    event.protocol = TransportProtocol::Tcp;

    event.kind = classify_sock_op(ctx.op());

    event.local_addr = local.address;
    event.remote_addr = remote.address;

    event.local_port = ctx.local_port() as u16;
    event.remote_port = ctx.remote_port() as u16;

    event.old_state = ctx.arg(0) as u8;
    event.new_state = ctx.arg(1) as u8;

    event
}
