//! What the rider does to the bike each step: the fall test, the weight, the lean and handlebar, the pitch assist, the
//! gyroscopic torque, the prescribed roll and the air drag (`docs/specs/ride6-lean-controller.md`, `ride6-rider-help.md`).

use glam::Vec3;

use super::curve::{curve, ramp};
use super::filter;
use super::lean::{self, LeanInput};
use super::pitch_assist::{self, PitchInput};
use super::{Bike, BikeInput, FRONT, REAR, choices, lean_of, pitch_of, rider};

impl Bike {
    /// The fall tests: the lean and the pitch at any speed, the angle between the velocity and the heading and the angular
    /// speed above the minimum speed of the spec. A fallen rider stays fallen until the bike is placed again.
    pub(super) fn check_fall(&mut self, speed: f32) {
        if self.fallen {
            return;
        }
        let rotation = self.body.rotation();
        let fall = self.spec.fall;
        let mut down = false;
        if choices::LEAN_AND_PITCH_FALL_AT_ANY_SPEED || speed > fall.min_speed {
            down = lean_of(rotation).abs() > fall.lean || pitch_of(rotation).abs() > fall.pitch;
        }
        if speed > fall.min_speed {
            let forward = rotation * Vec3::Z;
            let velocity = self.body.linear_velocity;
            let flat_velocity = Vec3::new(velocity.x, 0.0, velocity.z);
            let flat_forward = Vec3::new(forward.x, 0.0, forward.z);
            let angle = match flat_velocity.length() > 1.0 && flat_forward.length() > 1e-3 {
                true => flat_velocity.angle_between(flat_forward),
                false => 0.0,
            };
            down |= angle > fall.speed_direction || self.body.angular_velocity.length() > fall.angular_speed;
        }
        self.fallen = down;
    }

    /// The rider's weight follows its input through a filter, and the centre of gravity of bike and rider follows the
    /// weight. The centre of gravity stays where it is in the world (the rider moving makes the frame move the other way).
    pub(super) fn shift_weight(&mut self, dt: f32) {
        let weight = self.weight.step(self.weight_input, self.spec.chassis.rider_weight_rate, dt);
        let new = rider::combined_cog(&self.spec.chassis, weight);
        let old = self.body.cog();
        if new == old {
            return;
        }
        let rotation = self.body.rotation();
        self.body.position += rotation * (old - new);
        self.body.set_cog(new);
    }

    /// The angle (rad, positive right) of the front wheel's heading from the bike's, in the plane of the ground, at the
    /// handlebar angle `steer`.
    fn heading_angle(&self, steer: f32) -> f32 {
        let rotation = self.body.rotation();
        let (_, turn) = self.geometry.front_pose(steer, 0.0);
        let normal = self.ground_normal();
        let flat = |v: Vec3| v - normal * v.dot(normal);
        let body_forward = flat(rotation * Vec3::Z);
        let wheel_forward = flat(rotation * (turn * Vec3::Z));
        let right = normal.cross(body_forward);
        wheel_forward.dot(right).atan2(wheel_forward.dot(body_forward))
    }

    /// The angle (rad, positive nose up) of the line through the two hubs from the plane of the ground, less what it is at
    /// rest: 0 with both wheels down on any slope, the angle of a wheelie or a stoppie when one wheel is up.
    pub(super) fn lift_angle(&self) -> f32 {
        let front = self.pose(FRONT, self.wheels[FRONT].compression).hub;
        let rear = self.pose(REAR, self.wheels[REAR].compression).hub;
        let line = (front - rear).try_normalize().unwrap_or(Vec3::Z);
        let normal = match (self.wheels[FRONT].on_ground, self.wheels[REAR].on_ground) {
            (true, _) => self.wheels[FRONT].normal,
            (false, true) => self.wheels[REAR].normal,
            (false, false) => Vec3::Y,
        };
        let at_rest = (self.spec.front.radius - self.spec.rear.radius) / self.spec.chassis.wheelbase.max(f32::EPSILON);
        (line.dot(normal) - at_rest).clamp(-1.0, 1.0).asin()
    }

    /// The shares of the throttle and of the front brake the rider help lets through (it cuts the throttle as the front
    /// lifts and eases the front brake as the rear lifts).
    pub(super) fn help_factors(&self) -> (f32, f32) {
        let Some(assist) = &self.spec.wheelie else { return (1.0, 1.0) };
        let lift = self.lift_angle();
        let rate = -self.body.angular_velocity.dot(self.body.rotation() * Vec3::X);
        let asked = pitch_assist::target_position(assist, self.weight.value()) < 0.0;
        (pitch_assist::throttle_factor(assist, lift, rate, asked), pitch_assist::brake_factor(assist, lift, rate))
    }

    /// The normal of the ground under the front wheel, or up when it is in the air.
    fn ground_normal(&self) -> Vec3 {
        let front = &self.wheels[FRONT];
        match front.on_ground {
            true => front.normal,
            false => Vec3::Y,
        }
    }

    /// The angle (rad, positive right) of the front contact point's velocity from the bike's heading in the ground plane.
    fn front_velocity_angle(&self) -> f32 {
        let rotation = self.body.rotation();
        let front = &self.wheels[FRONT];
        let point = match front.on_ground {
            true => front.contact,
            false => self.pose(FRONT, front.compression).hub,
        };
        let normal = self.ground_normal();
        let flat = |v: Vec3| v - normal * v.dot(normal);
        let forward = flat(rotation * Vec3::Z);
        let right = normal.cross(forward);
        let velocity = flat(self.body.point_velocity(point));
        match velocity.length() > 0.5 {
            true => velocity.dot(right).atan2(velocity.dot(forward)),
            false => 0.0,
        }
    }

    /// The lean controller and the handlebar: plan the roll, turn the wanted front force into a slip angle, set the
    /// handlebar to the angle that gives the front wheel that slip, and filter it.
    pub(super) fn steer_handlebar(&mut self, dt: f32, speed: f32, stick: f32) {
        let rotation = self.body.rotation();
        let forward = rotation * Vec3::Z;
        let lean_angle = lean_of(rotation);
        let total = self.body.mass();
        let cog = self.body.cog();
        let wheelbase = self.spec.chassis.wheelbase.max(f32::EPSILON);
        let front_share = (0.5 + cog.z / wheelbase).clamp(0.0, 1.0);
        let rest_load = front_share * total * 9.81;
        let front = &self.wheels[FRONT];
        let load = match front.on_ground && front.load > 0.0 {
            true => front.load,
            false => rest_load,
        };
        let camber = match front.on_ground {
            true => front.camber,
            false => lean_angle,
        };
        let fz_kn = load * 1e-3;
        let gamma_deg = -camber.to_degrees();
        let grip = self.spec.front.tyre.lateral_peak(fz_kn) / load.max(1.0);
        let yaw_rate = self.body.angular_velocity.dot(rotation * Vec3::Y);
        let velocity = self.body.linear_velocity;
        let (flat_velocity, flat_forward) =
            (Vec3::new(velocity.x, 0.0, velocity.z), Vec3::new(forward.x, 0.0, forward.z));
        let heading_error = match flat_velocity.length() > 1.0 && flat_forward.length() > 1e-3 {
            true => flat_velocity.angle_between(flat_forward),
            false => 0.0,
        };
        let pitch = self.lift_angle();
        let pitch_cut = match &self.spec.wheelie {
            Some(assist) => {
                let params = match pitch >= 0.0 {
                    true => &assist.wheelie,
                    false => &assist.stoppie,
                };
                (params.lean_limit, ramp(pitch.abs(), params.lean_pitch_range))
            }
            None => (0.0, 0.0),
        };
        let steering = &self.spec.steering;
        let rider_term =
            self.spec.chassis.rider_mass * steering.lean.rider_mass_scale * self.spec.chassis.rider_offset.y.powi(2);
        let input = LeanInput {
            stick,
            speed,
            lean: lean_angle,
            lean_rate: -self.body.angular_velocity.dot(forward),
            lateral_grip: grip,
            lateral_g: speed * yaw_rate / 9.81,
            heading_error,
            pitch_cut,
            roll_inertia: self.body.inertia().z + rider_term,
            mass: total,
            cg_height: cog.y,
            front_share,
            dt,
        };
        let plan = self.lean.plan(steering, &input);
        self.plan = plan;
        let direct = stick.clamp(-1.0, 1.0) * steering.max_angle;
        let aim = match speed.abs() > steering.lean.blend_range[0] {
            true => {
                let alpha = lean::slip_for_force(&self.spec.front.tyre, fz_kn, gamma_deg, plan.front_force);
                let wanted = self.front_velocity_angle() - alpha;
                let controlled = lean::solve_steer(wanted, steering.max_angle, |steer| self.heading_angle(steer));
                lean::blend_to_stick(steering, speed, stick, controlled)
            }
            false => direct,
        };
        // The rider leads the handlebar: its filter lags its input by about the time constant of its slow pole, and a front
        // wheel that follows the direction of its own contact point with that lag pushes the nose further round (the front
        // tyre is stiffer than the rear one whenever the weight is forward, and a bike on the brakes then spins out). The
        // input is advanced by that time constant times its own rate (`choices::HANDLEBAR_LEAD`).
        let previous = self.aim_before.replace(aim);
        let lead = filter::slow_time_constant(steering.response, steering.damping_ratio);
        let aim = match previous.filter(|_| choices::HANDLEBAR_LEAD) {
            Some(previous) => aim + lead * (aim - previous) / dt,
            None => aim,
        };
        let angle = self.steer.step(aim, steering.response, steering.damping_ratio, dt);
        let limit = steering.max_angle;
        if angle.abs() > limit {
            let clamped = angle.clamp(-limit, limit);
            self.steer.reset(clamped);
        }
    }

    /// The lean help: a side force at the front tyre that pulls its side force towards the wanted one, within what the tyre can
    /// still give (`ride6-lean-controller.md`, "Lean help"). The reductions of the help by throttle, brake and slip are not
    /// modelled (their pairing was not recovered).
    fn apply_lean_help(&mut self, speed: f32) {
        let front = &self.wheels[FRONT];
        if !front.on_ground {
            return;
        }
        let spec = &self.spec.steering.lean;
        let share = curve(&spec.help_speed, speed.abs()).clamp(0.0, 1.0);
        if share <= 0.0 {
            return;
        }
        let (side, point, traction) = (front.side, front.contact, front.traction);
        let actual = front.force.dot(side);
        let ideal = actual + share * (self.plan.front_force - actual);
        let fz_kn = front.load * 1e-3;
        let tyre = &self.spec.front.tyre;
        let peak_y = tyre.lateral_peak(fz_kn).max(0.0);
        let peak_x = tyre.longitudinal_peak(fz_kn).max(1.0);
        let used = (traction / peak_x).powi(2).min(1.0);
        let cap = spec.help_limit * peak_y * (1.0 - used).sqrt();
        let total = match actual.abs() < ideal.abs() {
            true => ideal.clamp(-cap, cap),
            false => ideal,
        };
        self.body.apply_force_at(side * (total - actual), point);
    }

    /// The torques and forces that act on the whole bike once the tyres have done their part: the lean help, the pitch assist,
    /// the gyroscopic torques, the roll the rider prescribes, and the air drag.
    pub(super) fn apply_rider_and_air(&mut self, input: &BikeInput, speed: f32) {
        let rotation = self.body.rotation();
        let forward = rotation * Vec3::Z;
        let right = rotation * Vec3::X;
        let up = rotation * Vec3::Y;
        if !self.fallen {
            self.apply_lean_help(speed);
            if let Some(assist) = &self.spec.wheelie {
                let limit = self.plan.limit.max(0.05);
                let lift = self.lift_angle();
                let request = PitchInput {
                    weight: self.weight.value(),
                    throttle: input.throttle,
                    front_brake: input.front_brake,
                    pitch: lift,
                    // Rotating about the right axis lowers the nose.
                    pitch_rate: -self.body.angular_velocity.dot(right),
                    speed,
                    normalised_lean: (lean_of(rotation).abs() / limit).min(1.0),
                    front_on_ground: self.wheels[FRONT].on_ground,
                    rear_on_ground: self.wheels[REAR].on_ground,
                    gap: self.spec.chassis.wheelbase * lift.abs().sin(),
                    // The pitch inertia of the bike without the rider.
                    inertia: self.spec.chassis.mass * self.spec.chassis.tensor_scale.x,
                };
                let torque = pitch_assist::torque(assist, &request);
                self.body.apply_torque(right * -torque);
            }
            // The spin momentum of the wheels, turned by the bike's yaw, torques the frame about its length (a yaw to the right
            // rolls it to the left); turned by its roll, it torques the frame about the vertical (a roll to the left yaws it to
            // the left).
            let momentum =
                self.spec.front.inertia * self.wheels[FRONT].spin + self.spec.rear.inertia * self.wheels[REAR].spin;
            let omega = self.body.angular_velocity;
            let roll_torque = self.spec.gyro_roll_scale * lean_of(rotation).cos() * momentum * omega.dot(up);
            let yaw_torque = -self.spec.gyro_yaw_scale * momentum * omega.dot(forward);
            self.body.apply_torque(forward * roll_torque + up * yaw_torque);
            // The rider prescribes the roll: whatever torque the tyres and the rest left, he makes up, within his strength.
            let wanted = -self.body.inertia().z * self.plan.acceleration;
            let missing = wanted - self.body.pending_torque().dot(forward);
            let strength = choices::ROLL_TORQUE_AUTHORITY * self.body.mass() * 9.81 * self.body.cog().y;
            self.body.apply_torque(forward * missing.clamp(-strength, strength));
        }
        let velocity = self.body.linear_velocity;
        self.body.apply_force(-velocity * (velocity.length() * self.spec.drag));
    }
}
