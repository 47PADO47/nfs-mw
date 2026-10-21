//! Original traffic/trailer coefficients; traffic-suspension-runtime spec.

use blackbox_vehicle::VehicleSpec;
use blackbox_vehicle::tires::RollingTireResponse;
use blackbox_vehicle::vehicle::RollingChassisConfig;
use glam::Vec3;

pub fn traffic_chassis_config(spec: &VehicleSpec) -> RollingChassisConfig {
    RollingChassisConfig {
        tires: RollingTireResponse::Linear { gain: 2.0, corner_drag: 0.5 },
        steering_degrees: 45.0,
        force_arm_height: 0.5,
        front_drive_share: 1.0 - spec.transmission.torque_split,
        handbrake_front: false,
        cog: None,
        load_from_up: false,
    }
}

pub fn trailer_cog(spec: &VehicleSpec) -> Vec3 {
    let ride = spec.chassis.ride_height[0].max(spec.chassis.ride_height[1]) * 0.0254;
    Vec3::new(0.0, spec.chassis.roll_center * 0.0254 - (spec.dimension.y + ride), 0.0)
}

pub fn trailer_chassis_config(spec: &VehicleSpec) -> RollingChassisConfig {
    RollingChassisConfig {
        tires: RollingTireResponse::Curve {
            samples: [0.0, 0.4, 0.8, 1.2, 1.6, 1.75, 1.65],
            range: 1.0 / 3.0,
            rear_scale: [1.5, 0.75],
            corner_drag: 0.5,
        },
        steering_degrees: 0.0,
        force_arm_height: 1.0,
        front_drive_share: 0.0,
        handbrake_front: true,
        cog: Some(trailer_cog(spec)),
        load_from_up: true,
    }
}
