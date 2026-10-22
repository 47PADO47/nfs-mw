//! The links one endpoint sends on, with the send counters: the network transport and, on a server, the in-memory
//! link of the host's own client.

use std::net::SocketAddr;

use super::transport::Transport;
use crate::link::{Link, MemoryLink};

/// Send counters.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SendStats {
    pub(crate) datagrams: u64,
    pub(crate) bytes: u64,
    pub(crate) errors: u64,
}

/// The links of one endpoint.
pub(crate) struct Wire<T: Transport> {
    pub(crate) net: T,
    pub(crate) local: Option<MemoryLink>,
    pub(crate) stats: SendStats,
}

impl<T: Transport> Wire<T> {
    /// A wire over `net`, with an in-memory link when `local` is given.
    pub(crate) fn new(net: T, local: Option<MemoryLink>) -> Self {
        Self { net, local, stats: SendStats::default() }
    }

    /// Sends `bytes` to `to` on the in-memory link when `local`, else on the network; counts it.
    pub(crate) fn send(&mut self, local: bool, to: SocketAddr, bytes: &[u8]) {
        let result = match (local, self.local.as_mut()) {
            (true, Some(link)) => link.send(to, bytes),
            _ => self.net.send_to(to, bytes),
        };
        if result.is_err() {
            self.stats.errors += 1;
            return;
        }
        self.stats.datagrams += 1;
        self.stats.bytes += bytes.len() as u64;
    }
}
