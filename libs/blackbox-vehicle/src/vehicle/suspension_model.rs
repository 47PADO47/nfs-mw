//! The seam for an external suspension model: where the vehicle asks it, what it passes, and where the answer goes (contract in
//! [`crate::suspension`], `suspension/model.rs`).
//!
//! With no model installed none of this runs beyond clearing a camber that is already zero, and the step is bit-identical to a
//! build without it.

use super::Vehicle;
use super::wheels::WheelProbe;
use crate::suspension::{AxleConstants, SuspensionModel, SuspensionQuery, SuspensionWheelQuery, SuspensionWheelResult};

/// The acceleration of gravity the static load estimate uses (m/s^2).
const GRAVITY: f32 = 9.81;

/// The compression (m) at which the built-in spring of an axle carries `load` (N) on one wheel: the sag of a car at rest on
/// flat ground. The damper and the anti-roll bar do nothing at rest. The progressive spring is `((1 + c * p) * k) * c`.
fn rest_compression(axle: &AxleConstants, load: f32) -> f32 {
    let (k, p) = (axle.spring, axle.progression);
    if k <= 0.0 {
        return 0.0;
    }
    let linear = load / k;
    if p.abs() < 1e-9 {
        return linear.clamp(0.0, axle.travel);
    }
    let radicand = k * k + 4.0 * k * p * load;
    if radicand < 0.0 {
        return linear.clamp(0.0, axle.travel);
    }
    ((radicand.sqrt() - k) / (2.0 * k * p)).clamp(0.0, axle.travel)
}

impl Vehicle {
    /// Install an external suspension model. `role` is an opaque tag echoed in every query. Replaces a model already installed.
    /// The model runs from the next step, on the axles of a standard chassis only (not the rolling chassis, the coasting tyres
    /// or the simple police tyres, which keep their own rules).
    pub fn set_suspension_model(&mut self, model: Box<dyn SuspensionModel>, role: u32) {
        self.suspension_hook.install(model, role);
        self.hook_memory.point_velocity = None;
        self.hook_memory.tyre_lateral_force = [0.0; 4];
    }

    /// Remove the external suspension model, if any. The built-in springs run from the next step.
    pub fn clear_suspension_model(&mut self) {
        self.suspension_hook.clear();
        self.hook_memory.point_velocity = None;
        self.hook_memory.camber = [0.0; 4];
    }

    /// Whether an external suspension model is installed.
    pub fn has_suspension_model(&self) -> bool {
        self.suspension_hook.is_set()
    }

    /// The answers of the installed suspension model that the vehicle refused since it was installed.
    pub fn suspension_model_faults(&self) -> u32 {
        self.suspension_hook.faults()
    }

    /// Whether the installed model is asked in this vehicle's current configuration.
    pub fn suspension_model_applies(&self) -> bool {
        self.suspension_hook.is_set() && self.takes_external_models()
    }

    /// The camber (rad, outward positive) the suspension model gave wheel `i` in the last step; 0 when none answered, and for an
    /// index outside 0 to 3.
    pub fn wheel_camber(&self, i: usize) -> f32 {
        self.hook_memory.camber.get(i).copied().unwrap_or(0.0)
    }

    /// Drop a model that retired itself (see [`SuspensionModel::retired`]). Called at the start of each wheel loop.
    pub(super) fn drop_retired_suspension_model(&mut self) {
        self.suspension_hook.drop_retired();
    }

    /// Ask the model about both axles, front first, after the ground probe and before any wheel runs. The result of a wheel is
    /// `None` when no model applies, it declines, or its answer was refused. The camber of each answered wheel is kept for its
    /// tyre query.
    pub(super) fn suspension_answers(
        &mut self,
        dt: f32,
        probes: &[WheelProbe; 4],
        old_compression: &[f32; 4],
        axles: &[AxleConstants; 2],
    ) -> [Option<SuspensionWheelResult>; 4] {
        self.hook_memory.camber = [0.0; 4];
        let mut answers = [None; 4];
        if !self.suspension_model_applies() {
            self.hook_memory.point_velocity = None;
            return answers;
        }
        let up = self.body.rotation().y_axis;
        let previous = self.hook_memory.point_velocity.replace(std::array::from_fn(|i| probes[i].point_vel));
        let mass = self.body.mass();
        let front_share = (self.spec.chassis.front_weight_bias * 0.01).clamp(0.0, 1.0);
        for (axle, constants) in axles.iter().enumerate() {
            let share = if axle == 0 { front_share } else { 1.0 - front_share };
            let static_load = 0.5 * mass * GRAVITY * share;
            let wheel = |side: usize| {
                let i = axle * 2 + side;
                let compression = probes[i].compression();
                SuspensionWheelQuery {
                    loaded: probes[i].loaded(),
                    compression_m: compression,
                    compression_speed_mps: (compression - old_compression[i]) / dt,
                    steer_angle_rad: match i {
                        0 => self.wheel_angles.left,
                        1 => self.wheel_angles.right,
                        _ => 0.0,
                    },
                    body_accel_up_mps2: previous.map_or(0.0, |before| up.dot(probes[i].point_vel - before[i]) / dt),
                    tyre_lateral_force_n: self.hook_memory.tyre_lateral_force[i],
                    static_load_n: static_load,
                }
            };
            let query = SuspensionQuery {
                axle: axle as u32,
                role: self.suspension_hook.role(),
                dt,
                rest_travel_m: rest_compression(constants, static_load),
                max_travel_m: constants.travel,
                corner_mass_kg: 0.25 * mass,
                wheels: [wheel(0), wheel(1)],
            };
            let Some(result) = self.suspension_hook.ask(&query) else {
                continue;
            };
            for (side, wheel) in result.wheels.into_iter().enumerate() {
                answers[axle * 2 + side] = Some(wheel);
                self.hook_memory.camber[axle * 2 + side] = wheel.camber_rad;
            }
        }
        answers
    }
}
