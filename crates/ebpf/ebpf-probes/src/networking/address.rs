use crate::networking::types::{AddressFamily, SocketAddress};

#[inline(always)]
pub fn extract_address(family: u32, ipv4: u32, ipv6: [u32; 4]) -> SocketAddress {
    match AddressFamily::from_u32(family) {
        AddressFamily::IPv4 => SocketAddress::ipv4(ipv4),

        AddressFamily::IPv6 => SocketAddress::ipv6(ipv6),

        AddressFamily::Unknown => SocketAddress::empty(),
    }
}

#[inline(always)]
pub fn extract_ipv4(family: u32, address: u32) -> Option<SocketAddress> {
    match AddressFamily::from_u32(family) {
        AddressFamily::IPv4 => Some(SocketAddress::ipv4(address)),

        _ => None,
    }
}

#[inline(always)]
pub fn extract_ipv6(family: u32, address: [u32; 4]) -> Option<SocketAddress> {
    match AddressFamily::from_u32(family) {
        AddressFamily::IPv6 => Some(SocketAddress::ipv6(address)),

        _ => None,
    }
}
