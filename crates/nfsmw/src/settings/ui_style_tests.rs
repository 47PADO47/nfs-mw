//! `ui_style` across the layers, and the legacy `radio_hud` key it replaces.

use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

#[test]
fn ui_style_defaults_to_original_and_round_trips_every_token() {
    assert_eq!(Settings::from(Partial::default()).ui_style, UiStyle::Original);
    for (token, style) in
        [("original", UiStyle::Original), ("custom", UiStyle::Custom), ("remastered", UiStyle::Remastered)]
    {
        assert_eq!(token.parse(), Ok(style));
        assert_eq!(style.to_string(), token);
    }
    for invalid in ["ea_trax", "chyron", "wide", ""] {
        assert!(invalid.parse::<UiStyle>().is_err(), "{invalid:?}");
    }
}

#[test]
fn ui_style_layers_resolve_cli_over_env_over_file() {
    let file = file::parse("ui_style = 'custom'\nminimap = 'off'", "test");
    let env = env::read(|key| (key == env::UI_STYLE).then(|| "remastered".into()));
    let cli = Partial { ui_style: Some(UiStyle::Original), ..Partial::default() };
    assert_eq!(Settings::from(file).ui_style, UiStyle::Custom);
    assert_eq!(Settings::from(env.or(file)).ui_style, UiStyle::Remastered);
    assert_eq!(Settings::from(cli.or(env).or(file)).ui_style, UiStyle::Original);
    assert_eq!(Settings::from(file).minimap, MinimapMode::Off);
    let invalid_env = env::read(|key| (key == env::UI_STYLE).then(|| "wide".into()));
    assert_eq!(Settings::from(invalid_env.or(file)).ui_style, UiStyle::Custom);
    for invalid in ["ui_style = 'ea_trax'", "ui_style = true", "ui_style = 1"] {
        assert_eq!(file::parse(invalid, "test").ui_style, None, "{invalid:?}");
    }
}

#[test]
fn legacy_radio_hud_still_loads_and_maps_onto_ui_style() {
    assert_eq!(Settings::from(file::parse("radio_hud = 'custom'", "test")).ui_style, UiStyle::Custom);
    assert_eq!(Settings::from(file::parse("radio_hud = 'ea_trax'", "test")).ui_style, UiStyle::Original);
    assert_eq!(Settings::from(file::parse("radio_hud = 'ea-trax'", "test")).ui_style, UiStyle::Original);
    assert_eq!(Settings::from(file::parse("radio_hud = 'chyron'", "test")).ui_style, UiStyle::Original);
    // A layer that also sets `ui_style` ignores the legacy key, whatever the key order in the file.
    let both = file::parse("radio_hud = 'custom'\nui_style = 'remastered'", "test");
    assert_eq!(both.ui_style, Some(UiStyle::Remastered));
    assert_eq!(Settings::from(both).ui_style, UiStyle::Remastered);
    let env = env::read(|key| (key == env::RADIO_HUD).then(|| "custom".into()));
    assert_eq!(env.ui_style, Some(UiStyle::Custom));
    let env_both = env::read(|key| match key {
        env::RADIO_HUD => Some("custom".into()),
        env::UI_STYLE => Some("original".into()),
        _ => None,
    });
    assert_eq!(env_both.ui_style, Some(UiStyle::Original));
    let cli = Cli::try_parse_from(["nfsmw", "play", "--radio-hud", "custom"]).unwrap();
    let Some(Command::Play { view, .. }) = cli.command else { unreachable!() };
    assert_eq!(view.settings_layer().ui_style, Some(UiStyle::Custom));
}

#[test]
fn ui_style_writes_only_its_own_key_and_never_radio_hud() {
    let existing = "game_dir = 'D:/NFS'\nfuture_option = 42\nradio_hud = 'custom'\n";
    for style in UiStyle::ALL {
        let changes = Partial { ui_style: Some(style), ..Partial::default() };
        let text = write::merge(existing, &changes).unwrap();
        let table: toml::Table = text.parse().unwrap();
        assert_eq!(table["game_dir"].as_str(), Some("D:/NFS"));
        assert_eq!(table["future_option"].as_integer(), Some(42));
        assert_eq!(table["ui_style"].as_str(), Some(style.to_string().as_str()));
        assert_eq!(table["radio_hud"].as_str(), Some("custom"), "the legacy key is left alone");
        assert_eq!(file::parse(&text, "test").ui_style, Some(style));
    }
    let text = write::merge(existing, &Partial::default()).unwrap();
    assert_eq!(file::parse(&text, "test").ui_style, Some(UiStyle::Custom), "no change keeps the legacy value");
    assert!(write::merge("broken = [", &Partial { ui_style: Some(UiStyle::Custom), ..Partial::default() }).is_err());
}

#[test]
fn ui_style_cli_is_optional_validated_and_reaches_the_top_layer() {
    let parse = |flags: &[&str]| {
        let cli = Cli::try_parse_from(["nfsmw", "play"].into_iter().chain(flags.iter().copied())).unwrap();
        let Some(Command::Play { view, .. }) = cli.command else { unreachable!() };
        view.settings_layer()
    };
    assert_eq!(parse(&[]).ui_style, None);
    for style in UiStyle::ALL {
        let layer = parse(&["--ui-style", &style.to_string()]);
        let env = env::read(|key| (key == env::UI_STYLE).then(|| "custom".into()));
        assert_eq!(Settings::from(layer.or(env)).ui_style, style);
    }
    assert!(Cli::try_parse_from(["nfsmw", "play", "--ui-style", "wide"]).is_err());
    // `--ui-style` wins over the legacy flag when both are given.
    assert_eq!(parse(&["--radio-hud", "custom", "--ui-style", "original"]).ui_style, Some(UiStyle::Original));
}
