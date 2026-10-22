//! Hostile and degenerate input must never panic, never produce a non-finite state and never break determinism.

mod common;

use blackbox_vehicle::InputState;
use common::*;
use nfsmw_sim::SimInputs;
use nfsmw_sim::runner::{PerPlayer, Runner, WanderScript};
use nfsmw_sim::toy::{RaceConfig, ToyConfig};

fn hostile(n: u8, tick: u32) -> InputState {
    let values = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN, -0.0, 0.0, 1.0e-45, 7.5, -7.5];
    let pick = |salt: u32| values[((tick / 7 + u32::from(n) + salt) % values.len() as u32) as usize];
    InputState {
        throttle: pick(0),
        brake: pick(3),
        steer: pick(5),
        handbrake: pick(8),
        nos: tick % 3 == 0,
        shift_up: true,
        shift_down: true,
        gear_select: Some(usize::MAX),
        clutch: pick(1),
    }
}

fn assert_sane(runner: &Runner<nfsmw_sim::toy::ToyWorld>, bounds: f32) {
    for view in runner.entities() {
        let finite = view
            .position
            .iter()
            .chain(&view.orientation)
            .chain(&view.linear_velocity)
            .chain(&view.angular_velocity)
            .all(|v| v.is_finite());
        assert!(finite, "{view:?}");
        assert!(view.position[0].abs() <= bounds && view.position[2].abs() <= bounds);
    }
}

#[test]
fn hostile_input_for_ten_minutes_keeps_the_world_sane() {
    let config =
        ToyConfig { traffic: 12, police: 3, bounds: 300.0, race: Some(RaceConfig::circuit(2)), ..ToyConfig::default() };
    let mut runner = Runner::new(world_with(13, config, 16));
    let mut script = PerPlayer::new((0..16).map(pid), |player, tick| hostile(player.0, tick));
    for round in 0..(60 * 600) {
        runner.step(&mut script);
        if round % 600 == 0 {
            assert_sane(&runner, 300.0);
        }
    }
    assert_sane(&runner, 300.0);
}

#[test]
fn hostile_input_is_still_deterministic() {
    let run = || {
        let mut runner = Runner::new(world_with(13, ToyConfig::default(), 16));
        let mut script = PerPlayer::new((0..16).map(pid), |player, tick| hostile(player.0, tick));
        runner.run(60 * 60, &mut script)
    };
    assert_eq!(run(), run());
}

#[test]
fn nan_input_is_neutral_input() {
    let nan = InputState {
        throttle: f32::NAN,
        brake: f32::NAN,
        steer: f32::NAN,
        handbrake: f32::NAN,
        clutch: f32::NAN,
        ..InputState::default()
    };
    let mut a = Runner::new(world_with(2, ToyConfig::default(), 4));
    let mut b = Runner::new(world_with(2, ToyConfig::default(), 4));
    let ha = a.run(600, &mut PerPlayer::new((0..4).map(pid), |_, _| nan));
    let hb = b.run(600, &mut PerPlayer::new((0..4).map(pid), |_, _| InputState::default()));
    assert_eq!(ha, hb);
}

#[test]
fn out_of_range_input_clamps_to_the_range() {
    let huge = InputState { throttle: 1.0e30, steer: -1.0e30, ..InputState::default() };
    let range = InputState { throttle: 1.0, steer: -1.0, ..InputState::default() };
    let mut a = Runner::new(world_with(2, ToyConfig::bare(), 1));
    let mut b = Runner::new(world_with(2, ToyConfig::bare(), 1));
    assert_eq!(a.run(300, &mut hold(pid(0), huge)), b.run(300, &mut hold(pid(0), range)));
}

#[test]
fn input_for_a_player_who_is_not_seated_is_ignored() {
    let mut a = Runner::new(world_with(2, ToyConfig::default(), 2));
    let mut b = Runner::new(world_with(2, ToyConfig::default(), 2));
    let mut with_ghost = |_: u32, _: &nfsmw_sim::toy::ToyWorld, out: &mut SimInputs| {
        out.set(pid(0), full_throttle());
        out.set(pid(200), full_throttle());
    };
    let mut without = |_: u32, _: &nfsmw_sim::toy::ToyWorld, out: &mut SimInputs| out.set(pid(0), full_throttle());
    assert_eq!(a.run(300, &mut with_ghost), b.run(300, &mut without));
}

#[test]
fn degenerate_teleports_do_not_poison_the_state() {
    let mut runner = Runner::new(world_with(2, ToyConfig::default(), 2));
    let world = runner.world_mut();
    assert!(world.teleport(pid(0), [f32::NAN, f32::INFINITY], [f32::NAN, 0.0]));
    assert!(world.teleport(pid(1), [1.0e30, -1.0e30], [0.0, 0.0]));
    runner.run(120, &mut WanderScript::new(1, [pid(0), pid(1)]));
    assert_sane(&runner, 500.0);
}

#[test]
fn an_empty_world_runs() {
    let mut runner = Runner::new(world_with(2, ToyConfig::bare(), 0));
    runner.run(600, &mut idle());
    assert!(runner.entities().is_empty());
    assert!(runner.events().is_empty());
}

#[test]
fn police_without_players_patrol() {
    let config = ToyConfig { traffic: 0, police: 4, bounds: 200.0, ..ToyConfig::default() };
    let mut runner = Runner::new(world_with(3, config, 0));
    let before = runner.entities();
    runner.run(600, &mut idle());
    assert_ne!(before, runner.entities());
    assert_sane(&runner, 200.0);
}

#[test]
fn a_lone_car_on_an_empty_plane_has_nothing_to_report() {
    let mut runner = Runner::new(world_with(3, ToyConfig::bare(), 1));
    let mut script = WanderScript::new(1, [pid(0)]);
    runner.run(60 * 30, &mut script);
    assert!(runner.events().len() < 100, "a lone car on an empty plane has nothing to report");
}
