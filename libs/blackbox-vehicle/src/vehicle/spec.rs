use glam::Vec3;

use crate::aero::AeroSpec;
use crate::brakes::BrakeSpec;
use crate::drivetrain::TransmissionSpec;
use crate::engine::EngineSpec;
use crate::induction::InductionSpec;
use crate::nos::NosSpec;
use crate::rigid_body::RigidBodySpec;
use crate::suspension::ChassisSpec;
use crate::tires::TireSpec;

/// Everything that describes one car, in the units of the attribute data it is filled from.
#[derive(Clone, Debug)]
pub struct VehicleSpec {
    /// Mass in kg.
    pub mass: f32,
    /// Half extents of the body box in metres (x right, y up, z forward): the inertia and the ground
    /// contact box come from it, and the wheels hang from its bottom.
    pub dimension: Vec3,
    /// Per-axis multiplier of the box inertia tensor.
    pub tensor_scale: Vec3,
    pub body: RigidBodySpec,
    pub chassis: ChassisSpec,
    pub tires: TireSpec,
    pub brakes: BrakeSpec,
    pub engine: EngineSpec,
    pub transmission: TransmissionSpec,
    pub induction: InductionSpec,
    pub nos: NosSpec,
    pub aero: AeroSpec,
}

/// The player tuning sliders, each a plain number as the spec uses it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tunings {
    /// Steering range x (1 + 0.2 s).
    pub steering: f32,
    /// Traction circle: lateral x (1 + 0.2 h), longitudinal x (1 - 0.2 h).
    pub handling: f32,
    /// Brake balance: front x (1 + 0.5 b), rear x (1 - 0.5 b).
    pub brakes: f32,
    /// Added to the ride height of both axles, in inches.
    pub ride_height: f32,
    /// Drag and downforce x (1 + 0.25 a), in [-1, 1].
    pub aerodynamics: f32,
    /// Nitrous slider in [-1, 1].
    pub nos: f32,
    /// Forced induction slider in [-1, 1].
    pub induction: f32,
}
