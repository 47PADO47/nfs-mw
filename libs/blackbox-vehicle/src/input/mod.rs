//! Driver input: raw pedals and stick to the controls the car uses. Dead zones, automatic reverse through
//! the brake pedal, the idle auto-brake, handbrake priority and shift requests. Spec section 8 of
//! `docs/specs/vehicle-input-induction-brakes.md`.

use crate::drivetrain::GEAR_REVERSE;
use crate::math::ramp;
use crate::steering::SteeringDevice;

/// What the player (or an AI) asks for in one step.
#[derive(Clone, Copy, Debug, Default)]
pub struct InputState {
    /// Gas pedal, 0..1.
    pub throttle: f32,
    /// Brake pedal, 0..1 (also the reverse pedal when stopped).
    pub brake: f32,
    /// Steering, -1..1, + = right.
    pub steer: f32,
    /// Handbrake, 0..1.
    pub handbrake: f32,
    /// The nitrous button is held.
    pub nos: bool,
    /// Shift request edges: set for one step to shift up or down by one gear.
    pub shift_up: bool,
    pub shift_down: bool,
}

/// How raw input is interpreted.
#[derive(Clone, Copy, Debug)]
pub struct ControlConfig {
    /// Pedal values above `1 - dead_zone` snap to 1 and below `dead_zone` snap to 0.
    pub dead_zone: f32,
    /// The brake pedal selects reverse when stopped (and swaps the pedals in reverse).
    pub auto_reverse: bool,
    /// The idle auto-brake holds the car when no pedal is pressed.
    pub auto_brake: bool,
    /// The gearbox shifts by itself.
    pub automatic: bool,
    pub steering_device: SteeringDevice,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            dead_zone: 0.05,
            auto_reverse: true,
            auto_brake: true,
            automatic: true,
            steering_device: SteeringDevice::Pad,
        }
    }
}

/// A gear change the input layer asks the powertrain for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GearRequest {
    /// Engage reverse.
    Reverse,
    /// Engage first gear.
    First,
    /// Shift by one gear (+1 up, -1 down).
    Shift(i32),
}

/// The shaped controls the chassis and engine use.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Controls {
    pub gas: f32,
    pub brake: f32,
    pub handbrake: f32,
    pub steering: f32,
    pub nos: bool,
    pub gear_request: Option<GearRequest>,
}

/// State the shaping needs from the car.
#[derive(Clone, Copy, Debug)]
pub struct InputContext {
    /// Speed along the car forward axis, m/s (negative when moving backwards).
    pub forward_speed: f32,
    pub gear: usize,
    /// Engine blown or car destroyed: gas 0, brakes on.
    pub disabled: bool,
}

fn snap(v: f32, dead_zone: f32) -> f32 {
    let v = v.clamp(0.0, 1.0);
    if v > 1.0 - dead_zone {
        1.0
    } else if v < dead_zone {
        0.0
    } else {
        v
    }
}

/// Turns raw input into controls.
pub fn shape(input: &InputState, config: &ControlConfig, ctx: &InputContext) -> Controls {
    if ctx.disabled {
        return Controls {
            gas: 0.0,
            brake: 1.0,
            handbrake: 1.0,
            steering: input.steer.clamp(-1.0, 1.0),
            nos: false,
            gear_request: None,
        };
    }
    let mut gas = snap(input.throttle, config.dead_zone);
    let mut brake = snap(input.brake, config.dead_zone);
    let handbrake = input.handbrake.clamp(0.0, 1.0);
    let raw_gas = gas;
    let raw_brake = brake;
    let v = ctx.forward_speed;
    let mut gear_request = None;

    if config.auto_reverse {
        if ctx.gear != GEAR_REVERSE {
            if v < 2.5 && raw_brake > 0.0 && raw_gas == 0.0 {
                gear_request = Some(GearRequest::Reverse);
            }
        } else if v > -5.0 && (raw_brake == 0.0 || raw_gas > 0.0) {
            gear_request = Some(GearRequest::First);
        }
        let reversing = match gear_request {
            Some(GearRequest::Reverse) => true,
            Some(GearRequest::First) => false,
            _ => ctx.gear == GEAR_REVERSE,
        };
        if reversing {
            std::mem::swap(&mut gas, &mut brake);
        }
        if config.auto_brake && raw_gas == 0.0 && raw_brake == 0.0 {
            brake = if reversing {
                1.0 - 0.75 * ramp(-v, 0.0, 10.0)
            } else if v < 8.0 {
                1.0 - 0.75 * ramp(v, 0.0, 8.0)
            } else {
                0.0
            };
        }
    } else if config.auto_brake && raw_gas == 0.0 && raw_brake == 0.0 && v < 8.0 {
        brake = 1.0 - 0.75 * ramp(v, 0.0, 8.0);
    }
    if handbrake > 0.0 {
        brake = 0.0;
    }

    if gear_request.is_none() {
        let dir = input.shift_up as i32 - input.shift_down as i32;
        if dir != 0 && ctx.gear != GEAR_REVERSE {
            gear_request = Some(GearRequest::Shift(dir));
        }
    }
    Controls { gas, brake, handbrake, steering: input.steer.clamp(-1.0, 1.0), nos: input.nos, gear_request }
}

#[cfg(test)]
mod tests;
