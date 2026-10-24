//! Path validation over real loopback sockets: a client whose packets start to leave from another port (a NAT
//! rebinding) keeps its session once the new port answers the server's challenge, and does not move while it cannot.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::support::{
    WAIT, client_config, client_connected, client_event, loopback, server_config, server_connected, server_event,
    wait_for,
};
use crate::endpoint::{ClientEndpoint, ClientEvent, ServerEndpoint, ServerEvent, Transport, UdpTransport};

/// Two sockets for one client: it sends from the first until the switch is set, then from the second. It receives on
/// both, unless the second is deaf (the network does not deliver to the new port).
struct Rebinding {
    first: UdpTransport,
    second: UdpTransport,
    moved: Arc<AtomicBool>,
    deaf_second: Arc<AtomicBool>,
}

struct Switches {
    moved: Arc<AtomicBool>,
    deaf_second: Arc<AtomicBool>,
    second_addr: SocketAddr,
}

fn rebinding() -> (Rebinding, Switches) {
    let first = UdpTransport::bind(loopback()).expect("first socket");
    let second = UdpTransport::bind(loopback()).expect("second socket");
    let (moved, deaf_second) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)));
    let switches =
        Switches { moved: Arc::clone(&moved), deaf_second: Arc::clone(&deaf_second), second_addr: second.local_addr() };
    (Rebinding { first, second, moved, deaf_second }, switches)
}

impl Transport for Rebinding {
    fn send_to(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        match self.moved.load(Ordering::Relaxed) {
            true => self.second.send_to(to, bytes),
            false => self.first.send_to(to, bytes),
        }
    }

    fn recv_from(&mut self, buf: &mut [u8], wait: Duration) -> io::Result<Option<(SocketAddr, usize)>> {
        if let Some(got) = self.first.recv_from(buf, Duration::ZERO)? {
            return Ok(Some(got));
        }
        if !self.deaf_second.load(Ordering::Relaxed)
            && let Some(got) = self.second.recv_from(buf, Duration::ZERO)?
        {
            return Ok(Some(got));
        }
        if wait.is_zero() {
            return Ok(None);
        }
        std::thread::sleep(wait.min(Duration::from_millis(1)));
        Ok(None)
    }

    fn local_addr(&self) -> SocketAddr {
        self.first.local_addr()
    }
}

fn connected(deaf_second: bool) -> (ServerEndpoint, ClientEndpoint, Switches, crate::session::PeerId) {
    let mut server = ServerEndpoint::bind(loopback(), None, server_config()).expect("the server binds");
    let (transport, switches) = rebinding();
    switches.deaf_second.store(deaf_second, Ordering::Relaxed);
    let mut client =
        ClientEndpoint::spawn(transport, server.local_addr(), client_config(40)).expect("the client starts");
    client_connected(&mut client);
    let peer = server_connected(&mut server);
    (server, client, switches, peer)
}

#[test]
fn a_client_that_changes_port_keeps_its_session_after_it_answers_the_challenge() {
    let (mut server, mut client, switches, peer) = connected(false);
    server.send_reliable(peer, 0, b"before".to_vec());
    assert!(
        client_event(&mut client, |event| matches!(event, ClientEvent::Reliable { .. })).is_some(),
        "the session works on the first port"
    );
    switches.moved.store(true, Ordering::Relaxed);
    client.send_reliable(0, b"from the new port".to_vec());
    // The packet from the new port is taken in at once; the move waits for the answer to the challenge.
    let (mut heard, mut changed) = (false, None);
    let done = wait_for(WAIT, || {
        while let Some(event) = server.poll_event() {
            match event {
                ServerEvent::Reliable { bytes, .. } if bytes == b"from the new port" => heard = true,
                ServerEvent::PathChanged { peer, addr } => changed = Some((peer, addr)),
                _ => {}
            }
        }
        (heard && changed.is_some()).then_some(())
    });
    assert!(done.is_some(), "heard: {heard}, changed: {changed:?}");
    assert_eq!(changed, Some((peer, switches.second_addr)));
    server.send_reliable(peer, 0, b"after".to_vec());
    let after =
        client_event(&mut client, |event| matches!(event, ClientEvent::Reliable { bytes, .. } if bytes == b"after"));
    assert!(after.is_some(), "the server sends to the new port from now on");
}

#[test]
fn a_client_that_cannot_answer_at_its_new_port_does_not_move_the_session() {
    let (mut server, mut client, switches, peer) = connected(true);
    switches.moved.store(true, Ordering::Relaxed);
    client.send_reliable(0, b"from the new port".to_vec());
    // Its packets from the new port are real, so they are taken in, but the challenge never reaches it.
    let heard = server_event(
        &mut server,
        |event| matches!(event, ServerEvent::Reliable { bytes, .. } if bytes == b"from the new port"),
    );
    assert!(heard.is_some());
    let moved = wait_for(Duration::from_millis(1_500), || {
        while let Some(event) = server.poll_event() {
            if matches!(event, ServerEvent::PathChanged { .. }) {
                return Some(());
            }
        }
        None
    });
    assert!(moved.is_none(), "an address that never answers does not take the session");
    server.send_reliable(peer, 0, b"still here".to_vec());
    let still = client_event(
        &mut client,
        |event| matches!(event, ClientEvent::Reliable { bytes, .. } if bytes == b"still here"),
    );
    assert!(still.is_some(), "the server keeps sending to the first port, which the client still hears");
    assert!(WAIT > Duration::ZERO);
}
