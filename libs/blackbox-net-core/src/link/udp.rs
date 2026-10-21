use std::io;
use std::net::{SocketAddr, UdpSocket};

use super::Link;

/// A non-blocking UDP socket as a [`Link`].
#[derive(Debug)]
pub struct UdpLink {
    socket: UdpSocket,
    local: SocketAddr,
}

impl UdpLink {
    /// Binds a UDP socket to `addr` and sets it non-blocking. Port 0 picks a free port.
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        let socket = UdpSocket::bind(addr)?;
        socket.set_nonblocking(true)?;
        let local = socket.local_addr()?;
        Ok(Self { socket, local })
    }
}

impl Link for UdpLink {
    fn send(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        self.socket.send_to(bytes, to).map(|_| ())
    }

    fn recv(&mut self, buf: &mut [u8]) -> io::Result<Option<(SocketAddr, usize)>> {
        loop {
            match self.socket.recv_from(buf) {
                Ok((len, from)) => return Ok(Some((from, len))),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(None),
                // Windows reports an earlier send's ICMP port-unreachable here; skip it and read the next datagram.
                Err(e) if e.kind() == io::ErrorKind::ConnectionReset => continue,
                Err(e) => return Err(e),
            }
        }
    }

    fn local_addr(&self) -> SocketAddr {
        self.local
    }
}
