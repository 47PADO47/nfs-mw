//! Tyre landings use paired-wheel transitions independently of physical body impacts.
use crate::engine::TICK_SECONDS;
use crate::input::CarInput;
use crate::math::ramp;
const SOFT_AIR: f32 = 0.12;
const HARD_AIR: f32 = 0.7;
const PAIRS: [[usize; 2]; 4] = [[0, 1], [2, 3], [1, 2], [0, 3]];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Landing {
    pub magnitude: f32,
    pub hard: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct LandingFx {
    air: [f32; 4],
    touched: [bool; 4],
    leaning: bool,
}

impl LandingFx {
    pub fn tick(&mut self, input: &CarInput, landing: &mut Option<Landing>) {
        if input.wheels_on_ground() == 0 && input.up_dot < 0.8 {
            self.leaning = true;
        }
        let grounded = PAIRS.map(|pair| pair.into_iter().all(|i| input.wheels[i].on_ground));
        let front = grounded[0] && !self.touched[0] && self.air[0] > SOFT_AIR;
        let rear = grounded[1] && !self.touched[1] && self.air[1] > SOFT_AIR;
        let tipped = self.leaning && (0..4).any(|i| grounded[i] && !self.touched[i]);
        if front || (rear && !self.leaning) || tipped {
            let pair = if front { [0, 1] } else { [2, 3] };
            let squash = pair.map(|i| input.wheels[i].compression).into_iter().sum();
            let magnitude = (4.0 * ramp(squash, 0.0, 0.65, 0.0, 1.0) * 63.5 / 127.0).clamp(0.0, 1.0);
            let hard = tipped || (front && self.air[0] > HARD_AIR) || (rear && self.air[1] > HARD_AIR);
            *landing = Some(Landing { magnitude, hard });
        }
        if input.wheels_on_ground() == 1 {
            self.leaning = false;
        }
        for (i, grounded) in grounded.into_iter().enumerate() {
            self.touched[i] = grounded;
            self.air[i] = match grounded {
                true => 0.0,
                false => self.air[i] + TICK_SECONDS,
            };
        }
    }
}
