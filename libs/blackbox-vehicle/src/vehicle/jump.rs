//! Airborne force and angular stabilization with caller-supplied tuning and ground reference.
//! Behavioral specification: docs/specs/vehicle-jump-stabilizers.md.

use glam::{Mat3, Vec3};

use crate::math::{graph, ramp};

impl super::Vehicle {
    pub(super) fn apply_jump(&mut self, dt: f32, speed: f32, angular_velocity: Vec3) {
        let output = self.jump_state.update(
            &self.jump_config,
            &JumpInput {
                dt,
                mass: self.body.mass(),
                inertia: self.body.inertia(),
                rotation: self.body.rotation(),
                linear_velocity: self.body.linear_velocity,
                angular_velocity,
                speed,
                grounded_wheels: self.wheels_on_ground,
                body_contact: self.body_ground_contact || self.wheels_on_ground > 0,
                ground: self.ground_reference,
            },
        );
        self.body.apply_force(output.force);
        self.body.apply_torque(output.torque);
    }
}

/// A persistent terrain reference supplied by the world-query owner.
/// This consumer does not cast a ray or combine wheel contact normals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundReference {
    pub normal: Vec3,
    /// `(point_on_plane - body_position).dot(normal)`, negative above the plane.
    pub separation: f32,
}

/// Extra landing acceleration after a sustained airborne interval.
#[derive(Clone, Copy, Debug, Default)]
pub struct LandingConfig {
    pub acceleration: f32,
    pub minimum_time: f32,
    pub minimum_peak_altitude: f32,
    pub maximum_altitude: f32,
    pub minimum_alignment: f32,
    pub maximum_speed: f32,
}

/// Passive pitch/roll damping. Empty graph disables it.
#[derive(Clone, Copy, Debug, Default)]
pub struct PassiveConfig {
    pub minimum_up: f32,
    pub maximum_grounded_wheels: usize,
    pub maximum_speed: f32,
    pub damping: &'static [(f32, f32)],
}

/// Terrain-relative alignment. Zero pitch/roll coefficients disable it.
#[derive(Clone, Copy, Debug, Default)]
pub struct AlignmentConfig {
    pub minimum_ground_up: f32,
    pub minimum_body_up: f32,
    pub minimum_alignment: f32,
    pub minimum_altitude: f32,
    pub maximum_altitude: f32,
    pub maximum_speed: f32,
    /// Alignment corrections act only below this directional dot product.
    pub correction_threshold: f32,
    pub pitch: f32,
    pub roll: f32,
}

/// Game-specific coefficients are passed by the caller; defaults apply no assistance.
#[derive(Clone, Copy, Debug, Default)]
pub struct JumpConfig {
    pub extra_airborne_acceleration: f32,
    pub landing: LandingConfig,
    pub passive: PassiveConfig,
    pub alignment: AlignmentConfig,
}

/// Snapshot from the owning vehicle, in physics coordinates (world Y up).
#[derive(Clone, Copy, Debug)]
pub struct JumpInput {
    pub dt: f32,
    pub mass: f32,
    pub inertia: Vec3,
    pub rotation: Mat3,
    pub linear_velocity: Vec3,
    pub angular_velocity: Vec3,
    pub speed: f32,
    pub grounded_wheels: usize,
    pub body_contact: bool,
    /// Unknown terrain skips terrain-dependent landing and active alignment.
    pub ground: Option<GroundReference>,
}

/// World-space additions to pending force and torque.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct JumpForces {
    pub force: Vec3,
    pub torque: Vec3,
}

/// Time and highest known altitude since the last wheel contact.
#[derive(Clone, Copy, Debug, Default)]
pub struct JumpState {
    pub airborne_time: f32,
    pub peak_altitude: f32,
}

impl JumpState {
    pub fn update(&mut self, config: &JumpConfig, input: &JumpInput) -> JumpForces {
        if !input.dt.is_finite() || input.dt <= 0.0 {
            return JumpForces::default();
        }
        let mut result = JumpForces::default();
        if input.grounded_wheels == 0 {
            self.airborne_time += input.dt;
            if let Some(ground) = input.ground {
                self.peak_altitude = self.peak_altitude.max(-ground.separation);
            }
            let acceleration = config.extra_airborne_acceleration + self.landing(config, input);
            result.force = Vec3::NEG_Y * (input.mass * acceleration);
        }
        if input.grounded_wheels > 0 {
            *self = Self::default();
        }
        result.torque += passive(config.passive, input);
        result.torque += alignment(config.alignment, input);
        result
    }

    fn landing(&self, config: &JumpConfig, input: &JumpInput) -> f32 {
        let Some(ground) = input.ground else { return 0.0 };
        let c = config.landing;
        let altitude = -ground.separation;
        if self.airborne_time <= c.minimum_time
            || self.peak_altitude <= c.minimum_peak_altitude
            || input.rotation.y_axis.dot(ground.normal) <= c.minimum_alignment
            || input.linear_velocity.y >= 0.0
            || altitude >= c.maximum_altitude
        {
            return 0.0;
        }
        c.acceleration * (1.0 - ramp(altitude, 0.0, c.maximum_altitude)) * ramp(input.speed, 0.0, c.maximum_speed)
    }
}

fn passive(config: PassiveConfig, input: &JumpInput) -> Vec3 {
    if input.grounded_wheels >= config.maximum_grounded_wheels
        || input.rotation.y_axis.y <= config.minimum_up
        || input.body_contact
        || input.grounded_wheels > 0
    {
        return Vec3::ZERO;
    }
    let angular = input.rotation.transpose() * input.angular_velocity;
    let damping = ramp(input.speed, 0.0, config.maximum_speed) * graph(config.damping, angular.length());
    let moment = -angular * input.inertia * damping;
    input.rotation * Vec3::new(moment.x, 0.0, moment.z)
}

fn alignment(config: AlignmentConfig, input: &JumpInput) -> Vec3 {
    let Some(ground) = input.ground else { return Vec3::ZERO };
    let up = input.rotation.y_axis;
    let altitude = -ground.separation;
    if input.grounded_wheels != 0
        || ground.normal.y <= config.minimum_ground_up
        || up.y <= config.minimum_body_up
        || up.dot(ground.normal) <= config.minimum_alignment
        || altitude <= config.minimum_altitude
    {
        return Vec3::ZERO;
    }
    let right = input.rotation.x_axis;
    let forward = input.rotation.z_axis;
    let weight = input.mass
        * ramp(input.speed, 0.0, config.maximum_speed)
        * (1.0 - ramp(altitude, 0.0, config.maximum_altitude));
    let flat_forward = right.cross(ground.normal).normalize_or_zero();
    let flat_right = ground.normal.cross(forward).normalize_or_zero();
    correction(flat_forward, forward, weight * config.pitch, config.correction_threshold)
        + correction(flat_right, right, weight * config.roll, config.correction_threshold)
}

fn correction(flat: Vec3, direction: Vec3, weight: f32, threshold: f32) -> Vec3 {
    let dot = flat.dot(direction);
    if dot >= threshold {
        return Vec3::ZERO;
    }
    flat.cross(direction).normalize_or_zero() * (weight * (dot - 1.0))
}

#[cfg(test)]
mod tests;
