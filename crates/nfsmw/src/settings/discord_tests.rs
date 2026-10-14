use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

#[test]
fn discord_presence_is_on_by_default_with_the_time_played() {
    let settings = Settings::from(Partial::default());
    assert!(settings.discord);
    assert!(settings.discord_time_played);
}

#[test]
fn the_discord_keys_round_trip_through_the_config_file() {
    let changes = Partial { discord: Some(false), discord_time_played: Some(true), ..Partial::default() };
    let text = write::merge("", &changes).unwrap();
    let parsed = file::parse(&text, "test");
    assert_eq!((parsed.discord, parsed.discord_time_played), (Some(false), Some(true)));
    let changes = Partial { discord: Some(true), discord_time_played: Some(false), ..Partial::default() };
    let text = write::merge(&text, &changes).unwrap();
    let parsed = file::parse(&text, "test");
    assert_eq!((parsed.discord, parsed.discord_time_played), (Some(true), Some(false)));
}

#[test]
fn a_bad_discord_value_is_ignored_and_other_keys_survive() {
    let text = write::merge("future_option = 3", &Partial { discord: Some(false), ..Partial::default() }).unwrap();
    assert_eq!(text.parse::<toml::Table>().unwrap()["future_option"].as_integer(), Some(3));
    assert_eq!(file::parse("discord = 'yes'", "test").discord, None);
    let environment = env::read(|key| (key == env::DISCORD_TIME_PLAYED).then(|| "off".into()));
    assert_eq!(environment.discord_time_played, Some(false));
    assert_eq!(environment.discord, None);
}

#[test]
fn the_discord_flags_layer_over_the_environment_and_the_file() {
    let file = file::parse("discord = true\ndiscord_time_played = true", "test");
    let environment = env::read(|key| (key == env::DISCORD).then(|| "off".into()));
    let settings = Settings::from(environment.or(file.clone()));
    assert!(!settings.discord, "environment over file");
    assert!(settings.discord_time_played, "the file's value where the environment is silent");
    let cli = Cli::try_parse_from(["nfsmw", "play", "--discord", "--no-discord-time-played"]).unwrap();
    let Some(Command::Play { view, .. }) = cli.command else { unreachable!() };
    let settings = Settings::from(view.settings_layer().or(environment).or(file));
    assert!(settings.discord, "command line first");
    assert!(!settings.discord_time_played, "command line first");
    assert!(Cli::try_parse_from(["nfsmw", "play", "--no-discord", "--discord"]).is_err());
}
