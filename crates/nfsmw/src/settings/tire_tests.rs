use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

#[test]
fn tire_layers_resolve_independently_and_invalid_values_fall_through() {
    let file = file::parse("tire_smoke = false\nskid_marks = false\n", "test");
    let env = env::read(|name| match name {
        env::TIRE_SMOKE => Some("on".into()),
        env::SKID_MARKS => Some("maybe".into()),
        _ => None,
    });
    let resolved = Settings::from(env.or(file));
    assert!(resolved.tire_smoke, "valid environment value wins");
    assert!(!resolved.skid_marks, "invalid environment value falls through");
    let cli = Partial { tire_smoke: Some(false), skid_marks: Some(true), ..Partial::default() };
    let resolved = Settings::from(cli.or(env).or(file));
    assert!(!resolved.tire_smoke && resolved.skid_marks, "CLI wins per field");
    let invalid = file::parse("tire_smoke = 'off'\nskid_marks = false", "test");
    assert_eq!(invalid.tire_smoke, None);
    assert_eq!(invalid.skid_marks, Some(false));
    let defaults = Settings::from(Partial::default());
    assert!(defaults.tire_smoke && defaults.skid_marks);
}

#[test]
fn tire_cli_switches_override_lower_layers_and_conflicting_flags_are_rejected() {
    let parse = |flags: &[&str]| {
        let cli = Cli::try_parse_from(["nfsmw", "view-world"].into_iter().chain(flags.iter().copied())).unwrap();
        let Some(Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
        view.settings_layer()
    };
    let defaults = parse(&[]);
    assert_eq!((defaults.tire_smoke, defaults.skid_marks), (None, None));
    let switches = parse(&["--no-tire-smoke", "--skid-marks"]);
    assert_eq!((switches.tire_smoke, switches.skid_marks), (Some(false), Some(true)));
    assert!(Cli::try_parse_from(["nfsmw", "view-world", "--tire-smoke", "--no-tire-smoke"]).is_err());
    assert!(Cli::try_parse_from(["nfsmw", "view-world", "--skid-marks", "--no-skid-marks"]).is_err());
}
