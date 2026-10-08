use glam::Vec3;

use super::spec::VehicleSpec;
use crate::aero::AeroSpec;
use crate::brakes::BrakeSpec;
use crate::drivetrain::TransmissionSpec;
use crate::engine::EngineSpec;
use crate::induction::InductionSpec;
use crate::nos::NosSpec;
use crate::rigid_body::RigidBodySpec;
use crate::suspension::ChassisSpec;
use crate::tires::TireSpec;

impl VehicleSpec {
    /// A plausible rear-drive sports saloon (about 1450 kg, 290 hp, six gears) for tests, examples and
    /// bring-up. The numbers are ordinary engineering values, not the data of any game or car.
    pub fn example() -> Self {
        Self {
            mass: 1450.0,
            dimension: Vec3::new(0.9, 0.62, 2.2),
            tensor_scale: Vec3::ONE,
            body: RigidBodySpec::default(),
            chassis: ChassisSpec {
                spring_stiffness: [800.0, 700.0],
                spring_progression: [2.0, 2.0],
                shock_stiffness: [60.0, 55.0],
                shock_ext_stiffness: [80.0, 75.0],
                shock_valving: [0.0, 0.0],
                shock_digression: [0.0, 0.0],
                shock_blowout: 0.0,
                swaybar_stiffness: [120.0, 100.0],
                travel: [4.0, 4.0],
                ride_height: [4.5, 4.5],
                track_width: [1.55, 1.55],
                wheel_base: 2.7,
                front_axle: 1.4,
                front_weight_bias: 55.0,
                roll_center: 20.0,
            },
            tires: TireSpec {
                rim_size: [17.0, 17.0],
                section_width: [225.0, 235.0],
                aspect_ratio: [45.0, 45.0],
                grip_scale: [1.0, 1.0],
                static_grip: [1.05, 1.05],
                dynamic_grip: [0.95, 0.95],
                steering: 1.0,
                yaw_control: vec![1.0, 0.8, 0.6],
                yaw_speed: 1.0,
            },
            brakes: BrakeSpec { brakes: [260.0, 170.0], brake_lock: [0.9, 0.9], ebrake: 120.0 },
            engine: EngineSpec {
                torque: vec![190.0, 225.0, 250.0, 265.0, 265.0, 250.0, 225.0, 190.0, 150.0],
                idle: 900.0,
                red_line: 7000.0,
                max_rpm: 8000.0,
                flywheel_mass: 20.0,
                engine_braking: vec![0.15],
                speed_limiter: [0.0, 0.0],
            },
            transmission: TransmissionSpec {
                gear_ratio: vec![3.4, 0.0, 3.6, 2.2, 1.5, 1.1, 0.9, 0.75],
                gear_efficiency: vec![0.9; 8],
                final_gear: 3.7,
                torque_split: 0.0,
                differential: [0.0, 0.3, 0.0],
                torque_converter: 0.0,
                clutch_slip: 0.3,
                shift_speed: 0.1,
                optimal_shift: 0.0,
            },
            induction: InductionSpec::default(),
            nos: NosSpec {
                nos_capacity: 5.0,
                torque_boost: 0.4,
                nos_disengage: 0.5,
                recharge_min: 25.0,
                recharge_max: 8.0,
                recharge_min_speed: 20.0,
                recharge_max_speed: 100.0,
            },
            aero: AeroSpec { drag_coefficient: 0.35, aero_coefficient: 0.003, aero_cg: 50.0, ..AeroSpec::default() },
        }
    }
}
