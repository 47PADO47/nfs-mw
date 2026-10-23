//! Helpers shared by the toy-world integration tests.

#![allow(dead_code)]

use blackbox_vehicle::InputState;
use nfsmw_sim::runner::{InputScript, Runner};
use nfsmw_sim::toy::{ToyConfig, ToyWorld, cruise};
use nfsmw_sim::{EntityView, PlayerId, PlayerJoin, SimEvent, SimInputs, Tick, WorldSim};

pub fn pid(n: u8) -> PlayerId {
    PlayerId(n)
}

pub fn join() -> PlayerJoin {
    PlayerJoin::default()
}

/// A world with players `0..n` seated.
pub fn world_with(seed: u64, config: ToyConfig, players: u8) -> ToyWorld {
    let mut world = ToyWorld::new(seed, config);
    for n in 0..players {
        world.add_player(pid(n), &join()).expect("seat");
    }
    world
}

/// A world with no bots and `n` players.
pub fn bare_world(players: u8) -> ToyWorld {
    world_with(1, ToyConfig::bare(), players)
}

/// Seat a player at a spot.
pub fn add_at(world: &mut ToyWorld, id: PlayerId, x: f32, z: f32) {
    let join = PlayerJoin { spawn: Some([x, 0.0, z]), ..PlayerJoin::default() };
    world.add_player(id, &join).expect("seat");
}

pub fn full_throttle() -> InputState {
    InputState { throttle: 1.0, ..InputState::default() }
}

/// A script that holds one input for one player.
pub fn hold(player: PlayerId, input: InputState) -> impl InputScript<ToyWorld> {
    move |_: Tick, _: &ToyWorld, out: &mut SimInputs| out.set(player, input)
}

/// A script that gives nobody any input.
pub fn idle() -> impl InputScript<ToyWorld> {
    |_: Tick, _: &ToyWorld, _: &mut SimInputs| {}
}

/// Drive each listed player round the circuit at its own cruise speed.
pub fn lap_driver(speeds: Vec<(PlayerId, f32)>) -> impl InputScript<ToyWorld> {
    move |_: Tick, world: &ToyWorld, out: &mut SimInputs| {
        let Some(race) = &world.config().race else {
            return;
        };
        for &(player, speed) in &speeds {
            let Some(slot) = world.player(player) else {
                continue;
            };
            let target = race.checkpoints[usize::from(slot.race.next)];
            out.set(player, cruise_input(&cruise(&slot.car, target, speed)));
        }
    }
}

fn cruise_input(c: &nfsmw_sim::toy::Controls) -> InputState {
    InputState {
        throttle: c.throttle,
        brake: c.brake,
        steer: c.steer,
        handbrake: c.handbrake,
        nos: c.nos,
        ..InputState::default()
    }
}

pub fn entities(world: &impl WorldSim) -> Vec<EntityView> {
    let mut out = Vec::new();
    world.entities(&mut out);
    out
}

/// Events of the kinds picked by `pick`, from a runner's log.
pub fn log_of<W: WorldSim, T>(runner: &Runner<W>, pick: impl Fn(&SimEvent) -> Option<T>) -> Vec<(Tick, T)> {
    runner.events().iter().filter_map(|(tick, event)| pick(event).map(|v| (*tick, v))).collect()
}
