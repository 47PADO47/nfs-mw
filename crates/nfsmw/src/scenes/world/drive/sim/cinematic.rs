//! Temporary AI ownership of the player's car during a pursuit breaker's cut: the AI's input mapping and the simple
//! chassis, as the original's `EEnableAIPhysics` gives it (the suspension behaviour `SuspensionSimple`,
//! `docs/specs/pursuit-breaker-moment.md` section 5). The player's own controls and chassis come back afterwards; the
//! collision role and the physical body stay.

use blackbox_vehicle::ControlConfig;
use blackbox_vehicle::steering::SteeringDevice;
use blackbox_vehicle::vehicle::SimpleChassisConfig;

/// The player's own controls, chassis and tire choice, kept while the cut owns the car.
pub(super) type Saved = (ControlConfig, Option<SimpleChassisConfig>, bool);

impl super::CarSim {
    pub fn cinematic_control(&mut self, active: bool) {
        if active {
            if self.cinematic_controls.is_none() {
                let chassis = (self.vehicle.config, self.vehicle.simple_chassis(), self.vehicle.simple_tires());
                self.cinematic_controls = Some(chassis);
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
            self.vehicle.set_simple_chassis(Some(nfsmw_data::car::physics::simple_chassis_config()));
            self.vehicle.set_simple_tires(true);
            return;
        }
        if let Some((config, chassis, tires)) = self.cinematic_controls.take() {
            self.vehicle.config = config;
            self.vehicle.set_simple_chassis(chassis);
            self.vehicle.set_simple_tires(tires);
        }
    }
}
