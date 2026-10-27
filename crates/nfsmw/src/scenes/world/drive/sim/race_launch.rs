//! Human race staging and GO, from docs/specs/player-launch.md.

use blackbox_vehicle::LaunchConfig;

use super::CarSim;

impl CarSim {
    pub(in crate::scenes::world::drive) fn staged_launch_band(&self) -> Option<(f32, f32)> {
        self.vehicle.staged_launch_band()
    }

    /// Called after a human car is placed, including restarts and resets. AI stays unconfigured.
    pub(in crate::scenes::world::drive) fn configure_player_launch(&mut self, cancel_on_shift: bool) {
        self.vehicle.set_launch_config(Some(LaunchConfig {
            band_fraction: 0.25,
            redline_margin: 500.0,
            duration: 4.5,
            speed_cutoff: 26.8218,
            traction_gain: 0.25,
            staging_traction_scale: 0.25,
            cancel_on_shift,
        }));
    }

    pub(in crate::scenes::world::drive) fn set_race_staging(&mut self, held: bool) {
        self.vehicle.set_staging(held);
    }

    pub(in crate::scenes::world::drive) fn launch_from_grid(&mut self) -> bool {
        let launched = self.vehicle.launch();
        self.vehicle.set_staging(false);
        launched
    }
}
