//! Taking the answer of an external tyre model into the tire state (see [`super::model`]).

use super::Tire;
use super::model::{TyreQuery, TyreResult};

impl Tire {
    /// The state a loaded wheel keeps after a model answered for it: the same fields the built-in update fills, so the
    /// effects, the sound, the powertrain and a later fall back to the built-in update see a consistent tire.
    pub(crate) fn adopt_loaded(&mut self, query: &TyreQuery, result: &TyreResult) {
        self.load = query.vertical_load_n.max(0.0);
        self.road_speed = query.patch_forward_velocity;
        self.lateral_speed = query.patch_lateral_velocity;
        self.longitudinal_force = result.longitudinal_force_n;
        self.lateral_force = result.lateral_force_n;
        self.aligning_torque = result.aligning_torque_nm;
        self.traction = result.traction;
        self.slip = result.slip;
        self.slip_angle = result.slip_angle;
        self.brake_locked = false;
        self.angular_acc = 0.0;
        self.set_spin(result.wheel_spin);
    }

    /// The state an airborne wheel keeps after a model answered for it: only the spin comes from the model.
    pub(crate) fn adopt_free(&mut self, spin: f32) {
        self.load = 0.0;
        self.slip = 0.0;
        self.traction = 0.0;
        self.slip_angle = 0.0;
        self.lateral_force = 0.0;
        self.longitudinal_force = 0.0;
        self.aligning_torque = 0.0;
        self.lateral_speed = 0.0;
        self.brake_locked = false;
        self.angular_acc = 0.0;
        self.set_spin(spin);
    }

    /// Set the wheel spin and let the sign memory follow it, so a later built-in update does not mistake the change for a
    /// sign flip within a step.
    fn set_spin(&mut self, spin: f32) {
        const DEAD_BAND: f32 = 1e-6;
        self.av = spin;
        self.last_sign = match spin {
            s if s > DEAD_BAND => 1,
            s if s < -DEAD_BAND => -1,
            _ => 0,
        };
    }
}
