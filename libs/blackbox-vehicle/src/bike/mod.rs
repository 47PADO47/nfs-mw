//! Two-wheeled vehicle dynamics: a rigid body on a fork and a swingarm, two round tyres with the Magic Formula 94, an
//! engine with a clutch and an automatic gearbox, and a rider who prescribes the lean and steers the front tyre to
//! the force that lean needs.
//!
//! The model is described in `docs/specs/vehicle-two-wheel.md`. It is a full rigid body in the physics space of the crate
//! (x right, y up, z forward), so the weight transfer under braking and acceleration, and a slide of either wheel come out of
//! the forces; only the roll of the bike is prescribed (and realised by a bounded torque), as the rider's controller does.
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

mod brakes;
pub mod choices;
mod curve;
mod example;
mod filter;
mod frame;
mod geometry;
mod handling;
mod lean;
mod magic;
mod peaks;
mod pitch_assist;
mod powertrain;
mod rider;
mod spec;
mod spec_parts;
mod suspension;
mod tyre;
mod wheel;
mod wheel_step;

pub use frame::RestPose;
pub use magic::Mf94;
pub use spec::{
    AbsSpec, BikeSpec, BrakeSpec, ChassisSpec, Coupling, DamperSpec, EngineSpec, FallSpec, Hydraulic, LeanSpec,
    PitchParams, Progressive, SteeringSpec, SuspensionSpec, TransmissionSpec, WheelSpec, WheelieSpec,
};
pub use tyre::{TyreInput, TyreOutput};
pub use wheel::WheelState;

use glam::{Quat, Vec3};

use crate::ground::Ground;
use crate::rigid_body::{RigidBody, RigidBodySpec, box_inertia};
use brakes::AntiLock;
use filter::SecondOrder;
use geometry::Geometry;
use lean::{Lean, LeanPlan};
use powertrain::{DriveInput, Powertrain};
use rider::RiderWeight;
use suspension::Suspension;

/// Front wheel first.
pub const FRONT: usize = 0;
pub const REAR: usize = 1;

/// What the rider asks of the bike, each 0 to 1 except the stick, which is -1 (left) to 1 (right). The rider's weight
/// (back to forward) is set with [`Bike::set_weight`].
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
    /// The handlebar angle, through its filter.
    steer: SecondOrder,
    lean: Lean,
    plan: LeanPlan,
    weight: RiderWeight,
    weight_input: f32,
    antilock: AntiLock,
    fallen: bool,
    /// The handlebar angle the controller aimed at in the last step (for the rider's lead).
    aim_before: Option<f32>,
}

impl Bike {
    pub fn new(spec: BikeSpec) -> Self {
        let chassis = &spec.chassis;
        let geometry = Geometry::new(chassis, &spec.front, &spec.rear);
        let mass = spec.total_mass();
        let gravity = -9.81;
        let body_spec = RigidBodySpec { gravity, cg: rider::combined_cog(chassis, 0.0), ..RigidBodySpec::default() };
        // The inertia is the radius of gyration squared times the total mass; the body takes it as a scale of its box.
        let half = chassis.half_dimensions.max(Vec3::splat(0.001));
        let box_moments = box_inertia(mass.max(f32::EPSILON), half);
        let wanted = chassis.tensor_scale * mass;
        let scale = Vec3::new(
            wanted.x / box_moments.x.max(f32::EPSILON),
            wanted.y / box_moments.y.max(f32::EPSILON),
            wanted.z / box_moments.z.max(f32::EPSILON),
        );
        let body = RigidBody::new(mass, half, scale, body_spec);
        let share = spec.front_share();
        let front_load = (share * mass - spec.front.mass) * -gravity;
        let rear_load = ((1.0 - share) * mass - spec.rear.mass) * -gravity;
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
            steer: SecondOrder::new(0.0),
            lean: Lean::new(),
            plan: LeanPlan::default(),
            weight: RiderWeight::new(),
            weight_input: 0.0,
            antilock: AntiLock::default(),
            fallen: false,
            aim_before: None,
        }
    }

    pub fn spec(&self) -> &BikeSpec {
        &self.spec
    }

    /// The rigid body that carries the bike (frame, engine, wheels and rider together). Its origin is on the ground midway
    /// between the contact patches at rest; its centre of gravity is that of the bike and the rider, and moves with the
    /// rider's weight.
    pub fn body(&self) -> &RigidBody {
        &self.body
    }

    /// The rigid body, for a host that moves the bike by contacts of its own (a wall, another body).
    pub fn body_mut(&mut self) -> &mut RigidBody {
        &mut self.body
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
        self.body.set_cog(rider::combined_cog(&self.spec.chassis, 0.0));
        self.body.linear_velocity = orientation * Vec3::new(0.0, 0.0, speed);
        self.body.angular_velocity = Vec3::ZERO;
        self.wheels = [WheelState::default(), WheelState::default()];
        let spin = speed / self.spec.rear.radius;
        self.wheels[FRONT].spin = speed / self.spec.front.radius;
        self.wheels[REAR].spin = spin;
        self.steer.reset(0.0);
        self.aim_before = None;
        self.lean.reset();
        self.plan = LeanPlan::default();
        self.weight.reset();
        self.antilock.reset();
        self.fallen = false;
        self.powertrain = Powertrain::new(&self.spec.engine);
        self.powertrain.match_speed(&self.spec.engine, &self.spec.transmission, spin);
        true
    }

    /// The rider's weight on the bike, -1 (fully back) to 1 (fully forward). It moves the combined centre of gravity and
    /// asks the wheelie and stoppie control for a pitch.
    pub fn set_weight(&mut self, weight: f32) {
        self.weight_input = match weight.is_finite() {
            true => weight.clamp(-1.0, 1.0),
            false => 0.0,
        };
    }

    /// Advance the bike by `dt` seconds: [`Bike::begin_body`], then [`Bike::respond`].
    pub fn step(&mut self, dt: f32, input: &BikeInput, ground: &dyn Ground) {
        self.begin_body(dt);
        self.respond(dt, input, ground);
    }

    /// The first half of a step: integrate the forces of the previous step into the body. A host that resolves contacts of
    /// its own (walls, other bodies) between the two halves does so on [`Bike::body_mut`].
    pub fn begin_body(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.body.begin_frame(dt);
    }

    /// The second half of a step: the rider, the engine, the tyres and the suspension work out the forces that move the
    /// body in the next step.
    pub fn respond(&mut self, dt: f32, input: &BikeInput, ground: &dyn Ground) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let rotation = self.body.rotation();
        let forward = rotation * Vec3::Z;
        let speed = self.body.linear_velocity.dot(forward);
        self.check_fall(speed);
        self.shift_weight(dt);
        let (throttle_help, brake_help) = self.help_factors();
        let throttle = match self.fallen {
            true => 0.0,
            false => input.throttle.clamp(0.0, 1.0) * throttle_help,
        };
        let brakes = self.spec.brakes;
        let (front_lever, rear_lever) = brakes::joined(
            &brakes,
            input.front_brake.clamp(0.0, 1.0) * brake_help,
            input.rear_brake,
            speed.abs() > choices::STANDING_SPEED,
        );
        let (abs_front, abs_rear) = self.antilock.multipliers();
        let brake = [
            brakes::torque(&brakes, true, front_lever, abs_front, self.spec.front.radius),
            brakes::torque(&brakes, false, rear_lever, abs_rear, self.spec.rear.radius),
        ];
        let stick = match self.fallen {
            true => 0.0,
            false => input.steer,
        };
        self.steer_handlebar(dt, speed, stick);
        let drive = self.powertrain.drive(
            &self.spec.engine,
            &self.spec.transmission,
            &DriveInput { throttle, wheel_spin: self.wheels[REAR].spin, speed: speed.abs(), dt },
        );
        for index in [FRONT, REAR] {
            let applied = match index {
                REAR => drive,
                _ => powertrain::Drive::default(),
            };
            self.step_wheel(index, dt, brake[index], applied, ground);
        }
        self.powertrain.finish(&self.spec.engine, self.wheels[REAR].spin, dt);
        let normalised = |w: &WheelState| match w.on_ground {
            true => w.normalised_x,
            false => 0.0,
        };
        let (front_slip, rear_slip) = (normalised(&self.wheels[FRONT]), normalised(&self.wheels[REAR]));
        self.antilock.update(&brakes.abs, speed, front_slip, rear_slip, dt);
        self.apply_rider_and_air(input, speed);
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

    /// The pitch (rad), positive when the nose is up.
    pub fn pitch(&self) -> f32 {
        pitch_of(self.body.rotation())
    }

    /// The lean limit the rider's controller used in the last step, and the target lean it aimed at (rad, positive right).
    pub fn lean_limit(&self) -> f32 {
        self.plan.limit
    }

    pub fn target_lean(&self) -> f32 {
        self.plan.target
    }

    /// The handlebar angle (rad), positive right.
    pub fn steer_angle(&self) -> f32 {
        self.steer.output()
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

    /// The pose of a wheel assembly now, for drawing: the hub in the world and its orientation.
    pub fn wheel_pose(&self, index: usize) -> (Vec3, Quat) {
        let pose = self.pose(index, self.wheels[index.min(REAR)].compression);
        let (_, turn) = match index {
            FRONT => self.geometry.front_pose(self.steer.output(), 0.0),
            _ => self.geometry.rear_pose(0.0),
        };
        (pose.hub, self.body.orientation * turn)
    }
}

/// The lean (rad) of a body with orientation `rotation`: positive when its top leans to its right. The world's up, seen
/// from a body that leans right, points to the body's left.
fn lean_of(rotation: glam::Mat3) -> f32 {
    let up = rotation.transpose() * Vec3::Y;
    (-up.x).atan2(up.y)
}

/// The pitch (rad) of a body: positive when its forward axis points above the horizon.
fn pitch_of(rotation: glam::Mat3) -> f32 {
    (rotation * Vec3::Z).y.clamp(-1.0, 1.0).asin()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_behaviour;
