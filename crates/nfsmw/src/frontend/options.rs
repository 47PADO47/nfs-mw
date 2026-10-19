//! The rows of the option screens and how each changes a setting (docs/specs/frontend-menus.md, section 3.1).
//! The labels are the original language keys where one exists.

use std::str::FromStr;

use super::alias_options::AliasSetting;
use super::graphics_options::GraphicsSetting;
use super::ids::{LABEL_OFF, LABEL_ON};
use super::input_options::InputSetting;
#[cfg(test)]
use super::logic::Category;
use super::post_options::PostSetting;
use super::renderer_options::RendererSetting;
use crate::app::pacing::MaxFps;
use crate::devtools::ShowMetrics;
use crate::settings::{
    CameraMode, HudLayout, MinimapMode, Partial, Percent, Settings, SmokeQuality, SpeedUnits, Transmission, UiStyle,
    UiTheme, UpscaleMode, WindowMode,
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
    Input(InputSetting),
    Alias(AliasSetting),
    Hd(hd_options::HdSetting),
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
    /// A setting the original has no label for.
    Text(&'static str),
}

/// What a row's data shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Data {
    Label(u32),
    Text(String),
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

#[path = "options_caps.rs"]
mod caps_rows;

#[path = "options_steps.rs"]
mod steps;
use steps::{cycle, next_scale, nudge};

/// The original's Transmission row: its title and the two values (docs/specs/vehicle-manual-shifting.md, section 1).
const LABEL_TRANSMISSION: u32 = 0xD314_07E7;
const LABEL_AUTOMATIC: u32 = 0x8CD5_32A0;
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
const MAX_VALUES: usize = 8;
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
        percent_of(s, self).map_or(Control::Toggle, |p| Control::Slider(p.0))
    }

    /// Whether the row can be changed now. A disabled row still shows its value, dimmed, and ignores left and right.
    /// The row stays selectable so the whole list can be browsed, as the greyed-out icons are.
    pub fn enabled(self, s: &Settings) -> bool {
        match self {
            // With vsync on the display paces the frames, so the limiter is greyed out.
            Setting::MaxFps => !s.vsync,
            Setting::Graphics(setting) => setting.enabled(s),
            Setting::Post(PostSetting::Aa) => !s.upscaler.upscaler().is_temporal(),
            Setting::SmokeQuality => s.tire_smoke,
            // The time played is part of the Discord presence, so it is dimmed while that is off.
            Setting::DiscordTimePlayed => s.discord,
            Setting::RenderScale => matches!(s.upscaler, UpscaleMode::Bilinear | UpscaleMode::Fsr1),
            // The upscalers that sharpen: FSR 1 (its RCAS pass) and FSR 3 (the RCAS of its own pipeline).
            Setting::UpscaleSharpness => matches!(s.upscaler, UpscaleMode::Fsr1 | UpscaleMode::Fsr3),
            Setting::Renderer(RendererSetting::UpscaleQuality) => s.upscaler.upscaler().is_temporal(),
            _ => true,
        }
    }

    /// What the data string shows for a toggle.
    pub fn data(self, s: &Settings) -> Data {
        match self {
            Setting::AdvancedGraphics => Data::Text("Open F8".into()),
            Setting::TrafficPreset => Data::Text(s.traffic_preset().label().into()),
            Setting::Input(setting) => setting.data(s),
            Setting::Alias(setting) => Data::Text(setting.data().to_owned()),
            Setting::Post(setting) => setting.data(s),
            Setting::Graphics(setting) => setting.data(s),
            Setting::Renderer(setting) => setting.data(s),
            Setting::UpscaleSharpness => Data::Text(steps::sharpening_name(s.upscale_sharpness).to_owned()),
            Setting::Hd(setting) => setting.data(s),
            Setting::HudLayout => Data::Text(
                match s.hud_layout {
                    HudLayout::Pc => "PC",
                    HudLayout::Classic => "Centered",
                    HudLayout::Xbox360 => "Xbox 360",
                }
                .to_owned(),
            ),
            Setting::UiStyle => Data::Text(s.ui_style.label().to_owned()),
            Setting::SpeedUnits => Data::Text(s.speed_units.label().to_owned()),
            Setting::UiTheme => Data::Text(s.ui_theme.label().to_owned()),
            Setting::Minimap => Data::Text(
                match s.minimap {
                    MinimapMode::Fixed => "Fixed",
                    MinimapMode::Rotating => "Rotating",
                    MinimapMode::Off => "Off",
                }
                .into(),
            ),
            Setting::RenderScale => Data::Text(format!("{}%", s.render_scale.percent())),
            Setting::Upscaler => Data::Text(
                match s.upscaler {
                    UpscaleMode::Off => "Off",
                    UpscaleMode::Bilinear => "Bilinear",
                    UpscaleMode::Fsr1 => "FSR 1",
                    UpscaleMode::Fsr3 => "FSR 3",
                    UpscaleMode::Fsr4 => "FSR 4",
                    UpscaleMode::Dlss => "DLSS",
                }
                .to_owned(),
            ),
            Setting::Vsync => on_off(s.vsync),
            Setting::Hud => on_off(s.hud),
            Setting::TireSmoke => on_off(s.tire_smoke),
            Setting::SkidMarks => on_off(s.skid_marks),
            Setting::CollisionSparks if !s.collision_sparks => on_off(false),
            Setting::CollisionSparks => Data::Text(s.spark_style.label().into()),
            Setting::SpeedTrails => on_off(s.speed_trails),
            Setting::SpeedTrailStyle => Data::Text(s.speed_trail_style.label().into()),
            Setting::ExhaustFlames => on_off(s.exhaust_flames),
            Setting::CustomFonts => on_off(s.custom_fonts.enabled),
            Setting::TurnSignals => on_off(s.turn_signals),
            Setting::CopHealthBars => on_off(s.cop_health_bars),
            Setting::RearViewMirror => on_off(s.rear_view_mirror),
            Setting::Discord => on_off(s.discord),
            Setting::DiscordTimePlayed => on_off(s.discord_time_played),
            Setting::SmokeQuality => Data::Text(
                match s.smoke_quality {
                    SmokeQuality::Standard => "Standard",
                    SmokeQuality::High => "High",
                }
                .to_owned(),
            ),
            Setting::WindowMode => Data::Text(
                match s.window_mode {
                    WindowMode::Windowed => "Windowed",
                    WindowMode::Borderless => "Borderless",
                    WindowMode::Exclusive => "Exclusive",
                }
                .to_owned(),
            ),
            Setting::Transmission => Data::Label(match s.transmission {
                Transmission::Automatic => LABEL_AUTOMATIC,
                Transmission::Manual => LABEL_MANUAL,
            }),
            Setting::Camera => Data::Text(
                match s.camera_mode {
                    CameraMode::Chase => "Chase",
                    CameraMode::DynamicFov => "Dynamic FOV",
                    CameraMode::Bumper => "Bumper",
                    CameraMode::Hood => "Hood",
                    CameraMode::Close => "Close",
                    CameraMode::Far => "Far",
                    CameraMode::SuperFar => "Super far",
                    CameraMode::Drift => "Drift",
                    CameraMode::Pursuit => "Pursuit",
                }
                .to_owned(),
            ),
            Setting::MaxFps => Data::Text(match s.max_fps.to_string().as_str() {
                "unlocked" => "Unlocked".to_owned(),
                fps => format!("{fps} FPS"),
            }),
            Setting::Metrics => Data::Text(
                match s.show_metrics {
                    ShowMetrics::Off => "Off",
                    ShowMetrics::Basic => "Basic",
                    ShowMetrics::Advanced => "Advanced",
                }
                .to_owned(),
            ),
            _ => Data::Text(String::new()),
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
            Setting::Input(setting) => return setting.step(s, changed, forward),
            Setting::Post(setting) => return setting.step(s, changed, forward),
            Setting::Graphics(setting) => return setting.step(s, changed, forward),
            Setting::Renderer(setting) => return setting.step(s, changed, forward),
            Setting::Hd(setting) => return setting.step(s, changed, forward),
            Setting::Alias(_) => unreachable!("alias rows never reach the settings step"),
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
