//! A fully deterministic toy world: kinematic cars on a plane, driven at a fixed 60 Hz by player input and by bots.
//!
//! It exists so everything above the world seam (the server loop, the protocol, prediction, soak and bot runs) can be built and
//! tested without the game. It is not a physics model of Most Wanted.
//!
//! - **Roster.** Up to [`MAX_PLAYERS`] player cars take the lowest free entity id in `0..16`; a player who leaves frees the id
//!   and the next to join takes it. Traffic ids start at 16 and police ids follow the last traffic id, so [`WorldSim::entities`]
//!   lists ids in ascending order by construction.
//! - **Bots.** Traffic wanders between random points; police chase the nearest player car, start and end pursuits and bust a
//!   player who sits nearly still beside one.
//! - **Contacts.** Two touching cars push apart and lose speed; a hard touch emits [`SimEvent::Contact`].
//! - **Circuit.** With a [`RaceConfig`] each player drives the checkpoints in order and gets [`SimEvent::LapCompleted`] and
//!   [`SimEvent::RaceFinished`].
//! - **Determinism.** Fixed step, its own seeded [`rng`], no clock, no global state, only IEEE `+ - * /` and `sqrt`, and cars
//!   visited in id order. [`WorldSim::state_hash`] covers every bit that can change a later tick.

mod bots;
mod car;
mod chase;
mod config;
mod contact;
mod player;
mod race;
mod rng;
mod sim;

pub use bots::{Bot, cruise, distance_sq, steer_toward};
pub use car::{CAR_RADIUS, Car, Controls, DT};
pub use config::{FIRST_BOT_ID, MAX_PLAYERS, MAX_POLICE, MAX_TRAFFIC, RaceConfig, ToyConfig};
pub use player::{ChaseState, PlayerSlot, RaceProgress, grid_position};
pub use rng::ToyRng;

use crate::{EntityId, PlayerId, Tick};

/// Top speed of a traffic car is drawn from this range, metres per second.
const TRAFFIC_TOP_SPEED: (f32, f32) = (14.0, 22.0);
const POLICE_TOP_SPEED: f32 = 50.0;
const POLICE_POWER: f32 = 1.2;
const PLAYER_TOP_SPEED: f32 = 60.0;

/// The deterministic toy world. See the module documentation.
#[derive(Debug, Clone)]
pub struct ToyWorld {
    config: ToyConfig,
    rng: ToyRng,
    tick: Tick,
    /// Indexed by entity id.
    players: [Option<PlayerSlot>; MAX_PLAYERS],
    traffic: Vec<Bot>,
    police: Vec<Bot>,
    /// How many players have finished the race so far; the next finisher takes the next place.
    finished: u8,
    /// Players the host moved between ticks; the next step reports them.
    moved: Vec<PlayerId>,
}

impl ToyWorld {
    /// A world with the bots placed from `seed`. Two worlds with the same seed and config are bit-identical.
    pub fn new(seed: u64, config: ToyConfig) -> Self {
        let config = config.sanitised();
        let mut rng = ToyRng::new(seed);
        let reach = config.bounds * 0.8;
        let mut traffic = Vec::with_capacity(config.traffic);
        for _ in 0..config.traffic {
            let top = rng.range(TRAFFIC_TOP_SPEED.0, TRAFFIC_TOP_SPEED.1);
            traffic.push(Bot::new(random_car(&mut rng, reach, 1.0, top)));
        }
        let mut police = Vec::with_capacity(config.police);
        for _ in 0..config.police {
            police.push(Bot::new(random_car(&mut rng, reach, POLICE_POWER, POLICE_TOP_SPEED)));
        }
        Self { config, rng, tick: 0, players: [None; MAX_PLAYERS], traffic, police, finished: 0, moved: Vec::new() }
    }

    pub fn config(&self) -> &ToyConfig {
        &self.config
    }

    /// Players in the world.
    pub fn player_count(&self) -> usize {
        self.players.iter().flatten().count()
    }

    /// The seated players, in entity-id order.
    pub fn player_ids(&self) -> Vec<PlayerId> {
        self.players.iter().flatten().map(|slot| slot.id).collect()
    }

    /// The entity id of a player's car.
    pub fn entity_of(&self, id: PlayerId) -> Option<EntityId> {
        self.slot_index(id).map(|index| index as EntityId)
    }

    pub fn player(&self, id: PlayerId) -> Option<&PlayerSlot> {
        self.players[self.slot_index(id)?].as_ref()
    }

    /// A player's car.
    pub fn player_car(&self, id: PlayerId) -> Option<&Car> {
        self.player(id).map(|slot| &slot.car)
    }

    pub fn traffic(&self) -> &[Bot] {
        &self.traffic
    }

    pub fn police(&self) -> &[Bot] {
        &self.police
    }

    /// How many players have finished the race.
    pub fn finished_count(&self) -> u8 {
        self.finished
    }

    /// Whether a police pursuit against this player is running.
    pub fn in_pursuit(&self, id: PlayerId) -> bool {
        self.player(id).is_some_and(|slot| slot.chase.active)
    }

    /// Put a player's car at `pos` (`[x, z]`, clamped to the plane) facing `heading`, stopped. The next step reports
    /// [`crate::SimEvent::Moved`]. A heading that is not finite or has no length faces +z. Returns false for an absent player.
    pub fn teleport(&mut self, id: PlayerId, pos: [f32; 2], heading: [f32; 2]) -> bool {
        let Some(index) = self.slot_index(id) else {
            return false;
        };
        let bounds = self.config.bounds;
        let Some(slot) = self.players[index].as_mut() else {
            return false;
        };
        slot.car.pos = finite_or(pos, [0.0, 0.0]);
        slot.car.dir = unit_or_forward(heading);
        slot.car.speed = 0.0;
        slot.car.yaw_rate = 0.0;
        slot.car.keep_inside(bounds);
        slot.chase.bust_ticks = 0;
        if !self.moved.contains(&id) {
            self.moved.push(id);
        }
        true
    }

    /// Put a player's car back on its grid slot (a reset).
    pub fn reset_player(&mut self, id: PlayerId) -> bool {
        let Some(index) = self.slot_index(id) else {
            return false;
        };
        self.teleport(id, grid_position(index), [0.0, 1.0])
    }

    /// Clear every player's circuit progress and the finishing order; the race starts over from this tick.
    pub fn reset_race(&mut self) {
        let tick = self.tick;
        self.finished = 0;
        for slot in self.players.iter_mut().flatten() {
            slot.race = RaceProgress { start: tick, lap_start: tick, ..RaceProgress::default() };
        }
    }

    fn slot_index(&self, id: PlayerId) -> Option<usize> {
        self.players.iter().position(|slot| slot.as_ref().is_some_and(|slot| slot.id == id))
    }

    /// Every entity id that exists, ascending.
    fn entity_ids(&self) -> Vec<EntityId> {
        let mut ids = Vec::with_capacity(MAX_PLAYERS + self.traffic.len() + self.police.len());
        ids.extend(
            self.players.iter().enumerate().filter(|(_, slot)| slot.is_some()).map(|(index, _)| index as EntityId),
        );
        let bots = self.traffic.len() + self.police.len();
        ids.extend((0..bots).map(|index| FIRST_BOT_ID + index as EntityId));
        ids
    }

    /// The car with this entity id.
    fn car(&self, id: EntityId) -> Option<&Car> {
        let id = usize::from(id);
        if id < MAX_PLAYERS {
            return self.players[id].as_ref().map(|slot| &slot.car);
        }
        let index = id - usize::from(FIRST_BOT_ID);
        if index < self.traffic.len() {
            return Some(&self.traffic[index].car);
        }
        self.police.get(index - self.traffic.len()).map(|bot| &bot.car)
    }

    fn car_mut(&mut self, id: EntityId) -> Option<&mut Car> {
        let id = usize::from(id);
        if id < MAX_PLAYERS {
            return self.players[id].as_mut().map(|slot| &mut slot.car);
        }
        let index = id - usize::from(FIRST_BOT_ID);
        if index < self.traffic.len() {
            return Some(&mut self.traffic[index].car);
        }
        let police = index - self.traffic.len();
        self.police.get_mut(police).map(|bot| &mut bot.car)
    }

    /// The position of the player car nearest to `from`; the lowest id wins a tie.
    fn nearest_player(&self, from: [f32; 2]) -> Option<[f32; 2]> {
        let mut best: Option<(f32, [f32; 2])> = None;
        for slot in self.players.iter().flatten() {
            let d = distance_sq(slot.car.pos, from);
            if best.is_none_or(|(least, _)| d < least) {
                best = Some((d, slot.car.pos));
            }
        }
        best.map(|(_, pos)| pos)
    }
}

fn random_car(rng: &mut ToyRng, reach: f32, power: f32, top_speed: f32) -> Car {
    let pos = [rng.range(-reach, reach), rng.range(-reach, reach)];
    let dir = unit_or_forward([rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)]);
    Car::new(pos, dir, power, top_speed)
}

/// `v` normalised, or +z when it is not finite or has no length.
fn unit_or_forward(v: [f32; 2]) -> [f32; 2] {
    let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
    if !len.is_finite() || len < 1.0e-6 {
        return [0.0, 1.0];
    }
    [v[0] / len, v[1] / len]
}

fn finite_or(v: [f32; 2], fallback: [f32; 2]) -> [f32; 2] {
    let pick = |value: f32, other: f32| match value.is_finite() {
        true => value,
        false => other,
    };
    [pick(v[0], fallback[0]), pick(v[1], fallback[1])]
}
