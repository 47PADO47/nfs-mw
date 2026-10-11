//! The throttle and brake controller of racers and cops.
//! Spec: `docs/specs/ai-driver-control-pid.md` (§2).

use crate::pid_error::PidError;

const P: f32 = 0.4;
const I: f32 = 0.01;
const D: f32 = 0.1;
const STAGING_GAS: f32 = 0.8;

/// Pedals the controller asks for.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pedals {
    pub gas: f32,
    pub brake: f32,
    pub handbrake: f32,
}

/// What the throttle controller needs to know this tick.
#[derive(Debug, Clone, Copy)]
pub struct ThrottleInput {
    /// Signed forward speed, m/s.
    pub speed: f32,
    /// The speed to hold, m/s.
    pub want: f32,
    pub reversing_gear: bool,
    pub staging: bool,
    /// The steering controller wants a handbrake turn.
    pub steering_behind: bool,
    /// The reverse controller is backing up on purpose.
    pub reversing_speed: bool,
}

#[derive(Debug, Clone)]
pub struct ThrottlePid {
    /// Signed pedal state, positive gas, negative brake; it persists between ticks.
    state: f32,
    speed_error: PidError,
}

impl Default for ThrottlePid {
    fn default() -> Self {
        Self { state: 0.0, speed_error: PidError::new(4, 4, 30.0) }
    }
}

impl ThrottlePid {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn step(&mut self, input: &ThrottleInput, dt: f32) -> Pedals {
        let mut handbrake = 0.0;
        let error = input.speed - input.want;
        match () {
            _ if input.staging => self.state = STAGING_GAS,
            _ if !input.reversing_speed && input.steering_behind => {
                self.state = 1.0;
                handbrake = 1.0;
            }
            _ => {
                self.speed_error.record(error, dt);
                self.state = match () {
                    _ if input.want < 0.5 => -1.0,
                    _ if input.reversing_gear => match input.speed > 1.0 {
                        true => -1.0,
                        false => 1.0,
                    },
                    _ if input.speed < -1.0 => -1.0,
                    _ => {
                        let integral = self.speed_error.integral().clamp(-5.0, 5.0);
                        let derivative = self.speed_error.derivative().clamp(-10.0, 10.0);
                        self.state - P * error - I * integral - D * derivative
                    }
                };
            }
        }
        self.state = self.state.clamp(-1.0, 1.0);
        Pedals { gas: self.state.max(0.0), brake: (-self.state).max(0.0), handbrake }
    }
}
