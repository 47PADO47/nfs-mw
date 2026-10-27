use super::{parse, settings_cmd};
use crate::settings::{Partial, Settings};

#[test]
fn the_console_gets_and_sets_both_smoke_colour_keys() {
    let mut settings = Settings::from(Partial::default());
    assert_eq!(settings_cmd::get(&settings, "smoke_color_mode").unwrap(), "smoke_color_mode = off");
    assert_eq!(settings_cmd::get(&settings, "smoke_color").unwrap(), "smoke_color = red");
    settings_cmd::set(&mut settings, "smoke_color_mode", "rainbow").unwrap();
    settings_cmd::set(&mut settings, "smoke_color", "#00ff88").unwrap();
    assert_eq!(settings_cmd::get(&settings, "smoke_color_mode").unwrap(), "smoke_color_mode = rainbow");
    assert_eq!(settings.smoke_color.0, [0, 255, 136]);
    assert_eq!(settings_cmd::get(&settings, "smoke_color").unwrap(), "smoke_color = #00ff88");
    assert!(settings_cmd::set(&mut settings, "smoke_color_mode", "disco").is_err());
    assert!(settings_cmd::set(&mut settings, "smoke_color", "teal").is_err());
}

#[test]
fn the_keys_are_known_to_get_all_and_to_help() {
    let settings = Settings::from(Partial::default());
    let all = settings_cmd::get_all(&settings);
    assert!(
        all.lines().any(|line| line == "smoke_color_mode = off") && all.lines().any(|line| line == "smoke_color = red"),
        "{all}"
    );
    for key in ["smoke_color_mode", "smoke_color"] {
        assert!(parse::BUILT_IN.iter().any(|(usage, _)| usage.starts_with(key)), "{key} has a help line");
    }
}
