use std::collections::HashMap;
use std::net::IpAddr;

use super::{FailureLimiter, FailureRule, KeyedLimiter, TokenBucket};

/// The numbers of security-ops.md sections 2.1 to 2.4. Starting values, to tune with load tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreAuthLimits {
    /// Established connections from one source address.
    pub conns_per_ip: u32,
    /// New handshakes (slots allocated after a valid cookie) per source address, per `handshake_period_ms`.
    pub handshakes_per_ip: u32,
    /// The period of `handshakes_per_ip`.
    pub handshake_period_ms: u64,
    /// Half-open handshakes in total; a valid cookie beyond this is dropped.
    pub half_open_total: usize,
    /// Above this many half-open handshakes, `Hello` gets no `Challenge` at all.
    pub half_open_soft: usize,
    /// Above this many new handshakes per second in total, `Hello` gets no `Challenge` at all.
    pub new_handshakes_per_s: u32,
    /// Connections in total, the session's hard cap.
    pub max_connections: usize,
    /// Authorisation failures per address before the address is ignored.
    pub auth_failures: FailureRule,
    /// Discovery replies per second in total.
    pub discovery_per_s: u32,
    /// Discovery replies per second to one source.
    pub discovery_per_source_per_s: u32,
    /// Inbound bytes per second from one peer before excess datagrams are dropped undecoded.
    pub inbound_bytes_per_s: u32,
    /// Datagrams decoded per peer per tick.
    pub datagrams_per_tick: u32,
    /// Source addresses tracked by each per-address table; beyond it new addresses are refused.
    pub max_tracked: usize,
}

impl PreAuthLimits {
    /// LAN values: 4 connections and 5 handshakes per 10 s per address, 64 half-open, 24 connections.
    pub const LAN: Self = Self {
        conns_per_ip: 4,
        handshakes_per_ip: 5,
        handshake_period_ms: 10_000,
        half_open_total: 64,
        half_open_soft: 32,
        new_handshakes_per_s: 8,
        max_connections: 24,
        auth_failures: FailureRule::AUTH,
        discovery_per_s: 10,
        discovery_per_source_per_s: 2,
        inbound_bytes_per_s: 128 * 1024,
        datagrams_per_tick: 128,
        max_tracked: 4096,
    };

    /// Online values: 2 connections and 3 handshakes per 10 s per address, 256 half-open, 48 connections.
    pub const ONLINE: Self = Self {
        conns_per_ip: 2,
        handshakes_per_ip: 3,
        half_open_total: 256,
        max_connections: 48,
        max_tracked: 65_536,
        ..Self::LAN
    };
}

/// The address-level gate in front of the handshake and discovery: every check is a few comparisons, and a refusal
/// means a silent drop. Accounting is by address because nothing better is known before authentication.
#[derive(Debug, Clone)]
pub struct PreAuthGate {
    limits: PreAuthLimits,
    handshakes: KeyedLimiter<IpAddr>,
    new_handshakes: TokenBucket,
    auth: FailureLimiter<IpAddr>,
    discovery: TokenBucket,
    discovery_per_source: KeyedLimiter<IpAddr>,
    conns: HashMap<IpAddr, u32>,
    conns_total: usize,
}

impl PreAuthGate {
    /// A gate with every bucket full.
    pub fn new(limits: PreAuthLimits) -> Self {
        let per_source = limits.discovery_per_source_per_s;
        Self {
            limits,
            handshakes: KeyedLimiter::new(
                limits.handshakes_per_ip,
                limits.handshakes_per_ip,
                limits.handshake_period_ms,
                limits.max_tracked,
            ),
            new_handshakes: TokenBucket::new(limits.new_handshakes_per_s, limits.new_handshakes_per_s, 1000),
            auth: FailureLimiter::new(limits.auth_failures, limits.max_tracked),
            discovery: TokenBucket::new(limits.discovery_per_s, limits.discovery_per_s, 1000),
            discovery_per_source: KeyedLimiter::new(per_source, per_source, 1000, limits.max_tracked),
            conns: HashMap::new(),
            conns_total: 0,
        }
    }

    /// The limits in force.
    pub fn limits(&self) -> &PreAuthLimits {
        &self.limits
    }

    /// Whether anything from `ip` is ignored (too many authorisation failures).
    pub fn is_ignored(&self, now_ms: u64, ip: IpAddr) -> bool {
        self.auth.is_locked(now_ms, &ip)
    }

    /// Whether a `Hello` from `ip` gets a `Challenge`: not under load (`half_open` above the soft cap or the global
    /// new-handshake rate spent) and not from an ignored address. Takes nothing.
    pub fn hello_allowed(&self, now_ms: u64, ip: IpAddr, half_open: usize) -> bool {
        if half_open > self.limits.half_open_soft || self.is_ignored(now_ms, ip) {
            return false;
        }
        self.new_handshakes.would_allow(now_ms, 1)
    }

    /// Whether a sender with a valid cookie may get a handshake slot: below the half-open cap, within its address's
    /// handshake rate and connection cap, and within the global rate. Takes a token from both rates when it may.
    pub fn handshake_allowed(&mut self, now_ms: u64, ip: IpAddr, half_open: usize) -> bool {
        if half_open >= self.limits.half_open_total || self.is_ignored(now_ms, ip) {
            return false;
        }
        if self.conns.get(&ip).copied().unwrap_or(0) >= self.limits.conns_per_ip {
            return false;
        }
        if !self.handshakes.would_allow(now_ms, ip, 1) || !self.new_handshakes.would_allow(now_ms, 1) {
            return false;
        }
        self.handshakes.try_take(now_ms, ip, 1) && self.new_handshakes.try_take(now_ms, 1)
    }

    /// Whether a new handshake would pass the connection cap, counting the `half_open` handshakes already running (the
    /// `Reject` reason "server full").
    pub fn is_full(&self, half_open: usize) -> bool {
        self.conns_total + half_open >= self.limits.max_connections
    }

    /// Counts an established connection from `ip`.
    pub fn connection_opened(&mut self, ip: IpAddr) {
        *self.conns.entry(ip).or_insert(0) += 1;
        self.conns_total += 1;
    }

    /// Forgets an established connection from `ip`.
    pub fn connection_closed(&mut self, ip: IpAddr) {
        let Some(count) = self.conns.get_mut(&ip) else {
            return;
        };
        *count = count.saturating_sub(1);
        self.conns_total = self.conns_total.saturating_sub(1);
        if *count == 0 {
            self.conns.remove(&ip);
        }
    }

    /// Records a failed authorisation (a bad token, a bad join, a wrong password) from `ip`.
    pub fn auth_failed(&mut self, now_ms: u64, ip: IpAddr) {
        self.auth.record_failure(now_ms, ip);
    }

    /// Whether a discovery reply may go to `ip` (both the total and the per-source rate). Takes a token when it may.
    pub fn discovery_reply_allowed(&mut self, now_ms: u64, ip: IpAddr) -> bool {
        if !self.discovery.would_allow(now_ms, 1) || !self.discovery_per_source.would_allow(now_ms, ip, 1) {
            return false;
        }
        self.discovery.try_take(now_ms, 1) && self.discovery_per_source.try_take(now_ms, ip, 1)
    }
}

/// The inbound budget of one connection (security-ops.md 2.1): bytes per second and datagrams per tick. A datagram
/// over budget is dropped before it is opened or decoded.
#[derive(Debug, Clone, Copy)]
pub struct PeerBudget {
    bytes: TokenBucket,
    per_tick: u32,
    used_this_tick: u32,
}

impl PeerBudget {
    /// A budget from the limits.
    pub fn new(limits: &PreAuthLimits) -> Self {
        Self {
            bytes: TokenBucket::new(limits.inbound_bytes_per_s, limits.inbound_bytes_per_s, 1000),
            per_tick: limits.datagrams_per_tick,
            used_this_tick: 0,
        }
    }

    /// Starts a new tick: the per-tick datagram count goes back to zero.
    pub fn start_tick(&mut self) {
        self.used_this_tick = 0;
    }

    /// Whether a datagram of `len` bytes may be decoded now; counts it when it may.
    pub fn accept(&mut self, now_ms: u64, len: usize) -> bool {
        if self.used_this_tick >= self.per_tick {
            return false;
        }
        let len = u32::try_from(len).unwrap_or(u32::MAX);
        if !self.bytes.try_take(now_ms, len) {
            return false;
        }
        self.used_this_tick += 1;
        true
    }
}
