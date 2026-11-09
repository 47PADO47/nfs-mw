//! A bike in the car's simulation (docs/specs/vehicle-two-wheel-integration.md).
//!
//! The game's walls, objects, pairs, camera and sounds all work on a four-wheel [`Vehicle`]. A bike keeps one as a body
//! double: it has the bike's mass and inertia, the collision box of the car's own `BOUNDS.BIN`, and it never drives (no
//! wheel, no engine, no forces of its own). The bike's own dynamics (`blackbox_vehicle::bike`) move the real body. Between
//! the halves of a step the pose and the velocities are carried from one body to the other, so every contact of the game
//! acts on the double and the bike feels it:
//!
//! 1. the start of a step takes what the rest of the game did to the double since the last one (`pull`);
//! 2. the bike integrates its forces, and the double follows (`begin_body`, `push`);
//! 3. the walls and the objects push the double around, and the bike takes the result (`finish_body`, `pull`);
//! 4. the bike works out the forces of the next step (`respond`) and the double is made equal to it again.
//!
//! The two bodies have different origins (the bike's is on the ground between its tyres, the double's is the centre of the
//! collision box) and share their centre of gravity, so a velocity is the same on both.

use blackbox_vehicle::bike::{AbsLevelSpec, Bike, BikeInput, FRONT, REAR};
use blackbox_vehicle::input::Controls;
use blackbox_vehicle::rigid_body::box_inertia;
use blackbox_vehicle::{Ground, Vehicle, VehicleSpec, WheelState};
use glam::{Mat3, Vec3};
use nfsmw_data::car::WheelPose;
use nfsmw_data::car::physics::CarPhysics;

use super::super::rider::RiderSample;
use super::bike_slide::{Slide, slide_of};
use super::step::StepStart;
use super::{CarSim, MAX_VISIBLE_TRAVEL, Telemetry};

/// The drift angle is 0 below this speed (m/s): the direction of a velocity that small means nothing. A modelling choice.
const DRIFT_MIN_SPEED: f32 = 2.0;

mod command;
mod contact;
mod lie;
mod steering;
use command::{command, signed_unit, unit};
use steering::arcade_steering;
pub use steering::set_bike_steering;

/// The two-wheel dynamics of a bike and what joins them to the four-wheel body double.
#[derive(Debug, Clone)]
pub(in crate::scenes::world) struct BikeRide {
    pub(super) bike: Bike,
    /// Where the double's origin is from the bike's, in the body frame (m).
    offset: Vec3,
    /// What the rider asked of the bike in the last step, in the form the car's readouts take.
    controls: Controls,
    /// The roll of the front and the rear wheel (rad).
    spin: [f32; 2],
    /// How long the rider has been down (s).
    down: f32,
    /// The bike has lain still long enough to be stood up and waits for its rider to be ready (`lie.rs`).
    waiting: bool,
    /// The rider has got up and the bike may be stood up. The host says so every step; true without a rider who gets up.
    rider_ready: bool,
    /// Whether the body double has the mass and the centre of gravity of the bike without its rider.
    double_alone: bool,
}

impl BikeRide {
    /// The ride of a bike's physics, and the double's spec fitted to it. `None` for a car.
    pub(super) fn new(physics: &mut CarPhysics) -> Option<Self> {
        let data = physics.bike.as_ref()?;
        let mut spec = (*data.spec).clone();
        spec.steering.arcade = arcade_steering();
        let bike = Bike::new(spec);
        let offset = physics.bounds.pivot - Vec3::new(0.0, 0.0, data.origin_x);
        let ride = Self {
            bike,
            offset,
            controls: Controls::default(),
            spin: [0.0; 2],
            down: 0.0,
            waiting: false,
            rider_ready: true,
            double_alone: false,
        };
        ride.fit(&mut physics.spec);
        Some(ride)
    }

    /// Give the double's spec the bike's mass and inertia, so a hit moves it as it moves the bike.
    pub(super) fn fit(&self, spec: &mut VehicleSpec) {
        let body = self.bike.body();
        spec.mass = body.mass();
        let box_moments = box_inertia(spec.mass, spec.dimension);
        let moments = body.inertia();
        spec.tensor_scale = Vec3::new(
            moments.x / box_moments.x.max(f32::EPSILON),
            moments.y / box_moments.y.max(f32::EPSILON),
            moments.z / box_moments.z.max(f32::EPSILON),
        );
    }

    /// Put the double's centre of gravity where the bike's is, and make it equal to the bike.
    pub(super) fn attach(&self, vehicle: &mut Vehicle) {
        vehicle.body_mut().set_cog(self.bike.body().cog() - self.offset);
        self.push(vehicle);
    }

    /// The double takes the bike's pose and velocities.
    pub(super) fn push(&self, vehicle: &mut Vehicle) {
        let from = self.bike.body();
        let rotation = from.rotation();
        let to = vehicle.body_mut();
        to.set_orientation(from.orientation);
        to.position = from.position + rotation * self.offset;
        to.linear_velocity = from.linear_velocity;
        to.angular_velocity = from.angular_velocity;
        to.state = from.state;
    }

    /// The bike takes the double's pose and velocities: what the contacts of the game did to it.
    pub(super) fn pull(&mut self, vehicle: &Vehicle) {
        let from = vehicle.body();
        let to = self.bike.body_mut();
        to.set_orientation(from.orientation);
        to.position = from.position - to.rotation() * self.offset;
        to.linear_velocity = from.linear_velocity;
        to.angular_velocity = from.angular_velocity;
        to.state = from.state;
    }

    /// Put the bike on the ground below `(x, z)`, facing `yaw` and already rolling at `speed`.
    pub(super) fn place(&mut self, ground: &dyn Ground, at: (f32, f32, f32), yaw: f32, speed: f32) -> bool {
        self.spin = [0.0; 2];
        self.down = 0.0;
        self.waiting = false;
        self.controls = Controls::default();
        self.bike.place_moving(ground, at.0, at.1, at.2, yaw, speed)
    }

    /// The first half of a step: the bike integrates the forces of the last one, and the double follows.
    pub(super) fn begin_body(&mut self, vehicle: &mut Vehicle, dt: f32) {
        // The double's ground contacts are those of this step only (the car's own begin does this).
        vehicle.clear_body_contacts();
        self.sync_double(vehicle);
        self.bike.begin_body(dt);
        self.push(vehicle);
    }

    /// The double weighs what the bike does: with the rider (the spec's mass) or, from the moment he leaves it until the bike is
    /// placed again, without him, and its centre of gravity is the bike's.
    pub(super) fn sync_double(&mut self, vehicle: &mut Vehicle) {
        let alone = self.bike.rider_off();
        if alone == self.double_alone {
            return;
        }
        self.double_alone = alone;
        vehicle.body_mut().set_mass(self.bike.body().mass());
        self.attach(vehicle);
    }

    /// The end of the contacts: a fallen bike lies on its box (the double meets the ground) and takes the pose the contacts left; a
    /// bike with its rider takes what they did to it as `contact.rs` says.
    pub(super) fn finish_body(&mut self, vehicle: &mut Vehicle, dt: f32, ground: &dyn Ground) {
        if self.bike.fallen() {
            vehicle.finish_body_step(dt, ground);
            self.pull(vehicle);
            return;
        }
        self.take_contacts(vehicle);
        self.push(vehicle);
    }

    /// The second half of a step: the rider, the engine, the tyres and the suspension make the forces of the next step.
    pub(super) fn respond(&mut self, vehicle: &mut Vehicle, start: &StepStart, ground: &dyn Ground) {
        let input = &start.input;
        self.controls = Controls {
            gas: unit(input.throttle),
            brake: unit(input.brake),
            handbrake: unit(input.handbrake),
            steering: signed_unit(input.steer),
            nos: input.nos,
            ..Controls::default()
        };
        // On the grid of a race the rider holds the bike with both brakes until the start.
        let rider = match vehicle.staging() {
            true => BikeInput { throttle: 0.0, front_brake: 1.0, rear_brake: 1.0, steer: 0.0 },
            false => command(input, self.bike.spec().steering.arcade.stick_reach),
        };
        self.bike.set_weight(start.weight);
        self.bike.set_air_control(start.air);
        self.bike.respond(start.delta, &rider, ground);
        match self.bike.fallen() {
            true => self.lie(start.delta, ground),
            false => {
                self.down = 0.0;
                self.waiting = false;
            }
        }
        self.sync_double(vehicle);
        self.push(vehicle);
    }

    /// The wheels roll on.
    pub(super) fn advance(&mut self, dt: f32) {
        for index in [FRONT, REAR] {
            self.spin[index] += self.bike.wheel(index).spin * dt;
        }
    }

    /// Whether the bike's state is usable (no NaN or runaway values).
    pub(super) fn is_finite(&self) -> bool {
        self.bike.position().is_finite()
            && self.bike.orientation().is_finite()
            && self.bike.velocity().is_finite()
            && self.bike.rpm().is_finite()
            && self.bike.steer_angle().is_finite()
    }

    pub(super) fn controls(&self) -> &Controls {
        &self.controls
    }

    /// The wheels as a model with four corners poses them: the front wheel in 0, the rear wheel in 2. Corner 1 is the
    /// handlebar group, which steers as the front wheel does (its mount ignores the travel); nothing is on 3. The model steers
    /// to the left and the dynamics to the right.
    pub(super) fn wheel_poses(&self) -> [WheelPose; 4] {
        let pose = |index: usize| WheelPose {
            steer: match index {
                FRONT => -self.bike.steer_angle(),
                _ => 0.0,
            },
            spin: self.spin[index],
            travel: self.bike.wheel(index).compression.clamp(-MAX_VISIBLE_TRAVEL, MAX_VISIBLE_TRAVEL),
        };
        [pose(FRONT), pose(FRONT), pose(REAR), WheelPose::default()]
    }

    /// What the rider's animation needs of the bike (`drive/rider/params.rs`): the inputs the reference hands its animation
    /// blueprint (`ride6-rider-round6.md`, section 4). The wheelie and the stoppie are the height of the nose, the sine of the
    /// pitch, while one tyre is up and the other is down.
    pub(super) fn rider_sample(&self) -> RiderSample {
        let spec = self.bike.spec();
        let nose = (self.bike.orientation() * Vec3::Z).y.clamp(-1.0, 1.0);
        let body = self.bike.rider_rotation();
        let max_lean = spec.steering.max_lean.max(f32::EPSILON);
        RiderSample {
            // The rider's lean is the bike's and his body's own against it.
            lean: self.bike.lean() + body.roll_right(),
            max_lean: spec.steering.max_lean,
            // The blueprint's `BikeLean` (the roll) and `RiderLean` (the controller's target lean) over the steepest lean.
            bike_lean: (self.bike.lean() / max_lean).clamp(-1.0, 1.0),
            rider_lean: (self.bike.target_lean() / max_lean).clamp(-1.0, 1.0),
            elevation: nose,
            tuck_level: self.bike.tuck_level(),
            speed: self.bike.forward_speed(),
            throttle: self.controls.gas,
            front_brake: self.controls.brake,
            rear_brake: self.controls.brake.max(self.controls.handbrake),
            pitch_up: nose.asin(),
            front_down: self.bike.wheel(FRONT).on_ground,
            rear_down: self.bike.wheel(REAR).on_ground,
            offset: self.bike.rider_offset(),
            rotation: Vec3::new(body.roll_right(), body.pitch_up(), body.yaw_right()),
            weight: self.bike.rider_weight(),
            steer: self.controls.steering,
            drift_deg: self.drift_angle_deg(),
            clutch: self.bike.clutch_pulled(),
        }
    }

    /// The angle between the bike's path and its heading, degrees, positive when the heading is to the right of the path
    /// (the blueprint's `VehicleDriftAngleDeg`); 0 below walking pace, where the direction of the velocity means nothing.
    fn drift_angle_deg(&self) -> f32 {
        let velocity = self.bike.velocity();
        let orientation = self.bike.orientation();
        let (forward, right) = (velocity.dot(orientation * Vec3::Z), velocity.dot(orientation * Vec3::X));
        match velocity.length() > DRIFT_MIN_SPEED {
            true => (-right).atan2(forward).to_degrees(),
            false => 0.0,
        }
    }

    /// The width of the mark library wheel `index` leaves (m).
    pub(super) fn mark_width(&self, index: usize) -> f32 {
        super::bike_slide::mark_width(&self.bike, index)
    }

    /// The bike wheel behind library wheel `index` (0 front left, 1 front right, 2 rear left, 3 rear right).
    fn wheel_of(index: usize) -> usize {
        match index {
            0 | 1 => FRONT,
            _ => REAR,
        }
    }

    /// Library wheel `index` as the car's readouts take it. The wheels the bike does not have repeat the ones it has.
    pub(super) fn wheel_state(&self, index: usize) -> WheelState {
        let which = Self::wheel_of(index);
        let wheel = self.bike.wheel(which);
        let spec = self.bike.spec();
        let (radius, steer_angle) = match which {
            FRONT => (spec.front.radius, self.bike.steer_angle()),
            _ => (spec.rear.radius, 0.0),
        };
        let rotation = Mat3::from_quat(self.bike.orientation());
        // What slides is the tyre's own slide past the peak of its force curve, not the slip speeds, which grow with the speed
        // of the bike (`bike_slide.rs`). The sideways slide is the slip angle against the speed along the bike, at least 1 m/s
        // (the tyre model's own regularisation).
        let along = self.bike.forward_speed().abs().max(1.0);
        let Slide { slip, lateral_speed, speed: slide_speed, skid, smoke } = slide_of(wheel, along);
        let position = match wheel.on_ground {
            true => wheel.contact,
            false => self.bike.wheel_pose(which).0 - Vec3::Y * radius,
        };
        WheelState {
            position,
            radius,
            steer_angle,
            angular_velocity: wheel.spin,
            simulated_angular_velocity: wheel.spin,
            compression: wheel.compression,
            on_ground: wheel.on_ground,
            load: wheel.load,
            slip,
            lateral_speed,
            slip_angle: wheel.slip_angle,
            traction: 1.0 - skid,
            locked: wheel.on_ground && self.bike.forward_speed().abs() > 2.0 && wheel.spin.abs() * radius < 0.1,
            lateral_force: wheel.force.dot(rotation.x_axis),
            longitudinal_force: wheel.force.dot(rotation.z_axis),
            slide_speed,
            skid,
            smoke,
        }
    }

    /// The surface under library wheel `index`; `None` for a wheel off the ground.
    pub(super) fn wheel_surface(&self, index: usize) -> Option<u32> {
        let wheel = self.bike.wheel(Self::wheel_of(index));
        wheel.on_ground.then_some(wheel.surface_tag)
    }

    /// Where library wheel `index` met the road and the normal there. A bike has two wheels, reported as 0 and 2.
    pub(super) fn wheel_hit(&self, index: usize) -> Option<(Vec3, Vec3)> {
        if index == 1 || index == 3 {
            return None;
        }
        let wheel = self.bike.wheel(Self::wheel_of(index));
        wheel.on_ground.then_some((wheel.contact, wheel.normal))
    }

    /// The readout of the bike. `speed` is the body's speed along the bike.
    pub(super) fn telemetry(&self, speed: f32) -> Telemetry {
        let engine = &self.bike.spec().engine;
        Telemetry {
            speed_mps: speed,
            rpm: self.bike.rpm(),
            gear: i32::try_from(self.bike.gear()).unwrap_or(i32::MAX),
            nos: 0.0,
            nos_burning: false,
            has_nos: false,
            wheels_on_ground: self.wheels_down(),
            red_line: engine.max_rpm,
            max_rpm: engine.max_rpm,
            idle: engine.idle_rpm,
            shifting: self.bike.shifting(),
            shift_up: false,
            has_induction: false,
            boost_psi: 0.0,
        }
    }

    /// The wheels on the ground as a four-wheel count: each wheel of the bike stands for two, so both down is 4 (a car with
    /// every wheel down) and one down is 2.
    pub(super) fn wheels_down(&self) -> usize {
        2 * self.bike.wheels_on_ground()
    }

    /// The engine's rev limit (rpm).
    pub(super) fn engine_spec_max_rpm(&self) -> f32 {
        self.bike.spec().engine.max_rpm
    }
}

impl CarSim {
    /// The dot product of the car's up vector with the world's. A bike under its rider is never "tipped over": it rides leaned to
    /// 50 or 60 degrees, which is a car on its side to the sound and the police speech; only a fallen bike has its real value.
    pub fn up_dot(&self) -> f32 {
        match self.ride.as_deref() {
            Some(ride) if !ride.bike.fallen() => 1.0,
            _ => self.vehicle.rotation().y_axis.y,
        }
    }

    /// Whether the car is a bike.
    pub fn is_bike(&self) -> bool {
        self.ride.is_some()
    }

    /// The rider switches his ABS to the next lower level (1, 0.75, 0.5, 0.25, 0, then 1 again) and the new level is returned;
    /// `None` for a car, which has no such setting.
    pub fn cycle_abs_level(&mut self) -> Option<f32> {
        let ride = self.ride.as_mut()?;
        let next = AbsLevelSpec::cycle(ride.bike.abs_level());
        ride.bike.set_abs_level(next);
        Some(ride.bike.abs_level())
    }

    /// Whether the bike is in a burnout (the throttle and the front brake held with it standing or crawling); false for a car.
    pub fn bike_burnout(&self) -> bool {
        self.ride.as_ref().is_some_and(|ride| ride.bike.burnout())
    }

    /// The bike's state for its rider's animation; `None` for a car.
    pub fn rider_sample(&self) -> Option<RiderSample> {
        self.ride.as_ref().map(|ride| ride.rider_sample())
    }

    /// The state of library wheel `index` (0 front left, 1 front right, 2 rear left, 3 rear right). A bike repeats its front
    /// wheel as 0 and 1 and its rear wheel as 2 and 3.
    pub(super) fn wheel_state(&self, index: usize) -> WheelState {
        match &self.ride {
            Some(ride) => ride.wheel_state(index),
            None => self.vehicle.wheel(index),
        }
    }

    /// The surface under library wheel `index`, `None` for a wheel with no ground within reach.
    pub(super) fn wheel_surface(&self, index: usize) -> Option<u32> {
        match &self.ride {
            Some(ride) => ride.wheel_surface(index),
            None => self.vehicle.wheel_surface_tag(index),
        }
    }

    /// Where library wheel `index`'s ground ray met the road. A bike has hits for wheels 0 and 2 only.
    pub(super) fn wheel_hit(&self, index: usize) -> Option<(Vec3, Vec3)> {
        match &self.ride {
            Some(ride) => ride.wheel_hit(index),
            None => self.vehicle.wheel_ground_hit(index),
        }
    }

    /// How many wheels are on the ground, counted on four: a bike counts each of its wheels twice.
    pub(super) fn wheels_down(&self) -> usize {
        match &self.ride {
            Some(ride) => ride.wheels_down(),
            None => self.vehicle.wheels_on_ground(),
        }
    }

    /// What the driver asked of the car in the last step, shaped.
    pub(super) fn controls(&self) -> &Controls {
        match &self.ride {
            Some(ride) => ride.controls(),
            None => self.vehicle.controls(),
        }
    }

    /// The engine's rev limit, rpm.
    pub(super) fn max_rpm(&self) -> f32 {
        match &self.ride {
            Some(ride) => ride.engine_spec_max_rpm(),
            None => self.vehicle.powertrain().engine_spec().max_rpm,
        }
    }

    /// The engine speed now, rpm.
    pub(super) fn engine_rpm(&self) -> f32 {
        match &self.ride {
            Some(ride) => ride.bike.rpm(),
            None => self.vehicle.rpm(),
        }
    }

    /// The start of a step takes what the rest of the game did to a bike's body double since the last one.
    pub(super) fn pull_bike(&mut self) {
        let player = self.collision_role.player;
        if let Some(ride) = self.ride.as_deref_mut() {
            ride.pull(&self.vehicle);
            ride.bike.set_throttle_wheelie(crate::scenes::world::bike_assist::throttle_wheelie(player));
        }
    }
}

#[cfg(test)]
#[path = "bike_ride_tests.rs"]
mod tests;
