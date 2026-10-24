use blackbox_net_core::endpoint::{ClientEvent, DisconnectReason, reason};
use blackbox_net_core::secure::password::{self, PakeRole};

use super::*;
use crate::client_session::{EndReason, Status};
use crate::message::{Bytes, JoinRejected, Kick, KickReason, PakeConfirm, PakeScheme, PakeStart};
use crate::version::{ProtoVersion, RejectReason};

#[test]
fn connect_sends_auth_then_join() {
    let mut h = Harness::new(config());
    assert_eq!(*h.session.status(), Status::Connecting);
    h.connect();
    assert_eq!(*h.session.status(), Status::Admitting);
    let sent = h.sent_messages();
    let [Message::Auth(auth), Message::Join(join)] = sent.as_slice() else {
        panic!("Auth then Join, got {sent:?}");
    };
    assert_eq!(auth.resume, None);
    assert_eq!(auth.caps, 0);
    assert_eq!(join.display_name.as_str(), "tester");
    assert_eq!(join.loadout, Loadout::stock(3));
}

#[test]
fn accepted_seats_the_player() {
    let mut h = Harness::new(config());
    h.connect();
    h.reliable(accepted(SLOT, b"tok").into());
    h.frame();
    assert_eq!(h.take_events(), vec![SessionEvent::Admitted { slot: SLOT, entity: ENTITY, resumed: false }]);
    assert!(h.session.status().is_joined());
    assert_eq!((h.session.slot(), h.session.own_entity()), (Some(SLOT), Some(ENTITY)));
    assert_eq!(h.session.lobby().my_slot, Some(SLOT));
    assert!(h.session.lobby().settings.is_some());
}

#[test]
fn rejection_and_kick_end_the_session() {
    let mut h = Harness::new(config());
    h.connect();
    let rejected = JoinRejected {
        reason: RejectReason::DataMismatch,
        detail: 0,
        server_proto: ProtoVersion::CURRENT,
        server_data_id: DataId::compute(&[2; 32], &[]),
    };
    h.reliable(rejected.clone().into());
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::Rejected(rejected)));
    assert_eq!(h.peer.closed(), Some(reason::PROTOCOL));

    let mut h = Harness::joined();
    let kick = Kick { reason: KickReason::Banned, ban_minutes: 30 };
    h.reliable(kick.clone().into());
    h.peer.push(ClientEvent::Disconnected(DisconnectReason::Remote(reason::KICKED)));
    h.frame();
    assert_eq!(h.take_events(), vec![SessionEvent::Ended(EndReason::Kicked(kick.clone()))]);
    assert_eq!(h.session.lobby().kicked, Some(kick));
}

#[test]
fn a_different_schema_is_refused() {
    let mut h = Harness::new(config());
    h.connect();
    let mut bad = accepted(SLOT, b"t");
    bad.server_caps.schema_id ^= 1;
    h.reliable(bad.into());
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::SchemaMismatch));
}

#[test]
fn a_drop_before_the_seat_ends_the_session() {
    let mut h = Harness::new(config());
    h.peer.push(ClientEvent::Disconnected(DisconnectReason::Timeout));
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::Link(DisconnectReason::Timeout)));
}

/// Plays the host's side of the password exchange; returns the host's confirmation tag for the client's message.
fn host_exchange(h: &mut Harness, host_password: &[u8]) -> PakeConfirm {
    let hash = [5u8; 32];
    h.peer.set_handshake_hash(hash);
    let (state, message) = password::start(host_password, &hash, PakeRole::Host).unwrap();
    h.reliable(PakeStart { scheme: PakeScheme::Spake2, message: Bytes::new(message.to_vec()).unwrap() }.into());
    h.frame();
    let sent = h.sent_messages();
    let [Message::PakeMsg(theirs), Message::PakeConfirm(tag)] = sent.as_slice() else {
        panic!("PakeMsg then PakeConfirm, got {sent:?}");
    };
    let confirm = state.finish(&theirs.message).unwrap();
    assert_eq!(confirm.verify(&tag.tag), host_password == b"secret");
    PakeConfirm { tag: confirm.tag() }
}

#[test]
fn the_room_password_is_exchanged_and_checked() {
    let mut config = config();
    config.password = Some("secret".into());
    let mut h = Harness::new(config.clone());
    h.connect();
    h.sent();
    let tag = host_exchange(&mut h, b"secret");
    h.reliable(tag.into());
    h.reliable(accepted(SLOT, b"t").into());
    h.frame();
    assert!(h.session.status().is_joined());

    let mut h = Harness::new(config);
    h.connect();
    h.sent();
    let tag = host_exchange(&mut h, b"other");
    h.reliable(tag.into());
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::PasswordRejected));
}

#[test]
fn a_password_room_without_a_password_ends_the_session() {
    let mut h = Harness::new(config());
    h.connect();
    let (_, message) = password::start(b"secret", &[0; 32], PakeRole::Host).unwrap();
    h.reliable(PakeStart { scheme: PakeScheme::Spake2, message: Bytes::new(message.to_vec()).unwrap() }.into());
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::PasswordRequired));
}

#[test]
fn an_unconfirmed_password_refuses_the_seat() {
    let mut config = config();
    config.password = Some("secret".into());
    let mut h = Harness::new(config);
    h.connect();
    h.sent();
    host_exchange(&mut h, b"secret");
    h.reliable(accepted(SLOT, b"t").into());
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::PasswordRejected));
}

#[test]
fn a_refusal_after_the_password_exchange_is_a_wrong_password() {
    let mut config = config();
    config.password = Some("secret".into());
    let refusal = || JoinRejected {
        reason: RejectReason::ServerFull,
        detail: 0,
        server_proto: ProtoVersion::CURRENT,
        server_data_id: DataId::compute(&[2; 32], &[]),
    };

    let mut h = Harness::new(config.clone());
    h.connect();
    h.sent();
    host_exchange(&mut h, b"other");
    h.reliable(refusal().into());
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::PasswordRejected), "no server tag came back");

    let mut h = Harness::new(config);
    h.connect();
    h.reliable(refusal().into());
    h.frame();
    assert_eq!(*h.session.status(), Status::Ended(EndReason::Rejected(refusal())), "a full room stays a full room");
}
