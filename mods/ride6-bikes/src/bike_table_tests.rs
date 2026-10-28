//! The golden test of the physics importer: a trimmed, synthetic resolved vehicle (a made-up bike with round numbers, in the
//! layout of the physics dump) goes in, a manifest comes out, and the manifest is read back with the content schema of the
//! game. The numbers are not any bike's.

use blackbox_modding::content::{CarKind, parse_car};
use serde_json::{Value, json};

use super::*;
use crate::manifest::{Tables, car_toml};

/// The conversion done in the test's own words: the same f64 expression, then `f32`, as the manifest reads it back.
fn f(value: f64) -> f32 {
    value as f32
}

fn deg(value: f64) -> f32 {
    f(value.to_radians())
}

fn mps(kmh: f64) -> f32 {
    f(kmh / 3.6)
}

fn xy(x: f64, y: f64) -> Value {
    json!({"X": x, "Y": y})
}

fn translator(input: [f64; 2], output: [f64; 2]) -> Value {
    json!({"InputRange": xy(input[0], input[1]), "OutputRange": xy(output[0], output[1])})
}

/// A grip table row: the road row of a tyre.
fn tyre_row(peak: f64) -> Value {
    let mut row = serde_json::Map::new();
    let a = [1.1, -9.0, peak, 1100.0, 5.0, 0.003, 0.1, 0.2, -0.02, 0.3, 0.4];
    for (index, value) in a.iter().enumerate() {
        row.insert(format!("A{index}"), json!(value));
    }
    for (name, value) in [("A111", 1.5), ("A112", 0.5), ("A12", 0.25), ("A13", 0.125)] {
        row.insert(name.to_owned(), json!(value));
    }
    for index in 0..=10 {
        row.insert(format!("B{index}"), json!(1.0 + f64::from(index)));
    }
    for index in 0..=17 {
        row.insert(format!("C{index}"), json!(-1.0 - f64::from(index)));
    }
    row.insert("RollingResistanceConstant".to_owned(), json!(0.015));
    row.insert("RollingResistanceVelocity".to_owned(), json!(0.0005));
    row.insert("RollingResistanceSqrVelocity".to_owned(), json!(0.000002));
    row.insert("surface".to_owned(), json!("EPhysicalSurface::SurfaceType1"));
    row.insert("WET".to_owned(), json!(false));
    Value::Object(row)
}

fn wheel(toroid: f64, mass: f64, tyre: &str, extra: Value) -> Value {
    let mut wheel = json!({
        "ShapeIsToroid": true, "ShapeToroidFactor": toroid, "TireCompressionShapeFactor": 1.0, "Mass": mass,
        "Inertia": mass / 30.0, "SteerAngle": 40.0, "SlipCorrection": 0.04, "RestSlipCorrection": 0.2,
        "RestSpeedThreshold": 36.0, "LateralGripLimit": 1.1, "TireStiffness": 350000.0, "MaxTireDeformation": 2.5,
        "TireData": {"ObjectPath": format!("/PhysicsRide6/Physics/Tire/TireData/Fam/{tyre}.0")}
    });
    wheel.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    wheel
}

fn suspension(travel: f64, spring: f64, damper: [f64; 4], extra: Value) -> Value {
    let mut s = json!({
        "MaxCompression": travel, "RestCompressionOffset": 0.01, "SpringStrength": spring, "SpringMaxSpeed": 5.0,
        "TopOutSpringStiffness": 4200.0, "TopOutSpringLength": 0.05, "HydraulicComponentStiffness": 300.0,
        "ProgressiveHydraulicComponentFactor": 1.0, "ProgressiveHydraulicComponentLinearity": 2.0,
        "DamperSlowBump": damper[0], "DamperFastBump": damper[1], "DamperFastBumpThreshold": 0.25,
        "DamperSlowRebound": damper[2], "DamperFastRebound": damper[3], "DamperFastReboundThreshold": 0.25
    });
    s.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    s
}

fn element(kind: &str) -> Value {
    json!({"ObjectPath": format!("/PhysicsRide6/Physics/VehicleElements/{kind}/Synth_{kind}.0")})
}

/// The members of several JSON objects in one (the `json!` macro recurses once per member, so a long object is built in parts).
fn merged(parts: Vec<Value>) -> Value {
    let mut out = serde_json::Map::new();
    for part in parts {
        out.extend(part.as_object().unwrap().clone());
    }
    Value::Object(out)
}

fn handler() -> Value {
    let lean = json!({
        "SimulateGyroscopicEffects": true, "LeanK": 900.0, "LeanDamp": -3.5, "SteerK": 800.0, "SteerDamp": -2.0,
        "HandlebarK": 1200.0, "HandlebarDamp": -3.0, "bUseKDampFilterForTargetLeanAngle": true,
        "TargetLeanAngleFilter": {"KValue": 700.0, "DampValue": -2.25}, "LeanTimeScale": 1.0,
        "LeanTimeScaleSpeedTuning": translator([10.0, 200.0], [1.0, 0.6]),
        "LeanTimeScaleInputTuning": {"Curve": {"ExternalCurve": {"ObjectPath": "/PhysicsRide6/Physics/TuningCurve/Input.0"}}},
        "LeanLimitGForceTuning": {"Curve": {"ExternalCurve": null}},
        "LeanLimitSkidRange": xy(10.0, 40.0), "LeanLimitSkidLinearity": 2.0
    });
    let limits = json!({
        "LeanLimitSkidReduction": translator([0.0, 36.0], [0.0, 0.2]),
        "StoppedLeanLimit": 2.0, "StoppedSpeedThreshold": 36.0, "SteerAlignmentRange": xy(2.0, 4.0), "RiderMassScale": 1.0,
        "LeanHelpFactorSpeedTuning": translator([0.0, 18.0], [0.0, 0.3]),
        "LeanHelpLimitSpeedTuning": translator([0.0, 10.0], [1.0, 0.99]),
        "FallLeanLimitAirborne": 70.0, "FallPitchLimit": 65.0, "FallSpeedDirAngleLimit": 80.0, "FallAngularSpeedLimit": 4.4,
        "FallMinSpeed": 18.0, "JoinedBrakesFactor": 1.0
    });
    let gearbox = json!({
        "GearUpChangeTime": 0.05, "GearDownChangeTime": 0.2, "BlipperThrottleOnShift": 0.25,
        "TransmissionSettings": {"bAutomaticGearChangePoints": true, "GearChangePoints": [
            {"ChangeUpTolerance": 1.0, "ChangeDownTolerance": 0.9},
            {"ChangeUpTolerance": 1.0, "ChangeDownTolerance": 1.0},
            {"ChangeUpTolerance": 1.0, "ChangeDownTolerance": 1.0}
        ]},
        "ClucthReleaseVelocityRange": xy(0.0, 36.0),
        "ClutchFeatheringOnStart": {"bEnable": true, "ClutchFeatheringMaxRPMFactor": 0.6, "ClutchFeatheringPeakTorqueRPMFactor": 0.8}
    });
    let rider = json!({
        "LeanComponent": {
            "ReferenceWeightRange": xy(180.0, 352.0), "ReferenceTensorXRange": xy(0.2, 0.3),
            "ReferenceHeightRange": xy(80.0, 51.0), "ReferenceWheelbaseRange": xy(131.0, 167.0),
            "LeanDampMultiplierRange": xy(1.0, 1.5)
        },
        "bEnableWheelieLeanLimitTuning": true, "WheelieLeanLimitTuning": {"LeanLimit": 2.0, "AngleRange": xy(0.0, 40.0)},
        "bEnableStoppieLeanLimitTuning": true, "StoppieLeanLimitTuning": {"LeanLimit": 3.0, "AngleRange": xy(0.0, 20.0)},
        "Rider": {"SimulateRider": true, "Mass": 75.0, "COMOffset": {"X": -10.0, "Y": 0.0, "Z": 35.0},
                  "BackwardOffset": {"X": -20.0, "Y": 0.0, "Z": 0.0}}
    });
    merged(vec![lean, limits, gearbox, rider])
}

/// The whole synthetic vehicle in the layout of `vehicles/<Brand>/<Vehicle>.json`.
fn vehicle_json() -> Value {
    let rear_extra = json!({"bAffectedByHandbrake": true});
    json!({
        "Name": "Synth_Bike",
        "Properties": {
            "CHASSIS": {"Mass": 100.0, "WeightBalance": 0.48, "CGPosition": {"X": 0.0, "Y": 0.0, "Z": 60.0},
                        "InertiaTensorScale": {"X": 0.25, "Y": 0.5, "Z": 0.4}},
            "Skeleton": {"bOverrideSkeletonData": true, "OverrideWheelBase": 150.0,
                         "RakeTrailOverride": [{"Rake": 25.0, "Trail": 10.0}]},
            "FrontSuspension": suspension(0.25, 9500.0, [1800.0, 1200.0, 2600.0, 1600.0], json!({"BrakeTorqueCouplingFactor": 30.0})),
            "RearSuspension": suspension(0.24, 48000.0, [7000.0, 3000.0, 10000.0, 4500.0], json!({
                "SuspensionLeverageRatio": {"EditorCurveData": {"Points": [[0.0, 2.5], [1.0, 2.0]],
                    "InterpModes": ["RCIM_Cubic"], "Tangents": [[0.0, 0.0], [0.0, 0.0]]}, "ExternalCurve": null},
                "ChainPullFactor": -0.02,
                "ChainPullFactorCompressionFactor": translator([0.0, 1.0], [1.0, 0.5]),
                "TractionInfluenceOnLoad": -0.1, "BrakeTorqueCouplingFactor": 25.0
            })),
            "FrontWheel": wheel(0.8, 17.0, "Front", json!({})),
            "RearWheel": wheel(0.6, 19.0, "Back", rear_extra),
            "Engine": {
                "PeakTorque": 40.0, "MOI": 0.02, "EngineFrictionPeakTorque": 20.0, "NormalizeEngineFrictionCurveDomain": true,
                "MinRPM": 2000.0, "MaxRPM": 10000.0, "ThrottleAtMinimum": 0.1,
                "TorqueCurve": {"EditorCurveData": {"Points": [[0.0, 0.0], [20000.0, 9.0]], "InterpModes": ["RCIM_Linear"]},
                                "ExternalCurve": {"ObjectPath": "/PhysicsRide6/Physics/Engine/Torque.0"}},
                "EngineFrictionCurve": {"ExternalCurve": {"ObjectPath": "/PhysicsRide6/Physics/Engine/Friction.0"}},
                "EngineLimiter": [{"LimiterModulatorTuning": {"DurationRange": xy(0.02, 0.04)}}]
            },
            "Transmission": {
                "ForwardGears": [{"Ratio": 2.5}, {"Ratio": 2.0}, {"Ratio": 1.5}, {"Ratio": 1.2}], "FinalRatio": 6.0,
                "ClutchLinearity": 1.0, "SlipperClutchDeltaTorqueRange": xy(200000.0, 300000.0),
                "SlipperClutchMaxIntensity": 1.0
            },
            "Brake": {"BrakeForce": 1500.0, "HandbrakeForce": 800.0, "BrakeBalance": 1.0},
            "Aerodynamic": {"DragCoefficient": 0.2},
            "InputData": {"WeightLongitudinal": {"bUseDeprecatedRate": false, "InputRate": 5.0}},
            "VehicleElements": [element("InputHandler"), element("RiderHelp"), element("WheelieControl"),
                                element("ABSVehicleElement")]
        },
        "Linked": {
            "/PhysicsRide6/Physics/Tire/TireData/Fam/Front.0": {"Properties": {"TireModel": {"ObjectPath": "/PhysicsRide6/Physics/Tire/GripCurve/Fam/Front.0"}}},
            "/PhysicsRide6/Physics/Tire/TireData/Fam/Back.0": {"Properties": {"TireModel": {"ObjectPath": "/PhysicsRide6/Physics/Tire/GripCurve/Fam/Back.0"}}},
            "/PhysicsRide6/Physics/Tire/GripCurve/Fam/Front.0": {"Rows": {"Tarmac": tyre_row(1300.0), "Grass": tyre_row(700.0)}},
            "/PhysicsRide6/Physics/Tire/GripCurve/Fam/Back.0": {"Rows": {"Tarmac": tyre_row(1400.0)}},
            "/PhysicsRide6/Physics/Engine/Torque.0": {"Properties": {"FloatCurve": {"InterpModes": ["RCIM_Linear"],
                "Points": [[1500.0, 0.6], [4000.0, 0.8], [6000.0, 1.0], [8000.0, 0.9], [10000.0, 0.6]]}}},
            "/PhysicsRide6/Physics/Engine/Friction.0": {"Properties": {"FloatCurve": {"InterpModes": ["RCIM_Linear"],
                "Points": [[0.0, 0.0], [1.0, 1.0], [1.5, 2.0]]}}},
            "/PhysicsRide6/Physics/TuningCurve/Input.0": {"Properties": {"FloatCurve": {"InterpModes": ["RCIM_Linear"],
                "Points": [[-1.0, 0.5], [0.0, 1.0], [1.0, 0.75]]}}},
            "/PhysicsRide6/Physics/TuningCurve/Stoppie.0": {"Properties": {"FloatCurve": {"InterpModes": ["RCIM_Linear"],
                "Points": [[10.0, 0.0], [80.0, 1.0], [300.0, 0.0]]}}},
            "/PhysicsRide6/Physics/VehicleElements/InputHandler/Synth_InputHandler.0": {"Properties": handler()},
            "/PhysicsRide6/Physics/VehicleElements/RiderHelp/Synth_RiderHelp.0": {"Properties": {
                "bEnableRiderHelpThrottleControl": true, "AutoRiderWheelieThrottleControl": xy(0.0, 5.0),
                "AutoRiderForcedWheelieThrottleControl": xy(30.0, 45.0), "PitchSpeedWheelieThrottleControl": xy(0.2, 0.5),
                "bEnableRiderHelpBrakeControl": true, "AutoRiderStoppieBrakeControl": xy(0.0, 5.0),
                "PitchSpeedStoppieBrakeControl": xy(0.0, 0.5)}},
            "/PhysicsRide6/Physics/VehicleElements/WheelieControl/Synth_WheelieControl.0": {"Properties": {
                "WheelieParams": {"InputDeadzone": 0.3, "MaxAngle": 35.0, "MaxAngleFactorSpeedTuning": xy(0.0, 18.0),
                                  "GroundDistanceFactorRange": xy(0.0, 1.0), "HelpIntensityLeanTuning": xy(0.2, 1.0),
                                  "AccelerationTime": 0.1},
                "StoppieParams": {"InputDeadzone": 0.9, "MaxAngle": 45.0, "GroundDistanceFactorRange": xy(0.0, 1.0),
                                  "LookaheadTime": 1.0,
                                  "HelpIntensitySpeedCurve": {"ExternalCurve": {"ObjectPath": "/PhysicsRide6/Physics/TuningCurve/Stoppie.0"}}}}},
            "/PhysicsRide6/Physics/VehicleElements/ABSVehicleElement/Synth_ABSVehicleElement.0": {"Properties": {
                "ABSSpeedTuning": translator([10.0, 25.0], [0.0, 1.0]),
                "bFrontABSEnabled": true, "FrontABSTuning": {"Intensity": 1.0, "NormalizedSlipTuning": translator([-0.3, -0.5], [0.0, 1.0])},
                "bRearABSEnabled": true, "RearABSTuning": {"Intensity": 0.5, "NormalizedSlipTuning": translator([-0.5, -1.0], [0.0, 1.0])}}}
        }
    })
}

fn geometry() -> Geometry {
    let tyre =
        |x: f64, radius: f64, width: f64| TyreGeometry { hub: [x, 0.0, radius], radius_m: radius, width_m: width };
    Geometry { front: tyre(0.75, 0.3, 0.12), rear: tyre(-0.75, 0.31, 0.15), half_m: [0.4, 0.6, 1.0] }
}

fn table() -> BikeTable {
    let vehicle = Vehicle::parse(&vehicle_json().to_string(), None).unwrap();
    render(&vehicle, &geometry()).unwrap()
}

/// The manifest as `main.rs` joins it, from a built bike with the geometry of `geometry()`.
fn manifest_text(table: &BikeTable) -> String {
    let wheel =
        |g: &TyreGeometry| Wheel { hub: g.hub.map(|v| v as f32), radius: g.radius_m as f32, width: g.width_m as f32 };
    let built = Built {
        geometry: vec![],
        textures: vec![],
        bounds: vec![],
        half: [0.4, 0.6, 1.0],
        front: wheel(&geometry().front),
        rear: wheel(&geometry().rear),
        triangles: 0,
        front_end: Default::default(),
    };
    car_toml("Synth Bike", "Synth", &built, &Tables { bike: table, sound: None })
}

#[test]
fn the_manifest_reads_back_with_the_games_schema_and_the_conversions_are_exact() {
    let text = manifest_text(&table());
    let manifest = parse_car("synthbike", &text).unwrap_or_else(|error| panic!("{error}\n{text}"));
    assert_eq!(manifest.kind, CarKind::Bike);
    assert_eq!(manifest.name.as_deref(), Some("Synth Bike"));
    let layout = manifest.wheels.expect("the hubs");
    assert_eq!((layout.front.x_m, layout.rear.x_m, layout.front.face_m), (0.75, -0.75, 0.0));
    assert_eq!((layout.front.z_m, layout.rear.radius_m, layout.rear.width_m), (0.3, 0.31, 0.15));
    let bike = manifest.bike.expect("the bike table");

    // Chassis: centimetres to metres, the axes of the physics space, degrees to radians.
    let chassis = &bike.chassis;
    assert_eq!((chassis.mass_kg, chassis.rider_mass_kg, chassis.front_weight), (100.0, 75.0, 0.48));
    assert_eq!(chassis.rider_offset_m, [0.0, f(35.0 / 100.0), f(-10.0 / 100.0)]);
    assert_eq!(chassis.rider_forward_shift_m, [0.0; 3]);
    assert_eq!(chassis.rider_backward_shift_m, [0.0, 0.0, f(-20.0 / 100.0)]);
    assert_eq!(chassis.rider_weight_rate, 5.0);
    assert_eq!(chassis.half_dimensions_m, [0.4, 0.6, 1.0]);
    assert_eq!(chassis.tensor_scale, [0.5, 0.4, 0.25]);
    assert_eq!((chassis.cg_height_m, chassis.wheelbase_m, chassis.trail_m), (f(0.6), f(1.5), f(0.1)));
    assert_eq!(chassis.rake_rad, deg(25.0));
    assert_eq!((bike.drag, bike.gyro_roll_scale, bike.gyro_yaw_scale), (0.2, 1.0, 1.0));

    // The wheels: the mesh gives radius and width, the data the rest.
    let (front, rear) = (&bike.front, &bike.rear);
    assert_eq!((front.radius_m, front.width_m, rear.radius_m, rear.width_m), (0.3, 0.12, 0.31, 0.15));
    assert_eq!((front.toroid_factor, rear.toroid_factor, front.mass_kg), (0.8, 0.6, 17.0));
    assert_eq!((front.tire_stiffness_n_per_m, front.compression_shape_factor), (350_000.0, 1.0));
    assert_eq!(front.max_tire_deformation_m, f(2.5 / 100.0));
    assert_eq!(front.rest_speed_threshold_mps, mps(36.0));
    assert_eq!(front.rolling, [0.015, 0.0005, 0.000002]);
    assert_eq!(front.tyre.a[2], 1300.0);
    assert_eq!(rear.tyre.a[2], 1400.0);
    // a0..a10, then a111, a112, a12, a13; b0..b10; c0..c17.
    assert_eq!(&front.tyre.a[11..], &[1.5, 0.5, 0.25, 0.125]);
    assert_eq!((front.tyre.b[0], front.tyre.b[10], front.tyre.c[0], front.tyre.c[17]), (1.0, 11.0, -1.0, -18.0));

    // Suspension: metres and newtons per metre as they are, the rear leverage from its cubic curve, never under 1.
    let (fork, shock) = (&bike.front_suspension, &bike.rear_suspension);
    assert_eq!(
        (fork.max_travel_m, fork.spring_n_per_m, fork.top_out_length_m, fork.max_speed_mps),
        (0.25, 9500.0, 0.05, 5.0)
    );
    assert!(fork.leverage.is_empty());
    assert_eq!((fork.hydraulic.stiffness_n_per_m, fork.hydraulic.factor, fork.hydraulic.linearity), (300.0, 1.0, 2.0));
    assert_eq!((fork.damper.slow_bump, fork.damper.fast_rebound, fork.damper.bump_threshold), (1800.0, 1600.0, 0.25));
    assert_eq!((fork.coupling.brake, fork.coupling.brake_range, fork.coupling.chain), (30.0, [0.0, 1.0], 0.0));
    assert_eq!(shock.leverage.len(), 9);
    assert_eq!((shock.leverage[0], shock.leverage[4], shock.leverage[8]), ([0.0, 2.5], [0.5, 2.25], [1.0, 2.0]));
    assert_eq!((shock.coupling.chain, shock.coupling.chain_range), (-0.02, [1.0, 0.5]));
    assert_eq!((shock.coupling.traction, shock.coupling.traction_range), (-0.1, [0.0, 1.0]));

    // Engine: the external curve wins over the stale embedded one.
    let engine = &bike.engine;
    assert_eq!(engine.peak_torque_nm, 40.0);
    assert_eq!(engine.torque_curve.len(), 5);
    assert_eq!(engine.torque_curve[2], [6000.0, 1.0]);
    assert_eq!(engine.friction_curve, vec![[0.0, 0.0], [1.0, 1.0], [1.5, 2.0]]);
    assert_eq!(
        (engine.idle_rpm, engine.max_rpm, engine.moi_kgm2, engine.throttle_at_idle),
        (2000.0, 10_000.0, 0.02, 0.1)
    );
    assert_eq!(engine.limiter_cut_time_s, f(0.03));

    // Gearbox and clutch: Unreal torque units to newton metres, km/h to m/s.
    let gearbox = &bike.transmission;
    assert_eq!((gearbox.gears.clone(), gearbox.final_ratio), (vec![2.5, 2.0, 1.5, 1.2], 6.0));
    assert_eq!(gearbox.slipper_range_nm, [20.0, 30.0]);
    assert_eq!(gearbox.release_speed_mps, mps(36.0));
    assert_eq!((gearbox.feather_factor, gearbox.feather_max_factor), (0.8, 0.6));
    assert_eq!((gearbox.up_shift_time_s, gearbox.down_shift_time_s, gearbox.blip_throttle), (0.05, 0.2, 0.25));
    let down = gearbox.down_shift.expect("a generated down shift");
    assert!(down > 0.1 && down < gearbox.up_shift && gearbox.up_shift <= 1.0, "{down} {}", gearbox.up_shift);

    // Brakes: the front brake is `BrakeForce`, the rear brake the hand-brake channel `HandbrakeForce`.
    let brakes = &bike.brakes;
    assert_eq!((brakes.front_force_n, brakes.rear_force_n, brakes.joined_factor), (1500.0, 800.0, 1.0));
    assert_eq!((brakes.abs.front_intensity, brakes.abs.rear_intensity), (1.0, 0.5));
    assert_eq!(brakes.abs.speed_range_mps, [mps(10.0), mps(25.0)]);
    assert_eq!((brakes.abs.front_slip, brakes.abs.rear_slip), ([-0.3, -0.5], [-0.5, -1.0]));

    // Steering, the lean controller and the fall.
    let steering = &bike.steering;
    assert_eq!((steering.max_angle_rad, steering.response, steering.damping_ratio), (deg(40.0), 1200.0, 3.0));
    assert_eq!(steering.max_lean_rad, deg(70.0));
    let lean = &steering.lean;
    assert_eq!((lean.k, lean.damping_ratio, lean.steer_k, lean.steer_damping_ratio), (900.0, 3.5, 800.0, 2.0));
    assert_eq!((lean.target_k, lean.target_damping_ratio, lean.time_scale), (700.0, 2.25, 1.0));
    assert!(lean.damping_multiplier >= 1.0 && lean.damping_multiplier <= 1.5);
    assert_eq!(lean.time_scale_speed, vec![[mps(10.0), 1.0], [mps(200.0), 0.6]]);
    assert_eq!(lean.time_scale_input, vec![[-1.0, 0.5], [0.0, 1.0], [1.0, 0.75]]);
    assert!(lean.g_tuning.is_empty());
    assert_eq!((lean.skid_range_rad, lean.skid_linearity), ([deg(10.0), deg(40.0)], 2.0));
    assert_eq!(lean.skid_reduction, vec![[0.0, 0.0], [mps(36.0), 0.2]]);
    assert_eq!(
        (lean.stopped_limit_rad, lean.stopped_speed_mps, lean.blend_range_mps),
        (deg(2.0), mps(36.0), [2.0, 4.0])
    );
    assert_eq!(lean.help_speed, vec![[0.0, 0.0], [mps(18.0), 0.3]]);
    assert_eq!(lean.help_limit, 0.99);
    let fall = &bike.fall;
    assert_eq!((fall.lean_rad, fall.pitch_rad, fall.speed_direction_rad), (deg(70.0), deg(65.0), deg(80.0)));
    assert_eq!((fall.angular_speed, fall.min_speed_mps), (4.4, mps(18.0)));

    // The wheelie assist: the asset's fields, the constructor defaults for the rest, the handler's lean limits.
    let assist = bike.wheelie.expect("a wheelie control");
    assert_eq!(
        (assist.throttle_pitch_rad, assist.forced_throttle_pitch_rad),
        ([0.0, deg(5.0)], [deg(30.0), deg(45.0)])
    );
    assert_eq!(
        (assist.throttle_pitch_speed, assist.brake_pitch_rad, assist.brake_pitch_speed),
        ([0.2, 0.5], [0.0, deg(5.0)], [0.0, 0.5])
    );
    let wheelie = &assist.wheelie;
    assert_eq!((wheelie.deadzone, wheelie.max_angle_rad, wheelie.speed_ramp_mps), (0.3, deg(35.0), [0.0, mps(18.0)]));
    assert_eq!((wheelie.min_speed_mps, wheelie.acceleration, wheelie.acceleration_time_s), (mps(15.0), 10.0, 0.1));
    assert_eq!((wheelie.lean_limit_rad, wheelie.lean_pitch_range_rad), (deg(2.0), [0.0, deg(40.0)]));
    assert!(wheelie.help_speed.is_empty());
    let stoppie = &assist.stoppie;
    assert_eq!(
        (stoppie.deadzone, stoppie.speed_ramp_mps, stoppie.acceleration_time_s, stoppie.lookahead_s),
        (0.9, [0.0, 0.0], 0.005, 1.0)
    );
    assert_eq!(stoppie.help_speed, vec![[mps(10.0), 0.0], [mps(80.0), 1.0], [mps(300.0), 0.0]]);
    assert_eq!((stoppie.lean_limit_rad, stoppie.lean_pitch_range_rad), (deg(3.0), [0.0, deg(20.0)]));
}

#[test]
fn a_bike_with_no_wheelie_control_has_no_wheelie_table() {
    let mut json = vehicle_json();
    json["Properties"]["VehicleElements"] = json!([element("InputHandler"), element("ABSVehicleElement")]);
    let vehicle = Vehicle::parse(&json.to_string(), None).unwrap();
    let text = manifest_text(&render(&vehicle, &geometry()).unwrap());
    let bike = parse_car("synthbike", &text).unwrap().bike.unwrap();
    assert!(bike.wheelie.is_none());
}

#[test]
fn a_missing_field_is_named_and_a_brake_balance_under_one_is_refused() {
    let mut json = vehicle_json();
    json["Properties"]["Engine"].as_object_mut().unwrap().remove("MOI");
    let vehicle = Vehicle::parse(&json.to_string(), None).unwrap();
    let error = render(&vehicle, &geometry()).unwrap_err();
    assert!(format!("{error:#}").contains("Properties.Engine") && format!("{error:#}").contains("MOI"), "{error:#}");

    let mut json = vehicle_json();
    json["Properties"]["Brake"]["BrakeBalance"] = json!(0.6);
    let vehicle = Vehicle::parse(&json.to_string(), None).unwrap();
    assert!(render(&vehicle, &geometry()).is_err());
}

#[test]
fn the_dump_folder_finds_a_vehicle_by_its_id() {
    let dir = std::env::temp_dir().join(format!("ride6-bikes-dump-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("vehicles/KTM")).unwrap();
    std::fs::write(dir.join("vehicles/KTM/KTM_450SMR_2022.json"), "{}").unwrap();
    let found = locate(&dir, "KTM_450-SMR-2022").unwrap();
    assert!(found.ends_with("KTM_450SMR_2022.json"));
    assert!(locate(&dir, "KTM_690").is_err());
    assert_eq!(locate(&found, "anything").unwrap(), found);
    std::fs::remove_dir_all(dir).unwrap();
}
