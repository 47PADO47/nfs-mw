use std::sync::{Arc, Mutex};

use blackbox_net_core::endpoint::{ClientEvent, DisconnectReason, reason};

use super::*;
use crate::client_session::{EndReason, Status};

/// A joined session whose connector hands out fresh mock links; the test gets each new peer from `peers`.
fn with_connector() -> (Harness, Arc<Mutex<Vec<MockPeer>>>) {
    let peers = Arc::new(Mutex::new(Vec::new()));
    let made = Arc::clone(&peers);
    let (link, peer) = mock_link();
    let session = ClientSession::new(link, config(), car()).with_connector(Box::new(move || {
        let (link, peer) = mock_link();
        made.lock().unwrap().push(peer);
        Some(link)
    }));
    let mut h = Harness { session, peer, schema: GameSchema::new(), now: START_MS };
    h.connect();
    h.reliable(accepted(SLOT, b"token-1").into());
    h.frame();
    h.take_events();
    (h, peers)
}

#[test]
fn a_silent_drop_resumes_the_slot_with_the_token() {
    let (mut h, peers) = with_connector();
    h.peer.push(ClientEvent::Disconnected(DisconnectReason::Timeout));
    h.frame();
    assert_eq!(h.take_events(), vec![SessionEvent::ConnectionLost]);
    assert!(matches!(h.session.status(), Status::Reconnecting { .. }));
    h.run(1100);
    let peer = peers.lock().unwrap().first().cloned().expect("the connector was called");
    h.peer = peer;
    h.connect();
    let sent = h.sent_messages();
    let Some(Message::Auth(auth)) = sent.first() else {
        panic!("Auth first, got {sent:?}");
    };
    assert_eq!(auth.resume.as_deref(), Some(&b"token-1"[..]));
    h.reliable(accepted(SLOT, b"token-2").into());
    h.frame();
    assert_eq!(h.take_events(), vec![SessionEvent::Admitted { slot: SLOT, entity: ENTITY, resumed: true }]);
    assert!(h.session.status().is_joined());
    assert_eq!(h.session.counters().reconnects, 1);
}

#[test]
fn a_failed_reconnect_retries_until_the_window_closes() {
    let (mut h, peers) = with_connector();
    h.peer.push(ClientEvent::Disconnected(DisconnectReason::Remote(reason::TIMEOUT)));
    h.frame();
    for _ in 0..5 {
        h.run(1100);
        let peer = peers.lock().unwrap().last().cloned().expect("a link");
        peer.push(ClientEvent::Disconnected(DisconnectReason::Timeout));
    }
    assert!(peers.lock().unwrap().len() >= 5, "one attempt per second");
    h.run(30_000);
    assert_eq!(*h.session.status(), Status::Ended(EndReason::ResumeExpired));
}

#[test]
fn a_close_by_the_server_is_not_resumed() {
    let (mut h, _) = with_connector();
    let why = DisconnectReason::Remote(reason::SHUTTING_DOWN);
    h.peer.push(ClientEvent::Disconnected(why));
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::Link(why)));
}

#[test]
fn without_a_connector_the_game_hands_in_the_link() {
    let mut h = Harness::joined();
    h.peer.push(ClientEvent::Disconnected(DisconnectReason::Timeout));
    h.frame();
    let (link, peer) = mock_link();
    h.session.resume_with(link);
    h.peer = peer;
    h.connect();
    assert!(matches!(h.sent_messages().first(), Some(Message::Auth(auth)) if auth.resume.is_some()));
    // A new slot means the old one expired: a fresh seat, not a resume.
    h.reliable(accepted(SLOT + 1, b"t").into());
    h.frame();
    assert!(h.take_events().contains(&SessionEvent::Admitted {
        slot: SLOT + 1,
        entity: u16::from(SLOT + 1),
        resumed: false
    }));
}
