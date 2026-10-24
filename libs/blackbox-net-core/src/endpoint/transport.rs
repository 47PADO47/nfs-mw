//! The datagram seam under every endpoint. A [`Transport`] is like a [`Link`] that may also wait a little for the next
//! datagram, so the net thread can sleep on its socket instead of spinning.

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::Duration;

use socket2::{Domain, Protocol, Socket, Type};

use crate::link::{Link, LossyLink, MemoryLink, UdpLink};

/// The buffer size asked of the system for a game or discovery socket: a burst of snapshots to 16 players and their
/// inputs back must not be dropped while the net thread is between passes.
pub const SOCKET_BUFFER: usize = 256 * 1024;

/// A bound UDP socket with the buffers asked for; `shared` allows other sockets on the same address and port.
fn bound(addr: SocketAddr, shared: bool) -> io::Result<UdpSocket> {
    let socket = Socket::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP))?;
    if shared {
        socket.set_reuse_address(true)?;
        #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos", target_os = "cygwin"))))]
        socket.set_reuse_port(true)?;
    }
    // A system that refuses the size keeps its own: smaller buffers cost packets under load, not correctness.
    let _ = socket.set_recv_buffer_size(SOCKET_BUFFER);
    let _ = socket.set_send_buffer_size(SOCKET_BUFFER);
    socket.bind(&addr.into())?;
    Ok(socket.into())
}

/// The longest single sleep of a polled in-memory transport while it waits.
const MEMORY_NAP: Duration = Duration::from_millis(1);
/// Port-unreachable reports skipped in one receive before giving up for this call.
const RESET_SKIPS: usize = 64;
/// Windows `WSAEMSGSIZE`: a datagram longer than the buffer (already discarded by the system).
#[cfg(windows)]
const MSG_SIZE: i32 = 10040;

/// A datagram endpoint the net thread owns. Delivery is not guaranteed.
pub trait Transport: Send {
    /// Sends one datagram. An error means it could not be handed to the network.
    fn send_to(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()>;

    /// The next datagram, waiting up to `wait` for one (a zero wait never blocks). `Ok(None)` when none came.
    fn recv_from(&mut self, buf: &mut [u8], wait: Duration) -> io::Result<Option<(SocketAddr, usize)>>;

    /// The address peers send to.
    fn local_addr(&self) -> SocketAddr;

    /// The endpoint's clock, called once per loop: a simulated link releases the datagrams due by `now_ms`.
    fn set_now(&mut self, _now_ms: u64) {}
}

/// A blocking `std` UDP socket. Waits use the socket's read timeout; a zero wait switches it to non-blocking.
#[derive(Debug)]
pub struct UdpTransport {
    socket: UdpSocket,
    local: SocketAddr,
    mode: Option<Duration>,
}

impl UdpTransport {
    /// Binds a UDP socket to `addr`, with the send and receive buffers asked to be [`SOCKET_BUFFER`] bytes (the system
    /// may give less). Port 0 picks a free port.
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        Self::from_socket(bound(addr, false)?)
    }

    /// Binds a UDP socket to `addr` that other sockets may bind to as well, so two servers on one PC can both answer
    /// discovery on its port (broadcasts reach all of them; a unicast request reaches one).
    pub fn bind_shared(addr: SocketAddr) -> io::Result<Self> {
        Self::from_socket(bound(addr, true)?)
    }

    /// Binds a UDP socket to `addr` that may send to broadcast addresses (a discovery scan).
    pub fn bind_broadcast(addr: SocketAddr) -> io::Result<Self> {
        let transport = Self::bind(addr)?;
        transport.socket.set_broadcast(true)?;
        Ok(transport)
    }

    /// Wraps a socket the caller bound.
    pub fn from_socket(socket: UdpSocket) -> io::Result<Self> {
        let local = socket.local_addr()?;
        Ok(Self { socket, local, mode: None })
    }

    /// Puts the socket in the mode `wait` needs, with a system call only when the mode changes.
    fn set_wait(&mut self, wait: Duration) -> io::Result<()> {
        if self.mode == Some(wait) {
            return Ok(());
        }
        if wait.is_zero() {
            self.socket.set_nonblocking(true)?;
            self.mode = Some(wait);
            return Ok(());
        }
        self.socket.set_nonblocking(false)?;
        self.socket.set_read_timeout(Some(wait))?;
        self.mode = Some(wait);
        Ok(())
    }
}

impl Transport for UdpTransport {
    fn send_to(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        self.socket.send_to(bytes, to).map(|_| ())
    }

    fn recv_from(&mut self, buf: &mut [u8], wait: Duration) -> io::Result<Option<(SocketAddr, usize)>> {
        self.set_wait(wait)?;
        for _ in 0..RESET_SKIPS {
            let err = match self.socket.recv_from(buf) {
                Ok((len, from)) => return Ok(Some((from, len))),
                Err(err) => err,
            };
            if matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) {
                return Ok(None);
            }
            // Windows reports an earlier send's ICMP port-unreachable here, and an oversized datagram as an error;
            // both are skipped.
            if err.kind() == io::ErrorKind::ConnectionReset || is_oversize(&err) {
                continue;
            }
            return Err(err);
        }
        Ok(None)
    }

    fn local_addr(&self) -> SocketAddr {
        self.local
    }
}

#[cfg(windows)]
fn is_oversize(err: &io::Error) -> bool {
    err.raw_os_error() == Some(MSG_SIZE)
}

#[cfg(not(windows))]
fn is_oversize(_err: &io::Error) -> bool {
    false
}

/// Receives from a non-blocking link, napping once when nothing is waiting and `wait` allows.
fn polled<L: Link>(link: &mut L, buf: &mut [u8], wait: Duration) -> io::Result<Option<(SocketAddr, usize)>> {
    if let Some(got) = link.recv(buf)? {
        return Ok(Some(got));
    }
    if wait.is_zero() {
        return Ok(None);
    }
    thread::sleep(wait.min(MEMORY_NAP));
    link.recv(buf)
}

impl Transport for MemoryLink {
    fn send_to(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        Link::send(self, to, bytes)
    }

    fn recv_from(&mut self, buf: &mut [u8], wait: Duration) -> io::Result<Option<(SocketAddr, usize)>> {
        polled(self, buf, wait)
    }

    fn local_addr(&self) -> SocketAddr {
        Link::local_addr(self)
    }
}

impl Transport for UdpLink {
    fn send_to(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        Link::send(self, to, bytes)
    }

    fn recv_from(&mut self, buf: &mut [u8], wait: Duration) -> io::Result<Option<(SocketAddr, usize)>> {
        polled(self, buf, wait)
    }

    fn local_addr(&self) -> SocketAddr {
        Link::local_addr(self)
    }
}

impl<L: Link + Send> Transport for LossyLink<L> {
    fn send_to(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        Link::send(self, to, bytes)
    }

    fn recv_from(&mut self, buf: &mut [u8], wait: Duration) -> io::Result<Option<(SocketAddr, usize)>> {
        polled(self, buf, wait)
    }

    fn local_addr(&self) -> SocketAddr {
        Link::local_addr(self)
    }

    fn set_now(&mut self, now_ms: u64) {
        LossyLink::set_now(self, now_ms);
    }
}

impl Transport for Box<dyn Transport> {
    fn send_to(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        (**self).send_to(to, bytes)
    }

    fn recv_from(&mut self, buf: &mut [u8], wait: Duration) -> io::Result<Option<(SocketAddr, usize)>> {
        (**self).recv_from(buf, wait)
    }

    fn local_addr(&self) -> SocketAddr {
        (**self).local_addr()
    }

    fn set_now(&mut self, now_ms: u64) {
        (**self).set_now(now_ms);
    }
}
