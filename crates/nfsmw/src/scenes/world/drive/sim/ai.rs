//! What a computer driver needs of the car: its controls set-up, a view of the state and gear changes.

use blackbox_vehicle::ControlConfig;
use blackbox_vehicle::drivetrain::{GEAR_FIRST, GEAR_NEUTRAL, GEAR_REVERSE};
use blackbox_vehicle::steering::{ABSOLUTE_MAX_STEERING, SteeringDevice};
use glam::Vec3;

use super::CarSim;

impl CarSim {
    /// Makes the car take its controls from an AI: no dead zone, no automatic reverse or idle braking,
    /// the full steering angle for the input, and the gearbox shifts by itself.
    pub fn configure_ai(&mut self) {
        self.vehicle.config = ControlConfig {
            dead_zone: 0.0,
            auto_reverse: false,
            auto_brake: false,
            automatic: true,
            steering_device: SteeringDevice::Ai,
        };
    }

    /// What an AI driver needs to know of the car.
    pub fn driver_view(&self) -> blackbox_driver::VehicleView {
        let v = &self.vehicle;
        let velocity = v.linear_velocity();
        blackbox_driver::VehicleView {
            position: v.position(),
            forward: v.rotation().z_axis,
            forward_speed: v.forward_speed(),
            planar_speed: Vec3::new(velocity.x, 0.0, velocity.z).length(),
            gear_is_reverse: v.gear() == GEAR_REVERSE,
            max_steer: (ABSOLUTE_MAX_STEERING * v.spec().tires.steering).to_radians(),
            in_shock: false,
            staging: false,
        }
    }

    /// The body's velocity in physics space.
    pub fn velocity(&self) -> Vec3 {
        self.vehicle.linear_velocity()
    }

    pub fn shift_reverse(&mut self) {
        self.vehicle.shift_to(GEAR_REVERSE);
    }

    pub fn shift_first(&mut self) {
        self.vehicle.shift_to(GEAR_FIRST);
    }

    pub fn shift_neutral(&mut self) {
        self.vehicle.shift_to(GEAR_NEUTRAL);
    }
}
