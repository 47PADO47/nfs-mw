//! Racer world-contact steering coefficients; vehicle-steering-assists-aero §6.3.

use blackbox_vehicle::vehicle::WallSteeringConfig;

pub fn wall_steering_config() -> WallSteeringConfig {
    WallSteeringConfig {
        maximum_normal_y: 0.1,
        maximum_closing_speed_squared: 56.25,
        maximum_nose_speed_squared: 6.25,
        end_fraction: 0.75,
        side_epsilon: 1e-6,
        yaw_gain: 0.125,
    }
}
