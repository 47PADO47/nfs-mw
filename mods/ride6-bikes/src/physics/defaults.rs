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

/// `Rider.RiderK` and `RiderDamp`: the constructor defaults 20000 and 5000 (`docs/specs/ride6-rider-body-aero.md`).
pub const RIDER_K: f64 = 20_000.0;
pub const RIDER_DAMP: f64 = 5_000.0;

/// The constructor defaults of `FRiderComponent` that the assets leave out (`docs/specs/ride6-rider-body-aero.md`, "Rider
/// body", read from the constructor at `0x1450bb350`): `WeightInfluence` 1, `LeanInfluence` 0, the damping speed range 40 to
/// 60 km/h with the multiplier 1, `RiderAngularK` 1000, `RiderAngularDamp` 100, `RiderAngularDampChassisFactor` 1, and the
/// vectors `InertiaTensorScale` and `TorqueAppliedOnBikeMultiplier` (1, 1, 1).
pub const RIDER_WEIGHT_INFLUENCE: f64 = 1.0;
pub const RIDER_LEAN_INFLUENCE: f64 = 0.0;
pub const RIDER_DAMP_SPEED_RANGE_KMH: [f64; 2] = [40.0, 60.0];
pub const RIDER_DAMP_MULTIPLIER: f64 = 1.0;
pub const RIDER_ANGULAR_K: f64 = 1000.0;
pub const RIDER_ANGULAR_DAMP: f64 = 100.0;
pub const RIDER_ANGULAR_DAMP_CHASSIS_FACTOR: f64 = 1.0;
pub const RIDER_UNIT_VECTOR: [f64; 3] = [1.0; 3];
/// `DragScale` when an asset leaves it out: 1, so the scale curve alone shapes the drag (the default was not read; the scale
/// cancels out of the drag ratio the model uses).
pub const RIDER_DRAG_SCALE: f64 = 1.0;

/// The rear brake: `BrakeBalance` must be 1 (`docs/specs/ride6-engine-drivetrain.md`, "Brake torque and its distribution").
pub const BRAKE_BALANCE: f64 = 1.0;
/// `JoinedBrakesFrontFactor`, `JoinedBrakesRearFactor`: 0 when unset (same spec, "Joined brakes").
pub const JOINED_SHARE: f64 = 0.0;

/// `ClutchMaxTorqueFactor`: the C++ default `ClutchMaxTorqueType` takes `PeakTorque * 1` as the clutch's capacity
/// (`docs/specs/ride6-engine-drivetrain.md`, "Clutch and drivetrain coupling").
pub const CLUTCH_MAX_FACTOR: f64 = 1.0;
/// No efficiency factor was found in the drivetrain (`docs/specs/vehicle-two-wheel-parameters.md`, section 1).
pub const TRANSMISSION_EFFICIENCY: f64 = 1.0;

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
    /// `TerrainInclinationInfluence`.
    pub const TERRAIN_INFLUENCE: f64 = 0.0;
    /// `bFilterTargetAngle` and `timescale` (1/s): the filter on the target angle is on by default.
    pub const FILTER_TARGET: bool = true;
    pub const FILTER_TIMESCALE: f64 = 10.0;
}

/// `UTireData.Decay` and the tyre-wear element: constructor defaults (`docs/specs/ride6-tyre-thermal.md`, section 1).
pub mod thermal {
    pub const SECTIONS_DEPENDENCY: f64 = 1.0;
    pub const SECTIONS_ANGLE_RAD: f64 = 1.0;
    pub const HEAT_TRANSFER: f64 = 0.0;
    pub const SPESS: f64 = 3.0;
    pub const TEMPERATURE: f64 = 70.0;
    pub const MAX_TEMPERATURE: f64 = 200.0;
    pub const ASPHALT_IMPORTANCE: f64 = 0.8;
    /// `NominalWearDecayLength`, cm.
    pub const WEAR_LENGTH_CM: f64 = 500_000.0;
    pub const UNIFORM: bool = false;
    pub const STATIC_TIME_SCALE: f64 = 0.1;
    pub const DYNAMIC_TIME_SCALE: f64 = 1e-6;
    pub const DECAY_DISTANCE_FACTOR: f64 = 1.0;
    pub const WEAR_INTENSITY: f64 = 1.0;
}

/// The input handler's gearbox hold-offs and launch clutch (`docs/specs/ride6-gearbox-auto.md`, sections 2, 3 and 5).
pub mod gearbox {
    /// `PreventGear{Up,Down}ChangeLean{Factor}`, `PreventGearDownChangeThrottleFactor`: 0 when unset.
    pub const HOLD_FACTOR: f64 = 0.0;
    /// `Prevent...Linearity` when unset: the constructor default 2.
    pub const HOLD_LINEARITY: f64 = 2.0;
    /// `ClucthMinRPMPercentage`, `ClucthMaxRPMPercentage`: the constructor defaults 0.4 and 0.7.
    pub const CLUTCH_MIN_RPM: f64 = 0.4;
    pub const CLUTCH_MAX_RPM: f64 = 0.7;
    /// `ClutchFeatheringOnStart` constructor defaults: tolerance, exit tolerance, throttle cap, minimum slip, step, rpm raise,
    /// clutch release.
    pub const FEATHER_TARGET_TOLERANCE: f64 = 0.6;
    pub const FEATHER_EXIT_TOLERANCE: f64 = 0.8;
    pub const FEATHER_THROTTLE_CAP: f64 = 1.0;
    pub const FEATHER_MIN_SLIP: f64 = 0.1;
    pub const FEATHER_DELTA_TIME: f64 = 0.01;
    pub const FEATHER_RPM_RAISE: f64 = 0.05;
    pub const FEATHER_RELEASE_FACTOR: f64 = 1.0;
    /// `ClutchTimingOnGearChange` and `ThrottleTimingOnGearChange`: the constructor default, `(0, 0)`.
    pub const CLUTCH_TIMING: [f64; 2] = [0.0, 0.0];
    pub const THROTTLE_TIMING: [f64; 2] = [0.0, 0.0];
    /// `MinRPMSpeedScaleRange`: the constructor default.
    pub const MIN_RPM_SPEED_SCALE_RANGE: [f64; 2] = [1.05, 1.1];
    /// `ForceGearDownChangeBrakeFactor` and `...Linearity`: the constructor defaults of the road-bike handler, 0 (off) and 2.
    pub const FORCE_DOWN_FACTOR: f64 = 0.0;
    pub const FORCE_DOWN_LINEARITY: f64 = 2.0;
    /// `SlipperClutchFilterSpeed`: the constructor default of the transmission data, 0 (no filter).
    pub const SLIPPER_FILTER_SPEED: f64 = 0.0;
}
