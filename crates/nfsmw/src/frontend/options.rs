//! The rows of the option screens and how each changes a setting (docs/specs/frontend-menus.md, section 3.1).
//! The labels are the original language keys where one exists.

use std::str::FromStr;

use super::ids::{LABEL_OFF, LABEL_ON};
use super::logic::Category;
use crate::app::pacing::MaxFps;
use crate::devtools::ShowMetrics;
use crate::settings::{Partial, Percent, Settings, Transmission};

/// A setting a row edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    MasterVolume,
    SfxVolume,
    EngineVolume,
    MusicVolume,
    Vsync,
    MaxFps,
    Metrics,
    Hud,
    Transmission,
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
        ],
        Category::Video => vec![
            row(Setting::Vsync, Title::Label(0x6CEB_9CB6)),
            row(Setting::MaxFps, Title::Text("Frame Limit")),
            row(Setting::Metrics, Title::Text("Performance Overlay")),
        ],
        Category::Gameplay => {
            vec![
                row(Setting::Hud, Title::Label(0xAC14_8579)),
                row(Setting::Transmission, Title::Label(LABEL_TRANSMISSION)),
            ]
        }
    }
}

/// The original's Transmission row: its title and the two values (docs/specs/vehicle-manual-shifting.md, section 1).
const LABEL_TRANSMISSION: u32 = 0xD314_07E7;
const LABEL_AUTOMATIC: u32 = 0x8CD5_32A0;
const LABEL_MANUAL: u32 = 0x317D_3005;

/// Frame limits the row cycles through.
const FRAME_LIMITS: [&str; 6] = ["unlocked", "30", "60", "120", "144", "240"];
const METRICS: [ShowMetrics; 3] = [ShowMetrics::Off, ShowMetrics::Basic, ShowMetrics::Advanced];
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
        _ => return None,
    })
}

impl Setting {
    pub fn control(self, s: &Settings) -> Control {
        percent_of(s, self).map_or(Control::Toggle, |p| Control::Slider(p.0))
    }

    /// What the data string shows for a toggle.
    pub fn data(self, s: &Settings) -> Data {
        match self {
            Setting::Vsync => on_off(s.vsync),
            Setting::Hud => on_off(s.hud),
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
        let before = *s;
        match self {
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
            Setting::Vsync => {
                s.vsync = !s.vsync;
                changed.vsync = Some(s.vsync);
            }
            Setting::Hud => {
                s.hud = !s.hud;
                changed.hud = Some(s.hud);
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
        }
        before != *s
    }
}

fn nudge(p: Percent, forward: bool) -> Percent {
    let v = if forward { p.0.saturating_add(VOLUME_STEP).min(100) } else { p.0.saturating_sub(VOLUME_STEP) };
    Percent(v)
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
        assert_eq!(rows(Category::Audio).len(), 4);
        assert_eq!(rows(Category::Video).len(), 3);
        assert_eq!(rows(Category::Gameplay).len(), 2);
    }
}
