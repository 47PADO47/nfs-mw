//! An external tyre model: a replacement for the built-in tyre update of one vehicle, called once per wheel per tick.
//!
//! The vehicle asks the model for each wheel before it runs its own update. A model that answers takes over that wheel
//! for the tick: its forces go into the body, its wheel spin is written back, and its `traction`, `slip` and `slip_angle`
//! feed the effects (smoke, skid marks, sound). A model that returns `None` leaves the wheel to the built-in update. The
//! trait names no model structure: how the answer is produced (a curve, a table, a sub-stepped integration, a constant
//! friction coefficient) is the model's own business.
//!
//! # Frame, units and signs
//!
//! SI units throughout (newtons, metres, seconds, radians, kilograms, degrees Celsius). The wheel frame has a forward
//! axis (the wheel's heading, steering included) and a lateral axis that points to the right of the heading. A wheel
//! that rolls forward has a positive `wheel_spin`. Forces use the same axes: a positive longitudinal force pushes the car
//! forward, a positive lateral force pushes it to the right. A positive aligning torque turns the wheel to the right
//! (clockwise seen from above). `slip` is wheel speed minus ground speed along the heading (`wheel_spin * radius -
//! patch_forward_velocity`, m/s); `slip_angle` is in turns (1.0 is 360 degrees), positive when the patch moves to the
//! right of the heading.
//!
//! # What the vehicle guarantees
//!
//! - One call per wheel per tick, wheels in index order 0 to 3 (front left, front right, rear left, rear right), on the
//!   thread that steps the vehicle, from inside the step.
//! - `dt` is the tick length the vehicle is stepped with (1/60 s by default, 1/144 s in an offline comparison mode,
//!   1/240 s in a slow-motion mode). It is the same for the four wheels of a tick.
//! - A loaded wheel (`loaded` true) has a positive vertical load and patch velocities of the contact. An airborne wheel
//!   (`loaded` false) has no load, no patch velocity, no surface and no brake or drive torque beyond what the powertrain
//!   sent; the model advances the spin and answers zero forces, and the vehicle uses only `wheel_spin` of the answer.
//! - Between two calls the vehicle may change `wheel_spin` itself (the powertrain writes it); the model must read the
//!   value in the query, not keep its own copy.
//!
//! # What the model must return
//!
//! Finite numbers in the units above, inside the ranges of [`TyreResult::validate`]. An answer that fails the check is
//! discarded, the built-in update runs for that wheel and the vehicle counts a fault ([`TyreHook::faults`]).

use core::fmt;

/// Ambient air temperature the vehicle reports to a model (degrees Celsius). The vehicle has no weather of its own.
pub const AMBIENT_TEMPERATURE_C: f32 = 20.0;
/// Road surface temperature the vehicle reports to a model (degrees Celsius). The vehicle has no thermal model.
pub const ROAD_TEMPERATURE_C: f32 = 25.0;

/// Largest force component a model may return (N).
pub const MAX_FORCE_N: f32 = 1.0e6;
/// Largest aligning torque a model may return (N m).
pub const MAX_ALIGNING_TORQUE_NM: f32 = 1.0e5;
/// Largest wheel spin a model may return (rad/s, either sign).
pub const MAX_WHEEL_SPIN: f32 = 5.0e3;
/// Largest slip speed a model may return (m/s, either sign).
pub const MAX_SLIP_MS: f32 = 1.0e3;
/// Largest slip angle a model may return (turns, either sign).
pub const MAX_SLIP_ANGLE_TURNS: f32 = 1.0;

/// What the vehicle tells a model about one wheel for one tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TyreQuery {
    /// 0 front left, 1 front right, 2 rear left, 3 rear right.
    pub wheel_index: u32,
    /// The tag the caller gave with [`crate::Vehicle::set_tyre_model`]. Opaque to the vehicle.
    pub role: u32,
    /// True when the wheel presses on the ground. False for an airborne wheel.
    pub loaded: bool,
    /// The tick length (s).
    pub dt: f32,
    /// Vertical load on the tyre (N); 0 for an airborne wheel.
    pub vertical_load_n: f32,
    /// Speed of the contact patch along the wheel's lateral axis (m/s, positive to the right); 0 when airborne. Sampled at the
    /// start of this tick's wheel update, before this tick's forces move the body.
    pub patch_lateral_velocity: f32,
    /// Speed of the contact patch along the wheel's heading (m/s, positive forward); 0 when airborne. Sampled like the lateral one.
    pub patch_forward_velocity: f32,
    /// Speed of the whole body (m/s).
    pub body_speed: f32,
    /// Wheel spin (rad/s), positive when rolling forward.
    pub wheel_spin: f32,
    /// Drive torque at the hub (N m, signed: positive pushes the wheel forward, negative in reverse or under engine braking)
    /// the powertrain sends this tick.
    pub drive_torque_nm: f32,
    /// Total brake torque at the hub (N m), the handbrake included: always at or above 0, and it opposes the rotation.
    pub brake_torque_nm: f32,
    /// Rolling radius of the wheel (m).
    pub wheel_radius_m: f32,
    /// Moment of inertia of the wheel (kg m^2): the host's constant for its own models. A model may use its own.
    pub wheel_inertia_kgm2: f32,
    /// The sprung mass the host attributes to this wheel (kg): a quarter of the vehicle mass. A model can bound its low-speed
    /// force with it (a force above `corner_mass_kg * speed / dt` reverses the motion within one tick).
    pub corner_mass_kg: f32,
    /// Surface multiplier of lateral force (1 on dry tarmac).
    pub surface_lateral: f32,
    /// Surface multiplier of longitudinal (drive and brake) force (1 on dry tarmac).
    pub surface_drive: f32,
    /// Surface multiplier of rolling resistance (1 on dry tarmac).
    pub surface_rolling: f32,
    /// The surface id the caller of the vehicle gave the ground under this wheel (opaque; 0 when airborne).
    pub surface_tag: u32,
    /// 1 for a sound tyre, below 1 for a damaged one (0.3 when it is blown).
    pub tyre_condition: f32,
    /// Ambient air temperature (degrees Celsius): [`AMBIENT_TEMPERATURE_C`].
    pub ambient_temperature_c: f32,
    /// Road surface temperature (degrees Celsius): [`ROAD_TEMPERATURE_C`].
    pub road_temperature_c: f32,
}

/// What a model answers for one wheel.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TyreResult {
    /// Mean longitudinal force over the tick (N, positive forward).
    pub longitudinal_force_n: f32,
    /// Mean lateral force over the tick (N, positive to the right).
    pub lateral_force_n: f32,
    /// Mean self-aligning torque over the tick (N m, positive turns the wheel to the right).
    pub aligning_torque_nm: f32,
    /// The wheel spin at the end of the tick (rad/s).
    pub wheel_spin: f32,
    /// 1 while the tyre grips, below 1 while it slides (0 to 1). It feeds the effects.
    pub traction: f32,
    /// Wheel speed minus ground speed along the heading (m/s). It feeds the effects.
    pub slip: f32,
    /// Slip angle in turns (1.0 is 360 degrees), positive when the patch moves to the right. It feeds the effects.
    pub slip_angle: f32,
}

/// Why an answer was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TyreReject {
    /// A field is NaN or infinite.
    NotFinite,
    /// A field is finite but outside its range (see the `MAX_*` constants, and 0 to 1 for the traction).
    OutOfRange,
}

impl TyreResult {
    /// Whether the answer is usable: every field finite and inside its range.
    pub fn validate(&self) -> Result<(), TyreReject> {
        let fields = [
            (self.longitudinal_force_n, MAX_FORCE_N),
            (self.lateral_force_n, MAX_FORCE_N),
            (self.aligning_torque_nm, MAX_ALIGNING_TORQUE_NM),
            (self.wheel_spin, MAX_WHEEL_SPIN),
            (self.slip, MAX_SLIP_MS),
            (self.slip_angle, MAX_SLIP_ANGLE_TURNS),
        ];
        if !fields.iter().all(|(value, _)| value.is_finite()) || !self.traction.is_finite() {
            return Err(TyreReject::NotFinite);
        }
        if fields.iter().any(|(value, limit)| value.abs() > *limit) || !(0.0..=1.0).contains(&self.traction) {
            return Err(TyreReject::OutOfRange);
        }
        Ok(())
    }
}

/// A tyre model. See the module documentation for the contract.
pub trait TyreModel {
    /// Answer for one wheel, or `None` to leave the wheel to the built-in update.
    fn wheel(&mut self, query: &TyreQuery) -> Option<TyreResult>;

    /// The vehicle discarded the last answer. A model that counts its faults does it here.
    fn rejected(&mut self, _why: TyreReject) {}

    /// True once the model must not be called again (a mod that was quarantined). The vehicle drops a retired model at
    /// the start of its next wheel loop.
    fn retired(&self) -> bool {
        false
    }

    /// A copy for a cloned vehicle (a prediction or a replay). `None`, the default, gives the clone no model.
    fn duplicate(&self) -> Option<Box<dyn TyreModel>> {
        None
    }
}

/// The model slot of a vehicle: at most one model, and the count of answers the vehicle refused.
#[derive(Default)]
pub struct TyreHook {
    model: Option<Box<dyn TyreModel>>,
    role: u32,
    faults: u32,
}

impl TyreHook {
    /// An empty slot: the built-in tyres run.
    pub const fn empty() -> Self {
        TyreHook { model: None, role: 0, faults: 0 }
    }

    /// Put `model` in the slot; `role` is the tag echoed in every query. The fault count starts again.
    pub fn install(&mut self, model: Box<dyn TyreModel>, role: u32) {
        *self = TyreHook { model: Some(model), role, faults: 0 };
    }

    /// Empty the slot.
    pub fn clear(&mut self) {
        *self = TyreHook::empty();
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

    /// Ask the model about one wheel. `None` when no model is installed, when it declines, or when its answer fails the
    /// check (the model is told, and the fault counted).
    pub fn ask(&mut self, query: &TyreQuery) -> Option<TyreResult> {
        let model = self.model.as_mut()?;
        let result = model.wheel(query)?;
        let Err(why) = result.validate() else {
            return Some(result);
        };
        self.faults = self.faults.saturating_add(1);
        model.rejected(why);
        None
    }
}

impl Clone for TyreHook {
    /// A clone keeps the model only when the model can duplicate itself (see [`TyreModel::duplicate`]).
    fn clone(&self) -> Self {
        let model = self.model.as_ref().and_then(|model| model.duplicate());
        TyreHook { model, role: self.role, faults: self.faults }
    }
}

impl fmt::Debug for TyreHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TyreHook")
            .field("set", &self.model.is_some())
            .field("role", &self.role)
            .field("faults", &self.faults)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> TyreResult {
        TyreResult { traction: 1.0, ..TyreResult::default() }
    }

    #[test]
    fn a_plain_answer_is_valid() {
        assert_eq!(good().validate(), Ok(()));
    }

    #[test]
    fn a_non_finite_field_is_refused() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(TyreResult { lateral_force_n: bad, ..good() }.validate(), Err(TyreReject::NotFinite));
            assert_eq!(TyreResult { traction: bad, ..good() }.validate(), Err(TyreReject::NotFinite));
        }
    }

    #[test]
    fn an_out_of_range_field_is_refused() {
        assert_eq!(TyreResult { wheel_spin: MAX_WHEEL_SPIN * 2.0, ..good() }.validate(), Err(TyreReject::OutOfRange));
        assert_eq!(TyreResult { traction: 1.5, ..good() }.validate(), Err(TyreReject::OutOfRange));
        assert_eq!(TyreResult { traction: -0.1, ..good() }.validate(), Err(TyreReject::OutOfRange));
        assert_eq!(TyreResult { slip_angle: 2.0, ..good() }.validate(), Err(TyreReject::OutOfRange));
    }
}
