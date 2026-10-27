//! The assembled vehicle: one rigid body, four corners, tires, powertrain, steering and aerodynamics,
//! stepped together at a fixed rate. Spec: all of `docs/specs/vehicle-*.md`.

mod assists;
mod burnout;
mod coasting;
mod contact;
mod drift;
mod drive_assists;
mod example;
mod integration;
mod jump;
mod launch;
mod maneuver;
mod reconfigure;
mod retirement;
mod rolling_chassis;
mod settle;
mod simple_chassis;
mod spec;
mod staging;
mod state;
mod step;
mod wall_steering;
mod wheels;

#[cfg(test)]
mod tests;

pub use jump::{
    AlignmentConfig, GroundReference, JumpConfig, JumpForces, JumpInput, JumpState, LandingConfig, PassiveConfig,
};
pub use maneuver::ManeuverAssist;
pub use rolling_chassis::RollingChassisConfig;
pub use simple_chassis::SimpleChassisConfig;
pub use spec::{Tunings, VehicleSpec};
pub use state::{SKID_RANGE, SMOKE_RANGE, WheelState};
pub use wall_steering::WallSteeringConfig;

use glam::{Mat3, Quat, Vec3};

use crate::drivetrain::Powertrain;
use crate::ground::Ground;
use crate::input::{ControlConfig, Controls};
use crate::math::ramp;
use crate::rigid_body::{BodyState, RigidBody};
use crate::steering::{Steering, WheelAngles};
use crate::suspension::{Corner, Geometry, WheelContact};
use crate::tires::{Tire, TireIntegration, TireParams};

/// A drivable car. Create it with [`Vehicle::new`], put it somewhere with [`Vehicle::place`], then call
/// [`Vehicle::step`] once per fixed step ([`crate::FIXED_STEP`]).
#[derive(Clone, Debug)]
pub struct Vehicle {
    spec: VehicleSpec,
    /// Player tuning sliders; change them between steps.
    pub tunings: Tunings,
    /// How raw input is interpreted (dead zone, automatic gearbox, steering device).
    pub config: ControlConfig,
    simple_chassis: Option<SimpleChassisConfig>,
    simple_max_steering: f32,
    rolling_chassis: Option<RollingChassisConfig>,
    coasting_tires: Option<crate::tires::CoastingTireResponse>,
    retirement: retirement::Retirement,
    launch: crate::launch::LaunchState,
    /// Calibration hook: hold the front wheels at these angles instead of steering (a replay of a
    /// recorded drive). `None` in play.
    pub forced_wheel_angles: Option<crate::steering::WheelAngles>,
    /// Engine blown or car destroyed: gas 0, brakes on.
    pub disabled: bool,
    /// The player's recharge permission; disable while speed breaker is active.
    pub can_recharge_nos: bool,
    pub maneuver: ManeuverAssist,
    pub jump_config: JumpConfig,
    /// Supplied by a terrain-reference owner, independently of suspension ray hits.
    pub ground_reference: Option<GroundReference>,
    pub wall_steering_config: WallSteeringConfig,
    wall_steering: wall_steering::WallSteering,
    jump_state: JumpState,
    body_ground_contact: bool,
    body_contacts: Vec<crate::rigid_body::GroundContact>,
    /// Spike-strip damage per tire (see `tires::damage`): a blown tire brakes fully and grips less.
    pub tire_damage: crate::tires::TireDamage,
    body: RigidBody,
    geometry: Geometry,
    params: [TireParams; 4],
    powertrain: Powertrain,
    steering: Steering,
    wheel_angles: WheelAngles,
    tires: [Tire; 4],
    tire_integration: TireIntegration,
    corners: [Corner; 4],
    contacts: [Option<WheelContact>; 4],
    loaded: [bool; 4],
    patch_positions: [Vec3; 4],
    wheels_on_ground: usize,
    wheels_on_ground_prev: usize,
    controls: Controls,
    yaw_control: f32,
    burnout: burnout::Burnout,
    drift: drift::Drift,
    body_slip: f32,
    collided: bool,
    time: f32,
}

impl Vehicle {
    /// A car at the origin, upright, in first gear with the engine idling.
    pub fn new(spec: VehicleSpec) -> Self {
        let geometry = Geometry::new(&spec.chassis, &spec.tires, spec.dimension.y);
        let params = std::array::from_fn(|i| {
            let axle = i / 2;
            TireParams {
                radius: geometry.radius[i],
                grip_scale: spec.tires.grip_scale[axle],
                static_grip: spec.tires.static_grip[axle],
                dynamic_grip: spec.tires.dynamic_grip[axle],
                brake_spec: spec.brakes.brake_torque(axle),
                brake_lock_spec: spec.brakes.lock_torque(axle),
                ebrake_spec: spec.brakes.ebrake_torque(),
                front: axle == 0,
            }
        });
        let body = RigidBody::new(spec.mass, spec.dimension, spec.tensor_scale, spec.body);
        let powertrain = Powertrain::new(
            spec.engine.clone(),
            spec.transmission.clone(),
            spec.induction,
            spec.nos,
            spec.tires.average_radius(),
        );
        Self {
            tunings: Tunings::default(),
            config: ControlConfig::default(),
            simple_chassis: None,
            simple_max_steering: 0.0,
            rolling_chassis: None,
            coasting_tires: None,
            retirement: Default::default(),
            launch: Default::default(),
            forced_wheel_angles: None,
            disabled: false,
            can_recharge_nos: true,
            maneuver: ManeuverAssist::default(),
            jump_config: JumpConfig::default(),
            ground_reference: None,
            wall_steering_config: WallSteeringConfig::default(),
            wall_steering: wall_steering::WallSteering::default(),
            jump_state: JumpState::default(),
            body_ground_contact: false,
            body_contacts: Vec::new(),
            tire_damage: crate::tires::TireDamage::default(),
            body,
            geometry,
            params,
            powertrain,
            steering: Steering::default(),
            wheel_angles: WheelAngles::default(),
            tires: [Tire::default(); 4],
            tire_integration: TireIntegration::default(),
            corners: [Corner::default(); 4],
            contacts: [None; 4],
            loaded: [false; 4],
            patch_positions: [Vec3::ZERO; 4],
            wheels_on_ground: 0,
            wheels_on_ground_prev: 0,
            controls: Controls::default(),
            yaw_control: 1.0,
            burnout: burnout::Burnout::default(),
            drift: drift::Drift::default(),
            body_slip: 0.0,
            collided: false,
            time: 0.0,
            spec,
        }
    }

    pub fn spec(&self) -> &VehicleSpec {
        &self.spec
    }

    /// Body-ground observations from the latest tick, including material and pre-response velocity.
    pub fn body_contacts(&self) -> &[crate::rigid_body::GroundContact] {
        &self.body_contacts
    }

    /// Resets every dynamic state and puts the body at `position` with `orientation`, at rest.
    pub fn place(&mut self, position: Vec3, orientation: Quat) {
        self.reset_dynamics();
        self.body.place(position, orientation);
        self.powertrain.reset();
    }

    /// Like [`Vehicle::place`] but already moving forward at `speed` m/s along the car heading, with the
    /// wheels and the engine matched to it.
    pub fn place_moving(&mut self, position: Vec3, orientation: Quat, speed: f32) {
        self.reset_dynamics();
        self.body.place(position, orientation);
        self.body.linear_velocity = orientation * Vec3::new(0.0, 0.0, speed);
        self.powertrain.match_speed(speed);
        for (i, t) in self.tires.iter_mut().enumerate() {
            t.av = speed / self.params[i].radius;
            t.road_speed = speed;
        }
    }

    /// Puts the car on the ground below `(x, z)`, facing `yaw` radians about the up axis (0 = +z), with
    /// the springs at their unloaded length. The ground is searched for from height `top` down to 100 m
    /// below it; returns false if there is none.
    pub fn place_on_ground(&mut self, ground: &dyn Ground, x: f32, z: f32, top: f32, yaw: f32) -> bool {
        self.place_on_ground_moving(ground, x, z, top, yaw, 0.0)
    }

    /// Like [`Vehicle::place_on_ground`] but already moving forward at `speed` m/s with the wheels and the
    /// engine matched to it (traffic starts driving this way).
    pub fn place_on_ground_moving(
        &mut self,
        ground: &dyn Ground,
        x: f32,
        z: f32,
        top: f32,
        yaw: f32,
        speed: f32,
    ) -> bool {
        let Some(hit) = ground.hit(Vec3::new(x, top, z), Vec3::NEG_Y, 100.0) else { return false };
        let y = top - hit.distance;
        let ride = self
            .spec
            .chassis
            .axle(0, self.tunings.ride_height)
            .ride
            .max(self.spec.chassis.axle(1, self.tunings.ride_height).ride);
        let (position, orientation) = (Vec3::new(x, y + self.spec.dimension.y + ride, z), Quat::from_rotation_y(yaw));
        match speed > 0.0 {
            true => self.place_moving(position, orientation, speed),
            false => self.place(position, orientation),
        }
        true
    }

    fn reset_dynamics(&mut self) {
        self.launch.reset();
        self.powertrain.set_launch_context(false, 0.0);
        self.simple_max_steering = self.simple_chassis.map_or(0.0, |c| c.steering_range[0]);
        self.wall_steering = wall_steering::WallSteering::default();
        self.jump_state = JumpState::default();
        self.ground_reference = None;
        self.body_ground_contact = false;
        self.body_contacts.clear();
        self.tires = std::array::from_fn(|_| {
            let mut tire = Tire::default();
            tire.integration = self.tire_integration;
            tire
        });
        self.corners = [Corner::default(); 4];
        self.contacts = [None; 4];
        self.loaded = [false; 4];
        self.wheels_on_ground = 0;
        self.wheels_on_ground_prev = 0;
        self.steering.reset();
        self.wheel_angles = WheelAngles::default();
        self.controls = Controls::default();
        self.yaw_control = 1.0;
        self.burnout = burnout::Burnout::default();
        self.drift = drift::Drift::default();
        self.body_slip = 0.0;
        self.collided = false;
    }

    // -- State read-out -------------------------------------------------------------------------------

    pub fn body(&self) -> &RigidBody {
        &self.body
    }

    /// The rigid body, for the caller's own collision response (walls, other cars): use
    /// [`RigidBody::react_plane`] or add velocity changes directly.
    pub fn body_mut(&mut self) -> &mut RigidBody {
        &mut self.body
    }

    pub fn powertrain(&self) -> &Powertrain {
        &self.powertrain
    }

    pub fn position(&self) -> Vec3 {
        self.body.position
    }

    pub fn orientation(&self) -> Quat {
        self.body.orientation
    }

    pub fn rotation(&self) -> Mat3 {
        self.body.rotation()
    }

    pub fn linear_velocity(&self) -> Vec3 {
        self.body.linear_velocity
    }

    pub fn angular_velocity(&self) -> Vec3 {
        self.body.angular_velocity
    }

    /// Speed in m/s.
    pub fn speed(&self) -> f32 {
        self.body.linear_velocity.length()
    }

    /// Speed along the car forward axis (m/s, negative when reversing).
    pub fn forward_speed(&self) -> f32 {
        (self.body.rotation().transpose() * self.body.linear_velocity).z
    }

    pub fn gear(&self) -> usize {
        self.powertrain.gear()
    }

    /// Engine speed in rpm.
    pub fn rpm(&self) -> f32 {
        self.powertrain.rpm()
    }

    pub fn wheels_on_ground(&self) -> usize {
        self.wheels_on_ground
    }

    /// The controls the car used in the last step, after shaping.
    pub fn controls(&self) -> &Controls {
        &self.controls
    }

    /// Body slip angle in radians (+ = sliding to the right); 0 below 1 m/s forward speed.
    pub fn slip_angle(&self) -> f32 {
        self.body_slip
    }

    /// Seconds of simulated time.
    pub fn time(&self) -> f32 {
        self.time
    }

    pub fn is_asleep(&self) -> bool {
        self.body.state == BodyState::Asleep
    }

    /// Fill the installed nitrous tank without changing its performance specification.
    pub fn refill_nitrous(&mut self) {
        self.powertrain.nos.capacity = 1.0;
    }

    /// Add `amount` of a full tank to the nitrous, up to a full tank, for a game rule that pays nitrous (a drift, a near miss).
    /// A car with no nitrous system keeps none. Returns whether the car took it: it has a tank and `amount` is above zero.
    pub fn add_nitrous(&mut self, amount: f32) -> bool {
        if !self.powertrain.has_nos() || !amount.is_finite() || amount <= 0.0 {
            return false;
        }
        let nos = &mut self.powertrain.nos;
        nos.capacity = (nos.capacity + amount).min(1.0);
        true
    }

    /// Nitrous tank level 0..1.
    pub fn nos_level(&self) -> f32 {
        self.powertrain.nos.capacity
    }

    /// The nitrous is burning right now: the button is held and the gear, throttle and speed let it.
    pub fn nos_burning(&self) -> bool {
        self.powertrain.nos.boost > 1.0
    }

    /// The car has a nitrous system at all.
    pub fn has_nos(&self) -> bool {
        self.powertrain.has_nos()
    }

    /// Turbo gauge (psi, negative in vacuum).
    pub fn boost_psi(&self) -> f32 {
        self.powertrain.induction.psi
    }

    /// Left and right front wheel steering angles (radians, + = right).
    pub fn wheel_angles(&self) -> WheelAngles {
        self.wheel_angles
    }

    /// The state of wheel `i` (0 front left, 1 front right, 2 rear left, 3 rear right).
    /// Lift or restore the engine's rev limiter (Extra Options `NoRevLimiter`).
    pub fn set_no_rev_limiter(&mut self, on: bool) {
        self.powertrain.set_no_rev_limiter(on);
    }

    pub fn wheel(&self, i: usize) -> WheelState {
        let steer = match i {
            0 => self.wheel_angles.left,
            1 => self.wheel_angles.right,
            _ => 0.0,
        };
        let tire = &self.tires[i];
        let radius = self.params[i].radius;
        let on_ground = self.loaded[i];
        let slide_speed = if on_ground { tire.slip.hypot(tire.lateral_speed) } else { 0.0 };
        WheelState {
            position: self.patch_positions[i],
            radius,
            steer_angle: steer,
            angular_velocity: tire.display_av(radius, self.corners[i].compression > 0.0),
            simulated_angular_velocity: tire.av,
            compression: self.corners[i].compression,
            on_ground,
            load: tire.load,
            slip: tire.slip,
            lateral_speed: tire.lateral_speed,
            slip_angle: tire.slip_angle * std::f32::consts::TAU,
            traction: tire.traction,
            locked: tire.brake_locked,
            lateral_force: tire.lateral_force,
            longitudinal_force: tire.longitudinal_force,
            slide_speed,
            skid: if on_ground { ramp(slide_speed, SKID_RANGE.0, SKID_RANGE.1) } else { 0.0 },
            smoke: if on_ground { ramp(slide_speed, SMOKE_RANGE.0, SMOKE_RANGE.1) } else { 0.0 },
        }
    }

    /// The [`ground::SurfaceGrip::tag`](crate::ground::SurfaceGrip::tag) of the surface under wheel `i`
    /// (same order as [`wheel`](Self::wheel)); `None` for a wheel with no ground within reach.
    pub fn wheel_surface_tag(&self, i: usize) -> Option<u32> {
        self.contacts.get(i).copied().flatten().map(|c| c.surface.tag)
    }

    /// Where wheel `i`'s ground ray met the road in the last step and the surface normal there (a debug aid);
    /// `None` for a wheel with no ground within reach.
    pub fn wheel_ground_hit(&self, i: usize) -> Option<(Vec3, Vec3)> {
        let contact = self.contacts.get(i).copied().flatten()?;
        Some((contact.point, contact.normal))
    }

    /// Reports a hard hit (effective change of speed in m/s) that the caller resolved itself: limits steering for a moment
    /// and keeps the sleep damping off for this step.
    pub fn notify_collision(&mut self, impulse: f32) {
        self.steering.notify_collision(impulse);
        self.collided = true;
    }

    /// Shifts to `gear` (gear id: 0 reverse, 1 neutral, 2 first, ...), for manual control.
    pub fn shift_to(&mut self, gear: usize) -> bool {
        self.powertrain.shift(gear)
    }
}
