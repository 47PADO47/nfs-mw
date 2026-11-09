//! What the rider does to the bike each step: the weight, the lean and handlebar, the lean help, the pitch assist, the
//! gyroscopic torque and the air drag (the fall tests are in `fall.rs`) (`docs/specs/ride6-lean-controller.md`, `ride6-rider-help.md`).

use glam::Vec3;

use super::curve::ramp;
use super::lean::{self, LeanInput};
use super::{Bike, BikeInput, FRONT, REAR, choices, lean_of, rider, rider_place, wheelie_steer};

impl Bike {
    /// The rider's inputs follow their channels' rates, the place they ask for is worked out (`rider_place::place`: the weight,
    /// the lateral weight and the planned lean over its limit, at this speed), the rider's body follows that place on its spring
    /// and turns on its angular spring, and the centre of gravity of bike and rider follows. The centre of gravity stays where it
    /// is in the world (the rider moving makes the frame move the other way); the rider's turning puts its torque on the bike.
    pub(super) fn shift_weight(&mut self, dt: f32) {
        let rate = self.spec.chassis.rider_weight_rate;
        let weight = self.weight.step(self.trick.aid, rate, dt);
        let lateral = self.weight_lateral.step(self.weight_lateral_input, rider::LATERAL_WEIGHT_RATE, dt);
        let rotation = self.body.rotation();
        let speed = self.body.linear_velocity.dot(rotation * Vec3::Z);
        let lean = (self.plan.target / self.plan.limit.max(choices::MIN_LEAN_LIMIT)).clamp(-1.0, 1.0);
        let input = rider_place::PlaceInput { weight, weight_lateral: lateral, lean, speed };
        self.placement = rider_place::place(&self.spec.chassis, &self.spec.rider, &input);
        // The acceleration the rider feels, in the body frame: the bike's less gravity (the specific force), or the bike's alone
        // when his spring's rest place is raised to take his weight.
        let velocity = self.body.linear_velocity;
        let acceleration = (velocity - self.last_velocity) / dt.max(1e-6);
        self.last_velocity = velocity;
        self.acceleration = acceleration;
        let felt = match self.spec.rider.correct_gravity {
            true => acceleration,
            false => acceleration + Vec3::Y * 9.81,
        };
        // His own air drag pushes on him alone; it reaches the frame through his spring.
        let drag = self.rider_drag_force() / self.spec.chassis.rider_mass.max(f32::EPSILON);
        let target = rider::rider_position(&self.spec.chassis, self.placement);
        let felt = rotation.transpose() * (felt - drag);
        self.rider_body.step(&self.spec.chassis, &self.spec.rider, target, felt, speed, dt);
        let omega = rotation.transpose() * self.body.angular_velocity;
        let torque = self.rider_rotation.step(&self.spec.rider, self.spec.chassis.rider_mass, omega, speed, dt);
        self.body.apply_torque(rotation * torque);
        let new = rider::combined_cog(&self.spec.chassis, self.placement, self.rider_body.deflection);
        let old = self.body.cog();
        if new == old {
            return;
        }
        let rotation = self.body.rotation();
        self.body.position += rotation * (old - new);
        self.body.set_cog(new);
    }

    /// The air drag of the rider himself (N, world frame): `c(weight, tuck) (1 - s) |v| v` against the velocity, with the
    /// coefficient of [`rider_place::drag_coefficient`] at the weight after the rider help (`ride6-rider-round6.md`, section 1).
    pub(super) fn rider_drag_force(&self) -> Vec3 {
        let velocity = self.body.linear_velocity;
        let coefficient = rider_place::drag_coefficient(&self.spec.rider, self.weight.value(), self.trick.tuck);
        -velocity * (velocity.length() * coefficient * (1.0 - self.slipstream))
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

    /// The world velocity of the point one wheel radius below the front hub along the chassis' up axis, which is where the binary reads
    /// the front velocity for the handlebar (`0x145157eb0`: the hub less the radius along the chassis' up, not the tyre's own patch).
    fn front_velocity(&self) -> Vec3 {
        let hub = self.pose(FRONT, self.wheels[FRONT].compression).hub;
        let up = self.body.rotation() * Vec3::Y;
        self.body.point_velocity(hub - up * self.spec.front.radius)
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
        // The lateral grip peak the limits use is the front tyre's at its load at rest, camber 0 (the handler's tick stores it once
        // per step: `ride6-feel-round8.md`, section 2), not at the load of the moment.
        let rest_front = (self.spec.front_share() * self.spec.total_mass() * 9.81).max(1.0);
        let grip = self.spec.front.tyre.lateral_peak(rest_front * 1e-3) / rest_front;
        // The fall limit is that grip's arctangent times the wheel's `LateralGripLimit`; the lean limit's factor is the other one,
        // `LateralGripTolerance`.
        self.chassis.follow_grip(&self.spec.fall, grip.max(0.0).atan() * self.spec.front.lateral_grip_limit);
        let heading_error = self.heading_error();
        let pitch = self.lift_angle();
        let pitch_cut = match &self.spec.wheelie {
            Some(assist) => {
                let params = match pitch >= 0.0 {
                    true => &assist.wheelie,
                    false => &assist.stoppie,
                };
                // The wheelie's cut stops at `choices::WHEELIE_LEAN_FLOOR` (a choice); the stoppie's is the data's.
                let floor = match pitch >= 0.0 {
                    true => choices::WHEELIE_LEAN_FLOOR.max(0.0),
                    false => 0.0,
                };
                (params.lean_limit.max(floor), ramp(pitch.abs(), params.lean_pitch_range))
            }
            None => (0.0, 0.0),
        };
        let steering = &self.spec.steering;
        let front_arm = match front.on_ground && self.front_velocity().length() >= 1.0 {
            true => lean::lean_arm(rotation * Vec3::X, front.normal),
            false => 1.0,
        };
        let rider_term =
            self.spec.chassis.rider_mass * steering.lean.rider_mass_scale * self.spec.chassis.rider_offset.y.powi(2);
        let input = LeanInput {
            stick,
            speed,
            lean: lean_angle,
            lean_rate: -self.body.angular_velocity.dot(forward),
            // The tyre's grip times the surface's, as the bike believes it: dirt asks for a shallower lean, not a bike that goes down.
            lateral_grip: grip * self.surface_factor(),
            terrain_slope: self.terrain_slope(),
            long_g: self.acceleration.dot(forward) / 9.81,
            heading_error,
            pitch_cut,
            roll_inertia: self.body.inertia().z + rider_term,
            cg_height: cog.y,
            front_arm,
            target_cap: self.block_cap(speed),
            burnout_limit: self.burnout_lean_limit(),
            dt,
        };
        let plan = self.lean.plan(steering, &input);
        self.plan = plan;
        let direct = stick.clamp(-1.0, 1.0) * steering.max_angle;
        let aim = match speed.abs() > steering.lean.blend_range[0] {
            true => {
                let slip = lean::slip_for_force(&self.spec.front.tyre, fz_kn, gamma_deg, plan.front_force);
                let normal = match front.on_ground {
                    true => front.normal,
                    false => Vec3::Y,
                };
                let controlled = lean::handlebar_angle(self.front_velocity(), normal, rotation, slip)
                    .clamp(-steering.max_angle, steering.max_angle);
                lean::blend_to_stick(steering, speed, stick, controlled)
            }
            false => direct,
        };
        // With the front wheel up the mid-air steer takes the handlebar from the controller (`wheelie_steer.rs`).
        let weight = self.mid_air_weight();
        let aim = aim + (wheelie_steer::mid_air_angle(stick, steering.max_angle) - aim) * weight;
        // In the air, with no tyre to work through, the controller would turn the bars to their lock: the arcade help centres them.
        let aim = aim * (1.0 - self.bars_centred_share());
        let angle = self.steer.step(
            aim,
            steering.response * steering.arcade.handlebar_response.powi(2),
            steering.damping_ratio,
            dt,
        );
        let limit = steering.max_angle;
        if angle.abs() > limit {
            let clamped = angle.clamp(-limit, limit);
            self.steer.reset(clamped);
        }
        self.track_block(plan.capped, dt);
    }

    /// The most the target lean may be: with the block-fall assist on and above its speed, `ratio x fall lean x 0.99`
    /// (`ride6-lean-controller.md`, step 1); otherwise nothing holds it.
    fn block_cap(&self, speed: f32) -> f32 {
        let fall = &self.spec.fall;
        match fall.block_ratio > 0.0 && speed > fall.block_min_speed && self.block_wait <= 0.0 {
            true => fall.block_ratio * self.chassis.fall_lean * 0.99,
            false => f32::MAX,
        }
    }

    /// The assist holds the lean for `block_time` at most, then waits `block_wait` before it holds it again.
    fn track_block(&mut self, held: bool, dt: f32) {
        self.block_wait = (self.block_wait - dt).max(0.0);
        match held {
            true => self.block_held += dt,
            false => self.block_held = (self.block_held - dt).max(0.0),
        }
        if self.block_held > self.spec.fall.block_time {
            self.block_held = 0.0;
            self.block_wait = self.spec.fall.block_wait;
        }
    }

    /// The lean help: a side force at the front tyre that pulls its side force towards the wanted one, within what the tyre can
    /// still give (`ride6-lean-controller.md`, "Lean help"). The help factor and the split of the force into a force at the
    /// contact and a couple about the centre of gravity are those of `ride6-chassis-round6.md`: above 30 km/h it is all force;
    /// slower it turns into the couple, which at a standstill (factor 1) is the roll torque the controller asks for, as far as the
    /// tyre could give it. Two things of the reference that rounds 8 to 10 missed (`ride6-feel-round11.md`, section 1):
    ///
    /// - the tyre's limit on the help is its grip at the **rest load at least** (`max(load, rest load)` in `0x1450c3ae0`), so a front
    ///   wheel that the throttle has unloaded still has the help of a loaded one;
    /// - with the front wheel off the road the help does not stop: the force share is 0, so all of it is the couple about the
    ///   centre of gravity of a force at the point under the front hub, limited by the grip at the rest load (the reference fades
    ///   it with the wheel's ground distance over 20 to 35 cm, which a wheelie never reaches; here it counts while the rear wheel is on
    ///   the road). That is what holds a bike on its rear wheel against a stick; it replaces round 9's rider-balance torque (a
    ///   172 N m cap of this port's own).
    fn apply_lean_help(&mut self, speed: f32, pedals: f32) {
        let rotation = self.body.rotation();
        let rest_front = (self.spec.front_share() * self.spec.total_mass() * 9.81).max(1.0);
        let front = &self.wheels[FRONT];
        let spec = &self.spec.steering.lean;
        // The contact the help acts at: the road under the tyre, or - with the front wheel in the air over a rear wheel on the road -
        // the point one wheel radius under the hub along the chassis' up (`0x145157eb0`), in the horizontal direction of the bars' side.
        let (side, point, traction, actual, load, balance) = match front.on_ground {
            true => (
                front.side,
                front.contact,
                front.traction,
                front.force.dot(front.side),
                front.load.max(rest_front),
                lean::help::balance(spec, speed.abs()),
            ),
            false => {
                if !self.wheels[REAR].on_ground || self.departure.off {
                    return;
                }
                let right = rotation * Vec3::X;
                let Some(side) = (right - Vec3::Y * right.y).try_normalize() else { return };
                let hub = self.pose(FRONT, front.compression).hub;
                (side, hub - rotation * Vec3::Y * self.spec.front.radius, 0.0, 0.0, rest_front, 0.0)
            }
        };
        let input = lean::help::HelpInput { speed: speed.abs(), heading_error: self.heading_error(), pedals };
        // A collision adds to the help for a moment (`choices::COLLISION_LEAN_HELP`).
        // In a burnout the help factor is 1 and the balance 0: the help is all couple (`ride6-chassis-round6.md`, sections 2.1 and 2.2).
        // The factor is whole in a burnout and `LeanHelpFactorWheelie` with the front wheel up (`wheelie_steer.rs`).
        let factor = match (self.burnout, front.on_ground) {
            (true, _) => 1.0,
            (false, false) => wheelie_steer::LEAN_HELP_FACTOR_WHEELIE,
            (false, true) => lean::help::help_factor(spec, &input),
        };
        let balance = balance * f32::from(!self.burnout);
        let arcade = &self.spec.steering.arcade;
        let share = (factor * arcade.lean_help + self.chassis.collision_help.lean).clamp(0.0, 1.0);
        if share <= 0.0 {
            return;
        }
        let ideal = actual + share * (self.plan.front_force - actual);
        let fz_kn = load * 1e-3;
        let tyre = &self.spec.front.tyre;
        let peak_y = tyre.lateral_peak(fz_kn).max(0.0);
        let peak_x = tyre.longitudinal_peak(fz_kn).max(1.0);
        let used = (traction / peak_x).powi(2).min(1.0);
        let cap = spec.help_limit * arcade.lean_help_limit * peak_y * (1.0 - used).sqrt();
        let total = match actual.abs() < ideal.abs() {
            true => ideal.clamp(-cap, cap),
            false => ideal,
        };
        let force = side * (total - actual);
        self.body.apply_force_at(force * balance, point);
        let cog = self.body.world_cog();
        self.body.apply_torque(lean::help::couple(point, cog, force, balance));
    }

    /// The stability help (`ride6-chassis-round7.md`, section 3): a yaw torque, about the road's up, that pulls the yaw rate towards the
    /// one a steady turn at the present lean needs. The help is off when a wheel is off the road (the game scales it by the
    /// wheels' distance from the ground and switches it off beyond 30 cm; here any wheel in the air counts as far).
    fn apply_stability_help(&mut self, dt: f32, speed: f32, input: &BikeInput) {
        let spec = &self.spec.steering.lean.stability;
        if spec.speed.is_empty() || self.burnout || !self.wheels[FRONT].on_ground || !self.wheels[REAR].on_ground {
            return;
        }
        let rotation = self.body.rotation();
        let normalised = (self.plan.target / self.plan.limit.max(choices::MIN_LEAN_LIMIT)).clamp(-1.0, 1.0);
        let state = lean::help::StabilityInput {
            speed: speed.abs(),
            lean: lean_of(rotation),
            normalised_lean: normalised,
            throttle: input.throttle,
            brake: input.front_brake,
            extra_acceleration: self.cornering_acceleration(speed),
        };
        let factor =
            (lean::help::stability_factor(spec, &state) + self.chassis.collision_help.stability).clamp(0.0, 1.0);
        if factor <= 0.0 {
            return;
        }
        // The axis: the world's up, or the up perpendicular to the line between the two contacts.
        let line = (self.wheels[FRONT].contact - self.wheels[REAR].contact).try_normalize();
        let axis = match (spec.uses_terrain_normal, line) {
            (true, Some(line)) => (Vec3::Y - line * line.dot(Vec3::Y)).try_normalize().unwrap_or(Vec3::Y),
            _ => Vec3::Y,
        };
        // A turn to the right is a positive rotation about the up axis (x right, y up, z forward), as in the help.
        let yaw_right = self.body.angular_velocity.dot(axis);
        let acceleration = lean::help::stability_acceleration(spec, factor, &state, yaw_right, dt);
        // The torque of that angular acceleration through the inertia tensor of the body.
        let wanted = axis * acceleration;
        let local = rotation.transpose() * wanted;
        self.body.apply_torque(rotation * (local * self.body.inertia()));
    }

    /// The torques and forces that act on the whole bike once the tyres have done their part: the lean help, the pitch assist,
    /// the gyroscopic torques, and the air drag. (The rider adds no roll torque of his own: the lean the controller prescribes is
    /// reached through the front tyre and the help only.)
    pub(super) fn apply_rider_and_air(&mut self, dt: f32, speed: f32, input: &BikeInput) {
        let pedals = input.throttle - input.front_brake;
        let rotation = self.body.rotation();
        let forward = rotation * Vec3::Z;
        let up = rotation * Vec3::Y;
        self.chassis.collision_help.fade(dt);
        if !self.fallen {
            self.apply_lean_help(speed, pedals);
            self.apply_stability_help(dt, speed, input);
            self.apply_cornering_assist(speed);
            self.apply_wheelie_yaw(dt, speed);
            self.apply_burnout_help(dt, input.steer);
            self.apply_pitch_assist(dt, speed);
            self.apply_air_attitude();
            self.apply_landing_assist();
            // The spin momentum of the wheels, turned by the bike's yaw, torques the frame about its length (a yaw to the right
            // rolls it to the left); turned by its roll, it torques the frame about the vertical (a roll to the left yaws it to
            // the left).
            let momentum =
                self.spec.front.inertia * self.wheels[FRONT].spin + self.spec.rear.inertia * self.wheels[REAR].spin;
            let omega = self.body.angular_velocity;
            let roll_torque = self.spec.gyro_roll_scale * lean_of(rotation).cos() * momentum * omega.dot(up);
            let yaw_torque = -self.spec.gyro_yaw_scale * momentum * omega.dot(forward);
            self.body.apply_torque(forward * roll_torque + up * yaw_torque);
        }
        let velocity = self.body.linear_velocity;
        // `F = Cd (1 - s) v^2` against the velocity (`ride6-aero-drag.md`; the orientation tuning is 1 for every bike read),
        // and the rider's own, which the spring hands on to the frame (`ride6-rider-round6.md`, section 1).
        self.body.apply_force(-velocity * (velocity.length() * self.spec.drag * (1.0 - self.slipstream)));
        if !self.departure.off {
            self.body.apply_force(self.rider_drag_force());
        }
    }
}

#[path = "air.rs"]
mod air;
#[path = "chassis_state.rs"]
pub(super) mod chassis_state;
#[path = "fall.rs"]
mod fall;
#[path = "rough.rs"]
pub(super) mod rough;

#[cfg(test)]
#[path = "air_tests.rs"]
mod air_tests;
#[cfg(test)]
#[path = "chassis_tests.rs"]
mod chassis_tests;
#[cfg(test)]
#[path = "rough_tests.rs"]
mod rough_tests;
