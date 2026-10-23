//! Two toy worlds that start equal and see the same input stay equal, bit for bit.

mod common;

use blackbox_vehicle::InputState;
use common::*;
use nfsmw_sim::runner::{Runner, WanderScript, compare_runs, compare_runs_every};
use nfsmw_sim::toy::{RaceConfig, ToyConfig, ToyWorld};
use nfsmw_sim::{SimInputs, TICK_HZ, Tick, WorldSim};

fn busy_config() -> ToyConfig {
    ToyConfig { traffic: 20, police: 4, bounds: 400.0, race: Some(RaceConfig::circuit(3)), ..ToyConfig::default() }
}

fn players() -> Vec<nfsmw_sim::PlayerId> {
    (0..16).map(pid).collect()
}

#[test]
fn two_worlds_agree_on_every_tick_for_ten_simulated_minutes() {
    let ticks = 10 * 60 * TICK_HZ;
    let mut a = Runner::new(world_with(42, busy_config(), 16));
    let mut b = Runner::new(world_with(42, busy_config(), 16));
    let hashes_a = a.run_hashed(ticks, &mut WanderScript::new(7, players()));
    let hashes_b = b.run_hashed(ticks, &mut WanderScript::new(7, players()));
    assert_eq!(hashes_a, hashes_b);
    assert_eq!(a.events(), b.events());
    assert_eq!(a.world().tick(), ticks);
    let distinct: std::collections::HashSet<_> = hashes_a.iter().collect();
    assert!(distinct.len() as u32 > ticks / 2, "the hash must move as the world does");
    assert!(!a.events().is_empty(), "ten minutes of 16 cars must touch");
}

#[test]
fn different_seeds_give_different_worlds() {
    let mut a = Runner::new(world_with(1, busy_config(), 4));
    let mut b = Runner::new(world_with(2, busy_config(), 4));
    assert_ne!(a.hash(), b.hash(), "the bots start in different places");
    let ha = a.run(600, &mut WanderScript::new(7, players()));
    let hb = b.run(600, &mut WanderScript::new(7, players()));
    assert_ne!(ha, hb);
}

#[test]
fn different_scripts_give_different_worlds() {
    let mut a = Runner::new(world_with(1, busy_config(), 4));
    let mut b = Runner::new(world_with(1, busy_config(), 4));
    let ha = a.run(600, &mut WanderScript::new(7, players()));
    let hb = b.run(600, &mut WanderScript::new(8, players()));
    assert_ne!(ha, hb);
}

#[test]
fn compare_runs_reports_equal_worlds() {
    let a = world_with(5, busy_config(), 8);
    let b = world_with(5, busy_config(), 8);
    let result = compare_runs(a, b, 5 * TICK_HZ, &mut WanderScript::new(3, players()));
    assert!(result.is_equal());
    assert_eq!(result.ticks, 5 * TICK_HZ);
    assert_eq!(result.checks, 5, "one check a second");
    assert_eq!(result.hash_a, result.hash_b);
}

#[test]
fn compare_runs_catches_a_world_that_differs() {
    let a = world_with(5, busy_config(), 8);
    let b = world_with(6, busy_config(), 8);
    let result = compare_runs_every(a, b, 600, 10, &mut WanderScript::new(3, players()));
    assert!(!result.is_equal());
    let first = result.first_divergence.expect("the bots differ from the start");
    assert!(first <= 9, "the first check, at tick 10, sees the different bots");
}

#[test]
fn compare_runs_catches_one_extra_player() {
    let a = world_with(5, busy_config(), 8);
    let b = world_with(5, busy_config(), 9);
    let result = compare_runs(a, b, 120, &mut WanderScript::new(3, players()));
    assert!(!result.is_equal());
}

#[test]
fn a_clone_continues_identically() {
    let mut a = Runner::new(world_with(9, busy_config(), 6));
    a.run(900, &mut WanderScript::new(1, players()));
    let mut b = Runner::new(a.world().clone());
    assert_eq!(a.hash(), b.hash());
    let ha = a.run_hashed(900, &mut WanderScript::new(1, players()));
    let hb = b.run_hashed(900, &mut WanderScript::new(1, players()));
    assert_eq!(ha, hb);
}

#[test]
fn the_hash_moves_even_with_nobody_driving() {
    let mut runner = Runner::new(world_with(3, ToyConfig::bare(), 0));
    let before = runner.hash();
    runner.run(1, &mut idle());
    assert_ne!(before, runner.hash(), "the tick counter is state");
}

#[test]
fn one_changed_input_bit_changes_the_hash_later() {
    let steer = 0.5f32;
    let flipped = f32::from_bits(steer.to_bits() ^ (1 << 20));
    assert_ne!(steer, flipped);
    let run = |value: f32| {
        let mut runner = Runner::new(world_with(11, ToyConfig::bare(), 1));
        let mut script = |tick: Tick, _: &ToyWorld, out: &mut SimInputs| {
            let mut steer = 0.5;
            if tick == 100 {
                steer = value;
            }
            out.set(pid(0), InputState { throttle: 1.0, steer, ..InputState::default() });
        };
        runner.run(100, &mut script);
        let before = runner.hash();
        runner.run(1, &mut script);
        let at_change = runner.hash();
        runner.run(300, &mut script);
        (before, at_change, runner.hash())
    };
    let (before_a, at_a, end_a) = run(steer);
    let (before_b, at_b, end_b) = run(flipped);
    assert_eq!(before_a, before_b, "same history until the changed tick");
    assert_ne!(at_a, at_b);
    assert_ne!(end_a, end_b, "the change is never forgotten");
}

#[test]
fn a_single_button_changes_the_hash() {
    let run = |nos: bool| {
        let mut runner = Runner::new(world_with(11, ToyConfig::bare(), 1));
        runner.run(120, &mut hold(pid(0), InputState { throttle: 1.0, nos, ..InputState::default() }))
    };
    assert_ne!(run(true), run(false));
}

#[test]
fn negative_zero_input_is_the_same_as_zero() {
    let run = |value: f32| {
        let mut runner = Runner::new(world_with(11, ToyConfig::bare(), 1));
        runner.run(300, &mut hold(pid(0), InputState { throttle: 1.0, steer: value, ..InputState::default() }))
    };
    assert_eq!(run(0.0), run(-0.0));
}

#[test]
fn joining_and_leaving_are_part_of_the_state() {
    let mut a = world_with(2, ToyConfig::bare(), 2);
    let b = world_with(2, ToyConfig::bare(), 2);
    assert_eq!(a.state_hash(), b.state_hash());
    a.remove_player(pid(1));
    assert_ne!(a.state_hash(), b.state_hash());
    a.add_player(pid(1), &join()).unwrap();
    assert_eq!(a.state_hash(), b.state_hash(), "the same roster at the same tick is the same state");
}

#[test]
fn a_late_join_is_deterministic() {
    let run = || {
        let mut runner = Runner::new(world_with(4, busy_config(), 3));
        runner.run(300, &mut WanderScript::new(2, players()));
        runner.world_mut().add_player(pid(3), &join()).unwrap();
        runner.world_mut().remove_player(pid(1));
        runner.run(300, &mut WanderScript::new(2, players()))
    };
    assert_eq!(run(), run());
}

#[test]
fn the_runner_logs_events_with_their_tick() {
    let mut runner = Runner::new(world_with(4, busy_config(), 16));
    let mut script = WanderScript::new(2, players());
    let mut seen = 0;
    for _ in 0..1200 {
        let tick = runner.world().tick();
        let events = runner.step(&mut script).len();
        seen += events;
        assert!(runner.events().iter().rev().take(events).all(|(t, _)| *t == tick));
    }
    assert_eq!(seen, runner.events().len());
    assert_eq!(runner.take_events().len(), seen);
    assert!(runner.events().is_empty());
}
