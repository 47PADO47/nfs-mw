use super::tables::*;
use super::window::Window;
use crate::math::{graph, mph_to_ms, ramp, table};

/// 0.55 s of samples at 60 Hz.
const AVG_SAMPLES: usize = 33;
/// 0.15 s of samples at 60 Hz.
const SPEED_SAMPLES: usize = 9;

/// How the steering input is produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SteeringDevice {
    /// Gamepad or keyboard: speed-sensitive range and a limited rate.
    #[default]
    Pad,
    /// Racing wheel with a speed-sensitive range; the wheel itself sets the rate.
    WheelSpeedSensitive,
    /// Racing wheel with full range at every speed.
    WheelFixed,
    /// Computer-driven: the full angle times the input, no shaping.
    Ai,
}

/// Inputs of one steering update.
#[derive(Clone, Copy, Debug)]
pub struct SteeringInput {
    pub dt: f32,
    /// Steering input -1..1, + = right.
    pub steer: f32,
    pub gas: f32,
    pub brake: f32,
    pub ebrake: f32,
    /// Speed along the car forward axis (m/s, negative when reversing).
    pub forward_speed: f32,
    /// `STEERING` of the tire data.
    pub tire_steering: f32,
    /// Steering tuning slider: range x (1 + 0.2 s).
    pub tuning: f32,
    pub device: SteeringDevice,
    /// Slip angle in degrees of the rear left and rear right tires (+ = the patch slides to the right).
    pub rear_slip_deg: [f32; 2],
}

/// Steering shaping state.
#[derive(Clone, Copy, Debug, Default)]
pub struct Steering {
    prev_angle: f32,
    last_max: f32,
    last_input: f32,
    avg_input: Window<AVG_SAMPLES>,
    input_speed: Window<SPEED_SAMPLES>,
    /// Counts down from the hit strength (at most 1) to 0; limits the angle after a hard collision.
    collision_timer: f32,
}

impl Steering {
    /// Current inside-wheel target angle in degrees (+ = right).
    pub fn angle(&self) -> f32 {
        self.prev_angle
    }

    /// A hard hit with the given impulse limits the steering for up to a second.
    pub fn notify_collision(&mut self, impulse: f32) {
        if impulse > 10.0 {
            self.collision_timer = self.collision_timer.max(ramp(impulse, 10.0, 40.0));
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Returns the inside-wheel target angle in degrees (+ = right), at most 45.
    pub fn update(&mut self, i: &SteeringInput) -> f32 {
        let u = i.steer.clamp(-1.0, 1.0);
        if i.device == SteeringDevice::Ai {
            self.prev_angle = ABSOLUTE_MAX_STEERING * i.tire_steering * u;
            return self.prev_angle;
        }
        let v = i.forward_speed;
        self.collision_timer = (self.collision_timer - i.dt).max(0.0);

        // Averages of the (remapped) input and of how fast it moves.
        let remap = input_remap();
        let remapped = table(&remap, -1.0, 1.0, u);
        self.avg_input.push(remapped);
        let avg = self.avg_input.mean().abs();
        self.input_speed.push(((u - self.last_input) / i.dt).abs());
        self.last_input = u;
        let input_speed = self.input_speed.mean();

        let range = match i.device {
            SteeringDevice::Pad => table(&RANGE_PAD, 0.0, MAX_SPEED_AXIS, v),
            SteeringDevice::WheelSpeedSensitive => table(&RANGE_WHEEL, 0.0, MAX_SPEED_AXIS, v),
            SteeringDevice::WheelFixed | SteeringDevice::Ai => ABSOLUTE_MAX_STEERING,
        };
        let speed_coef = table(&STEER_SPEED, 0.0, MAX_SPEED_AXIS, v);
        // Braking widens the angle: 0 on full throttle, up to 1 braking with the handbrake.
        let tb = 1.0 - (i.gas + 1.0 - (i.brake + i.ebrake) * 0.5) * 0.5;
        let mut max = range * (1.45 * tb * speed_coef + 1.0);
        max *= table(&RANGE_COEF, 0.0, 1.0, avg);
        max *= 1.0 + 0.2 * i.tuning;

        if self.collision_timer > 0.0 {
            let secs = 1.0 - self.collision_timer;
            let speed_term = 1.0 - (v / (mph_to_ms(170.0) * 0.7)).clamp(0.0, 1.0);
            let blend = graph(&COLLISION_BLEND, secs);
            max *= speed_term * (1.0 - blend) + blend;
        }

        // Counter-steer: steering into a slide allows at least the slip angle of the sliding rear tire.
        if u > 0.0 && i.rear_slip_deg[1] > 0.0 {
            max = max.max(i.rear_slip_deg[1]).min(ABSOLUTE_MAX_STEERING);
        } else if u < 0.0 && i.rear_slip_deg[0] < 0.0 {
            max = max.max(-i.rear_slip_deg[0]).min(ABSOLUTE_MAX_STEERING);
        } else if avg >= 0.5 {
            max = max.max(self.last_max);
        }
        max = max.min(ABSOLUTE_MAX_STEERING);
        self.last_max = max;

        let target = (u * max * i.tire_steering).clamp(-ABSOLUTE_MAX_STEERING, ABSOLUTE_MAX_STEERING);
        self.prev_angle = if i.device == SteeringDevice::Pad {
            let rate = 180.0
                * i.tire_steering
                * speed_coef
                * table(&INPUT_SPEED_COEF, 0.0, 10.0, input_speed)
                * table(&INPUT_COEF, 0.0, 1.0, avg);
            target.clamp(self.prev_angle - rate * i.dt, self.prev_angle + rate * i.dt)
        } else {
            target
        };
        self.prev_angle
    }
}
