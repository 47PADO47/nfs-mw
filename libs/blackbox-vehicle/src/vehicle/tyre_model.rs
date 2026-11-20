//! The seam for an external tyre model: where the vehicle asks it, what it passes, and where the answer goes
//! (contract in [`crate::tires`], `tires/model.rs`).
//!
//! With no model installed none of this runs and the step is bit-identical to a build without it.

use super::Vehicle;
use crate::ground::SurfaceGrip;
use crate::tires::{
    AMBIENT_TEMPERATURE_C, BLOWN_TRACTION, LoadedInput, ROAD_TEMPERATURE_C, TireIntegration, TyreModel, TyreQuery,
    WHEEL_INERTIA,
};

/// Whether a vehicle in `integration` mode may take an external tyre model.
///
/// Both modes may. The reference mode feeds the powertrain with the road speed while a tyre grips (`traction` of 1), the
/// regularized mode with the wheel spin; the model fills `road_speed`, `load`, `traction` and the spin consistently in
/// both, so the two consumers see what they expect (tests: `vehicle/tests/tyre_model.rs`, one per mode). Every car the game
/// builds from its data runs in the reference mode, so refusing it would leave the seam without a user.
pub const fn integration_takes_model(integration: TireIntegration) -> bool {
    match integration {
        TireIntegration::Regularized | TireIntegration::Reference => true,
    }
}

impl Vehicle {
    /// Install an external tyre model. `role` is an opaque tag echoed in every query (the caller says what kind of vehicle
    /// this is). Replaces a model already installed. The model runs from the next step, on the wheels of a standard
    /// chassis only (not the rolling chassis, the coasting tyres or the simple police tyres, which keep their own rules).
    pub fn set_tyre_model(&mut self, model: Box<dyn TyreModel>, role: u32) {
        self.tyre_hook.install(model, role);
    }

    /// Remove the external tyre model, if any. The built-in tyres run from the next step.
    pub fn clear_tyre_model(&mut self) {
        self.tyre_hook.clear();
    }

    /// Whether an external tyre model is installed.
    pub fn has_tyre_model(&self) -> bool {
        self.tyre_hook.is_set()
    }

    /// The answers of the installed model that the vehicle refused (not finite, or out of range) since it was installed.
    pub fn tyre_model_faults(&self) -> u32 {
        self.tyre_hook.faults()
    }

    /// Self-aligning torque of wheel `i` (N m, positive turns the wheel to the right) from the last step: what an external
    /// tyre model reported, 0 for the built-in tyres. 0 for an index outside 0 to 3.
    pub fn wheel_aligning_torque(&self, i: usize) -> f32 {
        self.tires.get(i).map_or(0.0, |tire| tire.aligning_torque)
    }

    /// Whether the vehicle has the standard chassis: the one that takes external models of the tyres, the suspension and the
    /// controls. The rolling chassis, the coasting tyres and the simple police tyres keep their own rules.
    pub fn takes_external_models(&self) -> bool {
        integration_takes_model(self.tire_integration)
            && self.rolling_chassis.is_none()
            && self.coasting_tires.is_none()
            && !self.simple_tires_on()
    }

    /// Whether the installed model is asked in this vehicle's current configuration.
    pub fn tyre_model_applies(&self) -> bool {
        self.tyre_hook.is_set() && self.takes_external_models()
    }

    /// Drop a model that retired itself (see [`TyreModel::retired`]). Called at the start of each wheel loop.
    pub(super) fn drop_retired_tyre_model(&mut self) {
        self.tyre_hook.drop_retired();
    }

    fn tyre_query(&self, i: usize, dt: f32, loaded: Option<&LoadedInput>) -> TyreQuery {
        let tire = &self.tires[i];
        let params = &self.params[i];
        let surface = loaded.map_or(SurfaceGrip::DEFAULT, |input| input.surface);
        let condition = if self.tire_damage.is_blown(i) { BLOWN_TRACTION } else { 1.0 };
        TyreQuery {
            wheel_index: i as u32,
            role: self.tyre_hook.role(),
            loaded: loaded.is_some(),
            dt,
            vertical_load_n: loaded.map_or(0.0, |input| input.load.max(0.0)),
            patch_lateral_velocity: loaded.map_or(0.0, |input| input.lat_vel),
            patch_forward_velocity: loaded.map_or(0.0, |input| input.fwd_vel),
            body_speed: loaded.map_or_else(|| self.body.linear_velocity.length(), |input| input.body_speed),
            wheel_spin: tire.av,
            drive_torque_nm: tire.drive_torque,
            brake_torque_nm: tire.brake * params.brake_spec + tire.ebrake * params.ebrake_spec,
            wheel_radius_m: params.radius,
            wheel_inertia_kgm2: WHEEL_INERTIA,
            corner_mass_kg: 0.25 * self.body.mass(),
            surface_lateral: surface.lateral,
            surface_drive: surface.drive,
            surface_rolling: surface.rolling,
            surface_tag: surface.tag,
            tyre_condition: condition,
            ambient_temperature_c: AMBIENT_TEMPERATURE_C,
            road_temperature_c: ROAD_TEMPERATURE_C,
            camber_rad: self.hook_memory.camber[i],
        }
    }

    /// Ask the model about the loaded wheel `i`. On an answer the tire state is filled and the lateral force is returned,
    /// as the built-in update returns it. `None` when no model applies, it declines, or its answer was refused.
    pub(super) fn tyre_model_loaded(&mut self, i: usize, input: &LoadedInput) -> Option<f32> {
        if !self.tyre_model_applies() {
            return None;
        }
        let query = self.tyre_query(i, input.dt, Some(input));
        let result = self.tyre_hook.ask(&query)?;
        self.tires[i].adopt_loaded(&query, &result);
        Some(result.lateral_force_n)
    }

    /// Update the tyre of wheel `i` that is not pressed on the ground: the model advances the spin when it answers, the
    /// built-in free update runs otherwise.
    pub(super) fn update_unloaded_tyre(&mut self, i: usize, dt: f32) {
        if self.tyre_model_applies() {
            let query = self.tyre_query(i, dt, None);
            if let Some(result) = self.tyre_hook.ask(&query) {
                self.tires[i].adopt_free(result.wheel_spin);
                return;
            }
        }
        match self.rolling_chassis {
            Some(_) => self.tires[i].update_rolling_free(),
            None if self.simple_tires_on() => self.tires[i].update_simple_free(&self.params[i], dt),
            None => self.tires[i].update_free(&self.params[i], dt),
        }
    }
}
