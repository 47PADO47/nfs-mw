use blackbox_vehicle::InputState;

use super::lobby::{lobby_state, row};
use super::*;
use crate::input::FRAME_INPUT;
use crate::message::{CAP_SPECTATE_ONLY, Role};
use crate::spectate::Camera;

fn spectator() -> Harness {
    let mut h = Harness::new(config().spectator());
    h.connect();
    let sent = h.sent_messages();
    assert!(matches!(sent.first(), Some(Message::Auth(auth)) if auth.caps & CAP_SPECTATE_ONLY != 0));
    h.reliable(accepted(SLOT, b"t").into());
    let mut me = row(SLOT, Role::Spectator);
    me.ready = true;
    h.reliable(lobby_state(vec![row(1, Role::Racer), me, row(4, Role::Racer)]).into());
    h.frame();
    h.sync();
    h
}

#[test]
fn a_spectator_sends_no_input_and_follows_racers() {
    let mut h = spectator();
    assert!(h.session.is_spectator());
    let tick = server_tick(h.now) as Tick;
    let (one, four) = (car_view(1, Some(1), 1.0), car_view(4, Some(4), 4.0));
    let body = body(&h.schema, tick, 0, &[Entry::Full(&one), Entry::Full(&four)]);
    h.snapshot(tick, tick, Some(&own_block(0.0)), &body);
    for _ in 0..10 {
        h.frame();
        h.session.drive(|_| InputState { throttle: 1.0, ..InputState::default() });
    }
    assert_eq!(h.session.due_ticks(), None);
    assert!(h.session.own_pose().is_none(), "a spectator predicts nothing");
    let inputs =
        h.answer_pings().into_iter().filter(|sent| matches!(sent, Sent::Unreliable { frame_type: FRAME_INPUT, .. }));
    assert_eq!(inputs.count(), 0);

    assert_eq!(h.session.camera(), Camera::Leader);
    assert_eq!(h.session.follow_target(), Some(1));
    h.session.follow_next();
    assert_eq!(h.session.camera(), Camera::Follow(1));
    h.session.follow_next();
    assert_eq!(h.session.follow_target(), Some(4));
    assert!(!h.session.follow(Camera::Follow(SLOT)), "a spectator is not a target");
    assert!(h.session.follow(Camera::Free));
    assert_eq!(h.session.follow_target(), None);
}

#[test]
fn a_followed_racer_who_leaves_hands_the_camera_on() {
    let mut h = spectator();
    assert!(h.session.follow(Camera::Follow(4)));
    h.reliable(lobby_state(vec![row(1, Role::Racer), row(SLOT, Role::Spectator)]).into());
    h.frame();
    assert_eq!(h.session.camera(), Camera::Follow(1));
}
