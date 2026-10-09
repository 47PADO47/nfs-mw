use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

#[test]
fn optional_visuals_keep_pc_defaults_and_layer_independently() {
    let defaults = Settings::from(Partial::default());
    assert!(!defaults.collision_sparks && !defaults.speed_trails);
    let file = file::parse("collision_sparks = true\nspeed_trails = true", "test");
    let environment = env::read(|key| match key {
        env::COLLISION_SPARKS => Some("off".into()),
        env::SPEED_TRAILS => Some("invalid".into()),
        _ => None,
    });
    let resolved = Settings::from(environment.or(file));
    assert!(!resolved.collision_sparks && resolved.speed_trails);
    let cli = Cli::try_parse_from(["nfsmw", "play", "--collision-sparks", "--no-speed-trails"]).unwrap();
    let Some(Command::Play { view, .. }) = cli.command else { unreachable!() };
    let resolved = Settings::from(view.settings_layer().or(environment).or(file));
    assert!(resolved.collision_sparks && !resolved.speed_trails);
    assert!(Cli::try_parse_from(["nfsmw", "play", "--speed-trails", "--no-speed-trails"]).is_err());
    assert!(Cli::try_parse_from(["nfsmw", "play", "--collision-sparks", "--no-collision-sparks"]).is_err());
}

#[test]
fn visual_choices_round_trip_without_overwriting_other_settings() {
    let changes = Partial { collision_sparks: Some(true), speed_trails: Some(false), ..Partial::default() };
    let text = write::merge("future_option = 12\nsmoke_quality = 'high'", &changes).unwrap();
    let parsed = file::parse(&text, "test");
    assert_eq!(parsed.collision_sparks, Some(true));
    assert_eq!(parsed.speed_trails, Some(false));
    assert_eq!(parsed.smoke_quality, Some(SmokeQuality::High));
    assert_eq!(text.parse::<toml::Table>().unwrap()["future_option"].as_integer(), Some(12));
    let invalid = file::parse("collision_sparks = 'yes'\nspeed_trails = true", "test");
    assert_eq!(invalid.collision_sparks, None);
    assert_eq!(invalid.speed_trails, Some(true));
}
