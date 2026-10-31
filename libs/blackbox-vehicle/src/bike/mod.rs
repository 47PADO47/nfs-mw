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
mod brake_reduction;
mod brakes;
pub mod choices;
mod clutch_control;
mod curve;
mod engine_brake;
mod example;
mod filter;
mod frame;
mod gearbox;
mod geometry;
mod handling;
mod input_axes;
mod lean;
mod magic;
mod peaks;
mod pitch_assist;
mod powertrain;
mod readout;
mod release;
mod reverse;
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

pub use brake_reduction::{BrakeReduction, RangeMap, WheelReduction};
pub use clutch_control::{ClutchControl, ForcedDownshift, OverLimit};
pub use engine_brake::EngineBrakeReduction;
pub use frame::RestPose;
pub use handling::chassis_state::Environment;
pub use input_axes::InputRates;
pub use magic::Mf94;
pub use release::{FallKind, RiderRelease, VELOCITY_FRAMES};
pub use reverse::{ReverseDrive, ReverseSpec, ReverseState};
pub use rider_rotation::RiderRotation;
pub use rider_spec::RiderSpec;
pub use slipstream::Slipstream;
pub use spec::{
    AbsLevelSpec, AbsSpec, AutoRiderSpec, AutoTuckSpec, BikeSpec, BrakeSpec, ChassisSpec, Coupling, DamperSpec,
    EngineSpec, FallExtra, FallSpec, FeatherSpec, HelpExtra, Hydraulic, LeanSpec, PitchParams, Progressive,
    RiderHelpSpec, StabilitySpec, SteeringSpec, SuspensionSpec, TcsSpec, TerrainLimit, TransmissionSpec, TyreSection,
    TyreThermal, WheelSpec, WheelieSpec,
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
    /// How long (s) the forces left in the body by the last sub-step stand for; 0 when there are none.
    substep: f32,
    /// How long the block-fall assist has held the lean back, and how long it still waits (s).
    block_held: f32,
    block_wait: f32,
    antilock: AntiLock,
    fallen: bool,
    /// The throttle, the brake levers and the stick as they are now, after their rates.
    held: BikeInput,
    /// The rider leaving the bike in a fall (`release.rs`).
    departure: release::Departure,
    /// The day and the running state of the fall tests.
    chassis: ChassisState,
    /// The rider holding the brake with the bike standing, to walk it backwards (`reverse.rs`).
    reverse: ReverseState,
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
            substep: 0.0,
            block_held: 0.0,
            block_wait: 0.0,
            antilock: AntiLock::default(),
            fallen: false,
            held: BikeInput::default(),
            departure: release::Departure::default(),
            reverse: ReverseState::default(),
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
        self.held = BikeInput::default();
        self.rider_body.reset();
        self.rider_rotation.reset();
        self.weight_lateral.reset();
        self.placement = self.spec.rider.base_offset;
        self.last_velocity = self.body.linear_velocity;
        self.substep = 0.0;
        self.remember_motion();
        self.remember_motion();
        self.block_held = 0.0;
        self.block_wait = 0.0;
        self.chassis.reset(&self.spec.fall);
        self.reverse.reset();
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
        // The forces `respond` left stand for its last sub-step only; the earlier ones were integrated inside it.
        self.body.begin_frame(if self.substep > 0.0 { self.substep.min(dt) } else { dt });
    }

    /// The second half of a step: the rider, the engine, the tyres and the suspension work out the forces that move the
    /// body in the next step. A step longer than `choices::MAX_SUBSTEP` is divided into equal sub-steps, as the reference game
    /// divides a long vehicle step (`ride6-chassis-round7.md`, section 1): the body is integrated between them, and the forces
    /// of the last one are left to `begin_body`, so the bike advances exactly `dt` per `begin_body` and `respond`.
    pub fn respond(&mut self, dt: f32, input: &BikeInput, ground: &dyn Ground) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        // The axes move at the data's rates over the whole step; the sub-steps hold what they reach.
        self.held = self.spec.input.apply(&self.held, input, dt);
        let held = self.held;
        let input = &held;
        let count = ((dt / choices::MAX_SUBSTEP).ceil() as usize).clamp(1, choices::MAX_SUBSTEPS);
        let step = dt / count as f32;
        let mut integrated = 0.0;
        for index in 0..count {
            if index > 0 {
                // A bike without its rider lies where the host's contacts put it: no free sub-steps for it.
                if self.fallen {
                    break;
                }
                self.body.begin_frame(step);
                integrated += step;
            }
            // The fall tests run once per step (the reference's `FallCheckInterval`), before the body moves.
            self.respond_once(step, input, ground, (index == 0).then_some(dt));
        }
        self.substep = dt - integrated;
    }

    /// One sub-step of [`Bike::respond`].
    fn respond_once(&mut self, dt: f32, input: &BikeInput, ground: &dyn Ground, fall_check: Option<f32>) {
        let rotation = self.body.rotation();
        let forward = rotation * Vec3::Z;
        let speed = self.body.linear_velocity.dot(forward);
        if let Some(interval) = fall_check {
            self.check_fall(interval, speed);
        }
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
        let (front_lever, rear_lever) = self.step_reverse(dt, speed, input).levers(front_lever, rear_lever);
        // A bike braked hard in a lean has no grip left for the lean (the front tyre's force is spent on the brake, the rear wheel is
        // unloaded): the levers give way as the lean nears its limit (`choices::BRAKE_LEAN_RELIEF`).
        let lean_share = (lean_of(rotation).abs() / self.plan.limit.max(choices::MIN_LEAN_LIMIT)).min(1.0);
        let relief = 1.0 - choices::BRAKE_LEAN_RELIEF * lean_share * lean_share;
        let (front_lever, rear_lever) = (front_lever * relief, rear_lever * relief);
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
            // The slip the control watches is the length of the rear wheel's slip vector (the ratio and the sine of the angle: a rear that
            // slides sideways under power has lost its grip as much as one that spins), in a window narrowed by `choices::TCS_WINDOW_SCALE`
            // (`ride6-feel-round8.md`, section 5).
            Some(tcs) => {
                let rear = &self.wheels[REAR];
                let window = [tcs.slip[0] * choices::TCS_WINDOW_SCALE, tcs.slip[1] * choices::TCS_WINDOW_SCALE];
                ramp(rear.slip_ratio.abs().hypot(rear.slip_angle.sin()), window) * ramp(speed.abs(), tcs.speed_range)
            }
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
                rear_load: self.load_share(REAR),
                radius,
                lean: share_of_limit,
                traction_cut,
                wheel_inertia: self.spec.rear.inertia,
                traction_torque: self.wheels[REAR].traction * radius,
                brake_torque: brake[REAR],
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
        self.update_antilock(speed, [front_slip, rear_slip], input.throttle, dt);
        self.apply_rider_and_air(dt, speed, input);
        self.remember_motion();
    }
}

#[cfg(test)]
mod feel_tests;
#[cfg(test)]
mod release_tests;
#[cfg(test)]
mod rider_tests;
#[cfg(test)]
mod round10_tests;
#[cfg(test)]
mod round9_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_behaviour;
#[cfg(test)]
mod tests_tricks;
