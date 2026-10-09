//! The rows of the option screens and how each changes a setting (docs/specs/frontend-menus.md, section 3.1).
//! The labels are the original language keys where one exists.

use std::str::FromStr;

use super::graphics_options::GraphicsSetting;
use super::ids::{LABEL_OFF, LABEL_ON};
use super::input_options::InputSetting;
use super::logic::Category;
use super::post_options::PostSetting;
use crate::app::pacing::MaxFps;
use crate::devtools::ShowMetrics;
use crate::settings::{
    HudLayout, MinimapMode, Partial, Percent, RadioHudStyle, RenderScale, Settings, SmokeQuality, Transmission,
    UpscaleMode, WindowMode,
};

/// A setting a row edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
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
    ExhaustFlames,
    SmokeQuality,
    Transmission,
    Input(InputSetting),
    Post(PostSetting),
    Graphics(GraphicsSetting),
    HudLayout,
    RadioHud,
    Minimap,
    RenderScale,
    Upscaler,
    UpscaleSharpness,
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

/// The rows of a category.
pub fn rows(category: Category) -> Vec<Row> {
    match category {
        Category::Audio => vec![
            row(Setting::MasterVolume, Title::Label(0x2678_2C3E)),
            row(Setting::SfxVolume, Title::Label(0xFD48_7543)),
            row(Setting::EngineVolume, Title::Label(0xA2B1_F888)),
            row(Setting::MusicVolume, Title::Label(0x418E_681D)),
            row(Setting::SpeechVolume, Title::Label(0x9E5F_B82A)),
        ],
        Category::Video => vec![
            row(Setting::Vsync, Title::Label(0x6CEB_9CB6)),
            row(Setting::MaxFps, Title::Text("Frame Limit")),
            row(Setting::Metrics, Title::Text("Performance Overlay")),
            row(Setting::WindowMode, Title::Text("Window Mode")),
            row(Setting::TireSmoke, Title::Text("Tire Smoke")),
            row(Setting::SkidMarks, Title::Text("Skid Marks")),
            row(Setting::SmokeQuality, Title::Text("Smoke Quality")),
            row(Setting::CollisionSparks, Title::Text("Collision Sparks")),
            row(Setting::SpeedTrails, Title::Text("Speed Trails (Experimental)")),
            row(Setting::ExhaustFlames, Title::Text("Exhaust Flames")),
            row(Setting::RenderScale, Title::Text("Render Scale")),
            row(Setting::Upscaler, Title::Text("Upscaler")),
            row(Setting::UpscaleSharpness, Title::Text("Upscale Sharpness")),
        ]
        .into_iter()
        .chain(PostSetting::ALL.into_iter().map(|setting| row(Setting::Post(setting), setting.title())))
        .chain(GraphicsSetting::ALL.into_iter().map(|setting| row(Setting::Graphics(setting), setting.title())))
        .collect(),
        Category::Gameplay => vec![
            row(Setting::Hud, Title::Label(0xAC14_8579)),
            row(Setting::Transmission, Title::Label(LABEL_TRANSMISSION)),
            row(Setting::HudLayout, Title::Text("HUD Layout")),
            row(Setting::RadioHud, Title::Text("Radio HUD")),
            row(Setting::Minimap, Title::Text("Minimap")),
        ],
        Category::Controls => {
            InputSetting::ALL.into_iter().map(|setting| row(Setting::Input(setting), setting.title())).collect()
        }
    }
}

/// The original's Transmission row: its title and the two values (docs/specs/vehicle-manual-shifting.md, section 1).
const LABEL_TRANSMISSION: u32 = 0xD314_07E7;
const LABEL_AUTOMATIC: u32 = 0x8CD5_32A0;
const LABEL_MANUAL: u32 = 0x317D_3005;

/// Frame limits the row cycles through.
const FRAME_LIMITS: [&str; 6] = ["unlocked", "30", "60", "120", "144", "240"];
/// Render scales (percent) the row cycles through: FSR 1's quality modes and a few supersampling steps.
const RENDER_SCALES: [u16; 9] = [50, 59, 67, 77, 85, 100, 125, 150, 200];
const UPSCALERS: [UpscaleMode; 3] = [UpscaleMode::Off, UpscaleMode::Bilinear, UpscaleMode::Fsr1];
const METRICS: [ShowMetrics; 3] = [ShowMetrics::Off, ShowMetrics::Basic, ShowMetrics::Advanced];
const WINDOW_MODES: [WindowMode; 3] = [WindowMode::Windowed, WindowMode::Borderless, WindowMode::Exclusive];
/// A slider press moves the volume by this many percent.
const VOLUME_STEP: u8 = 10;

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
        Setting::UpscaleSharpness => s.upscale_sharpness,
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
            _ => true,
        }
    }

    /// What the data string shows for a toggle.
    pub fn data(self, s: &Settings) -> Data {
        match self {
            Setting::Input(setting) => setting.data(s),
            Setting::Post(setting) => setting.data(s),
            Setting::Graphics(setting) => setting.data(s),
            Setting::HudLayout => Data::Text(
                match s.hud_layout {
                    HudLayout::Pc => "PC",
                    HudLayout::Classic => "Centered",
                    HudLayout::Xbox360 => "Xbox 360",
                }
                .to_owned(),
            ),
            Setting::RadioHud => Data::Text(
                match s.radio_hud {
                    RadioHudStyle::EaTrax => "EA Trax",
                    RadioHudStyle::Custom => "Custom",
                }
                .to_owned(),
            ),
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
            Setting::ExhaustFlames => on_off(s.exhaust_flames),
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
    pub fn step(self, s: &mut Settings, changed: &mut Partial, forward: bool) -> bool {
        if let Setting::Input(setting) = self {
            return setting.step(s, changed, forward);
        }
        if let Setting::Post(setting) = self {
            return setting.step(s, changed, forward);
        }
        if let Setting::Graphics(setting) = self {
            return setting.step(s, changed, forward);
        }
        if !self.enabled(s) {
            return false;
        }
        let before = *s;
        match self {
            Setting::Input(_) | Setting::Post(_) | Setting::Graphics(_) => {
                unreachable!("input, post and graphics settings are handled above")
            }
            Setting::HudLayout => {
                let layouts = [HudLayout::Pc, HudLayout::Classic, HudLayout::Xbox360];
                let at = layouts.iter().position(|layout| *layout == s.hud_layout).unwrap_or(0);
                s.hud_layout = layouts[cycle(at, layouts.len(), forward)];
                changed.hud_layout = Some(s.hud_layout);
            }
            Setting::RadioHud => {
                let styles = [RadioHudStyle::EaTrax, RadioHudStyle::Custom];
                let at = styles.iter().position(|style| *style == s.radio_hud).unwrap_or(0);
                s.radio_hud = styles[cycle(at, styles.len(), forward)];
                changed.radio_hud = Some(s.radio_hud);
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
                s.upscale_sharpness = nudge(s.upscale_sharpness, forward);
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
            Setting::Vsync => {
                s.vsync = !s.vsync;
                changed.vsync = Some(s.vsync);
            }
            Setting::Hud => {
                s.hud = !s.hud;
                changed.hud = Some(s.hud);
            }
            Setting::TireSmoke => {
                s.tire_smoke = !s.tire_smoke;
                changed.tire_smoke = Some(s.tire_smoke);
            }
            Setting::SkidMarks => {
                s.skid_marks = !s.skid_marks;
                changed.skid_marks = Some(s.skid_marks);
            }
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
            Setting::SpeedTrails => {
                s.speed_trails = !s.speed_trails;
                changed.speed_trails = Some(s.speed_trails);
            }
            Setting::ExhaustFlames => {
                s.exhaust_flames = !s.exhaust_flames;
                changed.exhaust_flames = Some(s.exhaust_flames);
            }
            Setting::Transmission => {
                s.transmission = s.transmission.other();
                changed.transmission = Some(s.transmission);
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

fn nudge(p: Percent, forward: bool) -> Percent {
    let v = if forward { p.0.saturating_add(VOLUME_STEP).min(100) } else { p.0.saturating_sub(VOLUME_STEP) };
    Percent(v)
}

/// The next preset above (`forward`) or below the scale, wrapping around; a scale between presets (set from the
/// console or the config file) moves to the closest preset in that direction.
fn next_scale(current: RenderScale, forward: bool) -> RenderScale {
    let now = current.percent();
    let next = match forward {
        true => RENDER_SCALES.iter().copied().find(|p| *p > now).unwrap_or(RENDER_SCALES[0]),
        false => {
            RENDER_SCALES.iter().rev().copied().find(|p| *p < now).unwrap_or(RENDER_SCALES[RENDER_SCALES.len() - 1])
        }
    };
    RenderScale::new(next).unwrap_or(current)
}

fn cycle(at: usize, len: usize, forward: bool) -> usize {
    if forward { (at + 1) % len } else { (at + len - 1) % len }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> Settings {
        Settings::from(Partial::default())
    }

    #[test]
    fn volumes_move_by_ten_and_stop_at_the_ends() {
        let (mut s, mut c) = (defaults(), Partial::default());
        assert_eq!(s.master_volume, Percent(80));
        assert!(Setting::MasterVolume.step(&mut s, &mut c, true));
        assert_eq!(s.master_volume, Percent(90));
        assert!(Setting::MasterVolume.step(&mut s, &mut c, true));
        assert!(!Setting::MasterVolume.step(&mut s, &mut c, true), "100 is the end");
        assert_eq!(c.master_volume, Some(Percent(100)));
        for _ in 0..12 {
            Setting::MasterVolume.step(&mut s, &mut c, false);
        }
        assert_eq!(s.master_volume, Percent(0));
        assert_eq!(Setting::MasterVolume.control(&s), Control::Slider(0));
    }

    #[test]
    fn toggles_flip_and_cycle_and_record_the_change() {
        let (mut s, mut c) = (defaults(), Partial::default());
        assert!(s.vsync);
        Setting::Vsync.step(&mut s, &mut c, true);
        assert!(!s.vsync);
        assert_eq!(c.vsync, Some(false));
        assert_eq!(Setting::Vsync.data(&s), Data::Label(LABEL_OFF));
        Setting::Hud.step(&mut s, &mut c, false);
        assert_eq!((s.hud, c.hud), (false, Some(false)));

        assert_eq!(Setting::MaxFps.data(&s), Data::Text("Unlocked".into()));
        Setting::MaxFps.step(&mut s, &mut c, true);
        assert_eq!(Setting::MaxFps.data(&s), Data::Text("30 FPS".into()));
        Setting::MaxFps.step(&mut s, &mut c, false);
        Setting::MaxFps.step(&mut s, &mut c, false);
        assert_eq!(Setting::MaxFps.data(&s), Data::Text("240 FPS".into()), "left from unlocked wraps to the last");

        assert_eq!(Setting::Transmission.data(&s), Data::Label(LABEL_AUTOMATIC));
        Setting::Transmission.step(&mut s, &mut c, true);
        assert_eq!((s.transmission, c.transmission), (Transmission::Manual, Some(Transmission::Manual)));
        assert_eq!(Setting::Transmission.data(&s), Data::Label(LABEL_MANUAL));
        Setting::Transmission.step(&mut s, &mut c, false);
        assert_eq!(s.transmission, Transmission::Automatic, "left and right both toggle, as in the original");

        Setting::Metrics.step(&mut s, &mut c, true);
        assert_eq!(s.show_metrics, ShowMetrics::Basic);
        assert_eq!(c.show_metrics, Some(ShowMetrics::Basic));
    }

    #[test]
    fn every_category_has_rows() {
        assert_eq!(rows(Category::Audio).len(), 5);
        assert_eq!(rows(Category::Video).len(), 17);
        assert_eq!(rows(Category::Gameplay).len(), 5);
        assert_eq!(rows(Category::Controls).len(), 8);
    }

    #[test]
    fn the_speech_volume_row_is_a_slider_that_records_its_change() {
        let (mut s, mut changes) = (defaults(), Partial::default());
        assert_eq!(Setting::SpeechVolume.control(&s), Control::Slider(s.speech_volume.0));
        Setting::SpeechVolume.step(&mut s, &mut changes, false);
        assert_eq!(s.speech_volume.0, 80);
        assert_eq!(changes, Partial { speech_volume: Some(s.speech_volume), ..Partial::default() });
    }

    #[test]
    fn video_window_mode_cycles_both_directions_and_records_the_selection() {
        let (mut s, mut changes) = (defaults(), Partial::default());
        Setting::WindowMode.step(&mut s, &mut changes, true);
        assert_eq!((s.window_mode, changes.window_mode), (WindowMode::Borderless, Some(WindowMode::Borderless)));
        Setting::WindowMode.step(&mut s, &mut changes, true);
        assert_eq!(Setting::WindowMode.data(&s), Data::Text("Exclusive".into()));
        Setting::WindowMode.step(&mut s, &mut changes, true);
        assert_eq!(s.window_mode, WindowMode::Windowed);
        Setting::WindowMode.step(&mut s, &mut changes, false);
        assert_eq!(s.window_mode, WindowMode::Exclusive);
    }

    #[test]
    fn video_tire_toggles_record_independent_changes() {
        let (mut s, mut changes) = (defaults(), Partial::default());
        Setting::TireSmoke.step(&mut s, &mut changes, true);
        assert_eq!((s.tire_smoke, changes.tire_smoke), (false, Some(false)));
        assert!(s.skid_marks);
        Setting::SkidMarks.step(&mut s, &mut changes, false);
        assert_eq!((s.skid_marks, changes.skid_marks), (false, Some(false)));
        assert_eq!(Setting::TireSmoke.data(&s), Data::Label(LABEL_OFF));
    }

    #[test]
    fn smoke_quality_cycles_and_records_only_its_selection() {
        let (mut s, mut changes) = (defaults(), Partial::default());
        Setting::SmokeQuality.step(&mut s, &mut changes, true);
        assert_eq!(Setting::SmokeQuality.data(&s), Data::Text("High".into()));
        assert_eq!(changes, Partial { smoke_quality: Some(SmokeQuality::High), ..Partial::default() });
        Setting::SmokeQuality.step(&mut s, &mut changes, false);
        assert_eq!(Setting::SmokeQuality.data(&s), Data::Text("Standard".into()));
    }

    #[test]
    fn minimap_menu_cycles_both_directions_and_records_the_selection() {
        let (mut s, mut changes) = (defaults(), Partial::default());
        for mode in [MinimapMode::Rotating, MinimapMode::Off, MinimapMode::Fixed] {
            assert!(Setting::Minimap.step(&mut s, &mut changes, true));
            assert_eq!((s.minimap, changes.minimap), (mode, Some(mode)));
        }
        assert!(Setting::Minimap.step(&mut s, &mut changes, false));
        assert_eq!(s.minimap, MinimapMode::Off);
    }
}
