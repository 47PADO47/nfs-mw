//! Landings after a jump and bottoming out. Spec §8 ("Landing / bottom out").

use crate::engine::TICK_SECONDS;
use crate::input::CarInput;
use crate::math::ramp;

/// An axle pair must have been in the air this long to count as landing, and this long for a hard one.
const SOFT_AIR: f32 = 0.12;
const HARD_AIR: f32 = 0.7;
/// Seconds before another landing can sound.
const COOLDOWN: f32 = 0.25;

/// A landing the wheels made.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Landing {
    /// How hard, 0 to 1 (the original's 0 to 127 intensity divided by 127).
    pub magnitude: f32,
    /// After a long flight or on the side or roof.
    pub hard: bool,
}

/// The wheel pairs: front, rear, right and left (indices in the input's wheel order).
const PAIRS: [[usize; 2]; 4] = [[0, 1], [2, 3], [1, 2], [0, 3]];

#[derive(Debug, Clone, Default)]
pub(crate) struct LandingFx {
    air: [f32; 4],
    cooldown: f32,
}

impl LandingFx {
    pub fn tick(&mut self, input: &CarInput, landing: &mut Option<Landing>) {
        self.cooldown = (self.cooldown - TICK_SECONDS).max(0.0);
        let front = input.wheels[0].compression + input.wheels[1].compression;
        let rear = input.wheels[2].compression + input.wheels[3].compression;
        let leaning = input.up_dot < 0.8;
        for (air, pair) in self.air.iter_mut().zip(PAIRS) {
            let grounded = pair.iter().any(|&i| input.wheels[i].on_ground);
            if !grounded {
                *air += TICK_SECONDS;
                continue;
            }
            let flight = std::mem::take(air);
            if flight <= SOFT_AIR || self.cooldown > 0.0 {
                continue;
            }
            self.cooldown = COOLDOWN;
            let squash = front.max(rear);
            let magnitude = (4.0 * ramp(squash, 0.0, 0.65, 0.0, 1.0) * 63.5 / 127.0).clamp(0.0, 1.0);
            *landing = Some(Landing { magnitude, hard: flight > HARD_AIR || leaning });
        }
    }
}
