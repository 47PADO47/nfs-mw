//! The rows of the HUD options page (`docs/specs/custom-hud/menus.md` section 4): one per `[custom_hud]` key, with the
//! map's orientation (`minimap`) among them, in the spec's order. The page shows in every style: the camera, vibration
//! and volume rows apply to every look, the rest to the Custom style (the page's note says so under another style).
//!
//! Each row changes `Settings::hud_options` at once, so the HUD and the menus follow on the next frame, and records the
//! key in the changes the front end writes to the `[custom_hud]` table. Numbers step in fixed amounts and stop at their
//! range's ends; choices cycle; switches flip. Picking a preset applies its six options at once
//! (`HudSettings::apply_preset`), and records them with it, so the file reads back as the page shows.

use crate::frontend::ids::LABEL_OFF;
use crate::i18n::Args;
use crate::i18n::fmt::percent;
use crate::settings::hud::{
    HudLayer, HudPalette, HudPreset, HudSettings, MAP_OPACITY_RANGE, MAP_ZOOM_RANGE, MapMode, NotificationLevel,
    OPACITY_RANGE, SCALE_RANGE, TEXT_SIZE_RANGE, UNIT_RANGE,
};
use crate::settings::{MinimapMode, Partial, Settings, UiStyle};

use super::steps::cycle;
use super::{Control, Data, Title, message, on_off};

/// One row of the HUD page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HudSetting {
    Preset,
    Scale,
    Opacity,
    TextSize,
    MapMode,
    MapOpacity,
    MapZoom,
    /// The map's orientation: the `minimap` setting, North up (`fixed`) or Heading up (`rotating`).
    MapOrientation,
    AdvancedTelemetry,
    WorldMarkers,
    RaceInfo,
    PursuitInfo,
    FreeRoamExtras,
    Notifications,
    HighContrast,
    Palette,
    MotionReduction,
    CameraShake,
    FovIntensity,
    Haptics,
    HudVolume,
}

/// The step and range of a number row.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Steps {
    step: f32,
    range: (f32, f32),
}

const PRESETS: [HudPreset; 3] = [HudPreset::Minimal, HudPreset::Balanced, HudPreset::Advanced];
/// The map modes the row cycles through. Expanded is the `MapExpand` action's view, not a choice of the page; a file
/// that says `expanded` reads as Centered here.
const MAP_MODES: [MapMode; 3] = [MapMode::Off, MapMode::Compact, MapMode::Centered];
const NOTIFICATIONS: [NotificationLevel; 3] =
    [NotificationLevel::All, NotificationLevel::Important, NotificationLevel::Off];
const PALETTES: [HudPalette; 3] = [HudPalette::Default, HudPalette::Deuteranopia, HudPalette::Tritanopia];
const ORIENTATIONS: [MinimapMode; 2] = [MinimapMode::Fixed, MinimapMode::Rotating];

impl HudSetting {
    /// Every row, in the page's order (speed units sit after the map's orientation, `options_rows.rs`).
    pub const ALL: [HudSetting; 21] = [
        Self::Preset,
        Self::Scale,
        Self::Opacity,
        Self::TextSize,
        Self::MapMode,
        Self::MapOpacity,
        Self::MapZoom,
        Self::MapOrientation,
        Self::AdvancedTelemetry,
        Self::WorldMarkers,
        Self::RaceInfo,
        Self::PursuitInfo,
        Self::FreeRoamExtras,
        Self::Notifications,
        Self::HighContrast,
        Self::Palette,
        Self::MotionReduction,
        Self::CameraShake,
        Self::FovIntensity,
        Self::Haptics,
        Self::HudVolume,
    ];

    pub fn title(self) -> Title {
        Title::Key(match self {
            Self::Preset => "options-hud-preset",
            Self::Scale => "options-hud-scale",
            Self::Opacity => "options-hud-opacity",
            Self::TextSize => "options-hud-text-size",
            Self::MapMode => "options-hud-map-mode",
            Self::MapOpacity => "options-hud-map-opacity",
            Self::MapZoom => "options-hud-map-zoom",
            Self::MapOrientation => "options-hud-map-orientation",
            Self::AdvancedTelemetry => "options-hud-advanced-telemetry",
            Self::WorldMarkers => "options-hud-world-markers",
            Self::RaceInfo => "options-hud-race-info",
            Self::PursuitInfo => "options-hud-pursuit-info",
            Self::FreeRoamExtras => "options-hud-free-roam-extras",
            Self::Notifications => "options-hud-notifications",
            Self::HighContrast => "options-hud-high-contrast",
            Self::Palette => "options-hud-palette",
            Self::MotionReduction => "options-hud-motion-reduction",
            Self::CameraShake => "options-hud-camera-shake",
            Self::FovIntensity => "options-hud-fov-intensity",
            Self::Haptics => "options-hud-haptics",
            Self::HudVolume => "options-hud-volume",
        })
    }

    /// The step and range of a number row, or `None` for a choice or a switch.
    fn steps(self) -> Option<Steps> {
        let (step, range) = match self {
            Self::Scale => (0.05, SCALE_RANGE),
            Self::Opacity => (0.1, OPACITY_RANGE),
            Self::TextSize => (0.1, TEXT_SIZE_RANGE),
            Self::MapOpacity => (0.05, MAP_OPACITY_RANGE),
            Self::MapZoom => (0.25, MAP_ZOOM_RANGE),
            Self::CameraShake | Self::FovIntensity | Self::HudVolume => (0.1, UNIT_RANGE),
            _ => return None,
        };
        Some(Steps { step, range })
    }

    /// The value of a number row.
    fn number(self, s: &Settings) -> Option<f32> {
        let h = &s.hud_options;
        Some(match self {
            Self::Scale => h.scale,
            Self::Opacity => h.opacity,
            Self::TextSize => h.text_size,
            Self::MapOpacity => h.map_opacity,
            Self::MapZoom => h.map_zoom,
            Self::CameraShake => h.camera_shake,
            Self::FovIntensity => h.fov_intensity,
            Self::HudVolume => h.hud_volume,
            _ => return None,
        })
    }

    /// The value of a switch row.
    fn switch(self, s: &Settings) -> Option<bool> {
        let h = &s.hud_options;
        Some(match self {
            Self::AdvancedTelemetry => h.advanced_telemetry,
            Self::WorldMarkers => h.world_markers,
            Self::RaceInfo => h.race_info,
            Self::PursuitInfo => h.pursuit_info,
            Self::FreeRoamExtras => h.free_roam_extras,
            Self::HighContrast => h.high_contrast,
            Self::MotionReduction => h.motion_reduction,
            Self::Haptics => h.haptics,
            _ => return None,
        })
    }

    /// The 0 to 1 rows are sliders, like the volumes; the other numbers are steppers.
    pub fn control(self, s: &Settings) -> Control {
        let slider = matches!(self, Self::CameraShake | Self::FovIntensity | Self::HudVolume);
        match (slider, self.number(s)) {
            (true, Some(value)) => Control::Slider((value * 100.0).round().clamp(0.0, 100.0) as u8),
            _ => Control::Toggle,
        }
    }

    /// Whether the row can be changed now: the map's opacity and zoom are dimmed while the map is off.
    pub fn enabled(self, s: &Settings) -> bool {
        match self {
            Self::MapOpacity | Self::MapZoom => s.hud_options.map_mode != MapMode::Off,
            _ => true,
        }
    }

    /// What the row's value reads.
    pub fn data(self, s: &Settings) -> Data {
        let h = &s.hud_options;
        if let Some(on) = self.switch(s) {
            return on_off(on);
        }
        if let Some(value) = self.number(s) {
            return Data::Text(percent((value * 100.0).round() as i64));
        }
        match self {
            Self::Preset => message(match h.preset {
                HudPreset::Minimal => "options-hud-preset-minimal",
                HudPreset::Balanced => "options-hud-preset-balanced",
                HudPreset::Advanced => "options-hud-preset-advanced",
            }),
            Self::MapMode => match h.map_mode {
                MapMode::Off => Data::Label(LABEL_OFF),
                MapMode::Compact => message("options-hud-map-compact"),
                MapMode::Centered | MapMode::Expanded => message("options-hud-map-centered"),
            },
            Self::MapOrientation => match s.minimap {
                MinimapMode::Rotating => message("options-hud-map-heading-up"),
                MinimapMode::Fixed | MinimapMode::Off => message("options-hud-map-north-up"),
            },
            Self::Notifications => match h.notifications {
                NotificationLevel::All => message("options-hud-notifications-all"),
                NotificationLevel::Important => message("options-hud-notifications-important"),
                NotificationLevel::Off => Data::Label(LABEL_OFF),
            },
            Self::Palette => message(match h.palette {
                HudPalette::Default => "options-hud-palette-default",
                HudPalette::Deuteranopia => "options-hud-palette-deuteranopia",
                HudPalette::Tritanopia => "options-hud-palette-tritanopia",
            }),
            _ => Data::Text(String::new()),
        }
    }

    /// Moves the row one step (`forward`: right, else left) and records the change for the config file. Returns whether
    /// the value changed: a number at its range's end does not.
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        let before = *s;
        if let (Some(steps), Some(value)) = (self.steps(), self.number(s)) {
            self.set_number(s, &mut changed.hud_options, stepped(value, steps, forward));
            return before != *s;
        }
        if let Some(on) = self.switch(s) {
            self.set_switch(s, &mut changed.hud_options, !on);
            return before != *s;
        }
        self.step_choice(s, changed, forward);
        before != *s
    }

    fn set_number(self, s: &mut Settings, layer: &mut HudLayer, value: f32) {
        let h = &mut s.hud_options;
        let (field, recorded) = match self {
            Self::Scale => (&mut h.scale, &mut layer.scale),
            Self::Opacity => (&mut h.opacity, &mut layer.opacity),
            Self::TextSize => (&mut h.text_size, &mut layer.text_size),
            Self::MapOpacity => (&mut h.map_opacity, &mut layer.map_opacity),
            Self::MapZoom => (&mut h.map_zoom, &mut layer.map_zoom),
            Self::CameraShake => (&mut h.camera_shake, &mut layer.camera_shake),
            Self::FovIntensity => (&mut h.fov_intensity, &mut layer.fov_intensity),
            Self::HudVolume => (&mut h.hud_volume, &mut layer.hud_volume),
            _ => return,
        };
        *field = value;
        *recorded = Some(value);
    }

    fn set_switch(self, s: &mut Settings, layer: &mut HudLayer, on: bool) {
        let h = &mut s.hud_options;
        let (field, recorded) = match self {
            Self::AdvancedTelemetry => (&mut h.advanced_telemetry, &mut layer.advanced_telemetry),
            Self::WorldMarkers => (&mut h.world_markers, &mut layer.world_markers),
            Self::RaceInfo => (&mut h.race_info, &mut layer.race_info),
            Self::PursuitInfo => (&mut h.pursuit_info, &mut layer.pursuit_info),
            Self::FreeRoamExtras => (&mut h.free_roam_extras, &mut layer.free_roam_extras),
            Self::HighContrast => (&mut h.high_contrast, &mut layer.high_contrast),
            Self::MotionReduction => (&mut h.motion_reduction, &mut layer.motion_reduction),
            Self::Haptics => (&mut h.haptics, &mut layer.haptics),
            _ => return,
        };
        *field = on;
        *recorded = Some(on);
    }

    /// Cycles a choice row. A preset applies its six options and records them with it.
    fn step_choice(self, s: &mut Settings, changed: &mut Partial, forward: bool) {
        let h = &mut s.hud_options;
        let layer = &mut changed.hud_options;
        match self {
            Self::Preset => {
                let preset = PRESETS[cycle(position(&PRESETS, h.preset), PRESETS.len(), forward)];
                h.apply_preset(preset);
                *layer = HudLayer { preset: Some(preset), ..preset.layer() }.or(*layer);
            }
            Self::MapMode => {
                // Expanded reads as Centered, so it steps on from Centered's place.
                let shown = match h.map_mode {
                    MapMode::Expanded => MapMode::Centered,
                    mode => mode,
                };
                h.map_mode = MAP_MODES[cycle(position(&MAP_MODES, shown), MAP_MODES.len(), forward)];
                layer.map_mode = Some(h.map_mode);
            }
            Self::Notifications => {
                h.notifications = NOTIFICATIONS[cycle(position(&NOTIFICATIONS, h.notifications), 3, forward)];
                layer.notifications = Some(h.notifications);
            }
            Self::Palette => {
                h.palette = PALETTES[cycle(position(&PALETTES, h.palette), PALETTES.len(), forward)];
                layer.palette = Some(h.palette);
            }
            Self::MapOrientation => {
                s.minimap = ORIENTATIONS[cycle(position(&ORIENTATIONS, s.minimap), ORIENTATIONS.len(), forward)];
                changed.minimap = Some(s.minimap);
            }
            _ => {}
        }
    }
}

/// The line under the HUD page. Under the Custom style it is the result in words, as the rows change ("Centered map,
/// balanced information, high contrast"); under another style it says that most rows apply to the Custom style.
pub fn note(s: &Settings) -> String {
    if s.ui_style != UiStyle::Custom {
        return text("options-hud-note-other-style");
    }
    preview(&s.hud_options)
}

/// The options in words: the map, how much information (the preset whose six options they match, else custom), and
/// high contrast and reduced motion when they are on.
pub fn preview(h: &HudSettings) -> String {
    let map = match h.map_mode {
        MapMode::Off => "options-hud-preview-no-map",
        MapMode::Compact => "options-hud-preview-compact-map",
        MapMode::Centered => "options-hud-preview-centered-map",
        MapMode::Expanded => "options-hud-preview-expanded-map",
    };
    let information = match matching_preset(h) {
        Some(HudPreset::Minimal) => "options-hud-preview-minimal",
        Some(HudPreset::Balanced) => "options-hud-preview-balanced",
        Some(HudPreset::Advanced) => "options-hud-preview-advanced",
        None => "options-hud-preview-custom",
    };
    let mut parts = vec![text(map), text(information)];
    if h.high_contrast {
        parts.push(text("options-hud-preview-high-contrast"));
    }
    if h.motion_reduction {
        parts.push(text("options-hud-preview-reduced-motion"));
    }
    parts.join(", ")
}

/// The preset whose six options the settings hold now, if any. The expanded map counts as the centred one it grows
/// from.
fn matching_preset(h: &HudSettings) -> Option<HudPreset> {
    let map_mode = match h.map_mode {
        MapMode::Expanded => MapMode::Centered,
        mode => mode,
    };
    PRESETS.into_iter().find(|preset| {
        let layer = preset.layer();
        layer.advanced_telemetry == Some(h.advanced_telemetry)
            && layer.free_roam_extras == Some(h.free_roam_extras)
            && layer.race_info == Some(h.race_info)
            && layer.pursuit_info == Some(h.pursuit_info)
            && layer.notifications == Some(h.notifications)
            && layer.map_mode == Some(map_mode)
    })
}

/// A message of the catalogue with no arguments.
fn text(id: &str) -> String {
    crate::i18n::text(id, &Args::new())
}

/// The place of `value` in `choices`, or the first place for a value the row does not list.
fn position<T: PartialEq>(choices: &[T], value: T) -> usize {
    choices.iter().position(|choice| *choice == value).unwrap_or(0)
}

/// `value` moved one step up (`forward`) or down on the step's grid, kept inside the range and rounded to hundredths,
/// so repeated steps do not drift.
fn stepped(value: f32, Steps { step, range: (low, high) }: Steps, forward: bool) -> f32 {
    let index = (value / step).round();
    let next = match forward {
        true => index + 1.0,
        false => index - 1.0,
    };
    ((next * step).clamp(low, high) * 100.0).round() / 100.0
}

#[cfg(test)]
#[path = "hud_options_tests.rs"]
mod tests;
