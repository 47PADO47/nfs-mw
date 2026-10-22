//! The source filter of the discovery responder (security-ops.md 2.3, rule 2). The design answers only sources on a
//! subnet of one of the host's interfaces; the standard library cannot list interfaces, so the caller passes the
//! subnets, and [`SourcePolicy::private_lan`] is the safe default (the private and link-local ranges). Unspecified,
//! multicast and broadcast sources and port 0 are always refused; loopback only when the policy says so.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// An address range: `addr` with its first `prefix` bits significant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Subnet {
    /// Any address in the range.
    pub addr: IpAddr,
    /// Significant leading bits (clamped to 32 or 128).
    pub prefix: u8,
}

impl Subnet {
    /// The range of `addr` with `prefix` significant bits.
    pub const fn new(addr: IpAddr, prefix: u8) -> Self {
        Self { addr, prefix }
    }

    /// Whether `ip` is in the range. An IPv4-mapped IPv6 address counts as IPv4.
    pub fn contains(&self, ip: IpAddr) -> bool {
        match (canonical(self.addr), canonical(ip)) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let mask = mask32(self.prefix);
                u32::from(net) & mask == u32::from(ip) & mask
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let mask = mask128(self.prefix);
                u128::from(net) & mask == u128::from(ip) & mask
            }
            _ => false,
        }
    }
}

/// Which sources the discovery responder answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePolicy {
    /// Sources in any of these ranges are answered.
    pub subnets: Vec<Subnet>,
    /// Whether loopback sources are answered (another process on the same machine).
    pub allow_loopback: bool,
}

impl SourcePolicy {
    /// The private and link-local ranges: `10/8`, `172.16/12`, `192.168/16`, `169.254/16`, `fc00::/7` and `fe80::/10`.
    /// Loopback is refused.
    pub fn private_lan() -> Self {
        let v4 = |a, b, prefix| Subnet::new(IpAddr::V4(Ipv4Addr::new(a, b, 0, 0)), prefix);
        let v6 = |head, prefix| Subnet::new(IpAddr::V6(Ipv6Addr::new(head, 0, 0, 0, 0, 0, 0, 0)), prefix);
        Self {
            subnets: vec![
                v4(10, 0, 8),
                v4(172, 16, 12),
                v4(192, 168, 16),
                v4(169, 254, 16),
                v6(0xfc00, 7),
                v6(0xfe80, 10),
            ],
            allow_loopback: false,
        }
    }

    /// Only loopback sources: the same machine (tests, two instances on one PC).
    pub fn loopback_only() -> Self {
        Self { subnets: Vec::new(), allow_loopback: true }
    }

    /// The subnets the caller read from its interfaces, plus loopback when `allow_loopback`.
    pub fn subnets(subnets: Vec<Subnet>, allow_loopback: bool) -> Self {
        Self { subnets, allow_loopback }
    }

    /// Whether a request from `from` may be answered.
    pub fn allows(&self, from: SocketAddr) -> bool {
        let ip = canonical(from.ip());
        if from.port() == 0 || ip.is_unspecified() || ip.is_multicast() || is_broadcast(ip) {
            return false;
        }
        if ip.is_loopback() {
            return self.allow_loopback;
        }
        self.subnets.iter().any(|subnet| subnet.contains(ip))
    }
}

/// An IPv4-mapped IPv6 address as IPv4; anything else unchanged.
fn canonical(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
        IpAddr::V4(_) => ip,
    }
}

fn is_broadcast(ip: IpAddr) -> bool {
    matches!(ip, IpAddr::V4(v4) if v4.is_broadcast())
}

fn mask32(prefix: u8) -> u32 {
    match u32::from(prefix.min(32)) {
        0 => 0,
        bits => u32::MAX << (32 - bits),
    }
}

fn mask128(prefix: u8) -> u128 {
    match u32::from(prefix.min(128)) {
        0 => 0,
        bits => u128::MAX << (128 - bits),
    }
}
