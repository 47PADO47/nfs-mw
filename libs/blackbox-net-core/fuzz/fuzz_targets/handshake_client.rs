//! The client's handshake entry point on arbitrary datagrams from the server address.
#![no_main]

use std::net::SocketAddr;

use blackbox_net_core::MAX_DATAGRAM;
use blackbox_net_core::secure::{ClientConfig, HandshakeClient, IdentityKey};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(identity) = IdentityKey::from_secret(&[2; 32]) else {
        return;
    };
    let server = SocketAddr::from(([10, 0, 0, 1], 9000));
    let Ok(mut client) = HandshakeClient::new(ClientConfig::lan(1, b"fuzz/1"), &identity, server, 3) else {
        return;
    };
    let mut out = vec![0u8; MAX_DATAGRAM];
    let _ = client.poll_transmit(0, &mut out);
    let _ = client.handle(server, data);
    let _ = client.poll_transmit(10, &mut out);
    let _ = client.take_established();
});
