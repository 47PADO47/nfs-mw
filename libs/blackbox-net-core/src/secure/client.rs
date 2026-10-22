//! The client side of the handshake (protocol.md section 4), sans-IO. The caller sends what
//! [`HandshakeClient::poll_transmit`] writes to the server address and feeds every datagram from it to
//! [`HandshakeClient::handle`].
//!
//! Stages: `Hello` until a `Challenge`; `HelloCookie` (the same bytes on every send,
//! carrying Noise message 1) until `NoiseMsg2`; then the keys exist and `NoiseMsg3` is resent until the caller reports
//! the first sealed packet from the server with [`HandshakeClient::confirm`]. Each stage sends at once and then after
//! each gap of [`ClientConfig::retry_ms`]; when the last gap passes with no progress the handshake fails with
//! [`HandshakeError::NoAnswer`].
//!
//! The client nonce is drawn once per handshake, not per `Hello` resend: a `Challenge` does not say which `Hello` it
//! answers, so a late answer to an earlier send must still carry a cookie for the nonce the client will echo. A new
//! handshake (a new [`HandshakeClient`]) draws a new nonce.

use std::net::SocketAddr;

use snow::HandshakeState;

use super::crypto::random;
use super::noise::{self, Pattern};
use super::packet::{Challenge, Hello, HelloCookie, HsType, NoiseMsg, PacketKind, RejectReason, classify};
use super::{Established, HandshakeError, IdentityKey, NoiseSealer, PeerInfo, SealConfig};
use crate::MAX_DATAGRAM;

/// How a client handshakes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientConfig {
    /// The protocol major version sent in `Hello`.
    pub proto_major: u8,
    /// The Noise prologue: the protocol name and major version, the same on both sides.
    pub prologue: Vec<u8>,
    /// `XX` on a LAN, `XK` against a pinned server key.
    pub pattern: Pattern,
    /// The server's static key, required for `XK`.
    pub server_key: Option<[u8; 32]>,
    /// Waits after each send of a stage. The default sends at 0, 0.5 s, 1 s and 2 s and gives up at 4 s.
    pub retry_ms: Vec<u64>,
    /// Rekey settings of the resulting sealer.
    pub seal: SealConfig,
}

impl ClientConfig {
    /// A LAN client (`XX`) with the default retries.
    pub fn lan(proto_major: u8, prologue: &[u8]) -> Self {
        Self {
            proto_major,
            prologue: prologue.to_vec(),
            pattern: Pattern::Xx,
            server_key: None,
            retry_ms: vec![500, 500, 1000, 2000],
            seal: SealConfig::default(),
        }
    }
}

/// Where a client handshake stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientStatus {
    /// Still running.
    InProgress,
    /// The keys exist; take them with [`HandshakeClient::take_established`] (once), keep polling until confirmed.
    Established,
    /// Done: the server's first sealed packet confirmed the keys.
    Confirmed,
    /// Failed for good.
    Failed(HandshakeError),
}

enum Stage {
    Hello,
    Cookie { packet: Box<[u8; super::packet::HELLO_LEN]>, noise: Box<HandshakeState>, server_conn_id: u16 },
    Confirm { packet: Vec<u8> },
    Confirmed,
    Failed(HandshakeError),
}

/// One client handshake with one server.
pub struct HandshakeClient {
    config: ClientConfig,
    identity: IdentityKey,
    server: SocketAddr,
    conn_id: u16,
    nonce: [u8; 16],
    stage: Stage,
    sends: usize,
    next_send_ms: Option<u64>,
    established: Option<Established>,
}

impl HandshakeClient {
    /// A handshake from `identity` to `server`, answering to `client_conn_id`. Nothing is sent until the first poll.
    /// Fails with [`HandshakeError::Config`] for `XK` without a server key, or [`HandshakeError::Random`].
    pub fn new(
        config: ClientConfig,
        identity: &IdentityKey,
        server: SocketAddr,
        client_conn_id: u16,
    ) -> Result<Self, HandshakeError> {
        if config.pattern == Pattern::Xk && config.server_key.is_none() {
            return Err(HandshakeError::Config);
        }
        Ok(Self {
            config,
            identity: identity.clone(),
            server,
            conn_id: client_conn_id,
            nonce: random::<16>()?,
            stage: Stage::Hello,
            sends: 0,
            next_send_ms: None,
            established: None,
        })
    }

    /// Where the handshake stands.
    pub fn status(&self) -> ClientStatus {
        match &self.stage {
            Stage::Hello | Stage::Cookie { .. } => ClientStatus::InProgress,
            Stage::Confirm { .. } => ClientStatus::Established,
            Stage::Confirmed => ClientStatus::Confirmed,
            Stage::Failed(err) => ClientStatus::Failed(*err),
        }
    }

    /// The server address every datagram goes to.
    pub fn server(&self) -> SocketAddr {
        self.server
    }

    /// Writes the datagram due at `now_ms` into `out` (at least [`MAX_DATAGRAM`] bytes) and returns its length, or
    /// `None` when nothing is due. Moves the handshake to [`ClientStatus::Failed`] when the retries run out.
    pub fn poll_transmit(&mut self, now_ms: u64, out: &mut [u8]) -> Option<usize> {
        if out.len() < MAX_DATAGRAM || self.next_send_ms.is_some_and(|due| now_ms < due) {
            return None;
        }
        if matches!(self.stage, Stage::Confirmed | Stage::Failed(_)) {
            return None;
        }
        let Some(&wait) = self.config.retry_ms.get(self.sends) else {
            self.stage = Stage::Failed(HandshakeError::NoAnswer);
            return None;
        };
        let written = self.write_current(out);
        let len = match written {
            Ok(len) => len,
            Err(err) => {
                self.stage = Stage::Failed(err);
                return None;
            }
        };
        self.sends += 1;
        self.next_send_ms = Some(now_ms.saturating_add(wait));
        Some(len)
    }

    /// Takes one datagram from `from`. Anything not from the server, not expected in this stage or malformed is
    /// ignored; returns the status afterwards.
    pub fn handle(&mut self, from: SocketAddr, datagram: &[u8]) -> ClientStatus {
        if from != self.server {
            return self.status();
        }
        let result = match (classify(datagram), &self.stage) {
            (PacketKind::Handshake(HsType::Challenge), Stage::Hello) => self.on_challenge(datagram),
            (PacketKind::Handshake(HsType::NoiseMsg2), Stage::Cookie { .. }) => self.on_msg2(datagram),
            (PacketKind::Handshake(HsType::Reject), Stage::Cookie { .. }) => on_reject(datagram),
            _ => Ok(()),
        };
        if let Err(err) = result {
            self.fail_unless_malformed(err);
        }
        self.status()
    }

    /// The keys and peer once [`ClientStatus::Established`]; `None` before and after the first call.
    pub fn take_established(&mut self) -> Option<Established> {
        self.established.take()
    }

    /// The caller opened a sealed packet from the server: the keys are confirmed and `NoiseMsg3` stops.
    pub fn confirm(&mut self) {
        if matches!(self.stage, Stage::Confirm { .. }) {
            self.stage = Stage::Confirmed;
        }
    }

    fn enter(&mut self, stage: Stage) {
        self.stage = stage;
        self.sends = 0;
        self.next_send_ms = None;
    }

    /// A malformed datagram is dropped (it may be forged); a Noise failure or a `Reject` ends the handshake.
    fn fail_unless_malformed(&mut self, err: HandshakeError) {
        if matches!(err, HandshakeError::Packet(_)) {
            return;
        }
        self.enter(Stage::Failed(err));
    }

    fn write_current(&mut self, out: &mut [u8]) -> Result<usize, HandshakeError> {
        match &self.stage {
            Stage::Hello => {
                let hello = Hello {
                    proto_major: self.config.proto_major,
                    client_nonce: self.nonce,
                    client_conn_id: self.conn_id,
                };
                copy_out(out, &hello.encode())
            }
            Stage::Cookie { packet, .. } => copy_out(out, &packet[..]),
            Stage::Confirm { packet } => copy_out(out, packet),
            Stage::Confirmed | Stage::Failed(_) => Ok(0),
        }
    }

    fn on_challenge(&mut self, datagram: &[u8]) -> Result<(), HandshakeError> {
        let challenge = Challenge::decode(datagram)?;
        let server_key = self.config.server_key.as_ref();
        let mut noise = noise::initiator(self.config.pattern, &self.config.prologue, &self.identity, server_key)?;
        let mut msg1 = [0u8; super::packet::MAX_NOISE_MSG];
        let len = noise.write_message(&[], &mut msg1)?;
        let hello = HelloCookie {
            proto_major: self.config.proto_major,
            pattern: self.config.pattern,
            client_nonce: self.nonce,
            client_conn_id: self.conn_id,
            server_conn_id: challenge.server_conn_id,
            bucket: challenge.bucket,
            cookie: challenge.cookie,
            noise: msg1.get(..len).ok_or(HandshakeError::Noise)?,
        };
        let packet = Box::new(hello.encode()?);
        self.enter(Stage::Cookie { packet, noise: Box::new(noise), server_conn_id: challenge.server_conn_id });
        Ok(())
    }

    fn on_msg2(&mut self, datagram: &[u8]) -> Result<(), HandshakeError> {
        let msg = NoiseMsg::decode(datagram, HsType::NoiseMsg2)?;
        if msg.conn_id != self.conn_id {
            return Ok(());
        }
        let Stage::Cookie { noise, server_conn_id, .. } = &mut self.stage else {
            return Ok(());
        };
        let server_conn_id = *server_conn_id;
        let mut scratch = [0u8; super::packet::MAX_NOISE_MSG];
        noise.read_message(msg.msg, &mut scratch)?;
        let mut msg3 = [0u8; super::packet::MAX_NOISE_MSG];
        let len = noise.write_message(&[], &mut msg3)?;
        let finished = noise::finish(noise)?;
        let mut packet = vec![0u8; MAX_DATAGRAM];
        let reply = NoiseMsg { conn_id: server_conn_id, msg: msg3.get(..len).ok_or(HandshakeError::Noise)? };
        let size = reply.encode(HsType::NoiseMsg3, &mut packet)?;
        packet.truncate(size);
        let sealer =
            NoiseSealer::new(finished.initiator_to_responder, finished.responder_to_initiator, self.config.seal);
        let info = PeerInfo {
            addr: self.server,
            local_conn_id: self.conn_id,
            remote_conn_id: server_conn_id,
            remote_static: finished.remote_static,
            handshake_hash: finished.handshake_hash,
        };
        self.established = Some(Established { info, sealer });
        self.enter(Stage::Confirm { packet });
        Ok(())
    }
}

fn on_reject(datagram: &[u8]) -> Result<(), HandshakeError> {
    let reason = RejectReason::decode(datagram)?;
    Err(HandshakeError::Rejected(reason as u8))
}

fn copy_out(out: &mut [u8], bytes: &[u8]) -> Result<usize, HandshakeError> {
    let Some(dst) = out.get_mut(..bytes.len()) else {
        return Err(HandshakeError::Config);
    };
    dst.copy_from_slice(bytes);
    Ok(bytes.len())
}

impl std::fmt::Debug for HandshakeClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandshakeClient")
            .field("server", &self.server)
            .field("conn_id", &self.conn_id)
            .field("status", &self.status())
            .finish_non_exhaustive()
    }
}
