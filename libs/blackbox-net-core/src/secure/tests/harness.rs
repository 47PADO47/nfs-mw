//! Shared setup: fixed keys, a LAN server with a fixed cookie secret, and a handshake run by direct calls.

use std::net::SocketAddr;

use crate::MAX_DATAGRAM;
use crate::secure::{
    ClientConfig, ClientStatus, Established, HandshakeClient, HandshakeServer, IdentityKey, ServerAction, ServerConfig,
};

/// The prologue both sides use in the tests.
pub const PROLOGUE: &[u8] = b"test-proto/1";
/// The protocol major version in the tests.
pub const MAJOR: u8 = 1;

/// A loopback address with `port`.
pub fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([10, 0, 0, (port % 250) as u8 + 1], port))
}

/// A fixed identity from one seed byte.
pub fn identity(seed: u8) -> IdentityKey {
    IdentityKey::from_secret(&[seed; 32]).expect("any 32 bytes are a key")
}

/// A LAN server with identity 1 and a fixed cookie secret.
pub fn server(now_ms: u64) -> HandshakeServer {
    HandshakeServer::with_cookie_secret(ServerConfig::lan(MAJOR, PROLOGUE), identity(1), [7; 32], now_ms)
}

/// A LAN client with identity 2 talking to `server_addr`.
pub fn client(server_addr: SocketAddr, conn_id: u16) -> HandshakeClient {
    HandshakeClient::new(ClientConfig::lan(MAJOR, PROLOGUE), &identity(2), server_addr, conn_id).expect("client")
}

/// Sends the client's due datagram to the server and returns the server's action and output.
pub fn client_to_server(
    client: &mut HandshakeClient,
    server: &mut HandshakeServer,
    from: SocketAddr,
    now_ms: u64,
) -> (ServerAction, Vec<u8>) {
    let mut buf = vec![0u8; MAX_DATAGRAM];
    let len = client.poll_transmit(now_ms, &mut buf).expect("the client has something to send");
    let mut out = vec![0u8; MAX_DATAGRAM];
    let action = server.handle(now_ms, from, &buf[..len], &mut out);
    if let ServerAction::Reply(n) = action {
        out.truncate(n);
    }
    (action, out)
}

/// Runs a whole handshake with no loss and returns both ends.
pub fn direct_handshake(
    client: &mut HandshakeClient,
    server: &mut HandshakeServer,
    from: SocketAddr,
    now_ms: u64,
) -> (Established, Established) {
    let (action, challenge) = client_to_server(client, server, from, now_ms);
    assert!(matches!(action, ServerAction::Reply(36)), "Hello gets a 36-byte Challenge, got {action:?}");
    assert_eq!(client.handle(server_addr_of(client), &challenge), ClientStatus::InProgress);

    let (action, msg2) = client_to_server(client, server, from, now_ms);
    assert!(matches!(action, ServerAction::Reply(_)), "HelloCookie gets NoiseMsg2, got {action:?}");
    assert_eq!(client.handle(server_addr_of(client), &msg2), ClientStatus::Established);
    let client_end = client.take_established().expect("the client has keys");

    let (action, _) = client_to_server(client, server, from, now_ms);
    let ServerAction::Established(server_end) = action else {
        panic!("NoiseMsg3 finishes the handshake, got {action:?}");
    };
    (client_end, *server_end)
}

fn server_addr_of(client: &HandshakeClient) -> SocketAddr {
    client.server()
}
