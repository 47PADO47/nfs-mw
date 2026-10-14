//! Alternating launch grip state, chassis assist spec §4.3.

use crate::math::{graph, mph_to_ms};

const GRIP: [(f32, f32); 6] = [(0.0, 1.0), (5.0, 0.8), (9.0, 0.9), (12.6, 0.833), (17.1, 0.72), (25.0, 0.65)];

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Burnout {
    pub state: usize,
    pub traction: f32,
    time: f32,
    release_time: f32,
}

impl Burnout {
    pub fn update(&mut self, dt: f32, active: bool, speed: f32, body_slip: f32, slip: f32, wheel: usize) {
        if !active {
            *self = Self::default();
            return;
        }
        if self.state > 0 {
            if speed > mph_to_ms(5.0) && body_slip.abs() > 0.5 {
                *self = Self::default();
                return;
            }
            if slip < 0.5 {
                self.release_time += dt;
                if self.release_time > 1.0 {
                    *self = Self::default();
                }
                return;
            }
            self.release_time = 0.0;
            self.time -= dt;
            if self.time < 0.0 {
                self.state -= 1;
                self.time = 2.0;
            }
            return;
        }
        if speed >= mph_to_ms(20.0) || slip <= 0.5 {
            return;
        }
        let grip = graph(&GRIP, slip);
        self.traction = grip / 1.4;
        self.state = (6.0 * (1.5 - grip) + (wheel & 1) as f32) as usize;
        self.time = 2.0;
        self.release_time = 0.0;
    }
}
