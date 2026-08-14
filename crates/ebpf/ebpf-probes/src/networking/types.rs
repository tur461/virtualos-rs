#![allow(dead_code)]

use core::mem;

pub const AF_INET: u16 = 2;
pub const AF_INET6: u16 = 10;

pub const IPPROTO_TCP: u8 = 6;
pub const IPPROTO_UDP: u8 = 17;

pub const SOCK_STREAM: u32 = 1;
pub const SOCK_DGRAM: u32 = 2;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum AddressFamily {
    Unknown = 0,
    IPv4 = 4,
    IPv6 = 6,
}

impl AddressFamily {
    #[inline(always)]
    pub const fn from_u32(family: u32) -> Self {
        match family as u16 {
            AF_INET => Self::IPv4,
            AF_INET6 => Self::IPv6,
            _ => Self::Unknown,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IPv4Address {
    pub addr: u32,
}

impl IPv4Address {
    #[inline(always)]
    pub const fn new(addr: u32) -> Self {
        Self { addr }
    }

    #[inline(always)]
    pub const fn octets(self) -> [u8; 4] {
        self.addr.to_be_bytes()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct IPv6Address {
    pub addr: [u32; 4],
}

impl IPv6Address {
    #[inline(always)]
    pub const fn new(addr: [u32; 4]) -> Self {
        Self { addr }
    }

    #[inline(always)]
    pub fn octets(self) -> [u8; 16] {
        let mut out = [0u8; 16];

        let a = self.addr[0].to_be_bytes();
        let b = self.addr[1].to_be_bytes();
        let c = self.addr[2].to_be_bytes();
        let d = self.addr[3].to_be_bytes();

        out[0..4].copy_from_slice(&a);
        out[4..8].copy_from_slice(&b);
        out[8..12].copy_from_slice(&c);
        out[12..16].copy_from_slice(&d);

        out
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SocketAddress {
    pub family: AddressFamily,
    pub _pad: [u8; 3],

    // Always 16 bytes so the event has a stable ABI.
    pub address: [u8; 16],
}

impl SocketAddress {
    #[inline(always)]
    pub fn empty() -> Self {
        Self {
            family: AddressFamily::Unknown,
            _pad: [0; 3],
            address: [0; 16],
        }
    }

    #[inline(always)]
    pub fn ipv4(addr: u32) -> Self {
        let mut address = [0u8; 16];

        // Store IPv4 in the first four bytes.
        address[0..4].copy_from_slice(&addr.to_be_bytes());

        Self {
            family: AddressFamily::IPv4,
            _pad: [0; 3],
            address,
        }
    }

    #[inline(always)]
    pub fn ipv6(addr: [u32; 4]) -> Self {
        let mut address = [0u8; 16];

        address[0..4].copy_from_slice(&addr[0].to_be_bytes());
        address[4..8].copy_from_slice(&addr[1].to_be_bytes());
        address[8..12].copy_from_slice(&addr[2].to_be_bytes());
        address[12..16].copy_from_slice(&addr[3].to_be_bytes());

        Self {
            family: AddressFamily::IPv6,
            _pad: [0; 3],
            address,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SocketMetadata {
    pub pid: u32,
    pub tgid: u32,
    pub uid: u32,
    pub gid: u32,

    pub family: AddressFamily,

    pub protocol: u8,

    pub _pad: [u8; 2],

    pub local_port: u16,
    pub remote_port: u16,

    pub local_addr: [u8; 16],
    pub remote_addr: [u8; 16],

    pub socket_cookie: u64,
}

impl SocketMetadata {
    #[inline(always)]
    pub fn zeroed() -> Self {
        unsafe { mem::zeroed() }
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum TransportProtocol {
    Unknown = 0,
    Tcp = 6,
    Udp = 17,
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum SocketEventKind {
    Unknown = 0,

    TcpConnect = 1,
    TcpPassiveEstablished = 2,
    TcpListen = 3,
    TcpStateChange = 4,
    TcpClose = 5,

    UdpSend = 10,
    UdpReceive = 11,
    UdpBind = 12,
    UdpConnect = 13,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SocketEvent {
    pub timestamp_ns: u64,

    pub pid: u32,
    pub tgid: u32,
    pub uid: u32,
    pub gid: u32,

    pub socket_cookie: u64,

    pub family: AddressFamily,
    pub protocol: TransportProtocol,
    pub kind: SocketEventKind,

    pub old_state: u8,
    pub new_state: u8,

    pub _pad: [u8; 2],

    pub local_port: u16,
    pub remote_port: u16,

    pub local_addr: [u8; 16],
    pub remote_addr: [u8; 16],

    pub bytes: u64,
}

impl SocketEvent {
    #[inline(always)]
    pub fn zeroed() -> Self {
        unsafe { core::mem::zeroed() }
    }

    pub fn empty() -> Self {
        SocketEvent {
            timestamp_ns: 0,
            pid: 0,
            tgid: 0,
            uid: 0,
            gid: 0,

            socket_cookie: 0,

            family: AddressFamily::Unknown,
            protocol: TransportProtocol::Unknown,
            kind: SocketEventKind::Unknown,

            old_state: 0,
            new_state: 0,

            _pad: [0u8; 2],

            local_addr: [0u8; 16],
            local_port: 0,

            remote_addr: [0u8; 16],
            remote_port: 0,

            bytes: 0,
        }
    }
}
