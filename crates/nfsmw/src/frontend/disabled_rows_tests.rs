use super::logic::Category;
use super::options::{Data, Setting, rows};
use crate::settings::{Partial, Settings};

fn settings(vsync: bool) -> Settings {
    Settings { vsync, ..Settings::from(Partial::default()) }
}

#[test]
fn the_frame_limit_is_disabled_while_vsync_is_on() {
    let on = settings(true);
    let off = settings(false);
    assert!(!Setting::MaxFps.enabled(&on));
    assert!(Setting::MaxFps.enabled(&off));
    assert!(rows(Category::Video).iter().any(|row| row.setting == Setting::MaxFps), "still listed and selectable");
}

#[test]
fn a_disabled_row_ignores_left_and_right_and_keeps_its_value() {
    let mut s = settings(true);
    let mut changes = Partial::default();
    let before = Setting::MaxFps.data(&s);
    assert!(!Setting::MaxFps.step(&mut s, &mut changes, true));
    assert!(!Setting::MaxFps.step(&mut s, &mut changes, false));
    assert_eq!(Setting::MaxFps.data(&s), before);
    assert_eq!(changes, Partial::default(), "nothing is recorded for the config file");
}

#[test]
fn turning_vsync_off_enables_the_frame_limit_again() {
    let mut s = settings(true);
    let mut changes = Partial::default();
    assert!(Setting::Vsync.step(&mut s, &mut changes, true));
    assert!(!s.vsync);
    assert!(Setting::MaxFps.enabled(&s));
    assert!(Setting::MaxFps.step(&mut s, &mut changes, true));
    assert_eq!(Setting::MaxFps.data(&s), Data::Text("30 FPS".into()));
    assert_eq!(changes.max_fps.map(|m| m.to_string()), Some("30".to_owned()));
}

#[test]
fn the_other_rows_are_always_enabled() {
    for s in [settings(true), settings(false)] {
        for category in [Category::Audio, Category::Video, Category::Gameplay] {
            for row in rows(category) {
                if row.setting != Setting::MaxFps {
                    assert!(row.setting.enabled(&s), "{:?}", row.setting);
                }
            }
        }
    }
}
