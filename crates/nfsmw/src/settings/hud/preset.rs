//! What each HUD preset sets. The preset's layer sits under the player's own layer, so a value the player set always
//! wins and the preset fills only the rest (`HudSettings::from_layer`).

use super::{HudLayer, HudPreset, MapMode, NotificationLevel};

impl HudPreset {
    /// The values this preset fills. Every preset covers the same options, so picking one at run time
    /// (`HudSettings::apply_preset`) leaves nothing of the previous one behind.
    pub fn layer(self) -> HudLayer {
        match self {
            Self::Minimal => covered(false, false, NotificationLevel::Important, MapMode::Compact),
            Self::Balanced => covered(
                super::DEFAULT_ADVANCED_TELEMETRY,
                super::DEFAULT_FREE_ROAM_EXTRAS,
                NotificationLevel::default(),
                MapMode::default(),
            ),
            Self::Advanced => covered(true, true, NotificationLevel::All, MapMode::Centered),
        }
    }
}

/// The options a preset covers. Race and pursuit information stay on in every preset: they are the mode's essentials.
fn covered(telemetry: bool, extras: bool, notifications: NotificationLevel, map_mode: MapMode) -> HudLayer {
    HudLayer {
        advanced_telemetry: Some(telemetry),
        free_roam_extras: Some(extras),
        race_info: Some(true),
        pursuit_info: Some(true),
        notifications: Some(notifications),
        map_mode: Some(map_mode),
        ..HudLayer::default()
    }
}
