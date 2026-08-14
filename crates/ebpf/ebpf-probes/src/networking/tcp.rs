use aya_ebpf::bindings::{
    BPF_TCP_CLOSE, BPF_TCP_CLOSE_WAIT, BPF_TCP_CLOSING, BPF_TCP_ESTABLISHED, BPF_TCP_FIN_WAIT1,
    BPF_TCP_FIN_WAIT2, BPF_TCP_LAST_ACK, BPF_TCP_LISTEN, BPF_TCP_SYN_RECV, BPF_TCP_SYN_SENT,
    BPF_TCP_TIME_WAIT,
};

#[inline(always)]
pub const fn tcp_state_name(state: u32) -> &'static [u8] {
    match state {
        BPF_TCP_ESTABLISHED => b"ESTABLISHED",
        BPF_TCP_SYN_SENT => b"SYN_SENT",
        BPF_TCP_SYN_RECV => b"SYN_RECV",
        BPF_TCP_FIN_WAIT1 => b"FIN_WAIT1",
        BPF_TCP_FIN_WAIT2 => b"FIN_WAIT2",
        BPF_TCP_TIME_WAIT => b"TIME_WAIT",
        BPF_TCP_CLOSE => b"CLOSE",
        BPF_TCP_CLOSE_WAIT => b"CLOSE_WAIT",
        BPF_TCP_LAST_ACK => b"LAST_ACK",
        BPF_TCP_LISTEN => b"LISTEN",
        BPF_TCP_CLOSING => b"CLOSING",
        _ => b"UNKNOWN",
    }
}
