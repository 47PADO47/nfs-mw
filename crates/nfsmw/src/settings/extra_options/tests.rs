use super::*;

#[test]
fn skip_movies_becomes_the_skip_intro_layer() {
    let parsed = ExtraOptions::parse("[Misc]\nSkipMovies = 1\n");
    assert_eq!(parsed.options.skip_movies, Some(true));
    assert_eq!(parsed.options.layer().skip_intro, Some(true));
    assert!(parsed.unsupported.is_empty());
}

#[test]
fn unset_options_leave_the_layer_empty() {
    let parsed = ExtraOptions::parse("");
    assert_eq!(parsed.options, ExtraOptions::default());
    assert_eq!(parsed.options.layer(), Partial::default());
}

#[test]
fn unimplemented_keys_are_reported_not_applied() {
    let text = r#"[Menu]
ShowOnline = 1
[Misc]
SkipMovies = 0
ShowMessage = 1
"#;
    let parsed = ExtraOptions::parse(text);
    assert_eq!(parsed.options.skip_movies, Some(false));
    assert_eq!(parsed.unsupported, vec!["Menu.ShowOnline".to_owned(), "Misc.ShowMessage".to_owned()]);
}

#[test]
fn a_bad_switch_value_leaves_the_option_unset() {
    assert_eq!(ExtraOptions::parse("[Misc]\nSkipMovies = maybe\n").options.skip_movies, None);
}

#[test]
fn rain_options_become_the_weather_layer() {
    let parsed = ExtraOptions::parse("[Weather]\nAlwaysRain = 1\nGeneralRainAmount = 0.5\nRainSize = 0.01\n");
    let layer = parsed.options.layer();
    assert_eq!((layer.always_rain, layer.rain_amount), (Some(true), Some(Percent(50))));
    assert_eq!(parsed.unsupported, vec!["Weather.RainSize".to_owned()]);
}

#[test]
fn rain_amount_clamps_to_the_original_range() {
    assert_eq!(ExtraOptions::parse("[Weather]\nGeneralRainAmount = 3\n").options.rain_amount, Some(Percent(100)));
    assert_eq!(ExtraOptions::parse("[Weather]\nGeneralRainAmount = -1\n").options.rain_amount, Some(Percent(0)));
    assert_eq!(ExtraOptions::parse("[Weather]\nGeneralRainAmount = lots\n").options.rain_amount, None);
}

#[test]
fn game_speed_becomes_the_clock_layer() {
    let parsed = ExtraOptions::parse("[Gameplay]\nGameSpeed = 2\n");
    assert_eq!(parsed.options.layer().game_speed, Some(2.0));
    assert!(parsed.unsupported.is_empty());
}

#[test]
fn world_animation_speed_becomes_the_animation_layer() {
    let parsed = ExtraOptions::parse("[Gameplay]\nWorldAnimationSpeed = 45\n");
    assert_eq!(parsed.options.layer().animation_speed, Some(1.0), "45 is the original rate");
    assert!(parsed.unsupported.is_empty());
}

#[test]
fn enable_sound_and_music_mute_the_volumes_only_when_off() {
    let off = ExtraOptions::parse("[Misc]\nEnableSound = 0\nEnableMusic = 0\n");
    let layer = off.options.layer();
    assert_eq!((layer.master_volume, layer.music_volume), (Some(Percent(0)), Some(Percent(0))));
    assert!(off.unsupported.is_empty());
    let on = ExtraOptions::parse("[Misc]\nEnableSound = 1\nEnableMusic = 1\n").options.layer();
    assert_eq!((on.master_volume, on.music_volume), (None, None), "on leaves the volumes to the config");
}

#[test]
fn car_scale_becomes_the_scale_layer() {
    let parsed = ExtraOptions::parse("[Gameplay]\nCarScale = 1.5\n");
    assert_eq!(parsed.options.layer().car_scale, Some(1.5));
    assert!(parsed.unsupported.is_empty());
}

#[test]
fn the_headlights_hotkey_code_becomes_the_key_layer() {
    let parsed = ExtraOptions::parse("[Hotkeys]\nHeadlights = 72\n");
    assert_eq!(parsed.options.layer().headlights_key, Some(72));
    assert!(parsed.unsupported.is_empty());
}
