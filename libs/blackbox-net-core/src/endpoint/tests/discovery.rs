//! LAN discovery: the responder's rules without sockets (opt-in, exact size, source policy, rate limits, nonce echo),
//! then a real scan against a server on loopback and a direct connect to what it found.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use super::support::{MAJOR, WAIT, client_config, client_connected, loopback, server_config, wait_for};
use crate::discovery::{DATAGRAM_LEN, FLAG_PASSWORD, Reply, Request};
use crate::endpoint::{
    ClientEndpoint, DiscoveryResponder, DiscoveryScan, ScanConfig, ServerEndpoint, SourcePolicy, Subnet,
};
use crate::ratelimit::{PreAuthGate, PreAuthLimits};

fn reply(game_port: u16) -> Reply {
    Reply {
        proto_major: MAJOR,
        nonce: 0,
        proto_minor: 2,
        key_hash: [9; 8],
        game_port,
        players: 3,
        max_players: 8,
        flags: FLAG_PASSWORD,
        mode: 1,
        data_id: [1, 2, 3, 4],
        name: "test host".to_owned(),
    }
}

fn lan_source(last: u8) -> SocketAddr {
    SocketAddr::from(([192, 168, 1, last], 40_000))
}

fn request(nonce: u64) -> [u8; DATAGRAM_LEN] {
    Request { proto_major: MAJOR, nonce }.encode()
}

#[test]
fn the_responder_answers_only_after_opting_in_with_an_equal_size_reply() {
    let mut responder = DiscoveryResponder::new(SourcePolicy::private_lan());
    let mut gate = PreAuthGate::new(PreAuthLimits::LAN);
    assert_eq!(responder.answer(0, lan_source(2), &request(5), &mut gate), None, "not opted in");
    responder.set_reply(Some(&reply(7000)));
    let bytes = responder.answer(0, lan_source(2), &request(0xABCD), &mut gate).expect("answered");
    assert_eq!(bytes.len(), request(0).len(), "never larger than the request");
    let decoded = Reply::decode(&bytes).expect("a valid reply");
    assert_eq!(decoded.nonce, 0xABCD, "the nonce is echoed");
    assert_eq!(decoded, Reply { nonce: 0xABCD, ..reply(7000) });
    responder.set_reply(None);
    assert_eq!(responder.answer(5000, lan_source(3), &request(1), &mut gate), None, "opted out again");
    assert_eq!((responder.replies(), responder.refused()), (1, 2));
}

#[test]
fn the_responder_ignores_malformed_requests() {
    let mut responder = DiscoveryResponder::new(SourcePolicy::private_lan());
    responder.set_reply(Some(&reply(7000)));
    let mut gate = PreAuthGate::new(PreAuthLimits::LAN);
    let good = request(1);
    let mut bad_magic = good;
    bad_magic[0] ^= 1;
    let mut bad_padding = good;
    bad_padding[DATAGRAM_LEN - 1] = 1;
    for (index, datagram) in
        [&good[..63], &[&good[..], &[0]].concat()[..], &bad_magic[..], &bad_padding[..], &[0u8; 0][..]]
            .into_iter()
            .enumerate()
    {
        let source = lan_source(10 + index as u8);
        assert_eq!(responder.answer(0, source, datagram, &mut gate), None, "case {index}");
    }
}

#[test]
fn the_responder_keeps_to_the_source_policy_and_the_rate_limits() {
    let mut responder = DiscoveryResponder::new(SourcePolicy::private_lan());
    responder.set_reply(Some(&reply(7000)));
    let mut gate = PreAuthGate::new(PreAuthLimits::LAN);
    let refused = [
        SocketAddr::from(([8, 8, 8, 8], 4000)),
        SocketAddr::from(([127, 0, 0, 1], 4000)),
        SocketAddr::from(([255, 255, 255, 255], 4000)),
        SocketAddr::from(([224, 0, 0, 1], 4000)),
        SocketAddr::from(([192, 168, 1, 2], 0)),
    ];
    for source in refused {
        assert_eq!(responder.answer(0, source, &request(1), &mut gate), None, "{source} is refused");
    }
    // Two per second per source.
    let source = lan_source(2);
    assert!(responder.answer(0, source, &request(1), &mut gate).is_some());
    assert!(responder.answer(0, source, &request(2), &mut gate).is_some());
    assert!(responder.answer(0, source, &request(3), &mut gate).is_none(), "the third in a second is dropped");
    // Ten per second in total.
    let answered =
        (0..20).filter(|&last| responder.answer(10, lan_source(100 + last), &request(4), &mut gate).is_some());
    assert_eq!(answered.count(), 8, "the total budget of 10 per second, two already used");
    assert!(responder.answer(1_200, source, &request(5), &mut gate).is_some(), "the buckets refill");
}

#[test]
fn source_policy_ranges() {
    let policy = SourcePolicy::private_lan();
    for ip in [[10, 1, 2, 3], [172, 31, 0, 1], [192, 168, 0, 9], [169, 254, 3, 3]] {
        assert!(policy.allows(SocketAddr::from((ip, 9000))), "{ip:?}");
    }
    for ip in [[172, 32, 0, 1], [11, 0, 0, 1], [192, 169, 0, 1]] {
        assert!(!policy.allows(SocketAddr::from((ip, 9000))), "{ip:?}");
    }
    let mapped = Ipv4Addr::new(192, 168, 1, 1).to_ipv6_mapped();
    assert!(policy.allows(SocketAddr::new(IpAddr::V6(mapped), 9000)), "IPv4-mapped counts as IPv4");
    assert!(policy.allows(SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)), 9000)));
    assert!(!policy.allows(SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 9000)));
    assert!(SourcePolicy::loopback_only().allows(SocketAddr::from(([127, 0, 0, 1], 9000))));
    let own = SourcePolicy::subnets(vec![Subnet::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 0)), 24)], false);
    assert!(own.allows(SocketAddr::from(([203, 0, 113, 77], 1))));
    assert!(!own.allows(SocketAddr::from(([203, 0, 114, 77], 1))));
}

#[test]
fn a_scan_finds_the_server_and_its_game_port_connects() {
    let mut server = ServerEndpoint::bind(loopback(), Some(loopback()), server_config()).expect("the server binds");
    let game_port = server.local_addr().port();
    server.set_discovery_reply(Some(reply(game_port)));
    let discovery = server.discovery_addr().expect("a discovery port");
    let config =
        ScanConfig { targets: vec![discovery], proto_major: MAJOR, duration_ms: 3_000, requests: 6, capacity: 8 };
    let mut scan = DiscoveryScan::start(config).expect("the scan starts");
    let found = wait_for(WAIT, || scan.poll()).expect("the server answers");
    assert_eq!(found.from, discovery);
    assert_eq!(found.game_addr, server.local_addr());
    assert_eq!(found.reply.name, "test host");
    assert!(wait_for(WAIT, || scan.is_finished().then_some(())).is_some(), "the scan ends on its own");
    assert_eq!(scan.poll(), None, "each server is reported once");

    let mut client = ClientEndpoint::connect(found.game_addr, client_config(80)).expect("the client starts");
    client_connected(&mut client);
}

#[test]
fn a_server_that_did_not_opt_in_stays_silent() {
    let mut server = ServerEndpoint::bind(loopback(), Some(loopback()), server_config()).expect("the server binds");
    let discovery = server.discovery_addr().expect("a discovery port");
    let config =
        ScanConfig { targets: vec![discovery], proto_major: MAJOR, duration_ms: 600, requests: 3, capacity: 8 };
    let mut scan = DiscoveryScan::start(config).expect("the scan starts");
    assert!(wait_for(WAIT, || scan.is_finished().then_some(())).is_some());
    assert_eq!(scan.poll(), None);
    server.shutdown();
}

#[test]
fn a_server_refuses_sources_outside_its_policy() {
    let mut config = server_config();
    config.discovery_sources = SourcePolicy::private_lan();
    let mut server = ServerEndpoint::bind(loopback(), Some(loopback()), config).expect("the server binds");
    server.set_discovery_reply(Some(reply(1)));
    let discovery = server.discovery_addr().expect("a discovery port");
    let config =
        ScanConfig { targets: vec![discovery], proto_major: MAJOR, duration_ms: 600, requests: 3, capacity: 8 };
    let mut scan = DiscoveryScan::start(config).expect("the scan starts");
    assert!(wait_for(WAIT, || scan.is_finished().then_some(())).is_some());
    assert_eq!(scan.poll(), None, "loopback is not on the private LAN policy");
}
