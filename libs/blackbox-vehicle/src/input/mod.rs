//! Driver input: raw pedals and stick to the controls the car uses. Dead zones, automatic reverse through
//! the brake pedal, the idle auto-brake, handbrake priority and shift requests. Spec section 8 of
//! `docs/specs/vehicle-input-induction-brakes.md`.

use crate::drivetrain::GEAR_REVERSE;
use crate::math::{finite_or, ramp};
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
    /// A gear asked for by number (`GEAR_REVERSE` 0, `GEAR_NEUTRAL` 1, `GEAR_FIRST` 2, ...): a wheel's H-pattern
    /// shifter or a gear key. Only a manual gearbox listens, and it wins over the shift edges and the automatic
    /// reverse. The rewrite's own control, not the original's (spec `vehicle-manual-shifting.md`, section 6).
    pub gear_select: Option<usize>,
    /// Clutch pedal travel, 0 (released) to 1 (fully pressed). Read only when `ControlConfig::manual_clutch` is on.
    pub clutch: f32,
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
    /// A clutch pedal exists: while it is pressed the clutch stays open. Off by default, as in the original, whose
    /// clutch is automatic.
    pub manual_clutch: bool,
    /// The driver has a gear selector that holds a gear (an H-pattern shifter): with a manual gearbox the brake pedal
    /// never picks reverse and the pedals are never swapped, the selector decides.
    pub h_shifter: bool,
}

impl Default for ControlConfig {
    fn default() -> Self {
        Self {
            dead_zone: 0.05,
            auto_reverse: true,
            auto_brake: true,
            automatic: true,
            steering_device: SteeringDevice::Pad,
            manual_clutch: false,
            h_shifter: false,
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
    /// Engage this gear id directly (reverse only while crawling: the powertrain refuses it at speed).
    Select(usize),
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
    /// The clutch pedal, 0..1; `None` when there is no pedal (`ControlConfig::manual_clutch` off).
    pub clutch: Option<f32>,
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
    let v = finite_or(v, 0.0).clamp(0.0, 1.0);
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
            steering: finite_or(input.steer, 0.0).clamp(-1.0, 1.0),
            nos: false,
            gear_request: None,
            clutch: None,
        };
    }
    let mut gas = snap(input.throttle, config.dead_zone);
    let mut brake = snap(input.brake, config.dead_zone);
    let handbrake = finite_or(input.handbrake, 0.0).clamp(0.0, 1.0);
    let steer = finite_or(input.steer, 0.0).clamp(-1.0, 1.0);
    let raw_gas = gas;
    let raw_brake = brake;
    let v = ctx.forward_speed;
    // A gear asked for by number belongs to a manual gearbox; the automatic one picks its own.
    let mut gear_request = input.gear_select.filter(|_| !config.automatic).map(GearRequest::Select);
    let explicit = gear_request.is_some();
    // A selector that holds a gear (the H-pattern shifter) is the driver's own reverse: no pedal tricks.
    let auto_reverse = config.auto_reverse && !(config.h_shifter && !config.automatic);

    if auto_reverse {
        if explicit {
            // The request stands; the automatic reverse waits.
        } else if ctx.gear != GEAR_REVERSE {
            if v < 2.5 && raw_brake > 0.0 && raw_gas == 0.0 {
                gear_request = Some(GearRequest::Reverse);
            }
        } else if v > -5.0 && (raw_brake == 0.0 || raw_gas > 0.0) {
            gear_request = Some(GearRequest::First);
        }
        let reversing = match gear_request {
            Some(GearRequest::Reverse) => true,
            Some(GearRequest::First) => false,
            Some(GearRequest::Select(gear)) => gear == GEAR_REVERSE,
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
    let clutch = config.manual_clutch.then(|| finite_or(input.clutch, 0.0).clamp(0.0, 1.0));
    Controls { gas, brake, handbrake, steering: steer, nos: input.nos, gear_request, clutch }
}

#[cfg(test)]
mod tests;
