//! The `[bike]` table of a car manifest: the data of a two-wheeled vehicle, in SI units.
//!
//! A car with `kind = "bike"` is driven by the two-wheel dynamics (`blackbox_vehicle::bike`) instead of the four-wheel
//! ones. The table has one key for every number of `blackbox_vehicle::bike::BikeSpec`, in the same units as the spec, so
//! the manifest says everything about how the bike behaves and the engine adds nothing of its own. A number the manifest
//! leaves out refuses the car, because a made-up bike would be a bike nobody measured, with one kind of exception: a key
//! whose zero (or empty list) means the feature is not there (a rider, a hydraulic stop, a tyre that is rigid, an ABS) may be
//! left out, and the keys the reference game leaves to a constructor default that the model keeps as a named choice
//! (`blackbox_vehicle::bike::choices`: `transmission.down_shift`). The parts of the table are in `bike_parts.rs`.
//!
//! Space: x right, y up, z forward, the origin on the ground midway between the contact patches. The checks of the
//! ranges are in `bike_check.rs` and `bike_check_parts.rs`; they catch typing mistakes (a wheel of 3 m, a rake of 26
//! radians), not unusual bikes.

use blackbox_vehicle::bike::{BikeSpec, ChassisSpec, InputRates, Mf94, TcsSpec, WheelSpec};
use serde::Deserialize;

pub use super::bike_help::AbsLevel;
pub use super::bike_parts::{Brakes, Damper, Engine, Fall, Steering, Suspension, Transmission, Wheelie};
pub use super::bike_rider::Rider;
pub use super::bike_thermal::Thermal;

/// Number of coefficients of the three sets of the Magic Formula 94: `a0..a10, a111, a112, a12, a13`, `b0..b10`, `c0..c17`.
pub const MF_A: usize = 15;
pub const MF_B: usize = 11;
pub const MF_C: usize = 18;

/// A checked `[bike]` table.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bike {
    pub chassis: Chassis,
    pub front: Wheel,
    pub rear: Wheel,
    pub front_suspension: Suspension,
    pub rear_suspension: Suspension,
    pub engine: Engine,
    pub transmission: Transmission,
    pub brakes: Brakes,
    pub steering: Steering,
    pub fall: Fall,
    /// Where the rider sits as the weight, the lean and the speed change, and how his body is held to the bike
    /// (`[bike.rider]`); left out for a rider who only shifts along the bike with the weight.
    #[serde(default)]
    pub rider: Rider,
    /// The wheelie and stoppie assist (`[bike.wheelie]`); left out for a bike without one.
    #[serde(default)]
    pub wheelie: Option<Wheelie>,
    /// How the rider's ABS level reaches the brake (`[bike.abs_level]`); left out: the plain level.
    #[serde(default)]
    pub abs_level: AbsLevel,
    /// The traction control (`[bike.tcs]`); left out for a bike without one.
    #[serde(default)]
    pub tcs: Option<Tcs>,
    /// How fast the throttle, the brake levers and the stick follow what the rider holds (`[bike.input]`); left out: at once.
    #[serde(default)]
    pub input: Input,
    /// The scales of the gyroscopic roll and yaw torques of the wheels; 0 (the default) switches one off.
    #[serde(default)]
    pub gyro_roll_scale: f32,
    #[serde(default)]
    pub gyro_yaw_scale: f32,
    /// The quadratic air drag coefficient: the drag force is `drag * v^2` newtons, `v` in m/s.
    pub drag: f32,
}

/// The rise and fall rates, per second, of the rider's axes (`[bike.input]`): a rise moves away from 0 (the stick: in either
/// direction), a fall towards it. A rate of 0 (the default) follows at once. The brake rates are those of the front and the rear lever.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Input {
    pub throttle_rise: f32,
    pub throttle_fall: f32,
    pub brake_rise: f32,
    pub brake_fall: f32,
    pub steer_rise: f32,
    pub steer_fall: f32,
}

/// The traction control (`[bike.tcs]`): the slip ratios between which it cuts the throttle, and the speeds in m/s over which it
/// comes in.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tcs {
    pub slip: [f32; 2],
    pub speed_range_mps: [f32; 2],
}

/// Frame, mass and steering geometry (`[bike.chassis]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chassis {
    /// The bike without the rider, wheels included, kg.
    pub mass_kg: f32,
    /// The rider's mass, kg. 0 (the default): a bike without one.
    #[serde(default)]
    pub rider_mass_kg: f32,
    /// Where the rider's centre of mass is, from the bike's own centre of gravity, m (x right, y up, z forward).
    #[serde(default)]
    pub rider_offset_m: [f32; 3],
    /// How far the rider moves from there with the weight fully forward and fully back, m.
    #[serde(default)]
    pub rider_forward_shift_m: [f32; 3],
    #[serde(default)]
    pub rider_backward_shift_m: [f32; 3],
    /// How fast the rider's weight input moves, per second; 0 (the default) for at once.
    #[serde(default)]
    pub rider_weight_rate: f32,
    /// The spring (N/m) and damper (N s/m) that hold the rider to the bike; 0 (the default) for a rider rigid on it.
    #[serde(default)]
    pub rider_k_n_per_m: f32,
    #[serde(default)]
    pub rider_damp: f32,
    /// Half extents of the body box in m (x right, y up, z forward): its bounding radius, and the derived stats.
    pub half_dimensions_m: [f32; 3],
    /// The radius of gyration squared (m^2) about x (pitch), y (yaw) and z (roll): the inertia is this times the total mass.
    pub tensor_scale: [f32; 3],
    /// Height of the bike's own centre of gravity above the ground at rest, m.
    pub cg_height_m: f32,
    /// Share of the bike's weight (without the rider) on the front wheel at rest, 0 to 1.
    pub front_weight: f32,
    /// Distance between the wheel centres at rest, m.
    pub wheelbase_m: f32,
    /// Steering-head angle from the vertical, rad, the top leaning back.
    pub rake_rad: f32,
    /// Horizontal distance from the steering axis' ground point back to the contact patch, m.
    pub trail_m: f32,
}

/// The Magic Formula 94 coefficients of a tyre (`[bike.front.tyre]`), in the units of the published formula: vertical load
/// in kN, angles in degrees, slip in percent.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tyre {
    /// `a0..a10`, then `a111`, `a112`, `a12`, `a13`: the lateral force.
    #[serde(deserialize_with = "exact_a")]
    pub a: [f32; MF_A],
    /// `b0..b10`: the longitudinal force.
    #[serde(deserialize_with = "exact_b")]
    pub b: [f32; MF_B],
    /// `c0..c17`: the self-aligning moment.
    #[serde(deserialize_with = "exact_c")]
    pub c: [f32; MF_C],
}

/// A list of exactly `N` numbers. Serde's own array reader stops after `N` and ignores the rest of a longer list, which
/// would let a miscounted coefficient set through.
fn exact<'de, D: serde::Deserializer<'de>, const N: usize>(deserializer: D) -> Result<[f32; N], D::Error> {
    let list = Vec::<f32>::deserialize(deserializer)?;
    let found = list.len();
    <[f32; N]>::try_from(list).map_err(|_| serde::de::Error::invalid_length(found, &"the full set of coefficients"))
}

fn exact_a<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<[f32; MF_A], D::Error> {
    exact::<D, MF_A>(deserializer)
}

fn exact_b<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<[f32; MF_B], D::Error> {
    exact::<D, MF_B>(deserializer)
}

fn exact_c<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<[f32; MF_C], D::Error> {
    exact::<D, MF_C>(deserializer)
}

/// One wheel with its tyre (`[bike.front]`, `[bike.rear]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wheel {
    /// Outer radius, m.
    pub radius_m: f32,
    /// Width of the tyre, m.
    pub width_m: f32,
    /// 0 for a flat tread, 1 for a round tyre.
    pub toroid_factor: f32,
    /// Mass of the wheel and tyre, kg.
    pub mass_kg: f32,
    /// Moment of inertia about the axle, kg m^2.
    pub inertia_kgm2: f32,
    pub tyre: Tyre,
    /// Rolling resistance as a share of the load: a constant, a part per m/s and a part per (m/s)^2.
    pub rolling: [f32; 3],
    /// The slip term of the rolling resistance: its factor (0, the default: none) and the exponent of the lateral slip.
    #[serde(default)]
    pub rolling_slip: f32,
    #[serde(default = "one")]
    pub rolling_slip_linearity: f32,
    /// The temperature and wear of the tread (`[bike.front.thermal]`); left out for a tyre at its best.
    #[serde(default)]
    pub thermal: Option<Thermal>,
    /// The largest lateral force as a multiple of the load: informational (the shop's stats), the tyre curve decides.
    pub lateral_grip_limit: f32,
    /// Radial stiffness of the tyre, N/m; it shares the ground penetration with the suspension. 0 (the default): rigid.
    #[serde(default)]
    pub tire_stiffness_n_per_m: f32,
    /// The most the tyre deforms before the suspension takes the rest, m.
    #[serde(default)]
    pub max_tire_deformation_m: f32,
    /// How much of the tyre's deformation shortens its rolling radius, 0 to 1.
    #[serde(default)]
    pub compression_shape_factor: f32,
    /// The damping of the combined slip at speed and at rest, and the speed in m/s over which one gives way to the other.
    #[serde(default)]
    pub slip_correction: f32,
    #[serde(default)]
    pub rest_slip_correction: f32,
    #[serde(default)]
    pub rest_speed_threshold_mps: f32,
}

fn one() -> f32 {
    1.0
}

fn wheel_spec(wheel: &Wheel) -> WheelSpec {
    WheelSpec {
        radius: wheel.radius_m,
        width: wheel.width_m,
        toroid_factor: wheel.toroid_factor,
        mass: wheel.mass_kg,
        inertia: wheel.inertia_kgm2,
        tyre: Mf94 { a: wheel.tyre.a, b: wheel.tyre.b, c: wheel.tyre.c },
        rolling: wheel.rolling,
        rolling_slip: wheel.rolling_slip,
        rolling_slip_linearity: wheel.rolling_slip_linearity,
        thermal: wheel.thermal.as_ref().map(Thermal::to_spec),
        lateral_grip_limit: wheel.lateral_grip_limit,
        tire_stiffness: wheel.tire_stiffness_n_per_m,
        max_tire_deformation: wheel.max_tire_deformation_m,
        compression_shape_factor: wheel.compression_shape_factor,
        slip_correction: wheel.slip_correction,
        rest_slip_correction: wheel.rest_slip_correction,
        rest_speed_threshold: wheel.rest_speed_threshold_mps,
    }
}

fn chassis_spec(chassis: &Chassis) -> ChassisSpec {
    ChassisSpec {
        mass: chassis.mass_kg,
        rider_mass: chassis.rider_mass_kg,
        rider_offset: chassis.rider_offset_m.into(),
        rider_forward_shift: chassis.rider_forward_shift_m.into(),
        rider_backward_shift: chassis.rider_backward_shift_m.into(),
        rider_weight_rate: chassis.rider_weight_rate,
        rider_k: chassis.rider_k_n_per_m,
        rider_damp: chassis.rider_damp,
        half_dimensions: chassis.half_dimensions_m.into(),
        tensor_scale: chassis.tensor_scale.into(),
        cg_height: chassis.cg_height_m,
        front_weight: chassis.front_weight,
        wheelbase: chassis.wheelbase_m,
        rake: chassis.rake_rad,
        trail: chassis.trail_m,
    }
}

/// The table as the dynamics take it: the same numbers, in the same units.
impl From<&Bike> for BikeSpec {
    fn from(bike: &Bike) -> Self {
        Self {
            chassis: chassis_spec(&bike.chassis),
            rider: bike.rider.to_spec(),
            front: wheel_spec(&bike.front),
            rear: wheel_spec(&bike.rear),
            front_suspension: bike.front_suspension.to_spec(),
            rear_suspension: bike.rear_suspension.to_spec(),
            engine: bike.engine.to_spec(),
            transmission: bike.transmission.to_spec(),
            brakes: bike.brakes.to_spec(),
            steering: bike.steering.to_spec(),
            fall: bike.fall.to_spec(),
            wheelie: bike.wheelie.as_ref().map(Wheelie::to_spec),
            abs_level: bike.abs_level.to_spec(),
            tcs: bike.tcs.map(|tcs| TcsSpec { slip: tcs.slip, speed_range: tcs.speed_range_mps }),
            input: InputRates {
                throttle: [bike.input.throttle_rise, bike.input.throttle_fall],
                brake: [bike.input.brake_rise, bike.input.brake_fall],
                steer: [bike.input.steer_rise, bike.input.steer_fall],
            },
            gyro_roll_scale: bike.gyro_roll_scale,
            gyro_yaw_scale: bike.gyro_yaw_scale,
            drag: bike.drag,
        }
    }
}
