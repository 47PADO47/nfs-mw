//! The parameters of a two-wheeled vehicle, in SI units and in the physics space of the crate: x right, y up, z forward.
//!
//! Every field is a plain number the caller fills from its own data. `BikeSpec::example` (in `example.rs`) is a generic
//! 450 cc supermoto-class bike for tests and examples; it is not any game's data.

use glam::Vec3;

pub use super::abs_level::AbsLevelSpec;
pub use super::lean::extra::{FallExtra, HelpExtra, StabilitySpec, TerrainLimit};
use super::magic::Mf94;
pub use super::spec_parts::{
    Coupling, DamperSpec, EngineSpec, FeatherSpec, Hydraulic, Progressive, SuspensionSpec, TransmissionSpec,
    TyreSection, TyreThermal,
};
pub use super::trick_spec::{AutoRiderSpec, AutoTuckSpec, RiderHelpSpec};

/// The whole bike.
#[derive(Clone, Debug)]
pub struct BikeSpec {
    pub chassis: ChassisSpec,
    /// The rider's placement and body dynamics; the default leaves him where the chassis puts him.
    pub rider: super::rider_spec::RiderSpec,
    pub front: WheelSpec,
    pub rear: WheelSpec,
    pub front_suspension: SuspensionSpec,
    pub rear_suspension: SuspensionSpec,
    pub engine: EngineSpec,
    pub transmission: TransmissionSpec,
    pub brakes: BrakeSpec,
    pub steering: SteeringSpec,
    pub fall: FallSpec,
    /// The wheelie and stoppie assist; `None` for a bike without one.
    pub wheelie: Option<WheelieSpec>,
    /// How the rider's ABS level reaches the brake (`Bike::set_abs_level`).
    pub abs_level: AbsLevelSpec,
    /// The traction control; `None` for a bike without one.
    pub tcs: Option<TcsSpec>,
    /// How fast the throttle, the brake levers and the stick follow what the rider holds (`InputRates::default()`: at once).
    pub input: super::input_axes::InputRates,
    /// The scales of the gyroscopic roll torque (yaw rate) and yaw torque (roll rate) of the wheels; 0 switches one off.
    pub gyro_roll_scale: f32,
    pub gyro_yaw_scale: f32,
    /// The quadratic air drag coefficient: the drag force is `drag * v^2` (N, v in m/s).
    pub drag: f32,
    /// Reverse by walking the bike back; `None` for a bike without one (`super::reverse`).
    pub reverse: Option<super::reverse::ReverseSpec>,
}

/// Frame, mass and steering geometry.
#[derive(Clone, Debug)]
pub struct ChassisSpec {
    /// The bike without the rider, wheels included (kg).
    pub mass: f32,
    /// The rider's mass (kg); 0 for a bike without one.
    pub rider_mass: f32,
    /// Where the rider's centre of mass sits, from the bike's own centre of gravity (m; x right, y up, z forward).
    pub rider_offset: Vec3,
    /// How far the rider moves from there when the weight is fully forward and fully back (m).
    pub rider_forward_shift: Vec3,
    pub rider_backward_shift: Vec3,
    /// How fast the rider's weight input moves (per second); 0 for at once.
    pub rider_weight_rate: f32,
    /// The spring (N/m) and damper (N s/m) that hold the rider to the bike; 0 for a rider rigid on it.
    pub rider_k: f32,
    pub rider_damp: f32,
    /// Half extents of the body box (m): its bounding radius, and the derived stats of a host.
    pub half_dimensions: Vec3,
    /// Radius of gyration squared (m^2) about x (pitch), y (yaw) and z (roll): the inertia is this times the total mass.
    pub tensor_scale: Vec3,
    /// Height of the bike's own centre of gravity above the ground at rest (m).
    pub cg_height: f32,
    /// Share of the bike's weight (without the rider) on the front wheel at rest, 0 to 1.
    pub front_weight: f32,
    /// Distance between the wheel centres at rest (m).
    pub wheelbase: f32,
    /// Steering-head angle from the vertical (rad), the top leaning back.
    pub rake: f32,
    /// Horizontal distance from the steering axis' ground point back to the contact patch (m).
    pub trail: f32,
}

/// One wheel with its tyre.
#[derive(Clone, Debug)]
pub struct WheelSpec {
    /// Outer radius (m).
    pub radius: f32,
    /// Width of the tyre (m).
    pub width: f32,
    /// 0 for a flat tread, 1 for a round tyre: the radius of the tyre's cross-section is this share of half the width.
    pub toroid_factor: f32,
    /// Mass of the wheel and tyre (kg).
    pub mass: f32,
    /// Moment of inertia about the axle (kg m^2).
    pub inertia: f32,
    pub tyre: Mf94,
    /// Rolling resistance as a share of the load: a constant, a part per m/s and a part per (m/s)^2.
    pub rolling: [f32; 3],
    /// The slip term of the rolling resistance: its factor and the exponent of the lateral slip.
    pub rolling_slip: f32,
    pub rolling_slip_linearity: f32,
    /// The temperature and wear of the tread; `None` for a tyre at its best.
    pub thermal: Option<TyreThermal>,
    /// The largest lateral force as a multiple of the load: informational (derived stats), the tyre curve decides.
    pub lateral_grip_limit: f32,
    /// Radial stiffness of the tyre (N/m): it shares the ground penetration with the suspension. 0 for a rigid tyre.
    pub tire_stiffness: f32,
    /// The most the tyre deforms before the suspension takes the rest (m).
    pub max_tire_deformation: f32,
    /// How much of the tyre's deformation shortens its effective radius (0 to 1).
    pub compression_shape_factor: f32,
    /// The damping of the combined slip at speed and at rest, and the speed (m/s) over which one gives way to the other.
    pub slip_correction: f32,
    pub rest_slip_correction: f32,
    pub rest_speed_threshold: f32,
}

/// An anti-lock brake: the share of the brake it takes away from a wheel whose slip nears the force peak.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AbsSpec {
    /// How hard it acts on each wheel, 0 to 1; 0 for none.
    pub front_intensity: f32,
    pub rear_intensity: f32,
    /// The speed (m/s) over which it fades in.
    pub speed_range: [f32; 2],
    /// The normalised slip ratio (-1 is the force peak) at which it starts to act and at which it acts fully.
    pub front_slip: [f32; 2],
    pub rear_slip: [f32; 2],
    /// The brake reduction (lean, drift, unloading, throttle); none by default.
    pub reduction: super::brake_reduction::BrakeReduction,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrakeSpec {
    /// The force of the front brake and of the rear brake at the tyre patch at full lever (N).
    pub front_force: f32,
    pub rear_force: f32,
    /// The linked brakes: the rear is at least `joined_factor` times the front. When either of the two other factors is
    /// above 0 they replace it: front and rear are those shares of the larger lever.
    pub joined_factor: f32,
    pub joined_front: f32,
    pub joined_rear: f32,
    pub abs: AbsSpec,
}

/// The rider's lean controller (`docs/specs/ride6-lean-controller.md`).
#[derive(Clone, Debug)]
pub struct LeanSpec {
    /// Stiffness (1/s^2) and damping ratio of the roll spring-damper towards the target lean, and the multiplier of the
    /// ratio for the bike's weight and size.
    pub k: f32,
    pub damping_ratio: f32,
    pub damping_multiplier: f32,
    /// The filters of the stick and of the target lean: stiffness and damping ratio.
    pub steer_k: f32,
    pub steer_damping_ratio: f32,
    pub target_k: f32,
    pub target_damping_ratio: f32,
    /// The speed of the target lean's filter, and what its time scale falls with the lean, with the distance to the target,
    /// with the speed `(m/s, factor)` and with the stick `(stick, factor)`. Empty curves have no effect.
    pub time_scale: f32,
    pub time_scale_attenuation: f32,
    pub time_scale_sensitivity: f32,
    pub time_scale_speed: Vec<(f32, f32)>,
    pub time_scale_input: Vec<(f32, f32)>,
    /// The lean limit's factor against the lateral acceleration in g: `(g, factor)`.
    pub g_tuning: Vec<(f32, f32)>,
    /// The heading error (rad) at which the skid starts to cut the lean limit and at which it cuts fully, the exponent of
    /// the ramp, and the cut against the speed `(m/s, cut)`.
    pub skid_range: [f32; 2],
    pub skid_linearity: f32,
    pub skid_reduction: Vec<(f32, f32)>,
    /// The lean limit at a standstill (rad) and the speed (m/s) below which it blends in.
    pub stopped_limit: f32,
    pub stopped_speed: f32,
    /// The speeds (m/s) between which the handlebar goes from the stick to the controller.
    pub blend_range: [f32; 2],
    /// The weight of the rider in the roll inertia.
    pub rider_mass_scale: f32,
    /// The lean help: the share of the gap between the front tyre's side force and the wanted one it closes, against the speed
    /// `(m/s, share)` (empty for no help), and the share of the tyre's lateral peak it may add.
    pub help_speed: Vec<(f32, f32)>,
    pub help_limit: f32,
    /// The stopped blend, the force-or-couple balance and the skid cut of the help (`HelpExtra::default()` for none).
    pub help_extra: HelpExtra,
    /// The stability help (`StabilitySpec::default()` for none).
    pub stability: StabilitySpec,
    /// The terrain term of the lean limit (`TerrainLimit::default()` for none).
    pub terrain: TerrainLimit,
}

#[derive(Clone, Debug)]
pub struct SteeringSpec {
    /// The handlebar's lock to each side (rad).
    pub max_angle: f32,
    /// The handlebar filter: stiffness (1/s^2) and damping ratio.
    pub response: f32,
    pub damping_ratio: f32,
    /// The upper bound of the lean limit (rad).
    pub max_lean: f32,
    pub lean: LeanSpec,
}

/// When the rider has lost the bike.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FallSpec {
    /// The lean and the pitch (rad), the angle between the velocity and the heading (rad) and the angular speed (rad/s)
    /// beyond which the bike has fallen; the angle between the velocity and the heading only above `min_speed` (m/s) (the
    /// lean, the pitch and the angular speed are tested at any speed).
    pub lean: f32,
    pub pitch: f32,
    pub speed_direction: f32,
    pub angular_speed: f32,
    pub min_speed: f32,
    /// The block-fall assist: above `block_min_speed` (m/s) the target lean is held to `block_ratio` of the fall lean limit
    /// for up to `block_time` seconds, then the assist waits `block_wait` seconds. A ratio of 0 switches it off.
    pub block_ratio: f32,
    pub block_min_speed: f32,
    pub block_time: f32,
    pub block_wait: f32,
    /// The grip-following lean limit, the airborne limits and the duration (`FallExtra::default()` for none).
    pub extra: FallExtra,
}

/// The traction control (`docs/specs/ride6-rider-help.md`): it cuts the throttle about the zero-torque point as the driven wheel's
/// slip ratio rises through `slip`, once the bike is faster than `speed_range` (m/s) says. The shaping by lean and yaw rate is not
/// modelled (their exact use was not recovered).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TcsSpec {
    pub slip: [f32; 2],
    pub speed_range: [f32; 2],
}

/// The wheelie and stoppie assist: one parameter set for each, and the rider help that cuts the throttle when the front
/// lifts and eases the front brake when the rear does. A range of `[0, 0]` switches the part off.
#[derive(Clone, Debug)]
pub struct WheelieSpec {
    pub wheelie: PitchParams,
    pub stoppie: PitchParams,
    /// The lift angle (rad) over which the throttle is cut, when the rider is not asking for a wheelie and when he is, and
    /// the lift speed (rad/s) over which it is cut.
    pub throttle_pitch: [f32; 2],
    pub forced_throttle_pitch: [f32; 2],
    pub throttle_pitch_speed: [f32; 2],
    /// The same for the front brake against a stoppie (nose down: the angle and speed of the nose going down).
    pub brake_pitch: [f32; 2],
    pub brake_pitch_speed: [f32; 2],
    /// The rest of the rider help (launch blend, strengths, the auto-rider; `RiderHelpSpec::default()` for the defaults).
    pub help: RiderHelpSpec,
}

#[derive(Clone, Debug)]
pub struct PitchParams {
    /// The lever travel (0 to 1) the rider must pass before the assist acts.
    pub deadzone: f32,
    /// The pitch (rad) a full lever holds.
    pub max_angle: f32,
    /// The speeds (m/s) over which the angle fades in.
    pub speed_ramp: [f32; 2],
    /// The assist's strength against the speed `(m/s, factor)`; empty for 1.
    pub help_speed: Vec<(f32, f32)>,
    /// The normalised lean at which the assist starts to fade and at which it is gone.
    pub lean_help: [f32; 2],
    /// How far ahead (s) the pitch is looked at, along its speed.
    pub lookahead: f32,
    /// The speed (m/s) below which the assist is off, the largest pitch acceleration it asks for (rad/s^2), the time (s) it
    /// takes to reach it, and the height (m) of the lifted wheel over which the help fades out.
    pub min_speed: f32,
    pub acceleration: f32,
    pub acceleration_time: f32,
    pub ground_range: [f32; 2],
    /// The lean limit (rad) a full pitch brings, over the pitch range (rad) it is blended in.
    pub lean_limit: f32,
    pub lean_pitch_range: [f32; 2],
    /// The share of the road's inclination along the heading that is taken off the pitch (`TerrainInclinationInfluence`).
    pub terrain_influence: f32,
    /// The time scale (1/s) of the filter that moves the target angle; 0 leaves the target as it is.
    pub filter_timescale: f32,
}

impl BikeSpec {
    /// The total mass the rigid body carries (kg): the bike, wheels included, and the rider.
    pub fn total_mass(&self) -> f32 {
        self.chassis.mass + self.chassis.rider_mass
    }

    /// The share of the total weight on the front wheel at rest, with the rider at his rest position.
    pub fn front_share(&self) -> f32 {
        let chassis = &self.chassis;
        let total = self.total_mass().max(f32::EPSILON);
        let bike_z = chassis.wheelbase * (chassis.front_weight - 0.5);
        let rider_z = bike_z + chassis.rider_offset.z + self.rider.base_offset.z;
        let z = (chassis.mass * bike_z + chassis.rider_mass * rider_z) / total;
        0.5 + z / chassis.wheelbase.max(f32::EPSILON)
    }
}
