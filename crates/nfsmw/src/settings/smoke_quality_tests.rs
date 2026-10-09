use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

#[test]
fn smoke_quality_has_a_strict_round_trip_and_preserves_the_standard_default() {
    assert_eq!(Settings::from(Partial::default()).smoke_quality, SmokeQuality::Standard);
    for (text, quality) in [("standard", SmokeQuality::Standard), ("high", SmokeQuality::High)] {
        assert_eq!(text.parse::<SmokeQuality>().unwrap(), quality);
        assert_eq!(quality.to_string(), text);
    }
    for text in ["low", "ultra", "HIGH", "1", " high", ""] {
        assert!(text.parse::<SmokeQuality>().is_err(), "{text:?}");
    }
}

#[test]
fn quality_layers_resolve_in_order_and_bad_values_fall_through() {
    let file = file::parse("smoke_quality = 'high'\ntire_smoke = false", "test");
    let env = env::read(|name| (name == env::SMOKE_QUALITY).then(|| "standard".to_owned()));
    assert_eq!(Settings::from(file).smoke_quality, SmokeQuality::High);
    let environment = Settings::from(env.or(file));
    assert_eq!(environment.smoke_quality, SmokeQuality::Standard);
    assert!(!environment.tire_smoke, "each field resolves independently");
    let cli = Partial { smoke_quality: Some(SmokeQuality::High), ..Partial::default() };
    assert_eq!(Settings::from(cli.or(env).or(file)).smoke_quality, SmokeQuality::High);
    let invalid_env = env::read(|name| (name == env::SMOKE_QUALITY).then(|| "ultra".to_owned()));
    assert_eq!(Settings::from(invalid_env.or(file)).smoke_quality, SmokeQuality::High);
    for text in ["smoke_quality = 'ultra'", "smoke_quality = true", "smoke_quality = 2"] {
        assert_eq!(file::parse(text, "test").smoke_quality, None);
    }
}

#[test]
fn cli_quality_is_optional_and_strict_and_overrides_the_environment() {
    let parse = |flags: &[&str]| {
        let cli = Cli::try_parse_from(["nfsmw", "view-world"].into_iter().chain(flags.iter().copied())).unwrap();
        let Some(Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
        view.settings_layer()
    };
    assert_eq!(parse(&[]).smoke_quality, None);
    let cli = parse(&["--smoke-quality", "high"]);
    let env = env::read(|name| (name == env::SMOKE_QUALITY).then(|| "standard".to_owned()));
    assert_eq!(Settings::from(cli.or(env)).smoke_quality, SmokeQuality::High);
    assert_eq!(parse(&["--smoke-quality", "standard"]).smoke_quality, Some(SmokeQuality::Standard));
    for text in ["ultra", "HIGH", "1"] {
        assert!(Cli::try_parse_from(["nfsmw", "view-world", "--smoke-quality", text]).is_err());
    }
}
