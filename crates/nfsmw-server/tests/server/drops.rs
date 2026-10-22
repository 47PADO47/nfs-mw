//! Kicks, timeouts and resumes (MP-27, MP-50, MP-51).

use blackbox_net_core::endpoint::{DisconnectReason, reason};
use nfsmw_net::input::InputCode;
use nfsmw_net::lobby::FLAG_AWAY;
use nfsmw_net::message::{KickReason, LobbyChange, Message, Vote, VoteKind};
use nfsmw_sim::toy::ToyConfig;

use super::harness::{Harness, test_config, throttle};

#[test]
fn a_vote_kicks_a_player_who_hears_why_and_is_disconnected() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let clients: Vec<usize> = (0..3).map(|seed| h.connect(20 + seed)).collect();
    h.join_all(&clients);
    let target = h.clients[2].slot().expect("seated");
    for &voter in &clients[..2] {
        h.clients[voter].send(Message::Vote(Vote { kind: VoteKind::Kick, option: u32::from(target) }));
        h.run(3, InputCode::NEUTRAL);
    }
    h.until("the kick", 300, InputCode::NEUTRAL, |h| h.clients[2].disconnected.is_some());
    let kick = h.clients[2].kicked.clone().expect("a Kick message before the close");
    assert_eq!(kick.reason, KickReason::Voted);
    assert_eq!(h.clients[2].disconnected, Some(DisconnectReason::Remote(reason::KICKED)));
    assert!(h.server.lobby().member(target).is_none(), "the slot is free");
    let left = |message: &Message| matches!(message, Message::PlayerLeft(left) if left.slot == target);
    h.until("the leave event", 120, InputCode::NEUTRAL, |h| h.clients[0].got(left));
    assert_eq!(h.server.stats().kicks, 1);
}

#[test]
fn a_silent_client_times_out_and_takes_its_slot_back_with_its_resume_token() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let a = h.connect(30);
    let b = h.connect(31);
    h.join_all(&[a, b]);
    h.start_race(&[a, b]);
    h.run(60, throttle());
    let slot = h.clients[b].slot().expect("seated");
    let token = h.clients[b].accepted.as_ref().expect("accepted").resume_token.to_vec();

    h.clients[b].silent = true;
    h.until("the timeout", 900, throttle(), |h| h.clients[b].disconnected.is_some());
    assert_eq!(h.clients[b].disconnected, Some(DisconnectReason::Remote(reason::TIMEOUT)));
    let member = h.server.lobby().member(slot).expect("the slot is held");
    assert!(member.away);
    let away_row = |message: &Message| match message {
        Message::LobbyDelta(delta) => delta
            .changes
            .iter()
            .any(|change| matches!(change, LobbyChange::Update(row) if row.slot == slot && row.flags & FLAG_AWAY != 0)),
        _ => false,
    };
    h.until("the away row", 120, throttle(), |h| h.clients[a].got(away_row));
    assert_eq!(h.server.stats().timeouts, 1);

    let again = h.connect(31);
    h.clients[again].join("back", 1, Some(token), 0);
    h.until("the resume", 600, throttle(), |h| h.clients[again].accepted.is_some());
    assert_eq!(h.clients[again].slot(), Some(slot), "the same slot");
    h.run(60, throttle());
    let client = &h.clients[again];
    assert_eq!(client.snapshot_errors, 0);
    let first = client.snapshots.first().expect("snapshots after the resume");
    assert_eq!(first.baseline_distance, 0, "the first snapshot after a resume is full");
    assert!(client.snapshots.iter().any(|snapshot| snapshot.baseline_distance > 0), "then deltas");
    assert!(first.own.is_some(), "the car was kept");
    assert!(!h.server.lobby().member(slot).expect("member").away);
    assert_eq!(h.server.stats().resumes, 1);
}

#[test]
fn a_used_resume_token_is_a_new_join() {
    let mut h = Harness::new(test_config(), ToyConfig::bare());
    let a = h.connect(40);
    h.join_all(&[a]);
    let token = h.clients[a].accepted.as_ref().expect("accepted").resume_token.to_vec();
    let thief = h.connect(41);
    h.clients[thief].join("thief", 0, Some(token), 0);
    h.until("the thief's seat", 600, InputCode::NEUTRAL, |h| h.clients[thief].accepted.is_some());
    assert_ne!(h.clients[thief].slot(), h.clients[a].slot(), "a token bound to another player gives no slot");
    assert_eq!(h.server.stats().resumes, 0);
}
