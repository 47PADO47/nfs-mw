//! Two-wheeled vehicle dynamics: a rigid body on a fork and a swingarm, two round tyres with the Magic Formula 94, an
//! engine with an automatic gearbox, and a rider's balance that leans the bike to where the stick points.
//!
//! The model is described in `docs/specs/vehicle-two-wheel.md`. It is a full rigid body in the physics space of the crate
//! (x right, y up, z forward), so the lean, the weight transfer under braking and acceleration, and a slide of either
//! wheel come out of the forces rather than being scripted.
//!
//! ```
//! use blackbox_vehicle::bike::{Bike, BikeInput, BikeSpec};
//! use blackbox_vehicle::{FIXED_STEP, FlatGround};
//!
//! let ground = FlatGround::new(0.0);
//! let mut bike = Bike::new(BikeSpec::example());
//! bike.place_on_ground(&ground, 0.0, 0.0, 2.0, 0.0);
//! let input = BikeInput { throttle: 1.0, ..Default::default() };
//! for _ in 0..600 {
//!     bike.step(FIXED_STEP, &input, &ground);
//! }
//! assert!(bike.forward_speed() > 20.0);
//! assert!(bike.lean().abs() < 0.05);
//! ```

mod controller;
mod geometry;
mod magic;
mod powertrain;
mod spec;
mod suspension;
mod tyre;
mod wheel;

pub use magic::Mf94;
pub use spec::{
    BikeSpec, BrakeSpec, ChassisSpec, DamperSpec, EngineSpec, SteeringSpec, SuspensionSpec, TransmissionSpec, WheelSpec,
};
pub use tyre::{TyreInput, TyreOutput};
pub use wheel::WheelState;

use glam::{Quat, Vec3};

use crate::ground::Ground;
use crate::rigid_body::{RigidBody, RigidBodySpec};
use controller::Balance;
use geometry::Geometry;
use powertrain::Powertrain;
use suspension::Suspension;

/// Front wheel first.
pub const FRONT: usize = 0;
pub const REAR: usize = 1;

/// A lean beyond this (rad) is a fall: the rider lets go.
const FALL_LEAN: f32 = 1.4;
/// Speeds below this (m/s) the rider's feet hold the bike upright.
const STAND_SPEED: f32 = 2.5;
/// How fast the rider's feet pull the lean back (rad/s).
const STAND_RESPONSE: f32 = 4.0;

/// What the rider asks of the bike, each 0 to 1 except the stick, which is -1 (left) to 1 (right).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BikeInput {
    pub throttle: f32,
    pub front_brake: f32,
    pub rear_brake: f32,
    pub steer: f32,
}

/// A bike.
#[derive(Clone, Debug)]
pub struct Bike {
    spec: BikeSpec,
    body: RigidBody,
    geometry: Geometry,
    suspension: [Suspension; 2],
    powertrain: Powertrain,
    wheels: [WheelState; 2],
    steer: f32,
    steer_rate: f32,
    fallen: bool,
}

impl Bike {
    pub fn new(spec: BikeSpec) -> Self {
        let chassis = &spec.chassis;
        let geometry = Geometry::new(chassis, &spec.front, &spec.rear);
        let mass = spec.total_mass();
        let gravity = -9.81;
        let cg_z = chassis.wheelbase * (chassis.front_weight - 0.5);
        let body_spec = RigidBodySpec {
            gravity,
            cg: Vec3::new(0.0, chassis.cg_height, cg_z),
            drag: Vec3::new(0.0, 0.0, spec.drag),
            ..RigidBodySpec::default()
        };
        let body = RigidBody::new(mass, chassis.half_dimensions, chassis.tensor_scale, body_spec);
        let front_load = chassis.front_weight * mass * -gravity - spec.front.mass * -gravity;
        let rear_load = (1.0 - chassis.front_weight) * mass * -gravity - spec.rear.mass * -gravity;
        let suspension = [
            Suspension::new(spec.front_suspension.clone(), front_load.max(0.0) * geometry.axis.y),
            Suspension::new(spec.rear_suspension.clone(), rear_load.max(0.0)),
        ];
        let powertrain = Powertrain::new(&spec.engine);
        Self {
            spec,
            body,
            geometry,
            suspension,
            powertrain,
            wheels: [WheelState::default(), WheelState::default()],
            steer: 0.0,
            steer_rate: 0.0,
            fallen: false,
        }
    }

    pub fn spec(&self) -> &BikeSpec {
        &self.spec
    }

    /// Stand the bike on the ground below `(x, z)`, facing `yaw`; the search starts at height `top`. False if there is no
    /// ground.
    pub fn place_on_ground(&mut self, ground: &dyn Ground, x: f32, z: f32, top: f32, yaw: f32) -> bool {
        self.place_moving(ground, x, z, top, yaw, 0.0)
    }

    /// Like [`Bike::place_on_ground`], already rolling at `speed` m/s with the gear matched to it.
    pub fn place_moving(&mut self, ground: &dyn Ground, x: f32, z: f32, top: f32, yaw: f32, speed: f32) -> bool {
        let Some(hit) = ground.hit(Vec3::new(x, top, z), Vec3::NEG_Y, 100.0) else { return false };
        let orientation = Quat::from_rotation_y(yaw);
        self.body.place(Vec3::new(x, top - hit.distance, z), orientation);
        self.body.linear_velocity = orientation * Vec3::new(0.0, 0.0, speed);
        self.body.angular_velocity = Vec3::ZERO;
        self.wheels = [WheelState::default(), WheelState::default()];
        let spin = speed / self.spec.rear.radius;
        self.wheels[FRONT].spin = speed / self.spec.front.radius;
        self.wheels[REAR].spin = spin;
        self.steer = 0.0;
        self.steer_rate = 0.0;
        self.fallen = false;
        self.powertrain = Powertrain::new(&self.spec.engine);
        self.powertrain.match_speed(&self.spec.engine, &self.spec.transmission, spin);
        true
    }

    /// Advance the bike by `dt` seconds.
    pub fn step(&mut self, dt: f32, input: &BikeInput, ground: &dyn Ground) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.body.begin_frame(dt);
        let rotation = self.body.rotation();
        let forward = rotation * Vec3::Z;
        let speed = self.body.linear_velocity.dot(forward);
        let lean = lean_of(rotation);
        self.fallen |= lean.abs() > FALL_LEAN;
        let stick = if self.fallen { 0.0 } else { input.steer };
        self.steer_handlebar(dt, speed, lean, -self.body.angular_velocity.dot(forward), stick);
        let drive = self.drive_torque(input, dt);
        for index in [FRONT, REAR] {
            self.step_wheel(index, dt, input, drive, ground);
        }
        self.hold_upright(speed, lean);
        self.body.apply_drag();
    }

    /// The handlebar's response to the balance controller.
    fn steer_handlebar(&mut self, dt: f32, speed: f32, lean: f32, lean_rate: f32, stick: f32) {
        let steering = self.spec.steering;
        let chassis = &self.spec.chassis;
        let balance = Balance {
            speed,
            lean,
            lean_rate,
            mass: self.body.mass(),
            cg_height: chassis.cg_height,
            wheelbase: chassis.wheelbase,
            roll_inertia: self.body.inertia().z,
            gravity: 9.81,
        };
        let grip = self.spec.front.lateral_grip_limit.min(self.spec.rear.lateral_grip_limit);
        let target = controller::target_lean(&steering, stick, grip);
        let command = controller::handlebar(&steering, &balance, target, stick);
        let acceleration = steering.response * (command - self.steer) - steering.damping * self.steer_rate;
        self.steer_rate += acceleration * dt;
        self.steer = (self.steer + self.steer_rate * dt).clamp(-steering.max_angle, steering.max_angle);
    }

    /// The rider's feet: a torque about the bike's length that holds it up when it is too slow to be balanced by steering.
    fn hold_upright(&mut self, speed: f32, lean: f32) {
        let share = (1.0 - speed.abs() / STAND_SPEED).clamp(0.0, 1.0);
        if share == 0.0 || self.fallen {
            return;
        }
        let forward = self.body.rotation() * Vec3::Z;
        let tip = self.body.mass() * 9.81 * self.spec.chassis.cg_height * lean.sin();
        let rate = -self.body.angular_velocity.dot(forward);
        let height = self.spec.chassis.cg_height;
        let inertia = self.body.inertia().z + self.body.mass() * height * height;
        // Cancel the tipping, then pull the lean back with a critically damped response of its own.
        let hold = inertia * (STAND_RESPONSE * STAND_RESPONSE * lean + 2.0 * STAND_RESPONSE * rate);
        self.body.apply_torque(forward * (share * (tip + hold)));
    }

    fn drive_torque(&mut self, input: &BikeInput, dt: f32) -> f32 {
        let throttle = if self.fallen { 0.0 } else { input.throttle.clamp(0.0, 1.0) };
        let spin = self.wheels[REAR].spin;
        self.powertrain.step(&self.spec.engine, &self.spec.transmission, throttle, spin, dt)
    }

    /// The pose of a wheel for a suspension compression: the hub in the world, the unit axle and heading, and the
    /// suspension axis.
    fn pose(&self, index: usize, compression: f32) -> wheel::Pose {
        let rotation = self.body.rotation();
        let (hub, turn) = match index {
            FRONT => self.geometry.front_pose(self.steer, compression),
            _ => self.geometry.rear_pose(compression),
        };
        let axis = if index == FRONT { self.geometry.axis } else { Vec3::Y };
        wheel::Pose {
            hub: self.body.position + rotation * hub,
            axle: rotation * (turn * Vec3::X),
            heading: rotation * (turn * Vec3::Z),
            axis: rotation * axis,
        }
    }

    fn step_wheel(&mut self, index: usize, dt: f32, input: &BikeInput, drive: f32, ground: &dyn Ground) {
        let wheel_spec = if index == FRONT { &self.spec.front } else { &self.spec.rear };
        let travel = &self.suspension[index];
        let limits = (travel.rest_extension(), travel.rest_compression() + wheel::OVER_TRAVEL);
        let contact =
            wheel::settle(|c| self.pose(index, c), wheel_spec, ground, self.wheels[index].compression, limits);
        let spin = self.wheels[index].spin;
        let brake = self.brake_torque(index, input);
        let drive = if index == REAR { drive } else { 0.0 };
        let state = &mut self.wheels[index];
        let Some(contact) = contact else {
            state.release(limits.0);
            state.spin = tyre::spin_in_air(wheel_spec, spin, drive, brake, dt);
            return;
        };
        state.compression = contact.compression;
        let velocity = self.body.point_velocity(contact.point);
        // The frame moves towards the ground at the rate the suspension compresses.
        let speed = -velocity.dot(contact.normal) / contact.axis_cosine.max(0.3);
        let weight = wheel_spec.mass * 9.81;
        let cosine = contact.axis_cosine.max(0.3);
        let mass = self.axial_mass(contact.point, contact.normal) * cosine * cosine;
        let suspension_force = self.suspension[index].force(contact.compression, speed, dt, mass);
        let load = (suspension_force / contact.axis_cosine.max(0.3) + weight).max(0.0);
        let frame = wheel::ground_frame(contact.normal, contact.heading);
        let camber = geometry::camber(contact.axle, contact.normal);
        let input = TyreInput {
            load,
            v_long: velocity.dot(frame.forward),
            v_lat: velocity.dot(frame.right),
            spin,
            camber,
            grip: contact.surface,
        };
        let (new_spin, output) = tyre::advance_wheel(wheel_spec, &input, drive, brake, dt);
        let rolling = tyre::rolling_resistance(wheel_spec, load, input.v_long);
        let force =
            contact.normal * load + frame.forward * (output.longitudinal + rolling) + frame.right * output.lateral;
        self.body.apply_force_at(force, contact.point);
        let state = &mut self.wheels[index];
        state.on_ground = true;
        state.load = load;
        state.contact = contact.point;
        state.normal = contact.normal;
        state.force = force;
        state.slip_angle = output.slip_angle;
        state.slip_ratio = output.slip_ratio;
        state.camber = camber;
        state.surface_tag = contact.surface.tag;
        state.spin = new_spin;
    }

    /// The mass the body presents to a push along `direction` at the world point `point` (kg): the whole mass for a push
    /// through the centre of gravity, less for one that also turns the body.
    fn axial_mass(&self, point: Vec3, direction: Vec3) -> f32 {
        let lever = (point - self.body.world_cog()).cross(direction);
        let turning = lever.dot(self.body.inv_world_tensor() * lever);
        1.0 / (1.0 / self.body.mass() + turning)
    }

    /// The brake torque on a wheel (N m): the share of the brake force the wheel's lever takes, times its radius.
    fn brake_torque(&self, index: usize, input: &BikeInput) -> f32 {
        let brakes = self.spec.brakes;
        let (lever, share, radius) = match index {
            FRONT => (input.front_brake, brakes.front_share, self.spec.front.radius),
            _ => (input.rear_brake, 1.0 - brakes.front_share, self.spec.rear.radius),
        };
        lever.clamp(0.0, 1.0) * brakes.force * share * radius
    }

    pub fn position(&self) -> Vec3 {
        self.body.position
    }

    pub fn orientation(&self) -> Quat {
        self.body.orientation
    }

    pub fn velocity(&self) -> Vec3 {
        self.body.linear_velocity
    }

    /// Speed along the bike (m/s).
    pub fn forward_speed(&self) -> f32 {
        self.body.linear_velocity.dot(self.body.rotation() * Vec3::Z)
    }

    /// The lean from upright (rad), positive when the bike leans right.
    pub fn lean(&self) -> f32 {
        lean_of(self.body.rotation())
    }

    /// The handlebar angle (rad), positive right.
    pub fn steer_angle(&self) -> f32 {
        self.steer
    }

    pub fn rpm(&self) -> f32 {
        self.powertrain.rpm()
    }

    pub fn gear(&self) -> usize {
        self.powertrain.gear()
    }

    pub fn wheel(&self, index: usize) -> &WheelState {
        &self.wheels[index.min(REAR)]
    }

    pub fn wheels_on_ground(&self) -> usize {
        self.wheels.iter().filter(|w| w.on_ground).count()
    }

    /// Whether the rider has let go.
    pub fn fallen(&self) -> bool {
        self.fallen
    }

    /// The pose of the front wheel assembly now, for drawing: the hub in the world and its orientation.
    pub fn wheel_pose(&self, index: usize) -> (Vec3, Quat) {
        let pose = self.pose(index, self.wheels[index.min(REAR)].compression);
        let rotation = self.body.rotation();
        let local = rotation.transpose();
        let (_, turn) = match index {
            FRONT => self.geometry.front_pose(self.steer, 0.0),
            _ => self.geometry.rear_pose(0.0),
        };
        let _ = local;
        (pose.hub, self.body.orientation * turn)
    }
}

/// The lean (rad) of a body with orientation `rotation`: positive when its top leans to its right. The world's up, seen
/// from a body that leans right, points to the body's left.
fn lean_of(rotation: glam::Mat3) -> f32 {
    let up = rotation.transpose() * Vec3::Y;
    (-up.x).atan2(up.y)
}

#[cfg(test)]
mod tests;
