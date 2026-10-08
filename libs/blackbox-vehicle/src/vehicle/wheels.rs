//! The per-wheel loop: ground probe, compression, spring force, tire forces and their torque about the
//! centre of gravity (spec section 2).

use glam::Vec3;

use super::Vehicle;
use crate::ground::Ground;
use crate::suspension::{Side, SpringInput, compression, probe, spring_force};
use crate::tires::LoadedInput;

/// Sum over the four wheels.
pub(super) struct WheelOutput {
    pub force: Vec3,
    pub torque: Vec3,
    /// Largest distance any wheel is beyond its travel; the body is lifted by it.
    pub lift_need: f32,
}

impl Vehicle {
    /// Probes the ground under each wheel, updates springs and tires, and returns the summed force and
    /// torque about the centre of gravity (not yet applied to the body).
    pub(super) fn wheel_forces(&mut self, dt: f32, ground: &dyn Ground) -> WheelOutput {
        let rot = self.body.rotation();
        let up = rot.y_axis;
        let pos = self.body.position;
        let cog_world = rot * self.body.cog();
        let cog_point = pos + cog_world;
        let velocity = self.body.linear_velocity;
        let body_speed = velocity.length();
        let mass = self.body.mass();
        let ride_extra = self.tunings.ride_height;
        let axles = [self.spec.chassis.axle(0, ride_extra), self.spec.chassis.axle(1, ride_extra)];
        let old_compression: [f32; 4] = std::array::from_fn(|i| self.corners[i].compression);
        let steer_dir = |angle: f32| rot * Vec3::new(angle.sin(), 0.0, angle.cos());
        let dirs = [steer_dir(self.wheel_angles.left), steer_dir(self.wheel_angles.right), rot.z_axis, rot.z_axis];

        let mut out = WheelOutput { force: Vec3::ZERO, torque: Vec3::ZERO, lift_need: 0.0 };
        for i in 0..4 {
            let axle = i / 2;
            let a = &axles[axle];
            let arm_world = rot * self.geometry.arms[i];
            let point = pos + arm_world;
            let point_vel = velocity + self.body.angular_velocity.cross(arm_world - cog_world);
            let tolerance = ((-point_vel.y * dt).max(0.0)
                + self.geometry.radius[i]
                + Vec3::new(point_vel.x, 0.0, point_vel.z).length() * dt)
                .min(self.spec.dimension.y);

            let contact = probe(ground, point, tolerance);
            self.contacts[i] = contact;
            let c_old = old_compression[i];
            let comp = contact.map(|c| compression(a.ride, a.travel, c.normal.dot(up).clamp(0.0, 1.0), c.penetration));
            let c_new = comp.map_or(0.0, |c| c.value);
            self.corners[i].compression = c_new;
            self.corners[i].air_time = if c_new == 0.0 { self.corners[i].air_time + dt } else { 0.0 };

            let mut force = Vec3::ZERO;
            let loaded = comp.is_some_and(|c| c.loaded);
            self.loaded[i] = loaded;
            if let (true, Some(c), Some(comp)) = (loaded, contact, comp) {
                out.lift_need = out.lift_need.max(comp.lift_need);
                let f_spring = spring_force(
                    a,
                    &SpringInput {
                        c_old,
                        c_new,
                        c_left_old: old_compression[axle * 2],
                        c_right_old: old_compression[axle * 2 + 1],
                        side: if i % 2 == 0 { Side::Left } else { Side::Right },
                        dt,
                        mass,
                        blowout: self.spec.chassis.shock_blowout,
                    },
                );
                let n = c.normal;
                let f = dirs[i];
                let flat = (f.cross(n)).cross(f);
                let load = (4.0 * flat.dot(n) - 3.0).max(0.3) * f_spring;
                let lat_dir = {
                    let l = n.cross(f);
                    if l.length_squared() > 1e-8 { l.normalize() } else { rot.x_axis }
                };
                let fwd_speed = point_vel.dot(f);
                let lat_speed = point_vel.dot(lat_dir);
                let fy = self.tires[i].update_loaded(
                    &self.params[i],
                    &LoadedInput {
                        lat_vel: lat_speed,
                        fwd_vel: fwd_speed,
                        body_speed,
                        load,
                        dt,
                        quarter_mass: 0.25 * mass,
                        surface: c.surface,
                    },
                );
                let fy_limit = (lat_speed / dt).abs() * 0.25 * mass;
                let lateral = lat_dir * fy.clamp(-fy_limit, fy_limit);
                let drive_dir = lat_dir.cross(n);
                let drive_dir = if drive_dir.length_squared() > 1e-8 { drive_dir.normalize() } else { rot.z_axis };
                force = lateral + drive_dir * self.tires[i].longitudinal_force + up * f_spring;
            } else {
                self.tires[i].update_free(&self.params[i], dt);
            }

            // The force acts at the contact patch: the arm lowered by the unused ride height.
            let mut arm = self.geometry.arms[i];
            arm.y += c_new - a.ride;
            let patch = pos + rot * arm;
            self.patch_positions[i] = patch;
            if force.is_finite() {
                out.force += force;
                out.torque += (patch - cog_point).cross(force);
            }
        }
        self.wheels_on_ground = self.loaded.iter().filter(|l| **l).count();
        out
    }
}
