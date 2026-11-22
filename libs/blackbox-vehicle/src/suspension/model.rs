//! An external suspension model: a replacement for the built-in spring, damper and anti-roll force of one axle, called once
//! per axle per tick.
//!
//! The vehicle asks the model about an axle (both wheels together) before it runs the wheels of that axle. A model that
//! answers takes over the vertical force of both wheels for the tick: the force on the body replaces the built-in spring
//! force, the tyre load replaces the load the vehicle derives from it, and the camber is handed to the tyre model of the same
//! wheel the same tick. A model that returns `None` leaves the axle to the built-in spring. The trait names no model
//! structure: how the answer is produced (a linear spring, a damper with knees, a sub-stepped hub, an anti-roll bar) is the
//! model's own business.
//!
//! # Frame, units and signs
//!
//! SI units throughout (newtons, metres, seconds, radians, kilograms). Axes of the vehicle: x right, y up, z forward. Wheel
//! order inside an axle is left then right. Compression is positive when the wheel moves up towards the body. A force on the
//! body is positive when it pushes the body up. Camber is positive when the top of the wheel leans away from the centre line
//! of the car (outward), whichever side the wheel is on.
//!
//! # What the vehicle guarantees
//!
//! - One call per axle per tick, the front axle (index 0) before the rear (index 1), on the thread that steps the vehicle,
//!   from inside the step and before the tyre models of the wheels of the tick are asked. The two wheels of an axle arrive in
//!   one call so a model can integrate them in lockstep (an anti-roll bar couples them).
//! - `dt` is the tick length the vehicle is stepped with, the same for both axles of a tick.
//! - Every field of the query is measured or derived by the vehicle at the start of the tick; the compression is this tick's,
//!   the body acceleration and the tyre force are from the tick before.
//! - A wheel that does not press on the ground (`loaded` false) is in the query too, with a compression of 0: a hub that hangs
//!   from the body still has a spring and a stop.
//!
//! # What the model must return
//!
//! Finite numbers inside the ranges of [`SuspensionResult::validate`]. An answer that fails the check is discarded, the
//! built-in spring runs for both wheels of the axle for this tick and the vehicle counts a fault ([`SuspensionHook::faults`]).

use core::fmt;

use crate::tires::{MAX_FORCE_N, TyreReject};

/// Largest camber a model may return (rad, either sign).
pub const MAX_CAMBER_RAD: f32 = 0.6;

/// Why an answer was refused (the same two reasons as for a tyre answer).
pub type SuspensionReject = TyreReject;

/// What the vehicle tells a model about one wheel of the axle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SuspensionWheelQuery {
    /// True when the wheel presses on the ground.
    pub loaded: bool,
    /// How far the wheel is pushed up from its full droop (m): the ride height plus the penetration of the contact point,
    /// from 0 to `max_travel_m`. 0 means the wheel is not touching.
    pub compression_m: f32,
    /// The rate of change of the compression over the last tick (m/s, positive when compressing).
    pub compression_speed_mps: f32,
    /// The steering angle of this wheel (rad, positive to the right; 0 on an axle that does not steer).
    pub steer_angle_rad: f32,
    /// The acceleration of the body point above this wheel along the body's up axis (m/s^2), kinematic: 0 at rest, about -9.81
    /// in free fall. Taken over the last tick; 0 on the first tick after a model is installed or the vehicle is placed.
    pub body_accel_up_mps2: f32,
    /// The lateral force the tyre of this wheel put on the body in the last tick (N, positive to the right, along the wheel's
    /// lateral axis); 0 for an airborne wheel and on the first tick.
    pub tyre_lateral_force_n: f32,
    /// The vertical load the host estimates for this wheel at rest (N): the weight the axle carries, shared by its two wheels.
    pub static_load_n: f32,
}

/// What the vehicle tells a model about one axle for one tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SuspensionQuery {
    /// 0 front axle, 1 rear axle.
    pub axle: u32,
    /// The tag the caller gave with [`crate::Vehicle::set_suspension_model`]. Opaque to the vehicle.
    pub role: u32,
    /// The tick length (s).
    pub dt: f32,
    /// The compression of a wheel of this axle standing at rest on flat ground (m): the sag at which the vehicle's own spring carries
    /// the static load of the wheel. A road input a model derives from the compression is `compression_m - rest_travel_m`, which
    /// is 0 for a car at rest. It depends on the car (its spring and its weight on the axle), not on the ride-height tuning.
    pub rest_travel_m: f32,
    /// The largest compression (m).
    pub max_travel_m: f32,
    /// The sprung mass the host attributes to one wheel (kg): a quarter of the vehicle mass.
    pub corner_mass_kg: f32,
    /// The left wheel, then the right wheel.
    pub wheels: [SuspensionWheelQuery; 2],
}

/// What a model answers for one wheel.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SuspensionWheelResult {
    /// The force on the body at the wheel along the body's up axis (N, positive up). It may be negative: a droop stop pulls the
    /// body down.
    pub body_force_n: f32,
    /// The vertical force between the road and the tyre (N), at or above 0. It becomes the vertical load of the tyre of a
    /// loaded wheel.
    pub tyre_load_n: f32,
    /// The camber of the wheel (rad, positive outward). It is handed to the tyre model the same tick.
    pub camber_rad: f32,
}

/// What a model answers for one axle.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SuspensionResult {
    /// The left wheel, then the right wheel.
    pub wheels: [SuspensionWheelResult; 2],
}

impl SuspensionResult {
    /// Whether the answer is usable: every field finite and inside its range.
    pub fn validate(&self) -> Result<(), SuspensionReject> {
        let all_finite = self
            .wheels
            .iter()
            .all(|w| w.body_force_n.is_finite() && w.tyre_load_n.is_finite() && w.camber_rad.is_finite());
        if !all_finite {
            return Err(TyreReject::NotFinite);
        }
        let in_range = self.wheels.iter().all(|w| {
            w.body_force_n.abs() <= MAX_FORCE_N
                && (0.0..=MAX_FORCE_N).contains(&w.tyre_load_n)
                && w.camber_rad.abs() <= MAX_CAMBER_RAD
        });
        if !in_range {
            return Err(TyreReject::OutOfRange);
        }
        Ok(())
    }
}

/// A suspension model. See the module documentation for the contract.
pub trait SuspensionModel {
    /// Answer for one axle, or `None` to leave the axle to the built-in spring.
    fn axle(&mut self, query: &SuspensionQuery) -> Option<SuspensionResult>;

    /// The vehicle discarded the last answer. A model that counts its faults does it here.
    fn rejected(&mut self, _why: SuspensionReject) {}

    /// True once the model must not be called again (a mod that was quarantined). The vehicle drops a retired model at the
    /// start of its next wheel loop.
    fn retired(&self) -> bool {
        false
    }

    /// A copy for a cloned vehicle. `None`, the default, gives the clone no model.
    fn duplicate(&self) -> Option<Box<dyn SuspensionModel>> {
        None
    }
}

/// The model slot of a vehicle: at most one model, and the count of answers the vehicle refused.
#[derive(Default)]
pub struct SuspensionHook {
    model: Option<Box<dyn SuspensionModel>>,
    role: u32,
    faults: u32,
}

impl SuspensionHook {
    /// An empty slot: the built-in springs run.
    pub const fn empty() -> Self {
        SuspensionHook { model: None, role: 0, faults: 0 }
    }

    /// Put `model` in the slot; `role` is the tag echoed in every query. The fault count starts again.
    pub fn install(&mut self, model: Box<dyn SuspensionModel>, role: u32) {
        *self = SuspensionHook { model: Some(model), role, faults: 0 };
    }

    /// Empty the slot.
    pub fn clear(&mut self) {
        *self = SuspensionHook::empty();
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

    /// Ask the model about one axle. `None` when no model is installed, when it declines, or when its answer fails the check
    /// (the model is told, and the fault counted).
    pub fn ask(&mut self, query: &SuspensionQuery) -> Option<SuspensionResult> {
        let model = self.model.as_mut()?;
        let result = model.axle(query)?;
        let Err(why) = result.validate() else {
            return Some(result);
        };
        self.faults = self.faults.saturating_add(1);
        model.rejected(why);
        None
    }
}

impl Clone for SuspensionHook {
    /// A clone keeps the model only when the model can duplicate itself (see [`SuspensionModel::duplicate`]).
    fn clone(&self) -> Self {
        let model = self.model.as_ref().and_then(|model| model.duplicate());
        SuspensionHook { model, role: self.role, faults: self.faults }
    }
}

impl fmt::Debug for SuspensionHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SuspensionHook")
            .field("set", &self.model.is_some())
            .field("role", &self.role)
            .field("faults", &self.faults)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> SuspensionResult {
        let wheel = SuspensionWheelResult { body_force_n: 3000.0, tyre_load_n: 3500.0, camber_rad: -0.03 };
        SuspensionResult { wheels: [wheel, wheel] }
    }

    #[test]
    fn a_plain_answer_is_valid_and_a_pulling_force_is_allowed() {
        assert_eq!(good().validate(), Ok(()));
        let mut pulling = good();
        pulling.wheels[0].body_force_n = -2700.0;
        assert_eq!(pulling.validate(), Ok(()), "a droop stop pulls the body down");
    }

    #[test]
    fn a_non_finite_field_is_refused() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut result = good();
            result.wheels[1].camber_rad = bad;
            assert_eq!(result.validate(), Err(TyreReject::NotFinite));
            let mut result = good();
            result.wheels[0].tyre_load_n = bad;
            assert_eq!(result.validate(), Err(TyreReject::NotFinite));
        }
    }

    #[test]
    fn an_out_of_range_field_is_refused() {
        let mut result = good();
        result.wheels[0].tyre_load_n = -1.0;
        assert_eq!(result.validate(), Err(TyreReject::OutOfRange));
        let mut result = good();
        result.wheels[1].body_force_n = MAX_FORCE_N * 2.0;
        assert_eq!(result.validate(), Err(TyreReject::OutOfRange));
        let mut result = good();
        result.wheels[0].camber_rad = 0.7;
        assert_eq!(result.validate(), Err(TyreReject::OutOfRange));
    }
}
