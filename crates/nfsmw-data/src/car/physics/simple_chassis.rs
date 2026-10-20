//! Original NPC chassis contact-state coefficients; npc-chassis-contact-state spec.

use blackbox_vehicle::vehicle::SimpleChassisConfig;
use glam::Vec3;

pub fn simple_chassis_config() -> SimpleChassisConfig {
    SimpleChassisConfig {
        ride_extra: [-10.0, 0.0, 2.0],
        steering_range: [45.0, 60.0],
        landing_angular_retention: Vec3::new(1.0, 0.5, 1.0),
    }
}
