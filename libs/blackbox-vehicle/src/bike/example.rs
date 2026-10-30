//! A generic bike for tests and examples: `BikeSpec::example`.

use glam::Vec3;

use super::magic::Mf94;
use super::rider_spec::RiderSpec;
use super::spec::{
    AbsLevelSpec, AbsSpec, AutoRiderSpec, BikeSpec, BrakeSpec, ChassisSpec, Coupling, DamperSpec, EngineSpec, FallSpec,
    FeatherSpec, Hydraulic, LeanSpec, PitchParams, Progressive, RiderHelpSpec, SteeringSpec, SuspensionSpec, TcsSpec,
    TransmissionSpec, TyreSection, TyreThermal, WheelSpec, WheelieSpec,
};
use super::spec::{FallExtra, HelpExtra, StabilitySpec, TerrainLimit};

fn tyre(peak: f32) -> Mf94 {
    let mut a = [0.0; 15];
    a[0] = 1.2;
    a[1] = -10.0;
    a[2] = peak;
    a[3] = 1200.0;
    a[4] = 6.0;
    a[5] = 0.002;
    a[8] = -0.01;
    let mut b = [0.0; 11];
    b[0] = 1.5;
    b[1] = -10.0;
    b[2] = 1250.0;
    b[4] = 200.0;
    b[6] = -0.1;
    b[8] = 0.1;
    Mf94 { a, b, c: [0.0; 18] }
}

fn section() -> TyreSection {
    TyreSection {
        efficiency_limit: 0.9,
        static_coef: 0.009,
        dynamic_coef: 0.02,
        lat_slip_coef: 1e-5,
        long_slip_coef: 1e-5,
        rolling_coef: 0.0002,
        temperature_to_wear: vec![(88.0, 0.95), (105.0, 0.945), (150.0, 0.0)],
        wear_to_efficiency: vec![(0.1, 0.985), (0.49, 0.985), (0.855, 0.98), (0.917, 0.968), (1.0, 0.94)],
        temperature_to_efficiency: vec![(60.3, 0.975), (90.1, 1.0), (105.0, 1.0), (115.0, 0.975)],
    }
}

fn thermal() -> TyreThermal {
    TyreThermal {
        sections: vec![section(), section(), section()],
        sections_dependency: 1.0,
        sections_angle: 1.0,
        heat_transfer: 0.0,
        initial_thickness: 1.1,
        min_temperature: 0.0,
        max_temperature: 200.0,
        asphalt_importance: 1.0,
        wear_length: 5000.0,
        uniform: true,
        static_time_scale: 1.0,
        dynamic_time_scale: 1.0,
        decay_distance_factor: 1.0,
        wear_intensity: 1.0,
    }
}

fn wheel(tyre: Mf94, mass: f32) -> WheelSpec {
    WheelSpec {
        radius: 0.32,
        width: 0.12,
        toroid_factor: 0.7,
        mass,
        inertia: 0.65,
        tyre,
        rolling: [0.02, 0.0004, 0.000_001],
        rolling_slip: 0.2,
        rolling_slip_linearity: 1.0,
        thermal: Some(thermal()),
        lateral_grip_limit: 1.25,
        tire_stiffness: 400_000.0,
        max_tire_deformation: 0.03,
        compression_shape_factor: 1.0,
        slip_correction: 0.05,
        rest_slip_correction: 0.25,
        rest_speed_threshold: 25.0 / 3.6,
    }
}

fn lean() -> LeanSpec {
    LeanSpec {
        k: 1000.0,
        damping_ratio: 3.6,
        damping_multiplier: 1.0,
        steer_k: 1000.0,
        steer_damping_ratio: 2.5,
        target_k: 1000.0,
        target_damping_ratio: 2.25,
        time_scale: 1.0,
        time_scale_attenuation: 0.0,
        time_scale_sensitivity: 0.0,
        time_scale_speed: vec![(10.0 / 3.6, 1.0), (200.0 / 3.6, 0.65)],
        time_scale_input: vec![(-1.0, 0.425), (0.0, 1.0), (1.0, 0.7)],
        g_tuning: vec![(0.0, 1.0), (1.1, 0.9)],
        skid_range: [10.0_f32.to_radians(), 40.0_f32.to_radians()],
        skid_linearity: 1.8,
        skid_reduction: vec![(0.0, 0.0), (30.0 / 3.6, 0.2)],
        stopped_limit: 1.0_f32.to_radians(),
        stopped_speed: 30.0 / 3.6,
        blend_range: [2.0, 4.0],
        rider_mass_scale: 1.0,
        help_speed: vec![(0.0, 0.0), (10.0 / 3.6, 0.2)],
        help_limit: 1.0,
        stability: StabilitySpec::default(),
        terrain: TerrainLimit {
            speed: vec![(10.0 / 3.6, 0.0), (50.0 / 3.6, 1.0)],
            max_angle: 15.0_f32.to_radians(),
            increment: 0.666_667,
            reduction: 1.0,
        },
        help_extra: HelpExtra {
            stopped_range: [0.0, 10.0 / 3.6],
            balance_speed: vec![(10.0 / 3.6, 0.0), (30.0 / 3.6, 1.0)],
            skid_range: [10.0_f32.to_radians(), 45.0_f32.to_radians()],
            skid_linearity: 2.0,
            skid_reduction: vec![(0.0, 0.0), (30.0 / 3.6, 1.0)],
            skid_input: vec![(-0.1, 0.0), (0.0, 1.0)],
        },
    }
}

/// The ABS level of the example bike: the curves of the reference's 5-speed supermoto class.
fn abs_level() -> AbsLevelSpec {
    AbsLevelSpec {
        default: 1.0,
        front: vec![(0.0, 0.1), (0.25, 0.2), (0.5, 0.45), (0.75, 1.0), (1.0, 1.0)],
        rear: Vec::new(),
        rear_enabled: vec![(0.0, 0.0), (0.25, 0.0), (0.5, 1.0), (0.75, 1.0), (1.0, 1.0)],
        brake_relief: vec![(0.0, 1.0), (0.25, 0.75), (0.5, 0.5), (0.75, 0.0), (1.0, 0.0)],
    }
}

/// The rider help of the example bike: the throttle cut relaxes below 20 km/h, the cuts act through the engine, and the
/// auto-rider moves the weight against a lift of 3.6 to 4 degrees.
fn rider_help() -> RiderHelpSpec {
    RiderHelpSpec {
        launch_speed: 20.0 / 3.6,
        launch_throttle_multiplier: 0.0,
        throttle_pitch_electronic: true,
        throttle_speed_electronic: true,
        intensity: 1.0,
        weight_deadzone: 0.75,
        lookahead: 0.2,
        auto_rider: Some(AutoRiderSpec {
            wheelie_range: [3.6_f32.to_radians(), 4.0_f32.to_radians()],
            stoppie_range: [3.6_f32.to_radians(), 4.0_f32.to_radians()],
            throttle_curve: vec![(0.0, 0.0), (0.1, 1.0)],
            brake_curve: vec![(0.0, 0.0), (0.1, -1.0)],
            engage_speed: 5.0,
            manual_engage_speed: 5.0,
            ..AutoRiderSpec::default()
        }),
        ..RiderHelpSpec::default()
    }
}

fn pitch(deadzone: f32, max_angle: f32, lookahead: f32) -> PitchParams {
    PitchParams {
        deadzone,
        max_angle: max_angle.to_radians(),
        speed_ramp: [0.0, 15.0 / 3.6],
        help_speed: Vec::new(),
        lean_help: [0.2, 1.0],
        lookahead,
        min_speed: 15.0 / 3.6,
        acceleration: 10.0,
        acceleration_time: 0.1,
        ground_range: [0.0, 1.0],
        lean_limit: 1.0_f32.to_radians(),
        lean_pitch_range: [0.0, 40.0_f32.to_radians()],
        terrain_influence: 1.0,
        filter_timescale: 0.0,
    }
}

impl BikeSpec {
    /// A generic 450 cc supermoto-class bike with its rider: about 113 kg and 70 kg, a 1.47 m wheelbase, 17 inch wheels, a
    /// 5-speed box.
    pub fn example() -> Self {
        let damper = DamperSpec {
            slow_bump: 1600.0,
            fast_bump: 1250.0,
            bump_threshold: 0.3,
            slow_rebound: 2400.0,
            fast_rebound: 1650.0,
            rebound_threshold: 0.3,
        };
        Self {
            rider: RiderSpec::default(),
            chassis: ChassisSpec {
                mass: 113.0,
                rider_mass: 70.0,
                rider_offset: Vec3::new(0.0, 0.4, -0.05),
                rider_weight_rate: 6.0,
                rider_k: 20_000.0,
                rider_damp: 5_000.0,
                rider_forward_shift: Vec3::new(0.0, 0.0, 0.1),
                rider_backward_shift: Vec3::new(0.0, 0.0, -0.1),
                half_dimensions: Vec3::new(0.42, 0.64, 1.06),
                tensor_scale: Vec3::new(0.4, 0.45, 0.2),
                cg_height: 0.62,
                front_weight: 0.5,
                wheelbase: 1.47,
                rake: 26.0_f32.to_radians(),
                trail: 0.105,
            },
            front: wheel(tyre(1330.0), 17.0),
            rear: wheel(tyre(1360.0), 19.0),
            front_suspension: SuspensionSpec {
                max_travel: 0.28,
                spring: 8800.0,
                top_out_stiffness: 3800.0,
                top_out_length: 0.06,
                damper,
                leverage: Vec::new(),
                rest_offset: 0.012,
                progressive: Progressive::default(),
                hydraulic: Hydraulic {
                    stiffness: 400.0,
                    term: Progressive { start: 0.0, factor: 1.0, linearity: 2.0 },
                },
                max_speed: 5.0,
                coupling: Coupling { brake: 35.0, brake_range: [0.0, 1.0], ..Coupling::default() },
            },
            rear_suspension: SuspensionSpec {
                max_travel: 0.26,
                spring: 48000.0,
                top_out_stiffness: 17000.0,
                top_out_length: 0.035,
                damper: DamperSpec {
                    slow_bump: 7500.0,
                    slow_rebound: 10500.0,
                    bump_threshold: 0.2,
                    rebound_threshold: 0.2,
                    ..damper
                },
                leverage: vec![(0.0, 2.8), (1.0, 1.97)],
                rest_offset: 0.01,
                progressive: Progressive::default(),
                hydraulic: Hydraulic {
                    stiffness: 200.0,
                    term: Progressive { start: 0.0, factor: 1.0, linearity: 2.8 },
                },
                max_speed: 5.0,
                coupling: Coupling {
                    chain: -0.014,
                    chain_range: [1.0, 0.6],
                    traction: -0.15,
                    traction_range: [1.0, -0.4],
                    brake: 30.0,
                    brake_range: [0.0, 1.0],
                },
            },
            engine: EngineSpec {
                peak_torque: 46.0,
                torque_curve: vec![(2300.0, 0.7), (5000.0, 0.85), (7500.0, 1.0), (9500.0, 0.9), (11000.0, 0.7)],
                friction_torque: 12.0,
                friction_curve: vec![(0.0, 0.0), (1.0, 1.0), (1.5, 2.0)],
                idle_rpm: 2300.0,
                max_rpm: 11000.0,
                moi: 0.015,
                throttle_at_idle: 0.15,
                limiter_cut_time: 0.02,
                brake_reduction: super::engine_brake::EngineBrakeReduction {
                    factor: 1.0,
                    load: super::brake_reduction::RangeMap { input: [1.0, 0.0], output: [1.0, 0.9] },
                    slip: super::brake_reduction::RangeMap { input: [0.0, -0.33], output: [0.0, 1.0] },
                    ..super::engine_brake::EngineBrakeReduction::default()
                },
            },
            transmission: TransmissionSpec {
                gears: vec![2.1, 1.65, 1.35, 1.12, 0.96],
                final_ratio: 7.2,
                efficiency: 1.0,
                up_tolerance: vec![1.0; 5],
                down_tolerance: vec![0.4, 0.9, 1.0, 1.0, 1.0],
                prevent_up_lean: [0.5, 30.0],
                prevent_down_lean: [0.5, 50.0],
                prevent_down_throttle: [1.0, 0.5],
                up_shift_time: 0.06,
                down_shift_time: 0.15,
                blip_throttle: 0.3,
                clutch_linearity: 1.0,
                slipper_range: [25.0, 40.0],
                slipper_intensity: 1.0,
                release_speed: 50.0 / 3.6,
                feather_factor: 0.85,
                feather_max_factor: 0.7,
                feather: FeatherSpec { delta_time: 0.05, ..FeatherSpec::default() },
                launch_rpm: [0.4, 0.98],
                clutch_max_factor: 1.0,
                control: super::clutch_control::ClutchControl {
                    clutch_timing: [0.125, 0.2],
                    throttle_timing: [0.05, 0.9],
                    modulate_upshift_throttle: false,
                    forced_downshift: super::clutch_control::ForcedDownshift {
                        brake_range: [0.5, 1.0],
                        duration: 0.3,
                        factor: 1.0,
                        linearity: 3.0,
                        speed_range: [150.0 / 3.6, 150.0 / 3.6],
                    },
                    ..super::clutch_control::ClutchControl::default()
                },
            },
            brakes: BrakeSpec {
                front_force: 1800.0,
                rear_force: 1000.0,
                joined_factor: 1.0,
                joined_front: 0.0,
                joined_rear: 0.0,
                abs: AbsSpec {
                    front_intensity: 1.0,
                    rear_intensity: 1.0,
                    speed_range: [10.0 / 3.6, 25.0 / 3.6],
                    front_slip: [-0.3, -0.5],
                    rear_slip: [-0.5, -1.0],
                    reduction: super::brake_reduction::BrakeReduction::default(),
                },
            },
            steering: SteeringSpec {
                max_angle: 0.7,
                response: 1000.0,
                damping_ratio: 3.0,
                max_lean: 70.0_f32.to_radians(),
                lean: lean(),
            },
            fall: FallSpec {
                lean: 70.0_f32.to_radians(),
                pitch: 65.0_f32.to_radians(),
                speed_direction: 85.0_f32.to_radians(),
                angular_speed: 4.4,
                min_speed: 10.0 / 3.6,
                block_ratio: 0.975,
                block_min_speed: 40.0 / 3.6,
                block_time: 1.0,
                block_wait: 5.0,
                extra: FallExtra {
                    lean_by_grip: true,
                    lean_rise: 0.025,
                    lean_fall: 0.01,
                    lean_duration: 0.01,
                    airborne_lean: 70.0_f32.to_radians(),
                    airborne_pitch: 65.0_f32.to_radians(),
                    airborne_angular: 7.0,
                    terrain_influence: 1.0,
                    terrain_reduction: 1.0,
                    terrain_max_delta: 50.0_f32.to_radians(),
                    highside_lean: 70.0_f32.to_radians(),
                },
            },
            wheelie: Some(WheelieSpec {
                wheelie: pitch(0.3, 35.0, 0.0),
                stoppie: PitchParams { terrain_influence: 0.0, filter_timescale: 2.5, ..pitch(0.95, 45.0, 1.0) },
                throttle_pitch: [0.0, 4.0_f32.to_radians()],
                forced_throttle_pitch: [33.0_f32.to_radians(), 45.0_f32.to_radians()],
                throttle_pitch_speed: [0.2, 0.6],
                brake_pitch: [0.0, 4.0_f32.to_radians()],
                brake_pitch_speed: [0.0, 0.6],
                help: rider_help(),
            }),
            abs_level: abs_level(),
            tcs: Some(TcsSpec { slip: [0.09, 0.2], speed_range: [0.0, 5.0 / 3.6] }),
            input: super::input_axes::InputRates::default(),
            gyro_roll_scale: 1.0,
            gyro_yaw_scale: 1.0,
            drag: 0.15,
            reverse: None,
        }
    }
}
