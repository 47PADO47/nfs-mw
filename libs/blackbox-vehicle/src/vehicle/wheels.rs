//! The per-wheel loop: ground probe, compression, spring force, tire forces and their torque about the
//! centre of gravity (spec section 2).

use glam::Vec3;

use super::Vehicle;
use crate::ground::Ground;
use crate::rigid_body::transform_vector;
use crate::suspension::{Side, SpringInput, WheelContact, compression, probe, spring_force};
use crate::tires::LoadedInput;

fn reference_forward_speed(velocity: Vec3, direction: Vec3) -> f32 {
    (velocity.x * direction.x + velocity.z * direction.z) + velocity.y * direction.y
}

fn query_tolerance(velocity: Vec3, dt: f32, radius: f32, height: f32, reference: bool) -> f32 {
    let down = (-velocity.y * dt).max(0.0);
    let horizontal = Vec3::new(velocity.x, 0.0, velocity.z).length() * dt;
    match reference {
        true => ((horizontal + down) + radius).min(height),
        false => ((down + radius) + horizontal).min(height),
    }
}

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
    pub(super) fn wheel_forces(&mut self, dt: f32, ground: &dyn Ground, angular_velocity: Vec3) -> WheelOutput {
        let rot = self.body.rotation();
        let up = rot.y_axis;
        let pos = self.body.position;
        let cog_world = transform_vector(rot, self.body.cog());
        let velocity = self.body.linear_velocity;
        let body_speed = velocity.length();
        let mass = self.body.mass();
        let ride_extra = self.tunings.ride_height;
        let axles = [self.spec.chassis.axle(0, ride_extra), self.spec.chassis.axle(1, ride_extra)];
        let old_compression: [f32; 4] = std::array::from_fn(|i| self.corners[i].compression);
        let steer_dir = |angle: f32| transform_vector(rot, Vec3::new(angle.sin(), 0.0, angle.cos()));
        let dirs = [steer_dir(self.wheel_angles.left), steer_dir(self.wheel_angles.right), rot.z_axis, rot.z_axis];

        let mut out = WheelOutput { force: Vec3::ZERO, torque: Vec3::ZERO, lift_need: 0.0 };
        for i in 0..4 {
            let axle = i / 2;
            let a = &axles[axle];
            let arm_world = transform_vector(rot, self.geometry.arms[i]);
            let point = pos + arm_world;
            let point_vel = velocity + angular_velocity.cross(arm_world - cog_world);
            let tolerance = query_tolerance(
                point_vel,
                dt,
                self.geometry.radius[i],
                self.spec.dimension.y,
                self.tires[i].integration == crate::tires::TireIntegration::Reference,
            );

            let contact = ground
                .wheel_plane(i, point, tolerance, old_compression[i] > 0.0)
                .map(|plane| WheelContact {
                    point: point + plane.normal * plane.separation,
                    normal: plane.normal,
                    penetration: plane.separation,
                    surface: plane.surface,
                })
                .or_else(|| probe(ground, point, tolerance));
            self.contacts[i] = contact;
            let c_old = old_compression[i];
            let comp = contact.map(|c| compression(a.ride, a.travel, c.normal.dot(up).clamp(0.0, 1.0), c.penetration));
            let c_new = comp.map_or(0.0, |c| c.value);
            if let Some(comp) = comp {
                out.lift_need = out.lift_need.max(comp.lift_need);
            }
            self.corners[i].compression = c_new;
            self.corners[i].air_time = if c_new == 0.0 { self.corners[i].air_time + dt } else { 0.0 };

            let mut force = Vec3::ZERO;
            let loaded = comp.is_some_and(|c| c.loaded);
            self.loaded[i] = loaded;
            if let (true, Some(c)) = (loaded, contact) {
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
                let upness = match self.rolling_chassis.is_some_and(|config| config.load_from_up) {
                    true => n.dot(up),
                    false => flat.dot(n),
                };
                let load = (4.0 * upness - 3.0).max(0.3) * f_spring;
                let lat_dir = {
                    let l = n.cross(f);
                    if l.length_squared() > 1e-8 { l.normalize() } else { rot.x_axis }
                };
                let fwd_speed = match self.tires[i].integration {
                    crate::tires::TireIntegration::Reference => reference_forward_speed(point_vel, f),
                    crate::tires::TireIntegration::Regularized => point_vel.dot(f),
                };
                let lat_speed = point_vel.dot(lat_dir);
                let input = LoadedInput {
                    lat_vel: lat_speed,
                    fwd_vel: fwd_speed,
                    body_speed,
                    load,
                    dt,
                    quarter_mass: 0.25 * mass,
                    surface: c.surface,
                };
                let fy = match self.rolling_chassis {
                    Some(config) => self.tires[i].update_rolling(&self.params[i], &input, config.tires),
                    None => match self.coasting_tires {
                        Some(response) => self.tires[i].update_coasting(&self.params[i], &input, response),
                        None => self.tires[i].update_loaded(&self.params[i], &input),
                    },
                };
                let fy_limit = (lat_speed / dt).abs() * 0.25 * mass;
                let lateral = lat_dir * fy.clamp(-fy_limit, fy_limit);
                let drive_dir = lat_dir.cross(n);
                let drive_dir = if drive_dir.length_squared() > 1e-8 { drive_dir.normalize() } else { rot.z_axis };
                force = lateral + drive_dir * self.tires[i].longitudinal_force + up * f_spring;
            } else {
                match self.rolling_chassis {
                    Some(_) => self.tires[i].update_rolling_free(),
                    None => self.tires[i].update_free(&self.params[i], dt),
                }
            }

            // The force acts at the contact patch: the arm lowered by the unused ride height.
            let mut arm = self.geometry.arms[i];
            arm.y += c_new - a.ride;
            let patch = pos + transform_vector(rot, arm);
            self.patch_positions[i] = patch;
            if force.is_finite() {
                out.force += force;
                let mut arm = (patch - cog_world) - pos;
                if let Some(config) = self.rolling_chassis {
                    arm.y *= config.force_arm_height;
                }
                out.torque += arm.cross(force);
            }
        }
        self.wheels_on_ground = self.loaded.iter().filter(|l| **l).count();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_projection_preserves_xz_before_y_rounding() {
        let velocity = Vec3::new(4.598474, -0.8348535, 6.4309535);
        let direction = Vec3::new(0.41014934, -0.67413306, -0.1900244);
        assert_eq!(reference_forward_speed(velocity, direction).to_bits(), 0x3f9d089c);
    }

    #[test]
    fn reference_tolerance_adds_radius_last() {
        let velocity = Vec3::new(0.13, -0.013 / 0.007, 0.32);
        let reference = query_tolerance(velocity, 0.007, 0.27, 1.5, true);
        let regularized = query_tolerance(velocity, 0.007, 0.27, 1.5, false);
        assert_eq!(reference.to_bits(), 0x3e922248);
        assert_ne!(reference.to_bits(), regularized.to_bits());
    }
}
