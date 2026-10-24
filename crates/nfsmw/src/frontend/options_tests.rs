use super::*;

#[test]
fn traffic_range_cycles_in_video_and_persists_without_adding_player_rows() {
    use crate::settings::TrafficPreset;
    let mut settings = Settings::from(Partial::default());
    let mut layer = Partial::default();
    for expected in [TrafficPreset::Extended, TrafficPreset::Custom, TrafficPreset::Reference] {
        assert!(Setting::TrafficPreset.step(&mut settings, &mut layer, true));
        assert_eq!(settings.traffic_preset(), expected);
        assert_eq!(Settings::from(layer).traffic_preset(), expected);
    }
    let caps = crate::settings::test_caps::native();
    assert!(rows(Category::Video, &caps).iter().any(|r| r.setting == Setting::TrafficPreset));
    assert!(!rows(Category::Player, &caps).iter().any(|r| r.setting == Setting::TrafficPreset));
}

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
    assert!(rows(Category::Video, &crate::settings::test_caps::native()).len() < 40);
    assert_eq!(rows(Category::Gameplay, &crate::settings::test_caps::native()).len(), 9);
    assert_eq!(rows(Category::Player, &crate::settings::test_caps::native()).len(), 3);
    assert_eq!(rows(Category::Controls, &crate::settings::test_caps::native()).len(), 8);
    assert_eq!(rows(Category::Discord, &crate::settings::test_caps::native()).len(), 2);
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
fn the_speech_volume_row_reads_police_dispatch() {
    let audio = rows(Category::Audio, &crate::settings::test_caps::native());
    let row = audio.iter().find(|row| row.setting == Setting::SpeechVolume).expect("the speech volume row");
    assert_eq!(row.title, Title::Key("options-police-dispatch-volume"));
    assert_eq!(row.title.text(), "Police Dispatch");
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
#[test]
fn discord_toggles_record_their_changes_and_time_played_is_dimmed_while_discord_is_off() {
    let (mut s, mut changes) = (defaults(), Partial::default());
    assert!(s.discord && s.discord_time_played);
    assert!(Setting::DiscordTimePlayed.enabled(&s));
    Setting::Discord.step(&mut s, &mut changes, true);
    assert_eq!((s.discord, changes.discord), (false, Some(false)));
    assert_eq!(Setting::Discord.data(&s), Data::Label(LABEL_OFF));
    assert!(!Setting::DiscordTimePlayed.enabled(&s), "dimmed while Discord is off");
    assert!(!Setting::DiscordTimePlayed.step(&mut s, &mut changes, true), "a dimmed row ignores left and right");
    assert!(s.discord_time_played);
    assert_eq!(changes.discord_time_played, None);
    Setting::Discord.step(&mut s, &mut changes, true);
    assert!(Setting::DiscordTimePlayed.enabled(&s));
    Setting::DiscordTimePlayed.step(&mut s, &mut changes, true);
    assert_eq!((s.discord_time_played, changes.discord_time_played), (false, Some(false)));
    assert_eq!(Setting::DiscordTimePlayed.data(&s), Data::Label(LABEL_OFF));
}
