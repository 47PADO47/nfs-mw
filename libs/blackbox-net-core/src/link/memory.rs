use std::collections::{HashMap, VecDeque};
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard};

use super::Link;
use crate::MAX_DATAGRAM;

/// Datagrams one endpoint may hold before newer ones are dropped.
const QUEUE_CAPACITY: usize = 4096;

/// Waiting datagrams for one endpoint, oldest first, each with its source address.
type Queue = VecDeque<(SocketAddr, Vec<u8>)>;

/// An in-process network. Endpoints made from one `MemoryNet` exchange datagrams through shared queues.
///
/// Cheap to clone; every clone shares the same endpoints.
#[derive(Debug, Clone, Default)]
pub struct MemoryNet {
    inner: Arc<Mutex<HashMap<SocketAddr, Queue>>>,
}

impl MemoryNet {
    /// An empty network with no endpoints.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an endpoint at `addr` with an empty queue and returns its link.
    pub fn endpoint(&self, addr: SocketAddr) -> MemoryLink {
        self.lock().insert(addr, Queue::new());
        MemoryLink { net: self.clone(), addr }
    }

    fn lock(&self) -> MutexGuard<'_, HashMap<SocketAddr, Queue>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// One endpoint of a [`MemoryNet`] as a [`Link`].
#[derive(Debug)]
pub struct MemoryLink {
    net: MemoryNet,
    addr: SocketAddr,
}

impl Link for MemoryLink {
    fn send(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        if bytes.len() > MAX_DATAGRAM {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "datagram larger than MAX_DATAGRAM"));
        }
        let mut endpoints = self.net.lock();
        let queue = match endpoints.get_mut(&to) {
            Some(queue) => queue,
            // Nothing listens there: dropped, as UDP would drop it.
            None => return Ok(()),
        };
        if queue.len() >= QUEUE_CAPACITY {
            return Ok(());
        }
        queue.push_back((self.addr, bytes.to_vec()));
        Ok(())
    }

    fn recv(&mut self, buf: &mut [u8]) -> io::Result<Option<(SocketAddr, usize)>> {
        let mut endpoints = self.net.lock();
        let queue = match endpoints.get_mut(&self.addr) {
            Some(queue) => queue,
            None => return Ok(None),
        };
        let (from, bytes) = match queue.pop_front() {
            Some(item) => item,
            None => return Ok(None),
        };
        let len = bytes.len().min(buf.len());
        buf[..len].copy_from_slice(&bytes[..len]);
        Ok(Some((from, len)))
    }

    fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}
