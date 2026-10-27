use super::Vehicle;
use crate::drivetrain::GEAR_FIRST;
use crate::launch::{LaunchConfig, LaunchInput};

impl Vehicle {
    /// Activates caller-supplied race launch rules and clears previous launch state.
    pub fn set_launch_config(&mut self, config: Option<LaunchConfig>) {
        self.launch.configure(config);
        self.powertrain.set_launch_context(false, 0.0);
    }

    pub fn set_staging(&mut self, staging: bool) {
        self.launch.set_staging(staging);
    }

    pub fn staging(&self) -> bool {
        self.launch.staging()
    }

    pub fn launch(&mut self) -> bool {
        self.launch.launch()
    }

    pub fn launch_strength(&self) -> f32 {
        self.launch.strength()
    }

    pub fn staged_launch_band(&self) -> Option<(f32, f32)> {
        if !self.staging() {
            return None;
        }
        let engine = self.powertrain.engine_spec();
        self.launch.rpm_band(
            self.powertrain.gear() == GEAR_FIRST,
            engine.idle,
            engine.red_line,
            self.powertrain.peak_torque().1,
        )
    }

    pub(super) fn update_launch(&mut self, dt: f32, speed: f32) {
        let engine = self.powertrain.engine_spec();
        self.launch.update(&LaunchInput {
            dt,
            first_gear: self.powertrain.gear() == GEAR_FIRST,
            rpm: self.powertrain.rpm_display(),
            idle: engine.idle,
            redline: engine.red_line,
            peak_torque_rpm: self.powertrain.peak_torque().1,
            speed,
            shifting: self.powertrain.shifting(),
        });
        self.powertrain.set_launch_context(self.launch.staging(), self.launch.strength());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VehicleSpec;

    #[test]
    fn active_launch_adds_tire_traction_and_placement_clears_its_state() {
        let mut car = Vehicle::new(VehicleSpec::example());
        car.set_launch_config(Some(LaunchConfig {
            band_fraction: 0.25,
            redline_margin: 500.0,
            duration: 4.5,
            speed_cutoff: 26.8218,
            traction_gain: 0.25,
            staging_traction_scale: 0.25,
            cancel_on_shift: false,
        }));
        car.set_staging(true);
        car.launch.update(&LaunchInput {
            dt: 1.0 / 144.0,
            first_gear: true,
            rpm: 5875.0,
            idle: 1000.0,
            redline: 8000.0,
            peak_torque_rpm: 5000.0,
            speed: 0.0,
            shifting: false,
        });
        car.begin_tires(0.0);
        let ordinary = car.tires[0].traction_boost;
        assert!(car.launch());
        car.set_staging(false);
        car.begin_tires(0.0);
        for tire in &car.tires {
            assert!((tire.traction_boost - ordinary - 0.1875).abs() < 1e-6);
        }
        car.place(glam::Vec3::ZERO, glam::Quat::IDENTITY);
        assert!(!car.staging());
        assert_eq!(car.launch_strength(), 0.0);
        assert!(!car.launch());
        car.begin_tires(0.0);
        assert_eq!(car.tires[0].traction_boost, ordinary);
    }

    #[test]
    fn staging_scales_all_tires_after_other_traction_assists_without_a_drag_context() {
        let mut car = Vehicle::new(VehicleSpec::example());
        car.set_launch_config(Some(LaunchConfig {
            band_fraction: 0.25,
            redline_margin: 500.0,
            duration: 4.5,
            speed_cutoff: 26.8218,
            traction_gain: 0.25,
            staging_traction_scale: 0.25,
            cancel_on_shift: false,
        }));
        car.controls.steering = 0.3;
        car.controls.gas = 0.4;
        car.maneuver.front_grip_gain = 0.5;
        car.powertrain.nos.boost = 2.0;
        let mut ordinary = car.clone();
        ordinary.begin_tires(15.0);
        ordinary.tune_tires(1.0 / 144.0, 15.0, glam::Vec3::ZERO);
        car.set_staging(true);
        car.begin_tires(15.0);
        car.tune_tires(1.0 / 144.0, 15.0, glam::Vec3::ZERO);
        for (staged, normal) in car.tires.iter().zip(&ordinary.tires) {
            assert!((staged.traction_boost - normal.traction_boost * 0.25).abs() < 1e-6);
            assert_eq!(staged.lateral_boost, normal.lateral_boost);
        }
        car.set_launch_config(None);
        car.set_staging(true);
        assert_eq!(car.launch.staging_traction_scale(), 1.0);
    }
}
