//! Landing and low-speed damping. Reference contract: `vehicle-creep-response.md`.

use glam::Vec3;

use super::Vehicle;
use crate::rigid_body::transform_vector;

impl Vehicle {
    /// Halves the spin about the up and forward axes when the car lands after being fully airborne.
    pub(super) fn damp_landing(&mut self, angular_velocity: Vec3) -> Vec3 {
        if self.rolling_chassis.is_some() {
            return angular_velocity;
        }
        if self.wheels_on_ground_prev == 0 && self.wheels_on_ground > 0 {
            let rot = self.body.rotation();
            let mut w = rot.transpose() * angular_velocity;
            w *= self.simple_chassis.map_or(Vec3::new(1.0, 0.5, 0.5), |c| c.landing_angular_retention);
            self.body.angular_velocity = rot * w;
            return self.body.angular_velocity;
        }
        angular_velocity
    }

    /// Stops a nearly stopped car from creeping: with the brakes on and all four wheels down the speed is
    /// damped and the wheels stop; otherwise below 1 m/s the sideways speed and yaw rate fade.
    pub(super) fn damp_creep(&mut self, collided: bool, angular_velocity: Vec3) {
        if self.staging() {
            return;
        }
        let c = self.controls;
        let speed = self.body.linear_velocity.length();
        let spin = angular_velocity.length();
        if spin >= 0.25 || c.gas > 0.0 {
            return;
        }
        let rot = self.body.rotation();
        if speed < 0.5 && self.wheels_on_ground == 4 && (c.brake > 0.0 || c.handbrake > 0.0) && !collided {
            if self.tire_integration == crate::tires::TireIntegration::Reference {
                let amount = if speed < 0.000001 { 1.0 } else { 1.0 - speed };
                self.body.damp(amount);
            }
            if self.tire_integration == crate::tires::TireIntegration::Regularized {
                self.body.linear_velocity *= speed;
                self.body.angular_velocity *= speed;
            }
            for t in &mut self.tires {
                t.av = 0.0;
            }
        } else if speed < 1.0 {
            let reference = self.tire_integration == crate::tires::TireIntegration::Reference;
            let rotate = |matrix, vector| match reference {
                true => transform_vector(matrix, vector),
                false => matrix * vector,
            };
            let mut v = rotate(rot.transpose(), self.body.linear_velocity);
            v.x *= speed;
            self.body.linear_velocity = rotate(rot, v);
            let mut w = rotate(rot.transpose(), angular_velocity);
            w.y *= speed;
            self.body.angular_velocity = rotate(rot, w);
            let mut f = rotate(rot.transpose(), self.body.pending_force());
            let mut t = rotate(rot.transpose(), self.body.pending_torque());
            if reference {
                // Resolve adds the compensation vector. Its other local axes are retained,
                // so they add again rather than being replaced or left unchanged.
                f.x = -f.x * (1.0 - speed);
                t.y = -t.y * (1.0 - speed);
                self.body.apply_force(rotate(rot, f));
                self.body.apply_torque(rotate(rot, t));
                return;
            }
            f.x *= 1.0 - speed;
            t.y *= 1.0 - speed;
            self.set_pending(rot * f, rot * t);
        }
    }

    fn set_pending(&mut self, force: Vec3, torque: Vec3) {
        let (f0, t0) = (self.body.pending_force(), self.body.pending_torque());
        self.body.apply_force(force - f0);
        self.body.apply_torque(torque - t0);
    }
}
