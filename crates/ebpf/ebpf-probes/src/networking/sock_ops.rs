use aya_ebpf::{helpers::bpf_get_current_pid_tgid, macros::sock_ops, programs::SockOpsContext};

use crate::networking::{
    events::{emit_sock_event, emit_sock_meta_event},
    socket::{socket_event, socket_metadata},
    types::{SocketEventKind, SocketMetadata},
};

#[inline(always)]
fn current_pid() -> u32 {
    bpf_get_current_pid_tgid() as u32
}

#[inline(always)]
fn current_tgid() -> u32 {
    (bpf_get_current_pid_tgid() >> 32) as u32
}

#[sock_ops]
pub fn socket_metadata_probe(ctx: SockOpsContext) -> u32 {
    match try_socket_metadata_probe(&ctx) {
        Ok(_) => 0,
        Err(_) => 0,
    }
}

#[inline(always)]
fn try_socket_metadata_probe(ctx: &SockOpsContext) -> Result<(), i64> {
    let mut metadata: SocketMetadata = socket_metadata(ctx);

    metadata.pid = current_pid();
    metadata.tgid = current_tgid();

    /*
     * SockOps programs execute in the socket context.
     *
     * At this point we have:
     *
     *   family
     *   local address
     *   remote address
     *   local port
     *   remote port
     *
     * The event can now be submitted through the event
     * transport used by the rest of the probe framework.
     */

    emit_sock_meta_event(&metadata)?;

    Ok(())
}

#[sock_ops]
pub fn socket_lifecycle(ctx: SockOpsContext) -> u32 {
    match try_socket_lifecycle(&ctx) {
        Ok(()) => 0,
        Err(_) => 0,
    }
}

#[inline(always)]
fn try_socket_lifecycle(ctx: &SockOpsContext) -> Result<(), i64> {
    if let Some(event) = socket_event(ctx) {
        match event.kind {
            SocketEventKind::TcpConnect
            | SocketEventKind::TcpPassiveEstablished
            | SocketEventKind::TcpListen
            | SocketEventKind::TcpStateChange => {
                emit_sock_event(event)?;
            }

            _ => {}
        }
    }

    Ok(())
}
