use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

#[test]
fn the_mirror_defaults_on_and_layers_command_line_over_environment_over_file() {
    assert!(Settings::from(Partial::default()).rear_view_mirror, "on by default");
    let file = file::parse("rear_view_mirror = false", "test");
    assert!(!Settings::from(file).rear_view_mirror);
    let environment = env::read(|key| (key == env::REAR_VIEW_MIRROR).then(|| "on".into()));
    assert!(Settings::from(environment.or(file)).rear_view_mirror, "environment over file");
    let cli = Cli::try_parse_from(["nfsmw", "play", "--no-rear-view-mirror"]).unwrap();
    let Some(Command::Play { view, .. }) = cli.command else { unreachable!() };
    assert!(!Settings::from(view.settings_layer().or(environment).or(file)).rear_view_mirror, "command line first");
    let cli = Cli::try_parse_from(["nfsmw", "play", "--rear-view-mirror"]).unwrap();
    let Some(Command::Play { view, .. }) = cli.command else { unreachable!() };
    assert!(Settings::from(view.settings_layer().or(file)).rear_view_mirror);
    assert!(Cli::try_parse_from(["nfsmw", "play", "--rear-view-mirror", "--no-rear-view-mirror"]).is_err());
}

#[test]
fn a_bad_value_is_ignored_and_the_choice_round_trips_beside_other_keys() {
    let environment = env::read(|key| (key == env::REAR_VIEW_MIRROR).then(|| "maybe".into()));
    assert_eq!(environment.rear_view_mirror, None);
    assert_eq!(file::parse("rear_view_mirror = 'yes'", "test").rear_view_mirror, None);
    let changes = Partial { rear_view_mirror: Some(false), ..Partial::default() };
    let text = write::merge("future_option = 12\nexhaust_flames = true", &changes).unwrap();
    let parsed = file::parse(&text, "test");
    assert_eq!((parsed.rear_view_mirror, parsed.exhaust_flames), (Some(false), Some(true)));
    assert_eq!(text.parse::<toml::Table>().unwrap()["future_option"].as_integer(), Some(12));
}
