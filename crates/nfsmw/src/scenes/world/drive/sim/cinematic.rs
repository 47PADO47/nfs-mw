//! Temporary AI input mapping preserves the player's original controls, collision role and physical body.

use blackbox_vehicle::ControlConfig;
use blackbox_vehicle::steering::SteeringDevice;

impl super::CarSim {
    pub fn cinematic_control(&mut self, active: bool) {
        if active {
            if self.cinematic_controls.is_none() {
                self.cinematic_controls = Some(self.vehicle.config);
            }
            self.vehicle.config = ControlConfig {
                dead_zone: 0.0,
                auto_reverse: false,
                auto_brake: false,
                automatic: true,
                steering_device: SteeringDevice::Ai,
                manual_clutch: false,
                h_shifter: false,
            };
            return;
        }
        if let Some(previous) = self.cinematic_controls.take() {
            self.vehicle.config = previous;
        }
    }
}
