//! The discovery responder (security-ops.md 2.3), sans-IO: it decides whether a datagram gets a reply and builds it.
//!
//! 1. Opt-in: nothing is answered until the caller sets a reply ([`DiscoveryResponder::set_reply`]).
//! 2. Only a well-formed 64-byte request is answered, and the reply is 64 bytes: never larger than the request.
//! 3. The source must pass the [`SourcePolicy`].
//! 4. The rate limits of the pre-authentication gate: 10 replies per second in total, 2 per source (LAN values).
//! 5. No state per request: the reply is the caller's template with the request's nonce written in.

use std::net::SocketAddr;

use super::policy::SourcePolicy;
use crate::discovery::{DATAGRAM_LEN, Reply, Request};
use crate::ratelimit::PreAuthGate;

/// Bytes 6 to 13 of a reply: the echoed nonce.
const NONCE_AT: usize = 6;

/// Answers discovery requests with a fixed reply.
#[derive(Debug, Clone)]
pub struct DiscoveryResponder {
    template: Option<[u8; DATAGRAM_LEN]>,
    policy: SourcePolicy,
    replies: u64,
    refused: u64,
}

impl DiscoveryResponder {
    /// A responder that answers nothing until a reply is set.
    pub fn new(policy: SourcePolicy) -> Self {
        Self { template: None, policy, replies: 0, refused: 0 }
    }

    /// Answers with `reply` from now on (its nonce is replaced per request), or nothing with `None`.
    pub fn set_reply(&mut self, reply: Option<&Reply>) {
        self.template = reply.map(Reply::encode);
    }

    /// The reply to send back to `from` for `datagram`, or `None` to stay silent (counted).
    pub fn answer(
        &mut self,
        now_ms: u64,
        from: SocketAddr,
        datagram: &[u8],
        gate: &mut PreAuthGate,
    ) -> Option<[u8; DATAGRAM_LEN]> {
        let reply = self.build(now_ms, from, datagram, gate);
        match reply {
            Some(_) => self.replies += 1,
            None => self.refused += 1,
        }
        reply
    }

    /// Replies sent.
    pub fn replies(&self) -> u64 {
        self.replies
    }

    /// Datagrams not answered.
    pub fn refused(&self) -> u64 {
        self.refused
    }

    fn build(
        &self,
        now_ms: u64,
        from: SocketAddr,
        datagram: &[u8],
        gate: &mut PreAuthGate,
    ) -> Option<[u8; DATAGRAM_LEN]> {
        let mut reply = self.template?;
        if datagram.len() != DATAGRAM_LEN {
            return None;
        }
        let request = Request::decode(datagram).ok()?;
        if !self.policy.allows(from) || !gate.discovery_reply_allowed(now_ms, from.ip()) {
            return None;
        }
        reply.get_mut(NONCE_AT..NONCE_AT + 8)?.copy_from_slice(&request.nonce.to_le_bytes());
        Some(reply)
    }
}
