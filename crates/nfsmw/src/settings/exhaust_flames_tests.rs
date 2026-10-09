use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

#[test]
fn the_flames_default_on_and_layer_command_line_over_environment_over_file() {
    assert!(Settings::from(Partial::default()).exhaust_flames, "on: an idle car pays almost nothing");
    let file = file::parse("exhaust_flames = false", "test");
    assert!(!Settings::from(file).exhaust_flames);
    let environment = env::read(|key| (key == env::EXHAUST_FLAMES).then(|| "on".into()));
    assert!(Settings::from(environment.or(file)).exhaust_flames, "environment over file");
    let cli = Cli::try_parse_from(["nfsmw", "play", "--no-exhaust-flames"]).unwrap();
    let Some(Command::Play { view, .. }) = cli.command else { unreachable!() };
    assert!(!Settings::from(view.settings_layer().or(environment).or(file)).exhaust_flames, "command line first");
    let cli = Cli::try_parse_from(["nfsmw", "play", "--exhaust-flames"]).unwrap();
    let Some(Command::Play { view, .. }) = cli.command else { unreachable!() };
    assert!(Settings::from(view.settings_layer().or(file)).exhaust_flames);
    assert!(Cli::try_parse_from(["nfsmw", "play", "--exhaust-flames", "--no-exhaust-flames"]).is_err());
}

#[test]
fn a_bad_value_is_ignored_and_the_choice_round_trips_beside_other_keys() {
    let environment = env::read(|key| (key == env::EXHAUST_FLAMES).then(|| "maybe".into()));
    assert_eq!(environment.exhaust_flames, None);
    assert_eq!(file::parse("exhaust_flames = 'yes'", "test").exhaust_flames, None);
    let changes = Partial { exhaust_flames: Some(false), ..Partial::default() };
    let text = write::merge("future_option = 12\nspeed_trails = true", &changes).unwrap();
    let parsed = file::parse(&text, "test");
    assert_eq!((parsed.exhaust_flames, parsed.speed_trails), (Some(false), Some(true)));
    assert_eq!(text.parse::<toml::Table>().unwrap()["future_option"].as_integer(), Some(12));
}
