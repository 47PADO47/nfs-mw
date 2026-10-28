//! The values the importer takes when an asset leaves a field out. Each is a constructor default of the reference game that a
//! spec of this repository records, or a statement that the field is zero when no asset sets it; the document that says so
//! is named on every constant. Nothing here is a guess of the importer: a field with no default is an error.

/// `ProgressiveSpringFactor`, `ProgressiveHydraulicComponentFactor`: 0, a linear spring
/// (`docs/specs/ride6-suspension.md`, "Unset C++ defaults of the suspension and wheel").
pub const PROGRESSIVE_FACTOR: f64 = 0.0;
/// `ProgressiveSpringLinearity`, `ProgressiveHydraulicComponentLinearity`: 1 (same section).
pub const PROGRESSIVE_LINEARITY: f64 = 1.0;
/// `LinearSpringCompressionRatio`, `LinearHydraulicComponentCompressionRatio`: 0 (same section).
pub const PROGRESSIVE_START: f64 = 0.0;
/// The compression-factor remaps of the swingarm forces default to the identity (same section).
pub const REMAP_IDENTITY: [f64; 2] = [0.0, 1.0];
/// `ChainPullFactor`, `TractionInfluenceOnLoad`: set by no front fork asset. The fork carries no drive torque, so the term is
/// zero whatever its factor (`docs/specs/ride6-suspension.md`, "Chain pull, traction and brake coupling"); written as 0.
pub const FORK_SWINGARM_FACTOR: f64 = 0.0;

/// The gyroscopic scales default to 1 and the stock bikes keep them (`docs/specs/ride6-lean-controller.md`, "Gyroscopic terms").
pub const GYRO_SCALE: f64 = 1.0;
/// `LeanTimeScaleAttenuation`, `LeanTimeScaleSensitivity`: not set by the assets; the KTM's values in
/// `docs/formats/ride6-physics-elements.md` ("Lean and steering") are 0 and 0.
pub const LEAN_TIME_SCALE_ATTENUATION: f64 = 0.0;
pub const LEAN_TIME_SCALE_SENSITIVITY: f64 = 0.0;
/// `Rider.ForwardOffset`: no asset sets it; an `FVector` member starts at zero.
pub const RIDER_FORWARD_OFFSET: [f64; 3] = [0.0; 3];
/// `Rider.BackwardOffset` when an asset does not set it (same).
pub const RIDER_BACKWARD_OFFSET: [f64; 3] = [0.0; 3];

/// The rear brake: `BrakeBalance` must be 1 (`docs/specs/ride6-engine-drivetrain.md`, "Brake torque and its distribution").
pub const BRAKE_BALANCE: f64 = 1.0;
/// `JoinedBrakesFrontFactor`, `JoinedBrakesRearFactor`: 0 when unset (same spec, "Joined brakes").
pub const JOINED_SHARE: f64 = 0.0;

/// `ClutchMaxTorqueFactor`: the C++ default `ClutchMaxTorqueType` takes `PeakTorque * 1` as the clutch's capacity
/// (`docs/specs/ride6-engine-drivetrain.md`, "Clutch and drivetrain coupling").
pub const CLUTCH_MAX_FACTOR: f64 = 1.0;
/// No efficiency factor was found in the drivetrain (`docs/specs/vehicle-two-wheel-parameters.md`, section 1).
pub const TRANSMISSION_EFFICIENCY: f64 = 1.0;
/// A one-speed bike never changes up; the key is required, and this value (the whole rev range) never triggers.
pub const UP_SHIFT_WITHOUT_GEARS: f64 = 1.0;

/// `FWheelieControlParams` constructor defaults (`docs/specs/ride6-rider-help.md`, "Wheelie and stoppie control"): the assets
/// set only some of the fields of each parameter set.
pub mod pitch {
    /// `InputDeadzone`.
    pub const DEADZONE: f64 = 0.1;
    /// `MinSpeed`, km/h.
    pub const MIN_SPEED_KMH: f64 = 15.0;
    /// `MaxAngle`, degrees.
    pub const MAX_ANGLE_DEG: f64 = 30.0;
    /// `MaxAngleFactorSpeedTuning`, km/h.
    pub const SPEED_RAMP_KMH: [f64; 2] = [0.0, 0.0];
    /// `GroundDistanceFactorRange`.
    pub const GROUND_RANGE: [f64; 2] = [0.0, 0.0];
    /// `HelpIntensityLeanTuning`.
    pub const LEAN_HELP: [f64; 2] = [0.0, 1.0];
    /// `Acceleration`, rad/s^2.
    pub const ACCELERATION: f64 = 10.0;
    /// `AccelerationTime`, s.
    pub const ACCELERATION_TIME: f64 = 0.005;
    /// `LookaheadTime`, s.
    pub const LOOKAHEAD: f64 = 0.0;
}
