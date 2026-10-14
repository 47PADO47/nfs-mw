//! Authored road transition selection and Q15 parameters.
use crate::audio::mixer::ambient::AmbientLevels;
use blackbox_carsound::{CarInput, NO_ROAD_NOISE};

pub(super) const WHEELS: [[usize; 2]; 2] = [[0, 3], [1, 2]];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoadTerrain {
    pub key: u32,
    pub transition_on: i32,
    pub transition_off: i32,
    pub secondary: bool,
}

impl Default for RoadTerrain {
    fn default() -> Self {
        Self { key: 0, transition_on: -1, transition_off: -1, secondary: false }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RoadContext {
    pub terrain: [RoadTerrain; 4],
    /// 0 intact, 1 punctured, 2 fully blown; supplied by actual tyre state.
    pub damage: [u8; 4],
}

#[derive(Default)]
pub(super) struct History {
    previous: Option<RoadContext>,
    loops: [u8; 4],
}

impl History {
    pub fn observe(&mut self, input: &CarInput, context: Option<&RoadContext>) -> [Option<i32>; 2] {
        let Some(context) = context else {
            self.previous = None;
            return [None; 2];
        };
        let previous = self.previous.replace(*context);
        let old_loops = std::mem::replace(&mut self.loops, input.wheels.map(|w| w.road_noise_loop));
        let Some(previous) = previous else { return [None; 2] };
        std::array::from_fn(|side| {
            let wheels = WHEELS[side];
            let mut selected = None;
            if wheels.iter().any(|&w| input.wheels[w].on_ground) {
                for w in wheels {
                    if context.terrain[w].key == previous.terrain[w].key || self.loops[w] == old_loops[w] {
                        continue;
                    }
                    selected = transition_id(context.terrain[w].transition_on, previous.terrain[w].transition_off);
                }
            }
            if wheels.iter().any(|&w| previous.damage[w] == 0 && context.damage[w] == 1) {
                selected = Some(2);
            }
            if wheels.iter().any(|&w| previous.damage[w] < 2 && context.damage[w] == 2) {
                selected = Some(3);
            }
            selected
        })
    }
}

fn transition_id(on: i32, off: i32) -> Option<i32> {
    if on == 4 || off == 4 {
        return None;
    }
    if (0..4).contains(&on) {
        return Some(on);
    }
    (0..4).contains(&off).then_some(off)
}

pub(super) fn no_road(input: &CarInput, side: usize) -> bool {
    WHEELS[side].iter().all(|&w| input.wheels[w].road_noise_loop == NO_ROAD_NOISE)
}

pub(super) fn multiply(a: i32, b: i32) -> i32 {
    ((i64::from(a) * i64::from(b)) >> 15) as i32
}

pub(super) fn params(id: u8, volume: i32, pitch: i32, azimuth: i32, secondary: bool, wet: i32) -> [i32; 11] {
    [i32::from(id), volume, pitch, azimuth, 0, i32::from(secondary), 0, 25000, 0, 32767, wet]
}

pub(super) fn transition(id: i32, speed: f32, side: usize, levels: &AmbientLevels) -> [i32; 11] {
    let base = match id {
        0 | 1 => 12000,
        2 => 24500,
        _ => 32767,
    };
    let slot = match id {
        2 => 1,
        3 => 2,
        _ => 0,
    };
    let volume = multiply(multiply(base, 32767), levels.road_transition[slot]);
    let mph = match speed.abs() {
        v if v.is_finite() => v * 2.23699,
        _ => 0.0,
    };
    let pitch = (3600.0 + 800.0 * ((mph - 12.0) / 48.0).clamp(0.0, 1.0)) as i32;
    [
        id,
        volume.clamp(0, 32767),
        pitch.clamp(0, 8191),
        levels.road_azimuth[side],
        1,
        0,
        0,
        25000,
        0,
        32767,
        levels.road_wet,
    ]
}
