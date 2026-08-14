use aya_ebpf::bindings::{
    BPF_SOCK_OPS_PASSIVE_ESTABLISHED_CB, BPF_SOCK_OPS_STATE_CB, BPF_SOCK_OPS_TCP_CONNECT_CB,
    BPF_SOCK_OPS_TCP_LISTEN_CB,
};

use crate::networking::types::SocketEventKind;

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
