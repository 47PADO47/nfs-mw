//! The server's handshake entry point on a sequence of arbitrary datagrams (split on the input's first byte as a
//! length), so the fuzzer can reach states after a valid `Hello`.
#![no_main]

use std::net::SocketAddr;

use blackbox_net_core::MAX_DATAGRAM;
use blackbox_net_core::secure::{HandshakeServer, IdentityKey, ServerAction, ServerConfig};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(identity) = IdentityKey::from_secret(&[1; 32]) else {
        return;
    };
    let mut server = HandshakeServer::with_cookie_secret(ServerConfig::lan(1, b"fuzz/1"), identity, [7; 32], 0);
    let from = SocketAddr::from(([10, 0, 0, 2], 4000));
    let mut out = vec![0u8; MAX_DATAGRAM];
    let mut rest = data;
    let mut now = 0;
    while let Some((&len, tail)) = rest.split_first() {
        let take = usize::from(len).saturating_mul(4).min(tail.len());
        let (datagram, next) = tail.split_at(take);
        if let ServerAction::Reply(n) = server.handle(now, from, datagram, &mut out) {
            assert!(n <= 3 * 512, "a reply is bounded");
        }
        server.expire(now);
        rest = next;
        now += 100;
    }
});
