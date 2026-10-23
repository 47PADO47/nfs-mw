use super::*;
use crate::message::ServerName;

fn settings(max_spectators: u8) -> Settings {
    Settings { max_spectators, ..Settings::lan(ServerName::new("s").unwrap()) }
}

#[test]
fn a_chosen_spectator_needs_a_free_place() {
    assert_eq!(may_spectate(&settings(2), 1, SpectateReason::Chosen), Ok(()));
    assert_eq!(may_spectate(&settings(2), 2, SpectateReason::Chosen), Err(RejectReason::ServerFull));
    assert_eq!(may_spectate(&settings(0), 0, SpectateReason::WaitingForRace), Err(RejectReason::ServerFull));
}

#[test]
fn an_eliminated_racer_always_watches() {
    assert_eq!(may_spectate(&settings(0), 0, SpectateReason::Eliminated), Ok(()));
    assert_eq!(may_spectate(&settings(1), 8, SpectateReason::Eliminated), Ok(()));
}

#[test]
fn spectators_get_the_spectator_rate_and_send_no_input() {
    assert_eq!(snapshot_hz(RateClass::of(Role::Spectator), 30), SPECTATOR_SNAPSHOT_HZ);
    assert_eq!(snapshot_hz(RateClass::of(Role::Spectator), 5), 5);
    assert_eq!(snapshot_hz(RateClass::of(Role::Racer), 30), 30);
    assert_eq!(RateClass::of(Role::Cop), RateClass::Player);
    assert!(!sends_input(Role::Spectator));
    assert!(sends_input(Role::Racer) && sends_input(Role::Cop));
}

#[test]
fn the_camera_cycles_through_racers() {
    let racers = [1, 4, 6];
    let mut cam = FollowCam::new();
    assert_eq!(cam.target(&[6, 1, 4]), Some(6), "the leader");
    cam.next(&racers);
    assert_eq!(cam.camera(), Camera::Follow(1));
    cam.next(&racers);
    cam.next(&racers);
    assert_eq!(cam.camera(), Camera::Follow(6));
    cam.next(&racers);
    assert_eq!(cam.camera(), Camera::Follow(1), "wraps");
    cam.previous(&racers);
    assert_eq!(cam.camera(), Camera::Follow(6));
    assert!(!cam.set(Camera::Follow(2), &racers), "not a racer");
    assert!(cam.set(Camera::Free, &racers));
    assert_eq!(cam.target(&racers), None);
}

#[test]
fn a_followed_racer_leaving_moves_the_camera() {
    let mut cam = FollowCam::new();
    assert!(cam.set(Camera::Follow(4), &[1, 4, 6]));
    cam.target_left(4, &[1, 6]);
    assert_eq!(cam.camera(), Camera::Follow(6));
    cam.target_left(6, &[1]);
    assert_eq!(cam.camera(), Camera::Follow(1));
    cam.target_left(3, &[]);
    assert_eq!(cam.camera(), Camera::Follow(1), "another slot changes nothing");
    cam.target_left(1, &[]);
    assert_eq!(cam.camera(), Camera::Leader);
    cam.next(&[]);
    assert_eq!(cam.target(&[]), None);
}
