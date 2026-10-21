//! The speed trails setting as the world shows it: the car's wind trail, or the view streaks that follow the same
//! speed curve (`vehicle_effects::streak_strength`).

use super::WorldScene;
use super::vehicle_effects;
use crate::settings::SpeedTrailStyle;

impl WorldScene {
    /// Whether the speed trails setting is on as the car's wind trail.
    pub(super) fn car_trails(&self) -> bool {
        self.vehicle_effects[1] && self.speed_trail_style == SpeedTrailStyle::Car
    }

    /// Gives the drive the collision sparks and the car's wind trail, as the settings say.
    pub(super) fn apply_vehicle_effects(&mut self) {
        let [sparks, _] = self.vehicle_effects;
        self.impact_fx.set_enabled(sparks);
        let trails = self.car_trails();
        if let Some(drive) = self.drive.as_mut() {
            drive.vehicle_effects.set_enabled(sparks, trails);
        }
    }

    /// The view streaks' strength now, 0 to 0.75: only with the setting on as streaks, and from the car's speed.
    pub(super) fn speed_streak_strength(&self) -> f32 {
        let streaks = self.vehicle_effects[1] && self.speed_trail_style == SpeedTrailStyle::Streaks;
        if !streaks {
            return 0.0;
        }
        self.drive.as_ref().map_or(0.0, |drive| vehicle_effects::streak_strength(drive.speed()))
    }
}
