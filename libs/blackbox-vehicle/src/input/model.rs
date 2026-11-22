//! An external controls model: a filter between the driver's pedals and the vehicle, called once per tick for one vehicle.
//!
//! The vehicle shapes the driver's input as usual (dead zone, automatic reverse through the brake pedal, idle hold,
//! handbrake priority) and then asks the model. A model that answers replaces the throttle the engine sees, replaces the
//! brake split by a demand per wheel, and may replace the steering target. A model that returns `None` leaves the controls
//! as they are. The trait names no aid: traction control, ABS, stability control, a brake-bias map, a steering-feel model or
//! a plain pass-through all fit.
//!
//! # Frame, units and signs
//!
//! Pedals are 0 to 1. Steering is -1 to 1 with positive to the right. Speeds are m/s, accelerations m/s^2, angles rad unless a
//! field says turns. Positive yaw rate and positive lateral acceleration are to the right, positive longitudinal
//! acceleration is forward. A slip ratio is `(wheel surface speed - ground speed) / ground speed` measured along the
//! direction of travel of the wheel: above 0 the wheel spins faster than the ground moves under it, below 0 it is
//! dragging, -1 is a locked wheel. A slip angle is in turns (1.0 is 360 degrees), positive when the contact patch moves to the
//! right of the wheel's heading.
//!
//! # What the vehicle guarantees
//!
//! - One call per tick, before the controls are turned into the engine, brake and steering commands of the tick, on the
//!   thread that steps the vehicle, from inside the step. Not called on a tick the vehicle does not respond (a frozen body).
//! - The pedals in the query are the driver's request after the vehicle's input shaping: the dead zone is applied, the pedals
//!   are already swapped while the vehicle reverses, and the idle hold counts as a brake request. The steering is the raw
//!   input clamped to -1 to 1.
//! - Wheel slip, slip angle and the body's accelerations are the results of the previous tick (this tick's wheels have not run
//!   yet). A wheel that is not loaded reports 0 for both.
//! - `forward_speed` is signed: negative while the vehicle moves backwards.
//!
//! # What the model must return
//!
//! Finite numbers inside the ranges of [`ControlsResult::validate`]. The throttle replaces the throttle the engine is given.
//! The brake is a demand per wheel, bias folded in, where 1 is the full brake torque of that wheel; the host's own brake
//! balance slider and launch-stand rule are not applied on top of it. A handbrake request, a blown tyre and a disabled
//! vehicle still act as before. The steering target, when given, is the angle of the inside front wheel in radians (positive to
//! the right); the vehicle then bypasses its own steering shaping (speed-dependent range, rate limit, counter-steer window)
//! and keeps the Ackermann split, the 45 degree mechanical limit and the post-collision limit. A model that cannot answer
//! returns `None`.
//!
//! An answer that fails the check is discarded, the controls stay as shaped and the vehicle counts a fault
//! ([`ControlsHook::faults`]).

use core::fmt;

use crate::tires::TyreReject;

/// Largest brake demand a model may return for a wheel: 1.5 of the full brake torque (the host's own balance slider produces
/// that much on the heavier axle).
pub const MAX_WHEEL_BRAKE: f32 = 1.5;
/// Largest steering target a model may return (rad, either sign). The vehicle limits it further to its mechanical lock.
pub const MAX_STEER_TARGET_RAD: f32 = 1.5;

/// Why an answer was refused (the same two reasons as for a tyre answer).
pub type ControlsReject = TyreReject;

/// What the vehicle tells a model about the driver and the car for one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlsQuery {
    /// The tag the caller gave with [`crate::Vehicle::set_controls_model`]. Opaque to the vehicle.
    pub role: u32,
    /// The tick length (s).
    pub dt: f32,
    /// The throttle request, 0 to 1.
    pub throttle: f32,
    /// The brake request, 0 to 1.
    pub brake: f32,
    /// The handbrake request, 0 to 1.
    pub handbrake: f32,
    /// The steering request, -1 to 1, positive to the right.
    pub steer: f32,
    /// The speed of the vehicle along its heading (m/s, negative while moving backwards).
    pub forward_speed: f32,
    /// Rotation about the body's up axis (rad/s, positive to the right).
    pub yaw_rate: f32,
    /// Acceleration of the body along its lateral axis over the last tick (m/s^2, positive to the right).
    pub lateral_accel: f32,
    /// Acceleration of the body along its heading over the last tick (m/s^2, positive forward).
    pub longitudinal_accel: f32,
    /// Whether each wheel pressed on the ground in the last tick (front left, front right, rear left, rear right).
    pub wheel_loaded: [bool; 4],
    /// The slip ratio of each wheel in the last tick.
    pub slip_ratio: [f32; 4],
    /// The slip angle of each wheel in the last tick, in turns.
    pub slip_angle: [f32; 4],
}

/// What a model answers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlsResult {
    /// The throttle the engine is given, 0 to 1.
    pub throttle: f32,
    /// The brake demand of each wheel, 0 to [`MAX_WHEEL_BRAKE`]; 1 is the full brake torque of that wheel.
    pub brake: [f32; 4],
    /// The angle of the inside front wheel (rad, positive to the right), or `None` to leave the steering to the vehicle.
    pub steer_target: Option<f32>,
}

impl ControlsResult {
    /// Whether the answer is usable: every field finite and inside its range.
    pub fn validate(&self) -> Result<(), ControlsReject> {
        let steer = self.steer_target.unwrap_or(0.0);
        let all_finite = self.throttle.is_finite() && steer.is_finite() && self.brake.iter().all(|b| b.is_finite());
        if !all_finite {
            return Err(TyreReject::NotFinite);
        }
        let in_range = (0.0..=1.0).contains(&self.throttle)
            && steer.abs() <= MAX_STEER_TARGET_RAD
            && self.brake.iter().all(|b| (0.0..=MAX_WHEEL_BRAKE).contains(b));
        if !in_range {
            return Err(TyreReject::OutOfRange);
        }
        Ok(())
    }
}

/// A controls model. See the module documentation for the contract.
pub trait ControlsModel {
    /// Filter the controls, or `None` to leave them as they are.
    fn filter(&mut self, query: &ControlsQuery) -> Option<ControlsResult>;

    /// The vehicle discarded the last answer. A model that counts its faults does it here.
    fn rejected(&mut self, _why: ControlsReject) {}

    /// True once the model must not be called again (a mod that was quarantined). The vehicle drops a retired model at the
    /// start of its next controls update.
    fn retired(&self) -> bool {
        false
    }

    /// A copy for a cloned vehicle. `None`, the default, gives the clone no model.
    fn duplicate(&self) -> Option<Box<dyn ControlsModel>> {
        None
    }
}

/// The model slot of a vehicle: at most one model, and the count of answers the vehicle refused.
#[derive(Default)]
pub struct ControlsHook {
    model: Option<Box<dyn ControlsModel>>,
    role: u32,
    faults: u32,
}

impl ControlsHook {
    /// An empty slot: the controls stay as shaped.
    pub const fn empty() -> Self {
        ControlsHook { model: None, role: 0, faults: 0 }
    }

    /// Put `model` in the slot; `role` is the tag echoed in every query. The fault count starts again.
    pub fn install(&mut self, model: Box<dyn ControlsModel>, role: u32) {
        *self = ControlsHook { model: Some(model), role, faults: 0 };
    }

    /// Empty the slot.
    pub fn clear(&mut self) {
        *self = ControlsHook::empty();
    }

    /// Whether a model is installed.
    pub fn is_set(&self) -> bool {
        self.model.is_some()
    }

    /// The answers the vehicle refused since the model was installed.
    pub fn faults(&self) -> u32 {
        self.faults
    }

    /// The tag given at install time.
    pub fn role(&self) -> u32 {
        self.role
    }

    /// Drop a model that says it is retired. Returns whether one was dropped.
    pub fn drop_retired(&mut self) -> bool {
        if !self.model.as_ref().is_some_and(|model| model.retired()) {
            return false;
        }
        self.model = None;
        true
    }

    /// Ask the model. `None` when no model is installed, when it declines, or when its answer fails the check (the model is
    /// told, and the fault counted).
    pub fn ask(&mut self, query: &ControlsQuery) -> Option<ControlsResult> {
        let model = self.model.as_mut()?;
        let result = model.filter(query)?;
        let Err(why) = result.validate() else {
            return Some(result);
        };
        self.faults = self.faults.saturating_add(1);
        model.rejected(why);
        None
    }
}

impl Clone for ControlsHook {
    /// A clone keeps the model only when the model can duplicate itself (see [`ControlsModel::duplicate`]).
    fn clone(&self) -> Self {
        let model = self.model.as_ref().and_then(|model| model.duplicate());
        ControlsHook { model, role: self.role, faults: self.faults }
    }
}

impl fmt::Debug for ControlsHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControlsHook")
            .field("set", &self.model.is_some())
            .field("role", &self.role)
            .field("faults", &self.faults)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> ControlsResult {
        ControlsResult { throttle: 0.5, brake: [0.2, 0.2, 0.1, 0.1], steer_target: Some(0.1) }
    }

    #[test]
    fn a_plain_answer_is_valid() {
        assert_eq!(good().validate(), Ok(()));
        assert_eq!(ControlsResult { steer_target: None, ..good() }.validate(), Ok(()));
    }

    #[test]
    fn a_non_finite_field_is_refused() {
        for bad in [f32::NAN, f32::INFINITY] {
            assert_eq!(ControlsResult { throttle: bad, ..good() }.validate(), Err(TyreReject::NotFinite));
            assert_eq!(ControlsResult { brake: [0.0, bad, 0.0, 0.0], ..good() }.validate(), Err(TyreReject::NotFinite));
            assert_eq!(ControlsResult { steer_target: Some(bad), ..good() }.validate(), Err(TyreReject::NotFinite));
        }
    }

    #[test]
    fn an_out_of_range_field_is_refused() {
        assert_eq!(ControlsResult { throttle: 1.2, ..good() }.validate(), Err(TyreReject::OutOfRange));
        assert_eq!(ControlsResult { throttle: -0.1, ..good() }.validate(), Err(TyreReject::OutOfRange));
        assert_eq!(ControlsResult { brake: [0.0, 0.0, 1.6, 0.0], ..good() }.validate(), Err(TyreReject::OutOfRange));
        assert_eq!(ControlsResult { brake: [0.0, -0.2, 0.0, 0.0], ..good() }.validate(), Err(TyreReject::OutOfRange));
        assert_eq!(ControlsResult { steer_target: Some(2.0), ..good() }.validate(), Err(TyreReject::OutOfRange));
    }
}
