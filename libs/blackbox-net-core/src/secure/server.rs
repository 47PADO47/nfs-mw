//! The server side of the handshake (protocol.md section 4, security-ops.md 2.4), sans-IO. The caller routes data
//! packets to their connections by connection id and hands every handshake datagram to [`HandshakeServer::handle`],
//! sending back whatever it writes.
//!
//! - `Hello` costs one MAC: the `Challenge` is stateless, and none is sent under load (the gate's soft caps).
//! - `HelloCookie` is checked against the cookie in constant time before anything else; only then are the caps applied,
//!   a slot allocated and Noise run. A `Reject` goes only to an address that presented a valid cookie.
//! - Until `NoiseMsg3` verifies, a slot sends at most [`AMPLIFICATION_FACTOR`] times the bytes it received.
//! - A slot that does not finish in [`ServerConfig::handshake_timeout_ms`] is freed by [`HandshakeServer::expire`].

use std::collections::HashSet;
use std::net::SocketAddr;

use snow::HandshakeState;

use super::cookie::{CookieInput, CookieJar};
use super::crypto::random;
use super::noise::{self, Pattern};
use super::packet::{
    Challenge, Hello, HelloCookie, HsType, MAX_NOISE_MSG, NoiseMsg, PacketKind, RejectReason, classify,
};
use super::{
    AMPLIFICATION_FACTOR, Established, HANDSHAKE_TIMEOUT_MS, HandshakeError, IdentityKey, NoiseSealer, PeerInfo,
    SealConfig,
};
use crate::MAX_DATAGRAM;
use crate::ratelimit::{PreAuthGate, PreAuthLimits};

/// How a server handshakes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerConfig {
    /// The protocol major version served; a valid cookie with another one gets `Reject` reason 2.
    pub proto_major: u8,
    /// The Noise prologue, the same on both sides.
    pub prologue: Vec<u8>,
    /// Accept `XX` (LAN).
    pub accept_xx: bool,
    /// Accept `XK` (online).
    pub accept_xk: bool,
    /// The pre-authentication caps.
    pub limits: PreAuthLimits,
    /// Rekey settings of every sealer.
    pub seal: SealConfig,
    /// A half-open handshake is freed after this long.
    pub handshake_timeout_ms: u64,
}

impl ServerConfig {
    /// A LAN server: `XX` only, the LAN limits.
    pub fn lan(proto_major: u8, prologue: &[u8]) -> Self {
        Self {
            proto_major,
            prologue: prologue.to_vec(),
            accept_xx: true,
            accept_xk: false,
            limits: PreAuthLimits::LAN,
            seal: SealConfig::default(),
            handshake_timeout_ms: HANDSHAKE_TIMEOUT_MS,
        }
    }
}

/// Counters of every way a datagram ended, for `net.stats` and the logs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServerStats {
    /// `Challenge`s sent.
    pub challenges: u64,
    /// Handshakes completed.
    pub established: u64,
    /// `Reject`s sent.
    pub rejects: u64,
    /// Datagrams that failed to decode, or were not handshake packets a server takes.
    pub malformed: u64,
    /// `HelloCookie`s with an invalid or expired cookie.
    pub bad_cookies: u64,
    /// Datagrams dropped by a cap or rate limit, or from an ignored address.
    pub limited: u64,
    /// Noise steps that failed.
    pub noise_failures: u64,
    /// `NoiseMsg3`s for no known slot, or from another address.
    pub unknown: u64,
    /// Resends held back by the amplification limit.
    pub amplification_held: u64,
    /// Slots freed by the timeout.
    pub timeouts: u64,
}

/// What [`HandshakeServer::handle`] wants done.
#[derive(Debug)]
pub enum ServerAction {
    /// Nothing: the datagram was dropped (and counted) or needs no answer.
    None,
    /// Send the first `n` bytes of the output buffer to the sender.
    Reply(usize),
    /// A handshake finished; the connection is the caller's now.
    Established(Box<Established>),
}

struct Slot {
    conn_id: u16,
    addr: SocketAddr,
    client_conn_id: u16,
    client_nonce: [u8; 16],
    noise: Box<HandshakeState>,
    reply: Vec<u8>,
    received: usize,
    sent: usize,
    started_ms: u64,
}

/// The handshake responder of one server port.
pub struct HandshakeServer {
    config: ServerConfig,
    identity: IdentityKey,
    cookies: CookieJar,
    gate: PreAuthGate,
    slots: Vec<Slot>,
    ids: HashSet<u16>,
    next_id: u16,
    shutting_down: bool,
    stats: ServerStats,
}

impl HandshakeServer {
    /// A server with a random cookie secret.
    pub fn new(config: ServerConfig, identity: IdentityKey, now_ms: u64) -> Result<Self, HandshakeError> {
        let secret = random::<32>()?;
        Ok(Self::with_cookie_secret(config, identity, secret, now_ms))
    }

    /// A server with a given cookie secret (tests, or a secret the caller draws).
    pub fn with_cookie_secret(config: ServerConfig, identity: IdentityKey, secret: [u8; 32], now_ms: u64) -> Self {
        let gate = PreAuthGate::new(config.limits);
        Self {
            config,
            identity,
            cookies: CookieJar::new(secret, now_ms),
            gate,
            slots: Vec::new(),
            ids: HashSet::new(),
            next_id: 1,
            shutting_down: false,
            stats: ServerStats::default(),
        }
    }

    /// The server's identity (its public key is what clients pin).
    pub fn identity(&self) -> &IdentityKey {
        &self.identity
    }

    /// The counters.
    pub fn stats(&self) -> &ServerStats {
        &self.stats
    }

    /// Half-open handshakes.
    pub fn half_open(&self) -> usize {
        self.slots.len()
    }

    /// The address gate, shared with the discovery responder and the authorisation step.
    pub fn gate_mut(&mut self) -> &mut PreAuthGate {
        &mut self.gate
    }

    /// From now on a valid cookie gets `Reject` reason 3.
    pub fn set_shutting_down(&mut self, shutting_down: bool) {
        self.shutting_down = shutting_down;
    }

    /// The caller closed the connection `conn_id` from `addr`: its id and its place in the address cap are free.
    pub fn connection_closed(&mut self, conn_id: u16, addr: SocketAddr) {
        if self.ids.remove(&conn_id) {
            self.gate.connection_closed(addr.ip());
        }
    }

    /// Records an authorisation failure from `addr` (bad token, bad join, wrong password).
    pub fn auth_failed(&mut self, now_ms: u64, addr: SocketAddr) {
        self.gate.auth_failed(now_ms, addr.ip());
    }

    /// Makes `secret` the current cookie secret, keeping the old one.
    pub fn rotate_cookie_secret(&mut self, secret: [u8; 32], now_ms: u64) {
        self.cookies.rotate(secret, now_ms);
    }

    /// Frees half-open slots older than the timeout and rotates the cookie secret when an hour has passed (keeping the
    /// old secret if the random source fails). Returns the slots freed.
    pub fn expire(&mut self, now_ms: u64) -> usize {
        let timeout = self.config.handshake_timeout_ms;
        let before = self.slots.len();
        self.slots.retain(|slot| now_ms.saturating_sub(slot.started_ms) < timeout);
        let freed = before - self.slots.len();
        self.stats.timeouts += freed as u64;
        if self.cookies.rotation_due(now_ms)
            && let Ok(secret) = random::<32>()
        {
            self.cookies.rotate(secret, now_ms);
        }
        freed
    }

    /// Takes one handshake datagram from `from`; `out` must hold [`MAX_DATAGRAM`] bytes.
    pub fn handle(&mut self, now_ms: u64, from: SocketAddr, datagram: &[u8], out: &mut [u8]) -> ServerAction {
        if out.len() < MAX_DATAGRAM {
            return ServerAction::None;
        }
        if self.gate.is_ignored(now_ms, from.ip()) {
            self.stats.limited += 1;
            return ServerAction::None;
        }
        let action = match classify(datagram) {
            PacketKind::Handshake(HsType::Hello) => self.on_hello(now_ms, from, datagram, out),
            PacketKind::Handshake(HsType::HelloCookie) => self.on_hello_cookie(now_ms, from, datagram, out),
            PacketKind::Handshake(HsType::NoiseMsg3) => self.on_msg3(from, datagram),
            _ => Err(Dropped::Malformed),
        };
        match action {
            Ok(action) => action,
            Err(drop) => {
                self.count(drop);
                ServerAction::None
            }
        }
    }

    fn count(&mut self, drop: Dropped) {
        let counter = match drop {
            Dropped::Malformed => &mut self.stats.malformed,
            Dropped::Cookie => &mut self.stats.bad_cookies,
            Dropped::Limited => &mut self.stats.limited,
            Dropped::Noise => &mut self.stats.noise_failures,
            Dropped::Unknown => &mut self.stats.unknown,
            Dropped::Amplification => &mut self.stats.amplification_held,
        };
        *counter += 1;
    }

    fn on_hello(
        &mut self,
        now_ms: u64,
        from: SocketAddr,
        datagram: &[u8],
        out: &mut [u8],
    ) -> Result<ServerAction, Dropped> {
        let hello = Hello::decode(datagram).map_err(|_| Dropped::Malformed)?;
        if !self.gate.hello_allowed(now_ms, from.ip(), self.slots.len()) {
            return Err(Dropped::Limited);
        }
        let server_conn_id = self.pick_id().ok_or(Dropped::Limited)?;
        let input = CookieInput {
            addr: from,
            client_nonce: &hello.client_nonce,
            client_conn_id: hello.client_conn_id,
            server_conn_id,
        };
        let (bucket, cookie) = self.cookies.issue(now_ms, &input);
        let packet = Challenge { server_conn_id, bucket, cookie }.encode();
        self.stats.challenges += 1;
        write(out, &packet)
    }

    fn on_hello_cookie(
        &mut self,
        now_ms: u64,
        from: SocketAddr,
        datagram: &[u8],
        out: &mut [u8],
    ) -> Result<ServerAction, Dropped> {
        let hello = HelloCookie::decode(datagram).map_err(|_| Dropped::Malformed)?;
        let input = CookieInput {
            addr: from,
            client_nonce: &hello.client_nonce,
            client_conn_id: hello.client_conn_id,
            server_conn_id: hello.server_conn_id,
        };
        if !self.cookies.verify(now_ms, &input, hello.bucket, &hello.cookie) {
            return Err(Dropped::Cookie);
        }
        if let Some(slot) = self.slots.iter_mut().find(|slot| slot.conn_id == hello.server_conn_id) {
            return resend(slot, from, &hello, datagram.len(), out);
        }
        if self.ids.contains(&hello.server_conn_id) || hello.server_conn_id == 0 {
            return Err(Dropped::Unknown);
        }
        if hello.proto_major != self.config.proto_major {
            return self.reject(RejectReason::Version, out);
        }
        if self.shutting_down {
            return self.reject(RejectReason::ShuttingDown, out);
        }
        let accepted = match hello.pattern {
            Pattern::Xx => self.config.accept_xx,
            Pattern::Xk => self.config.accept_xk,
        };
        if !accepted {
            return Err(Dropped::Limited);
        }
        if self.gate.is_full(self.slots.len()) {
            return self.reject(RejectReason::Full, out);
        }
        if !self.gate.handshake_allowed(now_ms, from.ip(), self.slots.len()) {
            return Err(Dropped::Limited);
        }
        let mut noise =
            noise::responder(hello.pattern, &self.config.prologue, &self.identity).map_err(|_| Dropped::Noise)?;
        let mut scratch = [0u8; MAX_NOISE_MSG];
        noise.read_message(hello.noise, &mut scratch).map_err(|_| Dropped::Noise)?;
        let mut msg2 = [0u8; MAX_NOISE_MSG];
        let len = noise.write_message(&[], &mut msg2).map_err(|_| Dropped::Noise)?;
        let mut reply = vec![0u8; MAX_DATAGRAM];
        let message = NoiseMsg { conn_id: hello.client_conn_id, msg: msg2.get(..len).ok_or(Dropped::Noise)? };
        let size = message.encode(HsType::NoiseMsg2, &mut reply).map_err(|_| Dropped::Noise)?;
        reply.truncate(size);
        let mut slot = Slot {
            conn_id: hello.server_conn_id,
            addr: from,
            client_conn_id: hello.client_conn_id,
            client_nonce: hello.client_nonce,
            noise: Box::new(noise),
            reply,
            received: datagram.len(),
            sent: 0,
            started_ms: now_ms,
        };
        let action = send_reply(&mut slot, out);
        self.slots.push(slot);
        action
    }

    fn on_msg3(&mut self, from: SocketAddr, datagram: &[u8]) -> Result<ServerAction, Dropped> {
        let msg = NoiseMsg::decode(datagram, HsType::NoiseMsg3).map_err(|_| Dropped::Malformed)?;
        let index = self.slots.iter().position(|slot| slot.conn_id == msg.conn_id).ok_or(Dropped::Unknown)?;
        let Some(slot) = self.slots.get_mut(index) else {
            return Err(Dropped::Unknown);
        };
        if slot.addr != from {
            return Err(Dropped::Unknown);
        }
        let mut scratch = [0u8; MAX_NOISE_MSG];
        let read = slot.noise.read_message(msg.msg, &mut scratch);
        let finished = read.map_err(HandshakeError::from).and_then(|_| noise::finish(&mut slot.noise));
        let slot = self.slots.swap_remove(index);
        let Ok(finished) = finished else {
            return Err(Dropped::Noise);
        };
        self.ids.insert(slot.conn_id);
        self.gate.connection_opened(from.ip());
        self.stats.established += 1;
        let sealer =
            NoiseSealer::new(finished.responder_to_initiator, finished.initiator_to_responder, self.config.seal);
        let info = PeerInfo {
            addr: from,
            local_conn_id: slot.conn_id,
            remote_conn_id: slot.client_conn_id,
            remote_static: finished.remote_static,
            handshake_hash: finished.handshake_hash,
        };
        Ok(ServerAction::Established(Box::new(Established { info, sealer })))
    }

    fn reject(&mut self, reason: RejectReason, out: &mut [u8]) -> Result<ServerAction, Dropped> {
        self.stats.rejects += 1;
        write(out, &reason.encode())
    }

    /// A connection id not used by a slot or an established connection, and not 0.
    fn pick_id(&mut self) -> Option<u16> {
        for _ in 0..=u16::MAX {
            let candidate = self.next_id;
            self.next_id = self.next_id.wrapping_add(1).max(1);
            let in_slot = self.slots.iter().any(|slot| slot.conn_id == candidate);
            if candidate != 0 && !in_slot && !self.ids.contains(&candidate) {
                return Some(candidate);
            }
        }
        None
    }
}

/// Why a datagram was dropped.
#[derive(Debug, Clone, Copy)]
enum Dropped {
    Malformed,
    Cookie,
    Limited,
    Noise,
    Unknown,
    Amplification,
}

/// A repeated `HelloCookie` for a running slot: the same client gets the same `NoiseMsg2` again, within the
/// amplification limit; anything else is dropped.
fn resend(
    slot: &mut Slot,
    from: SocketAddr,
    hello: &HelloCookie<'_>,
    len: usize,
    out: &mut [u8],
) -> Result<ServerAction, Dropped> {
    let same =
        slot.addr == from && slot.client_nonce == hello.client_nonce && slot.client_conn_id == hello.client_conn_id;
    if !same {
        return Err(Dropped::Unknown);
    }
    slot.received = slot.received.saturating_add(len);
    send_reply(slot, out)
}

fn send_reply(slot: &mut Slot, out: &mut [u8]) -> Result<ServerAction, Dropped> {
    let budget = slot.received.saturating_mul(AMPLIFICATION_FACTOR);
    if slot.sent.saturating_add(slot.reply.len()) > budget {
        return Err(Dropped::Amplification);
    }
    slot.sent += slot.reply.len();
    write(out, &slot.reply)
}

fn write(out: &mut [u8], bytes: &[u8]) -> Result<ServerAction, Dropped> {
    let dst = out.get_mut(..bytes.len()).ok_or(Dropped::Malformed)?;
    dst.copy_from_slice(bytes);
    Ok(ServerAction::Reply(bytes.len()))
}

impl std::fmt::Debug for HandshakeServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandshakeServer")
            .field("half_open", &self.slots.len())
            .field("established", &self.ids.len())
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}
