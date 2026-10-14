//! Original racer jump coefficients; terrain reference acquisition is a separate world query.

use blackbox_vehicle::vehicle::{AlignmentConfig, JumpConfig, LandingConfig, PassiveConfig};

pub fn jump_config() -> JumpConfig {
    JumpConfig {
        extra_airborne_acceleration: 3.0,
        landing: LandingConfig {
            acceleration: 10.0,
            minimum_time: 1.0,
            minimum_peak_altitude: 2.0,
            maximum_altitude: 6.0,
            minimum_alignment: 0.96,
            maximum_speed: 20.0,
        },
        passive: PassiveConfig {
            minimum_up: 0.8,
            maximum_grounded_wheels: 2,
            maximum_speed: 5.0,
            damping: &[(0.0, 0.0), (0.4, 0.15), (2.0, 5.0)],
        },
        alignment: AlignmentConfig {
            minimum_ground_up: 0.9,
            minimum_body_up: 0.1,
            minimum_alignment: 0.8,
            minimum_altitude: 1e-6,
            maximum_altitude: 15.0,
            maximum_speed: 5.0,
            correction_threshold: 0.99,
            pitch: 40.0,
            roll: 20.0,
        },
    }
}
