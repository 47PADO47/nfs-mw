//! The room password: SPAKE2 inside the sealed channel, bound to the Noise handshake hash (security.md 4.6). A right
//! password seats the client and proves the server to it; a wrong one, none at all, or a stalled exchange is refused
//! like a full room; an open room does not take PAKE messages.

use nfsmw_net::input::InputCode;
use nfsmw_net::message::{Bytes, Message, PakeMsg};
use nfsmw_net::version::RejectReason;
use nfsmw_sim::toy::ToyConfig;

use super::harness::{Harness, test_config};

/// A test config with the room password `secret`.
fn locked_config() -> nfsmw_server::ServerConfig {
    let mut config = test_config();
    config.password = Some("secret".into());
    config
}

#[test]
fn the_right_password_over_the_real_handshake_seats_the_client_and_proves_the_server() {
    let mut h = Harness::lossy(locked_config(), ToyConfig::bare(), 0.0);
    let me = h.connect_lossy(60, 0.0);
    h.clients[me].password = Some("secret".into());
    h.join_all(&[me]);
    let client = &h.clients[me];
    assert_eq!(client.slot(), Some(0));
    assert_ne!(client.handshake_hash, [0; 32], "a real handshake has a hash to bind the password to");
    assert_eq!(client.server_proved, Some(true), "the server's tag matched");
    assert!(client.rejected.is_none());
}

#[test]
fn the_host_s_own_client_on_the_in_memory_link_uses_the_zero_hash() {
    let mut h = Harness::new(locked_config(), ToyConfig::bare());
    let me = h.connect(61);
    h.clients[me].password = Some("secret".into());
    h.join_all(&[me]);
    assert_eq!(h.clients[me].server_proved, Some(true));
    assert_eq!(h.clients[me].handshake_hash, [0; 32]);
}

#[test]
fn a_wrong_password_is_refused_like_a_full_room() {
    let mut h = Harness::lossy(locked_config(), ToyConfig::bare(), 0.0);
    let me = h.connect_lossy(62, 0.0);
    h.clients[me].password = Some("guess".into());
    h.clients[me].join("intruder", 0, None, 0);
    h.until("the refusal", 600, InputCode::NEUTRAL, |h| h.clients[me].rejected.is_some());
    let rejected = h.clients[me].rejected.clone().expect("refused");
    assert_eq!(rejected.reason, RejectReason::ServerFull, "no oracle for the password");
    assert!(h.clients[me].accepted.is_none());
    assert_eq!(h.clients[me].server_proved, None, "a failed exchange gets no tag back");
    assert_eq!(h.server.lobby().members().count(), 0);
}

#[test]
fn a_client_that_ignores_the_password_is_refused_after_the_timeout() {
    let mut h = Harness::lossy(locked_config(), ToyConfig::bare(), 0.0);
    let me = h.connect_lossy(63, 0.0);
    h.clients[me].join("stubborn", 0, None, 0);
    h.run(60, InputCode::NEUTRAL);
    assert!(h.clients[me].rejected.is_none(), "the join waits for the exchange");
    assert!(h.clients[me].got(|message| matches!(message, Message::PakeStart(_))), "the room asked for a password");
    h.until("the timeout", 600, InputCode::NEUTRAL, |h| h.clients[me].rejected.is_some());
    assert_eq!(h.server.lobby().members().count(), 0);
}

#[test]
fn an_open_room_sends_no_pake_and_takes_none() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let me = h.connect(64);
    h.join_all(&[me]);
    assert!(!h.clients[me].got(|message| matches!(message, Message::PakeStart(_))));
    let before = h.server.stats().strikes;
    let message = Bytes::new(vec![0; 33]).expect("fits");
    h.clients[me].send(Message::PakeMsg(PakeMsg { message }));
    h.run(10, InputCode::NEUTRAL);
    assert!(h.server.stats().strikes > before, "a PAKE message to an open room is out of order");
}
