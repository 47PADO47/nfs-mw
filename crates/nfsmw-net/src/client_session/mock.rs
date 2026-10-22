//! An in-process [`ClientLink`] for tests: the session holds a [`MockLink`], the test plays the server through the
//! [`MockPeer`] that shares its queues. Nothing is lost or reordered unless the test does it.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

use blackbox_net_core::endpoint::ClientEvent;

use super::ClientLink;

/// A reliable message or an unreliable frame the session sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    Reliable { channel: u8, bytes: Vec<u8> },
    Unreliable { frame_type: u8, bytes: Vec<u8>, tag: Option<u32> },
}

#[derive(Debug, Default)]
struct Shared {
    to_client: VecDeque<ClientEvent>,
    sent: VecDeque<Sent>,
    closed: Option<u8>,
    handshake_hash: Option<[u8; 32]>,
}

/// The session's end.
#[derive(Debug, Clone)]
pub struct MockLink {
    shared: Arc<Mutex<Shared>>,
}

/// The test's end: events in, sends out.
#[derive(Debug, Clone)]
pub struct MockPeer {
    shared: Arc<Mutex<Shared>>,
}

/// A connected pair.
pub fn mock_link() -> (MockLink, MockPeer) {
    let shared = Arc::new(Mutex::new(Shared::default()));
    (MockLink { shared: Arc::clone(&shared) }, MockPeer { shared })
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl ClientLink for MockLink {
    fn poll_event(&mut self) -> Option<ClientEvent> {
        lock(&self.shared).to_client.pop_front()
    }

    fn send_reliable(&mut self, channel: u8, bytes: Vec<u8>) {
        lock(&self.shared).sent.push_back(Sent::Reliable { channel, bytes });
    }

    fn send_unreliable(&mut self, frame_type: u8, bytes: Vec<u8>, tag: Option<u32>) {
        lock(&self.shared).sent.push_back(Sent::Unreliable { frame_type, bytes, tag });
    }

    fn disconnect(&mut self, reason: u8) {
        lock(&self.shared).closed = Some(reason);
    }

    fn handshake_hash(&self) -> Option<[u8; 32]> {
        lock(&self.shared).handshake_hash
    }
}

impl MockPeer {
    /// Delivers `event` to the session's next poll.
    pub fn push(&self, event: ClientEvent) {
        lock(&self.shared).to_client.push_back(event);
    }

    /// Everything the session sent since the last call, oldest first.
    pub fn take_sent(&self) -> Vec<Sent> {
        lock(&self.shared).sent.drain(..).collect()
    }

    /// The reason byte, if the session closed the link.
    pub fn closed(&self) -> Option<u8> {
        lock(&self.shared).closed
    }

    /// Sets the handshake hash the link reports.
    pub fn set_handshake_hash(&self, hash: [u8; 32]) {
        lock(&self.shared).handshake_hash = Some(hash);
    }
}
