//! Keeping the car calm: landing damping and the low-speed "sleep" damping (spec sections 2.6 and 7.4).

use glam::Vec3;

use super::Vehicle;

impl Vehicle {
    /// Halves the spin about the up and forward axes when the car lands after being fully airborne.
    pub(super) fn damp_landing(&mut self) {
        if self.wheels_on_ground_prev == 0 && self.wheels_on_ground > 0 {
            let rot = self.body.rotation();
            let mut w = rot.transpose() * self.body.angular_velocity;
            w.y *= 0.5;
            w.z *= 0.5;
            self.body.angular_velocity = rot * w;
        }
    }

    /// Stops a nearly stopped car from creeping: with the brakes on and all four wheels down the speed is
    /// damped and the wheels stop; otherwise below 1 m/s the sideways speed and yaw rate fade.
    pub(super) fn damp_creep(&mut self, collided: bool) {
        let c = self.controls;
        let speed = self.body.linear_velocity.length();
        let spin = self.body.angular_velocity.length();
        if spin >= 0.25 || c.gas > 0.0 {
            return;
        }
        let rot = self.body.rotation();
        if speed < 0.5 && self.wheels_on_ground == 4 && (c.brake > 0.0 || c.handbrake > 0.0) && !collided {
            // Unlike the original, the pending forces are not scaled: that froze a freshly landed car off
            // its resting ride height.
            self.body.linear_velocity *= speed;
            self.body.angular_velocity *= speed;
            for t in &mut self.tires {
                t.av = 0.0;
            }
        } else if speed < 1.0 {
            let mut v = rot.transpose() * self.body.linear_velocity;
            v.x *= speed;
            self.body.linear_velocity = rot * v;
            let mut w = rot.transpose() * self.body.angular_velocity;
            w.y *= speed;
            self.body.angular_velocity = rot * w;
            // Fade the sideways force and the yaw torque accumulated this step.
            let keep = 1.0 - speed;
            let mut f = rot.transpose() * self.body.pending_force();
            let mut t = rot.transpose() * self.body.pending_torque();
            f.x *= keep;
            t.y *= keep;
            self.set_pending(rot * f, rot * t);
        }
    }

    fn set_pending(&mut self, force: Vec3, torque: Vec3) {
        let (f0, t0) = (self.body.pending_force(), self.body.pending_torque());
        self.body.apply_force(force - f0);
        self.body.apply_torque(torque - t0);
    }
}
