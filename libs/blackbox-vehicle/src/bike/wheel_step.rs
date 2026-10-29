//! One wheel in a step: the suspension with the wheel's mass, the split of the ground penetration between the tyre and the
//! suspension, the load, the tyre forces and the spin (`docs/specs/ride6-suspension.md`, `docs/specs/ride6-tyre.md`).

use glam::Vec3;

use super::powertrain::Drive;
use super::thermal;
use super::tyre::{self, TyreInput};
use super::{Bike, FRONT, geometry, wheel};
use crate::ground::Ground;

/// The most the compression speed of a wheel state can take (m/s), so one violent step cannot poison the next.
const MAX_STATE_SPEED: f32 = 20.0;
/// The part of the axis' cosine below which a force along it is not trusted (a wheel lying on its side).
const MIN_AXIS_COSINE: f32 = 0.3;

impl Bike {
    /// The pose of a wheel for a suspension compression: the hub in the world, the unit axle and heading, and the
    /// suspension axis.
    pub(super) fn pose(&self, index: usize, compression: f32) -> wheel::Pose {
        let rotation = self.body.rotation();
        let (hub, turn) = match index {
            FRONT => self.geometry.front_pose(self.steer.output(), compression),
            _ => self.geometry.rear_pose(compression),
        };
        let axis = match index {
            FRONT => self.geometry.axis,
            _ => Vec3::Y,
        };
        wheel::Pose {
            hub: self.body.position + rotation * hub,
            axle: rotation * (turn * Vec3::X),
            heading: rotation * (turn * Vec3::Z),
            axis: rotation * axis,
        }
    }

    /// The mass the body presents to a push along `direction` at the world point `point` (kg): the whole mass for a push
    /// through the centre of gravity, less for one that also turns the body.
    fn axial_mass(&self, point: Vec3, direction: Vec3) -> f32 {
        let lever = (point - self.body.world_cog()).cross(direction);
        let turning = lever.dot(self.body.inv_world_tensor() * lever);
        1.0 / (1.0 / self.body.mass() + turning)
    }

    /// Work out wheel `index` for a step of `dt`: `brake` is the brake torque (N m), `drive` what the powertrain puts on it.
    pub(super) fn step_wheel(&mut self, index: usize, dt: f32, brake: f32, drive: Drive, ground: &dyn Ground) {
        let wheel_spec = match index {
            FRONT => &self.spec.front,
            _ => &self.spec.rear,
        };
        let suspension = &self.suspension[index];
        let before = &self.wheels[index];
        let (start, speed0, spin, traction0) =
            (before.compression, before.compression_speed, before.spin, before.traction);
        let limits = (suspension.rest_extension(), suspension.rest_compression() + wheel::OVER_TRAVEL);
        let weight = wheel_spec.mass * 9.81;
        // The wheel on its suspension, free: it follows the spring and the damper, and stops at the ends of the travel.
        let axis_up = self.pose(index, start).axis.y.max(0.0);
        let rest = -limits.0;
        let (free_total, free_speed) =
            suspension.free_step(rest + start, speed0, dt, wheel_spec.mass, weight * axis_up);
        let contact = wheel::settle(|c| self.pose(index, c), wheel_spec, ground, start, limits);
        let reach = contact.as_ref().map(|c| rest + c.compression);
        let Some((contact, reach)) = contact.zip(reach).filter(|&(_, reach)| free_total < reach) else {
            // In the air, or hanging just above the road: the wheel is where its suspension puts it.
            let (drive_torque, extra) = match index {
                super::REAR => (drive.torque, drive.extra_inertia),
                _ => (0.0, 0.0),
            };
            let state = &mut self.wheels[index];
            state.release(free_total - rest, free_speed);
            state.spin = tyre::spin_in_air(wheel_spec, spin, drive_torque, brake, extra, dt);
            if let Some(thermal) = &wheel_spec.thermal {
                let work = thermal::Work {
                    spin: state.spin,
                    environment: self.chassis.environment,
                    ..thermal::Work::default()
                };
                self.thermal[index].step(thermal, &work, dt);
            }
            return;
        };
        // The ground penetration, shared between the tyre and the suspension.
        let tyre_share = suspension.tyre_share(reach, wheel_spec.tire_stiffness);
        let tyre_nominal = (tyre_share * reach).min(wheel_spec.max_tire_deformation.max(0.0));
        let total = (reach - tyre_nominal).max(free_total).clamp(0.0, suspension.spec().max_travel);
        let deformation = (reach - total).max(0.0);
        // What the stops could not take, the tyre (or a stiff stop, for a rigid tyre) takes as a force.
        let overload = (deformation - tyre_nominal).max(0.0);
        let velocity = self.body.point_velocity(contact.point);
        let cosine = contact.axis_cosine.max(MIN_AXIS_COSINE);
        // The frame moves towards the ground at the rate the suspension compresses.
        let approach = (-velocity.dot(contact.normal) / cosine).clamp(-MAX_STATE_SPEED, MAX_STATE_SPEED);
        let speed = free_speed.max(approach).clamp(-MAX_STATE_SPEED, MAX_STATE_SPEED);
        let mass = self.axial_mass(contact.point, contact.normal) * cosine * cosine;
        let suspension = &self.suspension[index];
        let drive_torque = match index {
            super::REAR => drive.torque,
            _ => 0.0,
        };
        let coupled = suspension.coupling_force(total, drive_torque, traction0, brake, wheel_spec.radius);
        let spring = suspension.force(total, approach, dt, mass) + coupled;
        let inertia = wheel_spec.mass * (speed - speed0) / dt;
        let along_axis = spring + weight * axis_up + inertia;
        let mut load = (along_axis / cosine).max(0.0);
        // The end of the travel is a hard stop that adds no force of its own (`ride6-chassis-round6.md`, "End of travel"): the tyre
        // takes what is left over through its radial stiffness, a rigid tyre nothing.
        load += wheel_spec.tire_stiffness.max(0.0) * overload;
        let frame = wheel::ground_frame(contact.normal, contact.heading);
        let camber = geometry::camber(contact.axle, contact.normal);
        let input = TyreInput {
            load,
            v_long: velocity.dot(frame.forward),
            v_lat: velocity.dot(frame.right),
            spin,
            camber,
            compression: deformation,
            efficiency: self.thermal[index].efficiency(),
            grip: contact.surface,
        };
        let (drive_torque, extra) = match index {
            super::REAR => (drive.torque, drive.extra_inertia),
            _ => (0.0, 0.0),
        };
        let (new_spin, output) =
            tyre::advance_wheel(wheel_spec, &input, drive_torque, brake, extra, index == FRONT, dt);
        // The rolling resistance acts against the velocity of the whole bike, at the contact (above 1 m/s).
        let vehicle_velocity = self.body.linear_velocity;
        let vehicle_speed = vehicle_velocity.length();
        let rolling = tyre::rolling_resistance(wheel_spec, load, vehicle_speed, output.slip_y);
        let against = match vehicle_speed > 1.0 {
            true => -vehicle_velocity / vehicle_speed,
            false => Vec3::ZERO,
        };
        let force = contact.normal * load
            + frame.forward * output.longitudinal
            + frame.right * output.lateral
            + against * rolling;
        self.body.apply_force_at(force, contact.point);
        if let Some(thermal) = &wheel_spec.thermal {
            let radius = tyre::effective_radius(wheel_spec, camber, deformation);
            let work = thermal::Work {
                load,
                normalised_x: output.normalised_x,
                normalised_y: output.normalised_y,
                rolling,
                camber,
                slip_speed: radius * new_spin - input.v_long,
                contact_speed: velocity.length(),
                spin: new_spin,
                in_contact: true,
                environment: self.chassis.environment,
            };
            self.thermal[index].step(thermal, &work, dt);
        }
        let state = &mut self.wheels[index];
        state.compression = total - rest;
        state.compression_speed = speed;
        state.tyre_compression = deformation;
        state.on_ground = true;
        state.load = load;
        state.contact = contact.point;
        state.normal = contact.normal;
        state.force = force;
        state.slip_angle = output.slip_angle;
        state.slip_ratio = output.slip_ratio;
        state.normalised_x = output.normalised_x;
        state.aligning = output.aligning;
        state.traction = output.longitudinal;
        state.side = frame.right;
        state.camber = camber;
        state.surface_tag = contact.surface.tag;
        state.spin = new_spin;
    }
}
