//! The seam for an external controls model: where the vehicle asks it, what it passes, and where the answer goes (contract in
//! [`crate::input`], `input/model.rs`).
//!
//! The question is asked inside the controls update, right after the vehicle shaped the driver's input and before anything reads
//! the controls. With no model installed none of this runs beyond resetting two empty memories, and the step is bit-identical to
//! a build without it.

use glam::Vec3;

use super::Vehicle;
use crate::input::{Controls, ControlsModel, ControlsQuery};

/// The ground speed (m/s) below which a slip ratio is measured against this speed instead, so the ratio stays finite.
const SLIP_RATIO_MIN_SPEED: f32 = 0.5;
/// The largest magnitude of a slip ratio a model is told.
const SLIP_RATIO_LIMIT: f32 = 10.0;

impl Vehicle {
    /// Install an external controls model. `role` is an opaque tag echoed in every query. Replaces a model already installed. The
    /// model runs from the next step, on a standard chassis only (not the rolling chassis, the coasting tyres or the simple police
    /// tyres, which keep their own rules).
    pub fn set_controls_model(&mut self, model: Box<dyn ControlsModel>, role: u32) {
        self.controls_hook.install(model, role);
        self.hook_memory.body_velocity = None;
    }

    /// Remove the external controls model, if any. The controls stay as the vehicle shapes them from the next step.
    pub fn clear_controls_model(&mut self) {
        self.controls_hook.clear();
        self.hook_memory.body_velocity = None;
        self.hook_memory.brake_demand = None;
        self.hook_memory.steer_target = None;
    }

    /// Whether an external controls model is installed.
    pub fn has_controls_model(&self) -> bool {
        self.controls_hook.is_set()
    }

    /// The answers of the installed controls model that the vehicle refused since it was installed.
    pub fn controls_model_faults(&self) -> u32 {
        self.controls_hook.faults()
    }

    /// Whether the installed model is asked in this vehicle's current configuration.
    pub fn controls_model_applies(&self) -> bool {
        self.controls_hook.is_set() && self.takes_external_models() && !self.disabled
    }

    /// The slip ratio of wheel `i` in the last step (see [`crate::input::ControlsQuery`]); 0 for an airborne wheel.
    fn wheel_slip_ratio(&self, i: usize) -> f32 {
        if !self.loaded[i] {
            return 0.0;
        }
        let tire = &self.tires[i];
        let direction = if tire.road_speed < 0.0 { -1.0 } else { 1.0 };
        let ratio = tire.slip * direction / tire.road_speed.abs().max(SLIP_RATIO_MIN_SPEED);
        ratio.clamp(-SLIP_RATIO_LIMIT, SLIP_RATIO_LIMIT)
    }

    /// Ask the model about the controls the vehicle shaped for this tick. On an answer the throttle is replaced, and the brake
    /// demand per wheel and the steering target are kept for the brake and steering updates of the same step; the brake lamp
    /// (`Controls::brake`) shows the strongest demand. The controls are returned unchanged when no model applies, it declines, or
    /// its answer was refused.
    pub(super) fn filter_controls(&mut self, mut controls: Controls, dt: f32, forward_speed: f32) -> Controls {
        self.hook_memory.brake_demand = None;
        self.hook_memory.steer_target = None;
        self.controls_hook.drop_retired();
        if !self.controls_model_applies() {
            self.hook_memory.body_velocity = None;
            return controls;
        }
        let rot = self.body.rotation();
        let velocity = self.body.linear_velocity;
        let previous = self.hook_memory.body_velocity.replace(velocity);
        let accel = previous.map_or(Vec3::ZERO, |before| (velocity - before) / dt);
        let query = ControlsQuery {
            role: self.controls_hook.role(),
            dt,
            throttle: controls.gas,
            brake: controls.brake,
            handbrake: controls.handbrake,
            steer: controls.steering,
            forward_speed,
            yaw_rate: self.body.angular_velocity.dot(rot.y_axis),
            lateral_accel: accel.dot(rot.x_axis),
            longitudinal_accel: accel.dot(rot.z_axis),
            wheel_loaded: self.loaded,
            slip_ratio: std::array::from_fn(|i| self.wheel_slip_ratio(i)),
            slip_angle: std::array::from_fn(|i| if self.loaded[i] { self.tires[i].slip_angle } else { 0.0 }),
        };
        let Some(answer) = self.controls_hook.ask(&query) else {
            return controls;
        };
        controls.gas = answer.throttle;
        controls.brake = answer.brake.iter().copied().fold(0.0, f32::max).min(1.0);
        self.hook_memory.brake_demand = Some(answer.brake);
        self.hook_memory.steer_target = answer.steer_target;
        controls
    }
}
