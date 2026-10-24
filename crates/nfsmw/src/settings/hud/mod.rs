//! The options of the Custom HUD (`docs/hud-settings.md`): a preset, display size and opacity, the map, which
//! information shows, notifications, accessibility, the camera's shake and speed widening, haptics and the HUD's volume.
//! The map's orientation is not here: it is the `minimap` setting ([`MinimapMode`](super::MinimapMode)).
//!
//! Config: the `[custom_hud]` table (the HUD's on/off switch stays the top-level `hud` key). Console:
//! `set custom_hud.<key> <value>`. A number outside its range is clamped; a value of the wrong kind is reported and only
//! that key is ignored.
//!
//! - `kinds`: the preset, map mode, notification level and palette choices.
//! - `preset`: the layer each preset puts under the player's own values.
//! - `config`: reading and writing the `[custom_hud]` table.
//! - `console`: the `custom_hud.<key>` keys of `get` and `set`.

mod config;
#[cfg(test)]
mod config_tests;
mod console;
#[cfg(test)]
mod console_tests;
mod kinds;
mod preset;
#[cfg(test)]
mod tests;

pub use config::{merge_into, read};
pub use console::{flag, get, is_key, set, syntax};
pub use kinds::{HudPalette, HudPreset, MapMode, NotificationLevel};

/// HUD scale, a factor of the theme's size (`scale`).
pub const DEFAULT_SCALE: f32 = 1.0;
pub const SCALE_RANGE: (f32, f32) = (0.75, 1.5);
/// HUD opacity (`opacity`).
pub const DEFAULT_OPACITY: f32 = 0.9;
pub const OPACITY_RANGE: (f32, f32) = (0.2, 1.0);
/// Text size, a factor of the theme's size (`text_size`).
pub const DEFAULT_TEXT_SIZE: f32 = 1.0;
pub const TEXT_SIZE_RANGE: (f32, f32) = (0.8, 1.5);
/// The map's opacity (`map_opacity`): translucent, so the road stays readable under it.
pub const DEFAULT_MAP_OPACITY: f32 = 0.35;
pub const MAP_OPACITY_RANGE: (f32, f32) = (0.1, 0.9);
/// The map's zoom, a factor of its base scale (`map_zoom`).
pub const DEFAULT_MAP_ZOOM: f32 = 1.0;
pub const MAP_ZOOM_RANGE: (f32, f32) = (0.5, 2.0);
/// Camera shake and speed widening are multipliers: 1 keeps the camera as it is, 0 turns the effect off.
pub const DEFAULT_CAMERA_SHAKE: f32 = 1.0;
pub const DEFAULT_FOV_INTENSITY: f32 = 1.0;
/// The HUD's own sounds, 0 to 1 (`hud_volume`).
pub const DEFAULT_HUD_VOLUME: f32 = 0.8;
/// The range of the multipliers and the volume.
pub const UNIT_RANGE: (f32, f32) = (0.0, 1.0);

pub const DEFAULT_ADVANCED_TELEMETRY: bool = false;
pub const DEFAULT_WORLD_MARKERS: bool = true;
pub const DEFAULT_RACE_INFO: bool = true;
pub const DEFAULT_PURSUIT_INFO: bool = true;
pub const DEFAULT_FREE_ROAM_EXTRAS: bool = false;
pub const DEFAULT_HIGH_CONTRAST: bool = false;
pub const DEFAULT_MOTION_REDUCTION: bool = false;
pub const DEFAULT_HAPTICS: bool = true;

/// The options as the settings hold them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudSettings {
    /// The preset the options started from (`preset`).
    pub preset: HudPreset,
    /// HUD scale (`scale`, 0.75 to 1.5).
    pub scale: f32,
    /// HUD opacity (`opacity`, 0.2 to 1).
    pub opacity: f32,
    /// Text size (`text_size`, 0.8 to 1.5).
    pub text_size: f32,
    /// How the road map shows (`map_mode`).
    pub map_mode: MapMode,
    /// The map's opacity (`map_opacity`, 0.1 to 0.9).
    pub map_opacity: f32,
    /// The map's zoom (`map_zoom`, 0.5 to 2).
    pub map_zoom: f32,
    /// Extra telemetry, each value shown only where the game computes it (`advanced_telemetry`).
    pub advanced_telemetry: bool,
    /// Markers over the road for events, checkpoints and the like (`world_markers`).
    pub world_markers: bool,
    /// The race panel: place, lap, checkpoints and gaps (`race_info`).
    pub race_info: bool,
    /// The pursuit panel: heat, units, bounty and cooldown (`pursuit_info`).
    pub pursuit_info: bool,
    /// Cash, bounty and the distance to the next event in free roam (`free_roam_extras`).
    pub free_roam_extras: bool,
    /// Which notifications show (`notifications`).
    pub notifications: NotificationLevel,
    /// Stronger plates and outlines (`high_contrast`).
    pub high_contrast: bool,
    /// The colour palette (`palette`).
    pub palette: HudPalette,
    /// Calmer HUD animation (`motion_reduction`).
    pub motion_reduction: bool,
    /// The camera's contact shake, a multiplier (`camera_shake`, 0 to 1).
    pub camera_shake: f32,
    /// How far the Dynamic FOV camera widens with speed, a multiplier of the widening (`fov_intensity`, 0 to 1).
    pub fov_intensity: f32,
    /// Controller rumble for HUD events (`haptics`).
    pub haptics: bool,
    /// The volume of the HUD's own sounds (`hud_volume`, 0 to 1).
    pub hud_volume: f32,
}

impl Default for HudSettings {
    fn default() -> Self {
        Self::from_layer(HudLayer::default())
    }
}

impl HudSettings {
    /// The values of a merged layer. The chosen preset's layer fills what no layer set, then the defaults.
    pub fn from_layer(layer: HudLayer) -> Self {
        let preset = layer.preset.unwrap_or_default();
        let l = layer.or(preset.layer());
        Self {
            preset,
            scale: clamped(l.scale, DEFAULT_SCALE, SCALE_RANGE),
            opacity: clamped(l.opacity, DEFAULT_OPACITY, OPACITY_RANGE),
            text_size: clamped(l.text_size, DEFAULT_TEXT_SIZE, TEXT_SIZE_RANGE),
            map_mode: l.map_mode.unwrap_or_default(),
            map_opacity: clamped(l.map_opacity, DEFAULT_MAP_OPACITY, MAP_OPACITY_RANGE),
            map_zoom: clamped(l.map_zoom, DEFAULT_MAP_ZOOM, MAP_ZOOM_RANGE),
            advanced_telemetry: l.advanced_telemetry.unwrap_or(DEFAULT_ADVANCED_TELEMETRY),
            world_markers: l.world_markers.unwrap_or(DEFAULT_WORLD_MARKERS),
            race_info: l.race_info.unwrap_or(DEFAULT_RACE_INFO),
            pursuit_info: l.pursuit_info.unwrap_or(DEFAULT_PURSUIT_INFO),
            free_roam_extras: l.free_roam_extras.unwrap_or(DEFAULT_FREE_ROAM_EXTRAS),
            notifications: l.notifications.unwrap_or_default(),
            high_contrast: l.high_contrast.unwrap_or(DEFAULT_HIGH_CONTRAST),
            palette: l.palette.unwrap_or_default(),
            motion_reduction: l.motion_reduction.unwrap_or(DEFAULT_MOTION_REDUCTION),
            camera_shake: clamped(l.camera_shake, DEFAULT_CAMERA_SHAKE, UNIT_RANGE),
            fov_intensity: clamped(l.fov_intensity, DEFAULT_FOV_INTENSITY, UNIT_RANGE),
            haptics: l.haptics.unwrap_or(DEFAULT_HAPTICS),
            hud_volume: clamped(l.hud_volume, DEFAULT_HUD_VOLUME, UNIT_RANGE),
        }
    }

    /// Picks `preset` now: the options it covers take its values, the others stay as they are.
    pub fn apply_preset(&mut self, preset: HudPreset) {
        let chosen = HudLayer { preset: Some(preset), ..preset.layer() };
        *self = Self::from_layer(chosen.or(HudLayer::from(*self)));
    }
}

/// `value` or the default, inside `(low, high)`. A value that is not a number is the default.
fn clamped(value: Option<f32>, default: f32, (low, high): (f32, f32)) -> f32 {
    value.filter(|v| v.is_finite()).unwrap_or(default).clamp(low, high)
}

/// One layer of the options: what a config file set.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HudLayer {
    pub preset: Option<HudPreset>,
    pub scale: Option<f32>,
    pub opacity: Option<f32>,
    pub text_size: Option<f32>,
    pub map_mode: Option<MapMode>,
    pub map_opacity: Option<f32>,
    pub map_zoom: Option<f32>,
    pub advanced_telemetry: Option<bool>,
    pub world_markers: Option<bool>,
    pub race_info: Option<bool>,
    pub pursuit_info: Option<bool>,
    pub free_roam_extras: Option<bool>,
    pub notifications: Option<NotificationLevel>,
    pub high_contrast: Option<bool>,
    pub palette: Option<HudPalette>,
    pub motion_reduction: Option<bool>,
    pub camera_shake: Option<f32>,
    pub fov_intensity: Option<f32>,
    pub haptics: Option<bool>,
    pub hud_volume: Option<f32>,
}

impl HudLayer {
    /// This layer, with `lower` filling in what it leaves unset.
    pub fn or(self, lower: Self) -> Self {
        Self {
            preset: self.preset.or(lower.preset),
            scale: self.scale.or(lower.scale),
            opacity: self.opacity.or(lower.opacity),
            text_size: self.text_size.or(lower.text_size),
            map_mode: self.map_mode.or(lower.map_mode),
            map_opacity: self.map_opacity.or(lower.map_opacity),
            map_zoom: self.map_zoom.or(lower.map_zoom),
            advanced_telemetry: self.advanced_telemetry.or(lower.advanced_telemetry),
            world_markers: self.world_markers.or(lower.world_markers),
            race_info: self.race_info.or(lower.race_info),
            pursuit_info: self.pursuit_info.or(lower.pursuit_info),
            free_roam_extras: self.free_roam_extras.or(lower.free_roam_extras),
            notifications: self.notifications.or(lower.notifications),
            high_contrast: self.high_contrast.or(lower.high_contrast),
            palette: self.palette.or(lower.palette),
            motion_reduction: self.motion_reduction.or(lower.motion_reduction),
            camera_shake: self.camera_shake.or(lower.camera_shake),
            fov_intensity: self.fov_intensity.or(lower.fov_intensity),
            haptics: self.haptics.or(lower.haptics),
            hud_volume: self.hud_volume.or(lower.hud_volume),
        }
    }
}

impl From<HudSettings> for HudLayer {
    /// Every option set, for writing the whole set of values.
    fn from(s: HudSettings) -> Self {
        Self {
            preset: Some(s.preset),
            scale: Some(s.scale),
            opacity: Some(s.opacity),
            text_size: Some(s.text_size),
            map_mode: Some(s.map_mode),
            map_opacity: Some(s.map_opacity),
            map_zoom: Some(s.map_zoom),
            advanced_telemetry: Some(s.advanced_telemetry),
            world_markers: Some(s.world_markers),
            race_info: Some(s.race_info),
            pursuit_info: Some(s.pursuit_info),
            free_roam_extras: Some(s.free_roam_extras),
            notifications: Some(s.notifications),
            high_contrast: Some(s.high_contrast),
            palette: Some(s.palette),
            motion_reduction: Some(s.motion_reduction),
            camera_shake: Some(s.camera_shake),
            fov_intensity: Some(s.fov_intensity),
            haptics: Some(s.haptics),
            hud_volume: Some(s.hud_volume),
        }
    }
}
