//! The rows of the option screens and how each changes a setting (docs/specs/frontend-menus.md, section 3.1).
//! The labels are the original language keys where one exists; the rest are messages of the rewrite's catalogue
//! (`i18n/en/options.ftl`, `options-graphics.ftl`).

use std::str::FromStr;

use super::graphics_options::GraphicsSetting;
use super::ids::{LABEL_OFF, LABEL_ON};
use super::input_options::InputSetting;
#[cfg(test)]
use super::logic::Category;
use super::post_options::PostSetting;
use super::renderer_options::RendererSetting;
use crate::app::pacing::MaxFps;
use crate::devtools::ShowMetrics;
use crate::i18n::Args;
#[cfg(test)]
use crate::settings::Transmission;
use crate::settings::{
    CameraMode, HudLayout, MinimapMode, Partial, Percent, Settings, SmokeQuality, SpeedUnits, UiStyle, UiTheme,
    UpscaleMode, WindowMode,
};

/// A setting a row edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    AdvancedGraphics,
    TrafficPreset,
    MasterVolume,
    SfxVolume,
    EngineVolume,
    MusicVolume,
    SpeechVolume,
    Vsync,
    MaxFps,
    Metrics,
    Hud,
    WindowMode,
    TireSmoke,
    SkidMarks,
    CollisionSparks,
    SpeedTrails,
    SpeedTrailStyle,
    ExhaustFlames,
    CustomFonts,
    TurnSignals,
    CopHealthBars,
    RearViewMirror,
    SmokeQuality,
    Transmission,
    Camera,
    Discord,
    DiscordTimePlayed,
    DiscordJoin,
    Input(InputSetting),
    Hd(hd_options::HdSetting),
    /// A row of the HUD page: a `[custom_hud]` key, or the map's orientation (`hud_options.rs`).
    CustomHud(hud_options::HudSetting),
    Post(PostSetting),
    Graphics(GraphicsSetting),
    Renderer(RendererSetting),
    HudLayout,
    UiStyle,
    SpeedUnits,
    UiTheme,
    Minimap,
    RenderScale,
    Upscaler,
    UpscaleSharpness,
    /// The game's language (`options_language.rs`): its choices are the languages on offer, not a fixed list.
    Language,
    /// A row of the custom race's options (`custom_race/settings.rs`); its value is the session's, not a setting.
    Custom(super::custom_race::settings::Row),
    /// A row a mod added to the Mods page (`bb.menus`), by its handle; its label, value and control come from the bridge.
    Mod(u64),
    /// The on/off switch of a listed mod, by its position in the mod manager's list (`mod_options.rs`).
    ModManaged(usize),
    /// The UI theme the Mods page chooses among the mods' themes (`bb.ui`), or the built-in style. Its value is kept by
    /// the bridge, not the settings (`mod_options.rs`).
    ModTheme,
}

/// What a row's title shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Title {
    /// A key of the language table.
    Label(u32),
    /// A message of the rewrite's catalogue, for a setting the original has no label for. A Mods row has the empty id:
    /// its label comes from the mod (`mod_options::shown`).
    Key(&'static str),
}

impl Title {
    /// The title in the current language: a catalogue title's message, or a game label's text (empty when no table
    /// has it).
    pub fn text(self) -> String {
        match self {
            Title::Label(label) => crate::i18n::game(label).unwrap_or_default(),
            Title::Key(id) => crate::i18n::text(id, &Args::new()),
        }
    }
}

/// What a row's data shows: a key of the language table, or text already in the current language (a catalogue
/// message, a number, a name). Data is read each time the row is drawn, so a language change shows at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Data {
    Label(u32),
    Text(String),
}

/// A value from the rewrite's catalogue, in the current language.
pub(super) fn message(id: &'static str) -> Data {
    Data::Text(crate::i18n::text(id, &Args::new()))
}

/// The value a test expects for the English word: the original's On, Off and Auto are game labels (the tests load no
/// game table), every other word is the catalogue's English text.
#[cfg(test)]
pub(super) fn shown(english: &str) -> Data {
    match english {
        "On" => Data::Label(LABEL_ON),
        "Off" => Data::Label(LABEL_OFF),
        "Auto" => Data::Label(LABEL_AUTO),
        _ => Data::Text(english.into()),
    }
}

/// A slider moves a value from 0 to 100; a toggle cycles through values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Control {
    Slider(u8),
    Toggle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Row {
    pub setting: Setting,
    pub title: Title,
}

const fn row(setting: Setting, title: Title) -> Row {
    Row { setting, title }
}

#[path = "options_rows.rs"]
mod category_rows;
pub use category_rows::rows;

#[path = "hd_options.rs"]
mod hd_options;

#[path = "hud_options.rs"]
pub(super) mod hud_options;

#[path = "options_caps.rs"]
mod caps_rows;

#[path = "options_steps.rs"]
mod steps;
use steps::{cycle, next_scale, nudge};

#[path = "options_data.rs"]
mod data;

#[path = "options_language.rs"]
mod language;

/// The original's `COMMON_AUTO` ("Auto"): the automatic transmission, and the Auto value of every row that has one.
pub(super) const LABEL_AUTO: u32 = 0x8CD5_32A0;

/// The original's Transmission row: its title and the two values (docs/specs/vehicle-manual-shifting.md, section 1).
const LABEL_TRANSMISSION: u32 = 0xD314_07E7;
const LABEL_AUTOMATIC: u32 = LABEL_AUTO;
const LABEL_MANUAL: u32 = 0x317D_3005;

/// Frame limits the row cycles through.
const FRAME_LIMITS: [&str; 6] = ["unlocked", "30", "60", "120", "144", "240"];
const UPSCALERS: [UpscaleMode; 6] = [
    UpscaleMode::Off,
    UpscaleMode::Bilinear,
    UpscaleMode::Fsr1,
    UpscaleMode::Fsr3,
    UpscaleMode::Fsr4,
    UpscaleMode::Dlss,
];
const METRICS: [ShowMetrics; 3] = [ShowMetrics::Off, ShowMetrics::Basic, ShowMetrics::Advanced];
const WINDOW_MODES: [WindowMode; 3] = [WindowMode::Windowed, WindowMode::Borderless, WindowMode::Exclusive];
/// The most values any row cycles through; `advance` gives up after that many steps.
const MAX_VALUES: usize = 16;
fn on_off(on: bool) -> Data {
    Data::Label(if on { LABEL_ON } else { LABEL_OFF })
}

fn percent_of(s: &Settings, setting: Setting) -> Option<Percent> {
    Some(match setting {
        Setting::MasterVolume => s.master_volume,
        Setting::SfxVolume => s.sfx_volume,
        Setting::EngineVolume => s.engine_volume,
        Setting::MusicVolume => s.music_volume,
        Setting::SpeechVolume => s.speech_volume,
        Setting::Graphics(GraphicsSetting::AoIntensity) => s.ao_intensity,
        _ => return None,
    })
}

impl Setting {
    pub fn control(self, s: &Settings) -> Control {
        if let Setting::CustomHud(setting) = self {
            return setting.control(s);
        }
        percent_of(s, self).map_or(Control::Toggle, |p| Control::Slider(p.0))
    }

    /// Whether the row can be changed now. A disabled row still shows its value, dimmed, and ignores left and right.
    /// The row stays selectable so the whole list can be browsed, as the greyed-out icons are.
    pub fn enabled(self, s: &Settings) -> bool {
        match self {
            // With vsync on the display paces the frames, so the limiter is greyed out.
            Setting::MaxFps => !s.vsync,
            Setting::Custom(row) => super::custom_race::options::enabled(row),
            Setting::Graphics(setting) => setting.enabled(s),
            Setting::Post(PostSetting::Aa) => !s.upscaler.upscaler().is_temporal(),
            Setting::SmokeQuality => s.tire_smoke,
            // The time played and the join invite are part of the Discord presence, so they are dimmed while that is off.
            Setting::DiscordTimePlayed | Setting::DiscordJoin => s.discord,
            Setting::CustomHud(setting) => setting.enabled(s),
            Setting::RenderScale => matches!(s.upscaler, UpscaleMode::Bilinear | UpscaleMode::Fsr1),
            // The upscalers that sharpen: FSR 1 (its RCAS pass) and FSR 3 (the RCAS of its own pipeline).
            Setting::UpscaleSharpness => matches!(s.upscaler, UpscaleMode::Fsr1 | UpscaleMode::Fsr3),
            Setting::Renderer(RendererSetting::UpscaleQuality) => s.upscaler.upscaler().is_temporal(),
            _ => true,
        }
    }

    /// Moves the setting one step (`forward`: right, else left) and records the change for the config file.
    /// Returns whether the value changed (a slider at its end does not).
    #[cfg(test)]
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        if self == Setting::AdvancedGraphics {
            return false;
        }
        if !self.enabled(s) {
            return false;
        }
        let moved = self.step_value(s, changed, forward);
        // Changing one setting a preset covers leaves the preset behind: the row then reads Custom.
        if s.settle_preset() {
            changed.graphics_preset = Some(crate::settings::GraphicsPreset::Custom);
        }
        moved
    }

    fn step_value(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        macro_rules! toggle {
            ($s:ident, $c:ident, $field:ident) => {{
                $s.$field = !$s.$field;
                $c.$field = Some($s.$field);
            }};
        }
        let before = *s;
        match self {
            Setting::AdvancedGraphics => return false,
            // A Mods row's value lives in its mod (`mod_options.rs`), not in the settings. The theme choice lives in the
            // bridge too.
            Setting::Mod(_) | Setting::ModManaged(_) | Setting::ModTheme => return false,
            Setting::Custom(row) => {
                super::custom_race::options::step(row, forward);
                return true;
            }
            Setting::Input(setting) => return setting.step(s, changed, forward),
            Setting::Post(setting) => return setting.step(s, changed, forward),
            Setting::Graphics(setting) => return setting.step(s, changed, forward),
            Setting::Renderer(setting) => return setting.step(s, changed, forward),
            Setting::Hd(setting) => return setting.step(s, changed, forward),
            Setting::CustomHud(setting) => return setting.step(s, changed, forward),
            Setting::Language => return language::step(s, changed, forward),
            Setting::HudLayout => {
                let layouts = [HudLayout::Pc, HudLayout::Classic, HudLayout::Xbox360];
                let at = layouts.iter().position(|layout| *layout == s.hud_layout).unwrap_or(0);
                s.hud_layout = layouts[cycle(at, layouts.len(), forward)];
                changed.hud_layout = Some(s.hud_layout);
            }
            Setting::UiStyle => {
                let at = UiStyle::ALL.iter().position(|style| *style == s.ui_style).unwrap_or(0);
                s.ui_style = UiStyle::ALL[cycle(at, UiStyle::ALL.len(), forward)];
                changed.ui_style = Some(s.ui_style);
            }
            Setting::SpeedUnits => {
                let at = SpeedUnits::ALL.iter().position(|units| *units == s.speed_units).unwrap_or(0);
                s.speed_units = SpeedUnits::ALL[cycle(at, SpeedUnits::ALL.len(), forward)];
                changed.speed_units = Some(s.speed_units);
            }
            Setting::UiTheme => {
                let at = UiTheme::ALL.iter().position(|theme| *theme == s.ui_theme).unwrap_or(0);
                s.ui_theme = UiTheme::ALL[cycle(at, UiTheme::ALL.len(), forward)];
                changed.ui_theme = Some(s.ui_theme);
            }
            Setting::Minimap => {
                let modes = [MinimapMode::Fixed, MinimapMode::Rotating, MinimapMode::Off];
                let at = modes.iter().position(|m| *m == s.minimap).unwrap_or(0);
                s.minimap = modes[cycle(at, modes.len(), forward)];
                changed.minimap = Some(s.minimap);
            }
            Setting::RenderScale => {
                s.render_scale = next_scale(s.render_scale, forward);
                changed.render_scale = Some(s.render_scale);
            }
            Setting::Upscaler => {
                let at = UPSCALERS.iter().position(|m| *m == s.upscaler).unwrap_or(0);
                s.upscaler = UPSCALERS[cycle(at, UPSCALERS.len(), forward)];
                changed.upscaler = Some(s.upscaler);
            }
            Setting::UpscaleSharpness => {
                s.upscale_sharpness = steps::next_sharpening(s.upscale_sharpness, forward);
                changed.upscale_sharpness = Some(s.upscale_sharpness);
            }
            Setting::MasterVolume => {
                s.master_volume = nudge(s.master_volume, forward);
                changed.master_volume = Some(s.master_volume);
            }
            Setting::SfxVolume => {
                s.sfx_volume = nudge(s.sfx_volume, forward);
                changed.sfx_volume = Some(s.sfx_volume);
            }
            Setting::EngineVolume => {
                s.engine_volume = nudge(s.engine_volume, forward);
                changed.engine_volume = Some(s.engine_volume);
            }
            Setting::MusicVolume => {
                s.music_volume = nudge(s.music_volume, forward);
                changed.music_volume = Some(s.music_volume);
            }
            Setting::SpeechVolume => {
                s.speech_volume = nudge(s.speech_volume, forward);
                changed.speech_volume = Some(s.speech_volume);
            }
            Setting::Vsync => toggle!(s, changed, vsync),
            Setting::Hud => toggle!(s, changed, hud),
            Setting::TireSmoke => toggle!(s, changed, tire_smoke),
            Setting::SkidMarks => toggle!(s, changed, skid_marks),
            Setting::CollisionSparks => {
                let at = match (s.collision_sparks, s.spark_style) {
                    (false, _) => 0,
                    (true, crate::settings::SparkStyle::OriginalPc) => 1,
                    (true, crate::settings::SparkStyle::RestoredExperimental) => 2,
                };
                let next = cycle(at, 3, forward);
                s.collision_sparks = next != 0;
                s.spark_style = match next {
                    2 => crate::settings::SparkStyle::RestoredExperimental,
                    _ => crate::settings::SparkStyle::OriginalPc,
                };
                changed.collision_sparks = Some(s.collision_sparks);
                changed.spark_style = Some(s.spark_style);
            }
            Setting::SpeedTrails => toggle!(s, changed, speed_trails),
            Setting::SpeedTrailStyle => {
                s.speed_trail_style = match s.speed_trail_style {
                    crate::settings::SpeedTrailStyle::Streaks => crate::settings::SpeedTrailStyle::Car,
                    crate::settings::SpeedTrailStyle::Car => crate::settings::SpeedTrailStyle::Streaks,
                };
                changed.speed_trail_style = Some(s.speed_trail_style);
            }
            Setting::ExhaustFlames => toggle!(s, changed, exhaust_flames),
            Setting::CustomFonts => {
                s.custom_fonts.enabled = !s.custom_fonts.enabled;
                changed.custom_fonts.enabled = Some(s.custom_fonts.enabled);
            }
            Setting::TurnSignals => toggle!(s, changed, turn_signals),
            Setting::CopHealthBars => toggle!(s, changed, cop_health_bars),
            Setting::RearViewMirror => toggle!(s, changed, rear_view_mirror),
            Setting::Discord => toggle!(s, changed, discord),
            Setting::DiscordTimePlayed => toggle!(s, changed, discord_time_played),
            Setting::DiscordJoin => toggle!(s, changed, discord_join),
            Setting::TrafficPreset => {
                let choices = crate::settings::TrafficPreset::ALL;
                let at = choices.iter().position(|p| *p == s.traffic_preset()).unwrap_or(0);
                s.select_traffic_preset(choices[cycle(at, choices.len(), forward)]);
                changed.traffic = Some(s.traffic);
                changed.traffic_policy = Some(s.traffic_policy);
            }
            Setting::Transmission => {
                s.transmission = s.transmission.other();
                changed.transmission = Some(s.transmission);
            }
            Setting::Camera => {
                let at = CameraMode::ALL.iter().position(|m| *m == s.camera_mode).unwrap_or(0);
                s.camera_mode = CameraMode::ALL[cycle(at, CameraMode::ALL.len(), forward)];
                changed.camera_mode = Some(s.camera_mode);
            }
            Setting::MaxFps => {
                let at = FRAME_LIMITS.iter().position(|l| MaxFps::from_str(l).is_ok_and(|m| m == s.max_fps));
                let next = cycle(at.unwrap_or(0), FRAME_LIMITS.len(), forward);
                if let Ok(limit) = MaxFps::from_str(FRAME_LIMITS[next]) {
                    s.max_fps = limit;
                    changed.max_fps = Some(limit);
                }
            }
            Setting::Metrics => {
                let at = METRICS.iter().position(|m| *m == s.show_metrics).unwrap_or(0);
                s.show_metrics = METRICS[cycle(at, METRICS.len(), forward)];
                changed.show_metrics = Some(s.show_metrics);
            }
            Setting::WindowMode => {
                let at = WINDOW_MODES.iter().position(|m| *m == s.window_mode).unwrap_or(0);
                s.window_mode = WINDOW_MODES[cycle(at, WINDOW_MODES.len(), forward)];
                changed.window_mode = Some(s.window_mode);
            }
            Setting::SmokeQuality => {
                s.smoke_quality = match s.smoke_quality {
                    SmokeQuality::Standard => SmokeQuality::High,
                    SmokeQuality::High => SmokeQuality::Standard,
                };
                changed.smoke_quality = Some(s.smoke_quality);
            }
        }
        before != *s
    }
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;

#[path = "options_effective.rs"]
mod effective;
