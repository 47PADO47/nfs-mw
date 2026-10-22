use blackbox_vehicle::InputState;
use nfsmw_sim::PlayerId;

use super::*;
use crate::message::{CollisionMode, DamageMode, ServerName, Settings};

fn p(id: u8) -> PlayerId {
    PlayerId(id)
}

#[test]
fn catch_up_is_off_by_default_and_gives_no_boost() {
    let settings = Settings::lan(ServerName::new("LAN").expect("a name"));
    assert!(!settings.catch_up, "MP-32: off by default");
    let mut rule = CatchUpRule::new(settings.catch_up, 5_000.0, 0);
    assert!(!rule.update(0, &[(p(1), 80.0), (p(2), 20.0)]));
    assert_eq!(rule.boost(p(2)), 0.0);
}

#[test]
fn catch_up_boosts_the_trailing_racer_and_never_the_leader() {
    let mut rule = CatchUpRule::new(true, 5_000.0, 100);
    assert!(!rule.update(99, &[(p(1), 50.0), (p(2), 40.0)]), "not before the start");
    assert!(rule.update(100, &[(p(1), 50.0), (p(2), 40.0)]));
    assert_eq!(rule.boost(p(1)), 0.0, "the leader");
    let boost = rule.boost(p(2));
    assert!(boost > 0.0 && boost <= MAX_BOOST, "{boost}");
    assert!(!rule.update(101, &[(p(1), 50.0), (p(2), 40.0)]), "one sample per second");
    assert!(rule.update(100 + CATCH_UP_SAMPLE_TICKS, &[(p(1), 60.0)]));
    assert_eq!(rule.boost(p(2)), 0.0, "a racer that finished or left loses the boost");
}

#[test]
fn catch_up_boost_is_capped() {
    let mut rule = CatchUpRule::new(true, 100_000.0, 0);
    let mut tick = 0;
    for _ in 0..20 {
        rule.update(tick, &[(p(1), 100.0), (p(2), 0.0)]);
        tick += CATCH_UP_SAMPLE_TICKS;
    }
    assert_eq!(rule.boost(p(2)), MAX_BOOST);
}

#[test]
fn race_percent_spans_all_laps() {
    assert_eq!(race_percent(1, 50.0, 2), 75.0);
    assert_eq!(race_percent(0, 50.0, 0), 50.0, "a sprint is one lap");
    assert_eq!(race_percent(5, 0.0, 2), 100.0);
    assert_eq!(race_percent(0, f32::NAN, 3), 0.0);
}

#[test]
fn collision_modes_change_racer_pairs_only() {
    let racer = Body::solid(BodyKind::Racer);
    let traffic = Body::solid(BodyKind::Traffic);
    let world = Body::solid(BodyKind::World);
    assert!(pair_collides(CollisionMode::Full, racer, racer, 0));
    assert!(!pair_collides(CollisionMode::Ghost, racer, racer, u32::MAX));
    assert!(pair_collides(CollisionMode::Ghost, racer, traffic, 0), "traffic always collides");
    assert!(pair_collides(CollisionMode::Ghost, racer, Body::solid(BodyKind::Police), 0));
    assert!(pair_collides(CollisionMode::Ghost, world, racer, 0));
}

#[test]
fn a_ghost_start_ends_after_its_window() {
    let racer = Body::solid(BodyKind::Racer);
    assert!(!pair_collides(CollisionMode::GhostStart, racer, racer, 0), "the grid and the countdown");
    assert!(!pair_collides(CollisionMode::GhostStart, racer, racer, GHOST_START_TICKS - 1));
    assert!(pair_collides(CollisionMode::GhostStart, racer, racer, GHOST_START_TICKS));
}

#[test]
fn a_ghost_body_touches_only_the_world() {
    let parked = Body { kind: BodyKind::Racer, ghost: true };
    assert!(!pair_collides(CollisionMode::Full, parked, Body::solid(BodyKind::Racer), u32::MAX));
    assert!(!pair_collides(CollisionMode::Full, Body::solid(BodyKind::Traffic), parked, u32::MAX));
    assert!(pair_collides(CollisionMode::Full, parked, Body::solid(BodyKind::World), 0));
}

#[test]
fn nitrous_off_clears_the_button() {
    let input = InputState { nos: true, throttle: 1.0, ..InputState::default() };
    let off = Toggles { nitrous: false, ..Toggles::default() };
    assert!(!off.filter_input(&input).nos);
    assert_eq!(off.filter_input(&input).throttle, 1.0);
    assert!(Toggles::default().filter_input(&input).nos);
}

#[test]
fn toggles_follow_the_settings() {
    let mut settings = Settings::lan(ServerName::new("LAN").expect("a name"));
    settings.nitrous = false;
    settings.damage = DamageMode::Off;
    let toggles = Toggles::from_settings(&settings);
    assert!(!toggles.nitrous && !toggles.damage_shows() && toggles.repair_in_lobby);
}

#[test]
fn one_reset_per_ten_seconds_with_a_penalty_only_in_a_race() {
    let mut rule = ResetRule::new();
    assert_eq!(rule.request(p(1), 1_000, ResetState::Racing), Ok(ResetGrant { penalty_ms: RESET_PENALTY_MS }));
    let ready_at = 1_000 + RESET_COOLDOWN_TICKS;
    assert_eq!(rule.request(p(1), ready_at - 1, ResetState::Racing), Err(ResetRefusal::Cooldown { ready_at }));
    assert_eq!(rule.request(p(2), 1_001, ResetState::Roaming), Ok(ResetGrant { penalty_ms: 0 }), "per player");
    assert_eq!(rule.request(p(1), ready_at, ResetState::Roaming), Ok(ResetGrant { penalty_ms: 0 }), "free in roam");
}

#[test]
fn a_refused_reset_does_not_restart_the_wait() {
    let mut rule = ResetRule::new();
    rule.request(p(1), 0, ResetState::Roaming).expect("granted");
    assert!(rule.request(p(1), 300, ResetState::Roaming).is_err());
    assert!(rule.request(p(1), RESET_COOLDOWN_TICKS, ResetState::Roaming).is_ok());
}

#[test]
fn resets_are_refused_while_busted_spectating_counting_down_or_finished() {
    let mut rule = ResetRule::new();
    assert_eq!(rule.request(p(1), 0, ResetState::Busted), Err(ResetRefusal::Busted));
    assert_eq!(rule.request(p(1), 0, ResetState::Spectating), Err(ResetRefusal::Spectating));
    assert_eq!(rule.request(p(1), 0, ResetState::Countdown), Err(ResetRefusal::Countdown));
    assert_eq!(rule.request(p(1), 0, ResetState::Finished), Err(ResetRefusal::Finished));
    assert!(rule.request(p(1), 0, ResetState::Racing).is_ok(), "the refusals did not start a wait");
    rule.forget(p(1));
    assert!(rule.request(p(1), 1, ResetState::Racing).is_ok(), "a player who left starts afresh");
}

#[test]
fn wrong_way_shows_after_two_seconds_and_clears_after_half_a_second() {
    let mut state = WrongWay::new();
    for _ in 1..WRONG_WAY_SHOW_TICKS {
        assert_eq!(state.update(-1.0, 20.0), None);
    }
    assert_eq!(state.update(-1.0, 20.0), Some(true));
    assert_eq!(state.update(-1.0, 20.0), None, "shown once");
    for _ in 1..WRONG_WAY_CLEAR_TICKS {
        assert_eq!(state.update(1.0, 20.0), None);
    }
    assert_eq!(state.update(1.0, 20.0), Some(false));
    assert!(!state.showing());
}

#[test]
fn wrong_way_holds_below_the_speed_and_resets_on_a_turn() {
    let mut state = WrongWay::new();
    for _ in 0..WRONG_WAY_SHOW_TICKS - 1 {
        state.update(-1.0, 20.0);
    }
    assert_eq!(state.update(-1.0, WRONG_WAY_MIN_SPEED - 0.1), None, "a slow car is not judged");
    assert_eq!(state.update(0.5, 20.0), None, "turning back resets the count");
    assert_eq!(state.update(-1.0, 20.0), None);
    assert_eq!(state.update(WRONG_WAY_DOT, 20.0), None, "the threshold itself is not wrong");
}

#[test]
fn heading_dot_is_the_cosine() {
    assert_eq!(heading_dot([0.0, 0.0, 10.0], [0.0, 0.0, 2.0]), 1.0);
    assert_eq!(heading_dot([0.0, 0.0, -5.0], [0.0, 0.0, 1.0]), -1.0);
    assert_eq!(heading_dot([0.0; 3], [0.0, 0.0, 1.0]), 0.0, "a stopped car");
    assert_eq!(heading_dot([f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0]), 0.0);
}
