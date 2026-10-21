//! The scene shared by the loopback test, the allocation test and the bench: a car-like and a traffic-like kind, and
//! entities moving on deterministic circles, quantised the way the game would quantise them.
#![allow(dead_code)]

use std::f64::consts::{FRAC_PI_2, TAU};

use blackbox_net_core::replicate::{FieldDesc, History, KindId, Schema, SpatialHash};
use blackbox_net_core::rng::SplitMix64;

/// The car-like kind: position, yaw, speed and flags.
pub const CAR: KindId = KindId(0);
/// The traffic-like kind: position, height and yaw.
pub const TRAFFIC: KindId = KindId(1);
/// The relevancy radius around a client's player, in metres.
pub const RELEVANCE_M: f32 = 400.0;
/// The cell size of the spatial hash, in metres.
pub const CELL_M: f32 = 100.0;
/// Ticks per second of the simulation.
pub const TICK_HZ: u32 = 60;

/// Step of the horizontal position code, in metres.
const POSITION_STEP_M: f64 = 0.02;
/// Positions are offset by this many metres before quantising, so the code of a point in the +/-2 km world is positive.
const POSITION_OFFSET_M: f64 = 2048.0;
/// Step of the height code, in metres.
const HEIGHT_STEP_M: f64 = 0.01;

/// The schema every target registers: the car-like kind, then the traffic-like kind.
pub fn schema() -> Schema {
    let mut schema = Schema::new();
    let car = schema.add_kind(&[
        FieldDesc { bits: 20, diff: true },  // x
        FieldDesc { bits: 20, diff: true },  // y
        FieldDesc { bits: 20, diff: true },  // z
        FieldDesc { bits: 10, diff: false }, // yaw
        FieldDesc { bits: 10, diff: false }, // speed
        FieldDesc { bits: 4, diff: false },  // flags
    ]);
    assert_eq!(car, Ok(CAR));
    let traffic = schema.add_kind(&[
        FieldDesc { bits: 18, diff: true },  // x
        FieldDesc { bits: 18, diff: true },  // z
        FieldDesc { bits: 12, diff: false }, // y
        FieldDesc { bits: 8, diff: false },  // yaw
    ]);
    assert_eq!(traffic, Ok(TRAFFIC));
    schema
}

/// The code of a horizontal coordinate: `bits` bits, masked.
fn position_code(metres: f64, bits: u32) -> u32 {
    let code = ((metres + POSITION_OFFSET_M) / POSITION_STEP_M).round().max(0.0) as u64;
    (code & ((1u64 << bits) - 1)) as u32
}

/// The code of a height: `bits` bits, clamped.
fn height_code(metres: f64, bits: u32) -> u32 {
    let top = (1u64 << bits) - 1;
    ((metres / HEIGHT_STEP_M).round().max(0.0) as u64).min(top) as u32
}

/// The code of an angle in radians: `bits` bits over one turn.
fn angle_code(radians: f64, bits: u32) -> u32 {
    let turns = radians.rem_euclid(TAU) / TAU;
    let steps = (1u64 << bits) as f64;
    ((turns * steps).round() as u64 % (1u64 << bits)) as u32
}

/// One entity: a point on a circle with its own radius, speed and phase.
#[derive(Debug, Clone, Copy)]
pub struct Mover {
    /// The entity id.
    pub id: u16,
    /// The kind the entity is sent as.
    pub kind: KindId,
    centre: (f64, f64),
    radius: f64,
    speed: f64,
    phase: f64,
    height: f64,
    flags: u32,
}

impl Mover {
    /// The position at `tick` as `[x, y, z]` in metres.
    pub fn position(&self, tick: u32) -> [f64; 3] {
        let angle = self.angle(tick);
        [self.centre.0 + self.radius * angle.cos(), self.height, self.centre.1 + self.radius * angle.sin()]
    }

    /// The angle around the circle at `tick`.
    fn angle(&self, tick: u32) -> f64 {
        self.phase + self.speed / self.radius * f64::from(tick) / f64::from(TICK_HZ)
    }

    /// The quantised field values at `tick` and how many of them the kind has (6 for the car, 4 for the traffic).
    pub fn values(&self, tick: u32) -> ([u32; 6], usize) {
        let [x, y, z] = self.position(tick);
        let heading = self.angle(tick) + FRAC_PI_2;
        match self.kind {
            CAR => {
                let speed = (self.speed * 8.0).round().clamp(0.0, 1023.0) as u32;
                let values = [
                    position_code(x, 20),
                    position_code(y, 20),
                    position_code(z, 20),
                    angle_code(heading, 10),
                    speed,
                    self.flags,
                ];
                (values, 6)
            }
            _ => {
                let values =
                    [position_code(x, 18), position_code(z, 18), height_code(y, 12), angle_code(heading, 8), 0, 0];
                (values, 4)
            }
        }
    }
}

/// The players (ids `0..players`, one per client, the first movers) and the other entities (ids `100..`), all moving.
#[derive(Debug, Clone)]
pub struct Scene {
    movers: Vec<Mover>,
    players: u16,
}

impl Scene {
    /// A scene from `seed`: `players` cars around the origin, then `others` cars and traffic in a 3 km square.
    pub fn new(seed: u64, players: u16, others: u16) -> Self {
        let mut rng = SplitMix64(seed);
        let mut movers = Vec::with_capacity(usize::from(players) + usize::from(others));
        for id in 0..players {
            movers.push(Mover {
                id,
                kind: CAR,
                centre: (uniform(&mut rng, -300.0, 300.0), uniform(&mut rng, -300.0, 300.0)),
                radius: uniform(&mut rng, 30.0, 120.0),
                speed: uniform(&mut rng, 15.0, 40.0),
                phase: uniform(&mut rng, 0.0, TAU),
                height: 0.0,
                flags: u32::from(id) % 16,
            });
        }
        for index in 0..others {
            let kind = match index % 2 {
                0 => CAR,
                _ => TRAFFIC,
            };
            let speed = match rng.below(4) {
                0 => 0.0,
                _ => uniform(&mut rng, 0.0, 25.0),
            };
            movers.push(Mover {
                id: 100 + index,
                kind,
                centre: (uniform(&mut rng, -1500.0, 1500.0), uniform(&mut rng, -1500.0, 1500.0)),
                radius: uniform(&mut rng, 20.0, 300.0),
                speed,
                phase: uniform(&mut rng, 0.0, TAU),
                height: uniform(&mut rng, 0.0, 1.5),
                flags: u32::from(index) % 16,
            });
        }
        Self { movers, players }
    }

    /// How many players the scene has: ids `0..players`.
    pub fn players(&self) -> u16 {
        self.players
    }

    /// Every entity, sorted by id.
    pub fn movers(&self) -> &[Mover] {
        &self.movers
    }

    /// The index of entity `id` in [`Scene::movers`].
    pub fn index_of(&self, id: u16) -> Option<usize> {
        self.movers.binary_search_by_key(&id, |mover| mover.id).ok()
    }

    /// Writes the position of every entity at `tick` into `out`, by index.
    pub fn positions_into(&self, tick: u32, out: &mut Vec<[f64; 3]>) {
        out.clear();
        out.extend(self.movers.iter().map(|mover| mover.position(tick)));
    }

    /// Writes every entity into the history frame of `tick`.
    pub fn fill(&self, schema: &Schema, history: &mut History, tick: u32) {
        let frame = history.begin(tick);
        for mover in &self.movers {
            let (values, count) = mover.values(tick);
            let pushed = frame.push(schema, mover.id, mover.kind, &values[..count]);
            assert!(pushed.is_ok(), "entity {} at tick {tick}: {pushed:?}", mover.id);
        }
    }
}

/// A uniform number in `low..high` from `rng`.
fn uniform(rng: &mut SplitMix64, low: f64, high: f64) -> f64 {
    low + (high - low) * rng.next_f64()
}

/// Every entity of `scene` with weight 1, sorted by id: a client that sees the whole scene.
pub fn all_candidates(scene: &Scene) -> Vec<(u16, f32)> {
    scene.movers().iter().map(|mover| (mover.id, 1.0)).collect()
}

/// Rebuilds `spatial` from the positions of every entity at this tick.
pub fn build_spatial(scene: &Scene, positions: &[[f64; 3]], spatial: &mut SpatialHash) {
    spatial.clear();
    for (mover, position) in scene.movers().iter().zip(positions) {
        spatial.insert(mover.id, position[0] as f32, position[2] as f32);
    }
}

/// The entities relevant to the client whose player is entity index `own`: every player, and the others within
/// [`RELEVANCE_M`] of that player. Weights fall with distance, `1 / (1 + d / 50)`. Sorted by id, into `out`.
pub fn relevant_candidates(
    scene: &Scene,
    positions: &[[f64; 3]],
    spatial: &SpatialHash,
    own: usize,
    scratch: &mut Vec<u16>,
    out: &mut Vec<(u16, f32)>,
) {
    let centre = positions[own];
    spatial.query(centre[0] as f32, centre[2] as f32, RELEVANCE_M, scratch);
    out.clear();
    let weight = |index: usize| -> f32 {
        let position = positions[index];
        let distance = ((position[0] - centre[0]).powi(2) + (position[2] - centre[2]).powi(2)).sqrt();
        (1.0 / (1.0 + distance / 50.0)) as f32
    };
    for id in 0..scene.players() {
        let index = scene.index_of(id).unwrap_or(0);
        out.push((id, weight(index)));
    }
    for &id in scratch.iter() {
        if id < scene.players() {
            continue;
        }
        let index = scene.index_of(id).unwrap_or(0);
        out.push((id, weight(index)));
    }
}
