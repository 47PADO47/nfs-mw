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
    assert_eq!(rows(Category::Audio, &crate::settings::test_caps::native()).len(), 5);
    assert_eq!(rows(Category::Video, &crate::settings::test_caps::native()).len(), 19);
    assert_eq!(rows(Category::Gameplay, &crate::settings::test_caps::native()).len(), 5);
    assert_eq!(rows(Category::Controls, &crate::settings::test_caps::native()).len(), 8);
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
