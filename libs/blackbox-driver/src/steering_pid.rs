//! The steering controller of racers and cops.
//! Spec: `docs/specs/ai-driver-control-pid.md` (§3).

use crate::adaptive::{AdaptivePid, AdaptiveSettings};
use crate::lookup::{Graph, Table};
use crate::pid_error::PidError;

/// Heading error (degrees) to the rate (degrees per second) it should change at.
const HEADING_ERROR_MODEL: Graph<10> = Graph {
    points: [
        (0.0, 0.0),
        (3.0, -1.0),
        (6.0, -2.05),
        (9.0, -3.5),
        (12.0, -4.94),
        (15.0, -6.14),
        (18.0, -7.35),
        (21.0, -8.55),
        (24.0, -9.4),
        (27.0, -10.0),
    ],
};

/// Coefficients below `SEED_SPEED` come from these speed-indexed tables (0…160 m/s).
const SEED_SPEED: f32 = 10.0;
const SEED_P: Table<10> =
    Table { data: [0.328, 0.22, 0.148, 0.115, 0.09, 0.074, 0.057, 0.049, 0.043, 0.04], min: 0.0, max: 160.0 };
const SEED_I: Table<10> =
    Table { data: [0.21, 0.244, 0.267, 0.29, 0.305, 0.321, 0.328, 0.336, 0.341, 0.344], min: 0.0, max: 160.0 };
const SEED_D: Table<10> =
    Table { data: [0.0, 0.0758, 0.0758, 0.0738, 0.0604, 0.047, 0.047, 0.0403, 0.0403, 0.0403], min: 0.0, max: 160.0 };

#[derive(Debug, Clone)]
pub struct SteeringPid {
    body_error: PidError,
    controller: AdaptivePid,
}

impl SteeringPid {
    pub fn new(settings: AdaptiveSettings) -> Self {
        Self { body_error: PidError::new(5, 5, 30.0), controller: AdaptivePid::new(settings) }
    }

    pub fn reset(&mut self) {
        self.body_error.reset();
    }

    /// The steering in -1…1 for a signed heading error to the target (`angle`, radians, positive when
    /// the target is on the right), at planar `speed`. `max_steer` is the car's maximum steering angle in
    /// radians; `clock` is the simulation time in seconds.
    pub fn step(&mut self, angle: f32, speed: f32, max_steer: f32, dt: f32, clock: f32) -> f32 {
        self.body_error.record(angle, dt);
        let angle_error = self.body_error.error();
        let integral = self.body_error.integral().clamp(-0.5, 0.5);
        let derivative = self.body_error.derivative().clamp(-10.0, 10.0);
        self.controller.set_terms(angle_error, integral, derivative);
        let growing = angle_error * derivative >= 0.0;
        let rate = match growing {
            true => derivative.abs(),
            false => -derivative.abs(),
        };
        let model = HEADING_ERROR_MODEL.lookup(angle_error.to_degrees().abs());
        self.controller.update(model, rate.to_degrees(), dt, clock);
        if speed < SEED_SPEED {
            self.controller.force_coefficients(SEED_P.lookup(speed), SEED_I.lookup(speed), SEED_D.lookup(speed));
        }
        (self.controller.output() / max_steer.max(1e-3)).clamp(-1.0, 1.0)
    }
}
