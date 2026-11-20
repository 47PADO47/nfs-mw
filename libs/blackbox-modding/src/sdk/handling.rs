//! `Handling`: replace how a vehicle's tyres turn load, slip and torque into forces (catalogue:
//! docs/modding/api-handling.md).
//!
//! The hook is one closure, called once per wheel per physics tick. It names no model: what it computes is the mod's own.

use super::ctx::Keep;
use super::ffi::HostTable;
use super::ffi::handling::{self as host_handling, HandlingFns};
use super::{Ctx, HandlingHookId, ModResult, Role};

/// What the host tells the hook about one wheel for one tick. SI units. The wheel frame has a forward axis (the wheel's
/// heading, steering included) and a lateral axis that points to the right of it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TyreQuery {
    /// 0 front left, 1 front right, 2 rear left, 3 rear right.
    pub wheel_index: u32,
    /// The kind of vehicle; `None` for a role this SDK does not know.
    pub role: Option<Role>,
    /// Whether the wheel presses on the ground. False for an airborne wheel: answer zero forces and advance the spin.
    pub loaded: bool,
    /// Changes whenever the vehicle changes: the `revision` of `Vehicles::spec`.
    pub vehicle_revision: u32,
    /// The surface id under the wheel (opaque; 0 when airborne).
    pub surface_tag: u32,
    /// The tick length (s).
    pub dt: f32,
    /// Vertical load on the tyre (N); 0 when airborne.
    pub vertical_load_n: f32,
    /// Speed of the contact patch along the lateral axis (m/s, positive to the right); 0 when airborne. Sampled at the start of
    /// this tick's wheel update, before this tick's forces move the body.
    pub patch_lateral_velocity: f32,
    /// Speed of the contact patch along the wheel's heading (m/s, positive forward); 0 when airborne.
    pub patch_forward_velocity: f32,
    /// Speed of the whole body (m/s).
    pub body_speed: f32,
    /// Wheel spin (rad/s), positive when rolling forward. Read it every call: the host may change it between calls.
    pub wheel_spin: f32,
    /// Drive torque at the hub this tick (N m, signed: positive pushes the wheel forward).
    pub drive_torque_nm: f32,
    /// Total brake torque at the hub this tick (N m), the handbrake included: at or above 0, opposing the rotation.
    pub brake_torque_nm: f32,
    /// Rolling radius of the wheel (m).
    pub wheel_radius_m: f32,
    /// Moment of inertia of the wheel (kg m^2): the host's constant for its own models. A mod may use its own.
    pub wheel_inertia_kgm2: f32,
    /// The sprung mass the host attributes to this wheel (kg): a quarter of the vehicle mass.
    pub corner_mass_kg: f32,
    /// Surface multiplier of lateral force (1 on dry tarmac).
    pub surface_lateral: f32,
    /// Surface multiplier of longitudinal force (1 on dry tarmac).
    pub surface_drive: f32,
    /// Surface multiplier of rolling resistance (1 on dry tarmac).
    pub surface_rolling: f32,
    /// 1 for a sound tyre, below 1 for a damaged one.
    pub tyre_condition: f32,
    /// Ambient air temperature (degrees Celsius).
    pub ambient_temperature_c: f32,
    /// Road surface temperature (degrees Celsius).
    pub road_temperature_c: f32,
}

/// What the hook answers for one wheel. Every number must be finite; the ranges are in the catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TyreResult {
    /// Mean longitudinal force over the tick (N, positive forward).
    pub longitudinal_force_n: f32,
    /// Mean lateral force over the tick (N, positive to the right).
    pub lateral_force_n: f32,
    /// Mean self-aligning torque over the tick (N m, positive turns the wheel to the right).
    pub aligning_torque_nm: f32,
    /// The wheel spin at the end of the tick (rad/s).
    pub wheel_spin: f32,
    /// 1 while the tyre grips, below 1 while it slides (0 to 1); feeds the effects.
    pub traction: f32,
    /// Wheel speed minus ground speed along the heading (m/s); feeds the effects.
    pub slip: f32,
    /// Slip angle in turns (1.0 is 360 degrees), positive when the patch moves to the right; feeds the effects.
    pub slip_angle: f32,
}

/// The `bb.handling` interface of the host, as a mod uses it. Obtained from `Ctx::handling`.
pub struct Handling {
    fns: HandlingFns,
    host: HostTable,
    keep: Keep,
}

impl Handling {
    pub(crate) fn new(fns: HandlingFns, host: HostTable, keep: Keep) -> Self {
        Handling { fns, host, keep }
    }

    /// Register the tyre hook: `hook` is called once per wheel per tick for the vehicles the host applies hooks to (the
    /// player's car in the first version). Return `None` to leave a wheel to the built-in tyres. One hook per mod; a second
    /// call answers `Error::Exists`. `Error::Denied` in an online session. The hook is removed when the mod unloads.
    pub fn register_tyre<F>(&self, hook: F) -> ModResult<HandlingHookId>
    where
        F: FnMut(&mut Ctx, &TyreQuery) -> Option<TyreResult> + 'static,
    {
        host_handling::register_tyre(&self.fns, self.host, &self.keep, hook)
    }

    /// Remove the tyre hook this mod registered. The built-in tyres run from the next tick.
    pub fn unregister_tyre(&self, id: HandlingHookId) -> ModResult<()> {
        host_handling::unregister_tyre(&self.fns, id)
    }
}
