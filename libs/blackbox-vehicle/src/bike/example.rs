//! A generic bike for tests and examples: `BikeSpec::example`.

use glam::Vec3;

use super::magic::Mf94;
use super::spec::{
    AbsSpec, BikeSpec, BrakeSpec, ChassisSpec, Coupling, DamperSpec, EngineSpec, FallSpec, Hydraulic, LeanSpec,
    PitchParams, Progressive, SteeringSpec, SuspensionSpec, TransmissionSpec, WheelSpec, WheelieSpec,
};

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

fn wheel(tyre: Mf94, mass: f32) -> WheelSpec {
    WheelSpec {
        radius: 0.32,
        width: 0.12,
        toroid_factor: 0.7,
        mass,
        inertia: 0.65,
        tyre,
        rolling: [0.02, 0.0004, 0.000_001],
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
            chassis: ChassisSpec {
                mass: 113.0,
                rider_mass: 70.0,
                rider_offset: Vec3::new(0.0, 0.4, -0.05),
                rider_weight_rate: 6.0,
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
            },
            transmission: TransmissionSpec {
                gears: vec![2.1, 1.65, 1.35, 1.12, 0.96],
                final_ratio: 7.2,
                efficiency: 1.0,
                up_shift: 0.9,
                down_shift: 0.45,
                up_shift_time: 0.06,
                down_shift_time: 0.15,
                blip_throttle: 0.3,
                clutch_linearity: 1.0,
                slipper_range: [25.0, 40.0],
                slipper_intensity: 1.0,
                release_speed: 50.0 / 3.6,
                feather_factor: 0.85,
                feather_max_factor: 0.7,
                clutch_max_factor: 1.0,
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
            },
            wheelie: Some(WheelieSpec {
                wheelie: pitch(0.3, 35.0, 0.0),
                stoppie: pitch(0.95, 45.0, 1.0),
                throttle_pitch: [0.0, 4.0_f32.to_radians()],
                forced_throttle_pitch: [33.0_f32.to_radians(), 45.0_f32.to_radians()],
                throttle_pitch_speed: [0.2, 0.6],
                brake_pitch: [0.0, 4.0_f32.to_radians()],
                brake_pitch_speed: [0.0, 0.6],
            }),
            gyro_roll_scale: 1.0,
            gyro_yaw_scale: 1.0,
            drag: 0.15,
        }
    }
}
