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

mod abs_level;
mod brakes;
pub mod choices;
mod curve;
mod example;
mod filter;
mod frame;
mod gearbox;
mod geometry;
mod handling;
mod lean;
mod magic;
mod peaks;
mod pitch_assist;
mod powertrain;
mod readout;
mod release;
mod rider;
pub mod rider_place;
mod rider_rotation;
mod rider_spec;
mod slipstream;
mod spec;
mod spec_parts;
mod suspension;
mod thermal;
mod trick;
mod trick_spec;
mod tyre;
mod wheel;
mod wheel_step;

pub use frame::RestPose;
pub use gearbox::{ClutchControl, ForcedDownshift};
pub use handling::chassis_state::Environment;
pub use magic::Mf94;
pub use release::{FallKind, RiderRelease, VELOCITY_FRAMES};
pub use rider_rotation::RiderRotation;
pub use rider_spec::RiderSpec;
pub use slipstream::Slipstream;
pub use spec::{
    AbsLevelSpec, AbsSpec, AutoRiderSpec, AutoTuckSpec, BikeSpec, BrakeSpec, ChassisSpec, Coupling, DamperSpec,
    EngineSpec, FallExtra, FallSpec, FeatherSpec, HelpExtra, Hydraulic, LeanSpec, PitchParams, Progressive,
    RiderHelpSpec, SteeringSpec, SuspensionSpec, TcsSpec, TransmissionSpec, TyreSection, TyreThermal, WheelSpec,
    WheelieSpec,
};
pub use tyre::{TyreInput, TyreOutput};
pub use wheel::WheelState;

use glam::{Quat, Vec3};

use crate::ground::Ground;
use crate::rigid_body::{RigidBody, RigidBodySpec, box_inertia};
use brakes::AntiLock;
use curve::ramp;
use filter::SecondOrder;
use geometry::Geometry;
use handling::chassis_state::ChassisState;
use lean::{Lean, LeanPlan};
use powertrain::{DriveInput, Powertrain};
use readout::{lean_of, pitch_of};
use rider::{RiderBody, RiderWeight};
use suspension::Suspension;
use thermal::TyreState;
use trick::TrickState;

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
    thermal: [TyreState; 2],
    /// The handlebar angle, through its filter.
    steer: SecondOrder,
    lean: Lean,
    plan: LeanPlan,
    weight: RiderWeight,
    weight_input: f32,
    /// What the rider help keeps from step to step (`trick.rs`).
    trick: TrickState,
    abs_level: f32,
    slipstream: f32,
    rider_body: RiderBody,
    /// The rider's body turning against the bike's attitude, his lateral weight input, the tuck, and the place the weight, the
    /// lean and the speed asked for in the last step (m, from his rest place).
    rider_rotation: RiderRotation,
    weight_lateral: RiderWeight,
    weight_lateral_input: f32,
    tuck: f32,
    placement: Vec3,
    last_velocity: Vec3,
    /// How long the block-fall assist has held the lean back, and how long it still waits (s).
    block_held: f32,
    block_wait: f32,
    antilock: AntiLock,
    fallen: bool,
    /// The rider leaving the bike in a fall (`release.rs`).
    departure: release::Departure,
    /// The day and the running state of the fall tests.
    chassis: ChassisState,
}

impl Bike {
    pub fn new(spec: BikeSpec) -> Self {
        let chassis = &spec.chassis;
        let geometry = Geometry::new(chassis, &spec.front, &spec.rear);
        let mass = spec.total_mass();
        let gravity = -9.81;
        let body_spec = RigidBodySpec {
            gravity,
            cg: rider::combined_cog(chassis, spec.rider.base_offset, Vec3::ZERO),
            ..RigidBodySpec::default()
        };
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
        let powertrain = Powertrain::new(&spec.engine, &spec.transmission);
        let thermal = [TyreState::new(spec.front.thermal.as_ref()), TyreState::new(spec.rear.thermal.as_ref())];
        let base_offset = spec.rider.base_offset;
        Self {
            chassis: ChassisState::new(&spec.fall),
            abs_level: AbsLevelSpec::snap(spec.abs_level.default),
            spec,
            body,
            geometry,
            suspension,
            powertrain,
            wheels: [WheelState::default(), WheelState::default()],
            thermal,
            steer: SecondOrder::new(0.0),
            lean: Lean::new(),
            plan: LeanPlan::default(),
            weight: RiderWeight::new(),
            weight_input: 0.0,
            trick: TrickState::default(),
            slipstream: 0.0,
            rider_body: RiderBody::default(),
            rider_rotation: RiderRotation::default(),
            weight_lateral: RiderWeight::new(),
            weight_lateral_input: 0.0,
            tuck: 0.0,
            placement: base_offset,
            last_velocity: Vec3::ZERO,
            block_held: 0.0,
            block_wait: 0.0,
            antilock: AntiLock::default(),
            fallen: false,
            departure: release::Departure::default(),
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
        self.restore_rider();
        self.body.set_cog(rider::combined_cog(&self.spec.chassis, self.spec.rider.base_offset, Vec3::ZERO));
        self.body.linear_velocity = orientation * Vec3::new(0.0, 0.0, speed);
        self.body.angular_velocity = Vec3::ZERO;
        self.wheels = [WheelState::default(), WheelState::default()];
        self.thermal =
            [TyreState::new(self.spec.front.thermal.as_ref()), TyreState::new(self.spec.rear.thermal.as_ref())];
        let spin = speed / self.spec.rear.radius;
        self.wheels[FRONT].spin = speed / self.spec.front.radius;
        self.wheels[REAR].spin = spin;
        self.steer.reset(0.0);
        self.lean.reset();
        self.plan = LeanPlan::default();
        self.weight.reset();
        self.trick = TrickState::default();
        self.antilock.reset();
        self.fallen = false;
        self.rider_body.reset();
        self.rider_rotation.reset();
        self.weight_lateral.reset();
        self.placement = self.spec.rider.base_offset;
        self.last_velocity = self.body.linear_velocity;
        self.remember_motion();
        self.remember_motion();
        self.block_held = 0.0;
        self.block_wait = 0.0;
        self.chassis.reset(&self.spec.fall);
        self.powertrain = Powertrain::new(&self.spec.engine, &self.spec.transmission);
        self.powertrain.match_speed(&self.spec.engine, &self.spec.transmission, spin, self.spec.rear.radius);
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

    /// The rider's weight now (-1 back to 1 forward): the weight the rider help aims at after the rate limit of the input
    /// channel. It differs from the one set with [`Bike::set_weight`] while the auto-rider has the weight.
    pub fn rider_weight(&self) -> f32 {
        self.weight.value()
    }

    /// The rider's lateral weight, -1 (to the left) to 1 (to the right): with the lean it asks for the hang-off (the lateral
    /// shift of his place, `RiderSpec::weight_influence`).
    pub fn set_weight_lateral(&mut self, weight: f32) {
        self.weight_lateral_input = match weight.is_finite() {
            true => weight.clamp(-1.0, 1.0),
            false => 0.0,
        };
    }

    /// How far the rider tucks in behind the screen, 0 (upright) to 1: it scales the air drag by the rider's drag curve.
    pub fn set_tuck(&mut self, tuck: f32) {
        self.tuck = match tuck.is_finite() {
            true => tuck.clamp(0.0, 1.0),
            false => 0.0,
        };
    }

    /// Where the rider's centre of mass is from his rest place on the bike, in the body frame (m; x right, y up, z forward): the
    /// place the weight, the lean and the speed ask for plus how far his spring has let him move.
    pub fn rider_offset(&self) -> Vec3 {
        self.placement + self.rider_body.deflection
    }

    /// The rider's body against the bike's attitude (see [`RiderRotation`]).
    pub fn rider_rotation(&self) -> &RiderRotation {
        &self.rider_rotation
    }

    /// The wake of a vehicle ahead, 0 to 1: the share of the air drag it takes away (see [`Slipstream`]). 0 on an empty road.
    pub fn set_slipstream(&mut self, share: f32) {
        self.slipstream = match share.is_finite() {
            true => share.clamp(0.0, 1.0),
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
        self.check_fall(dt, speed);
        // A rider who is off the bike holds no lever and no bar.
        let let_go = BikeInput::default();
        let input = match self.departure.off {
            true => &let_go,
            false => input,
        };
        let help = self.rider_help(dt, input, speed);
        if !self.departure.off {
            self.shift_weight(dt);
        }
        let throttle = match self.fallen {
            true => 0.0,
            false => help.throttle,
        };
        let brakes = self.spec.brakes;
        let (front_lever, rear_lever) =
            brakes::joined(&brakes, help.brake, input.rear_brake, speed.abs() > choices::STANDING_SPEED);
        let (abs_front, abs_rear) = self.antilock.multipliers();
        let brake = [
            brakes::torque(&brakes, true, front_lever, abs_front, self.spec.front.radius),
            brakes::torque(&brakes, false, rear_lever, abs_rear, self.spec.rear.radius),
        ];
        let stick = match self.fallen {
            true => 0.0,
            false => input.steer,
        };
        match self.departure.off {
            true => self.free_handlebar(dt),
            false => self.steer_handlebar(dt, speed, stick),
        }
        let share_of_limit = (lean_of(rotation).abs() / self.plan.limit.max(0.05)).min(1.0);
        let traction_cut = match &self.spec.tcs {
            Some(tcs) => ramp(self.wheels[REAR].slip_ratio, tcs.slip) * ramp(speed.abs(), tcs.speed_range),
            None => 0.0,
        };
        // The traction control and the rider help each cut what the other leaves.
        let traction_cut = 1.0 - (1.0 - traction_cut) * (1.0 - help.engine_cut);
        let radius =
            tyre::effective_radius(&self.spec.rear, self.wheels[REAR].camber, self.wheels[REAR].tyre_compression);
        let drive = self.powertrain.drive(
            &self.spec.engine,
            &self.spec.transmission,
            &DriveInput {
                throttle,
                wheel_spin: self.wheels[REAR].spin,
                speed: speed.abs(),
                front_brake: help.brake,
                radius,
                lean: share_of_limit,
                traction_cut,
                dt,
            },
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
        self.update_antilock(speed, [front_slip, rear_slip], dt);
        self.apply_rider_and_air(dt, speed, input.throttle - input.front_brake);
        self.remember_motion();
    }
}

#[cfg(test)]
mod release_tests;
#[cfg(test)]
mod rider_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_behaviour;
#[cfg(test)]
mod tests_tricks;
