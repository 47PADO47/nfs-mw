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

#[test]
fn the_gameplay_page_keeps_the_rider_gear_switch_and_no_row_for_the_gear_itself() {
    // The gear is bought in a bike's shop or set from the developer tools: no helmet, suit, gloves, boots or body row, in the main
    // menu's options or the pause menu's (both list the same page).
    let page = rows(Category::Gameplay, &crate::settings::test_caps::native());
    assert!(
        page.iter().any(|row| row.setting == Setting::RiderGear),
        "the on/off switch of the external content stays"
    );
    let gear_rows = page
        .iter()
        .filter(
            |row| matches!(row.title, Title::Key(id) if id.starts_with("options-rider-") && id != "options-rider-gear"),
        )
        .count();
    assert_eq!(gear_rows, 0, "no row for a part of the gear");
}

fn defaults() -> Settings {
    Settings::from(Partial::default())
}
#[test]
fn volumes_move_by_ten_and_stop_at_the_ends() {
    let (mut s, mut c) = (defaults(), Partial::default());
    assert_eq!(s.master_volume, Percent(100), "the PC default");
    assert!(!Setting::MasterVolume.step(&mut s, &mut c, true), "100 is the end");
    assert!(Setting::MasterVolume.step(&mut s, &mut c, false));
    assert_eq!(s.master_volume, Percent(90));
    assert!(Setting::MasterVolume.step(&mut s, &mut c, true));
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
    assert_eq!(rows(Category::Audio, &crate::settings::test_caps::native()).len(), 6);
    assert!(rows(Category::Video, &crate::settings::test_caps::native()).len() < 40);
    assert_eq!(rows(Category::Gameplay, &crate::settings::test_caps::native()).len(), 25);
    assert_eq!(rows(Category::Player, &crate::settings::test_caps::native()).len(), 3);
    assert_eq!(rows(Category::Controls, &crate::settings::test_caps::native()).len(), 8);
    assert_eq!(rows(Category::Discord, &crate::settings::test_caps::native()).len(), 3);
}
#[test]
fn the_speech_volume_row_is_a_slider_that_records_its_change() {
    let (mut s, mut changes) = (defaults(), Partial::default());
    assert_eq!(Setting::SpeechVolume.control(&s), Control::Slider(s.speech_volume.0));
    Setting::SpeechVolume.step(&mut s, &mut changes, false);
    assert_eq!(s.speech_volume.0, 90);
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
fn smoke_colour_rows_cycle_both_ways_and_record_only_their_selection() {
    use crate::settings::{SmokeColor, SmokeColorMode};
    let (mut s, mut changes) = (defaults(), Partial::default());
    assert!(!Setting::SmokeColor.enabled(&s), "the colour waits for a mode that uses it");
    assert_eq!(Setting::SmokeColorMode.data(&s), Data::Text("Off".into()));
    Setting::SmokeColorMode.step(&mut s, &mut changes, true);
    assert_eq!((s.smoke_color_mode, changes.smoke_color_mode), (SmokeColorMode::Solid, Some(SmokeColorMode::Solid)));
    assert!(Setting::SmokeColor.enabled(&s));
    Setting::SmokeColorMode.step(&mut s, &mut changes, false);
    assert_eq!(s.smoke_color_mode, SmokeColorMode::Off, "back one step");
    Setting::SmokeColorMode.step(&mut s, &mut changes, false);
    assert_eq!(s.smoke_color_mode, SmokeColorMode::Slip, "and round the other way");
    Setting::SmokeColor.step(&mut s, &mut changes, true);
    assert_eq!(Setting::SmokeColor.data(&s), Data::Text("Orange".into()));
    assert_eq!(changes.smoke_color.map(|c| c.to_string()).as_deref(), Some("orange"));
    Setting::SmokeColor.step(&mut s, &mut changes, false);
    assert_eq!(s.smoke_color, SmokeColor::default());
    s.tire_smoke = false;
    assert!(!Setting::SmokeColorMode.enabled(&s), "no smoke, no colour");
}
#[test]
fn minimap_menu_cycles_both_directions_and_records_the_selection() {
    use crate::settings::MinimapContext::{FreeRoam, Race};
    let (mut s, mut changes) = (defaults(), Partial::default());
    let race = s.minimap_mode(Race);
    for mode in [MinimapMode::Rotating, MinimapMode::Off, MinimapMode::Fixed] {
        assert!(Setting::ContextMinimap(FreeRoam).step(&mut s, &mut changes, true));
        assert_eq!((s.free_roam_map_mode, changes.free_roam_map_mode), (Some(mode), Some(mode)));
        assert_eq!(s.minimap_mode(Race), race);
    }
    assert!(Setting::ContextMinimap(FreeRoam).step(&mut s, &mut changes, false));
    assert_eq!(s.free_roam_map_mode, Some(MinimapMode::Off));
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

#[test]
fn batch18_gameplay_map_rows_edit_contexts_independently() {
    use crate::settings::MinimapContext::{FreeRoam, Race};
    let offered = rows(Category::Gameplay, &crate::settings::test_caps::native());
    assert!(offered.iter().any(|row| row.setting == Setting::ContextMinimap(FreeRoam)));
    assert!(offered.iter().any(|row| row.setting == Setting::ContextMinimap(Race)));
    let (mut s, mut changes) = (defaults(), Partial::default());
    s.free_roam_map_mode = Some(MinimapMode::Fixed);
    s.race_map_mode = Some(MinimapMode::Off);
    assert!(Setting::ContextMinimap(FreeRoam).step(&mut s, &mut changes, true));
    assert_eq!(s.free_roam_map_mode, Some(MinimapMode::Rotating));
    assert_eq!(s.race_map_mode, Some(MinimapMode::Off));
    assert_eq!(changes.free_roam_map_mode, Some(MinimapMode::Rotating));
    assert_eq!(changes.race_map_mode, None);
    assert_eq!(changes.minimap, None);
    s.set_global_minimap(MinimapMode::Off);
    changes.set_global_minimap(MinimapMode::Off);
    assert_eq!(s.free_roam_map_mode, Some(s.minimap));
    assert_eq!(s.race_map_mode, Some(s.minimap));
}

#[test]
fn batch18_music_rows_edit_only_the_selected_context() {
    let mut settings = Settings::from(Partial::default());
    let mut changes = Partial::default();
    let caps = crate::settings::test_caps::native();
    let rows = rows(Category::Audio, &caps);
    for setting in [Setting::MenuMusicVolume, Setting::GameMusicVolume] {
        assert!(rows.iter().any(|row| row.setting == setting), "both independent Audio controls are reachable");
    }
    Setting::MenuMusicVolume.step(&mut settings, &mut changes, false);
    assert_eq!((settings.menu_music_volume, settings.game_music_volume), (Percent(70), Percent(80)));
    assert_eq!(
        (changes.menu_music_volume, changes.game_music_volume, changes.music_volume),
        (Some(Percent(70)), None, None)
    );
    Setting::GameMusicVolume.step(&mut settings, &mut changes, true);
    assert_eq!((settings.menu_music_volume, settings.game_music_volume), (Percent(70), Percent(90)));
}

#[test]
fn batch18_gameplay_offers_a_profile_autosave_control() {
    let caps = crate::settings::test_caps::native();
    assert!(rows(Category::Gameplay, &caps).iter().any(|r| r.title == Title::Key("options-autosave")));
}
