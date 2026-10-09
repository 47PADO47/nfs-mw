use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

#[test]
fn hud_layout_tokens_round_trip_and_pc_is_the_default() {
    assert_eq!(Settings::from(Partial::default()).hud_layout, HudLayout::Pc);
    for (token, layout) in [("pc", HudLayout::Pc), ("classic", HudLayout::Classic), ("xbox360", HudLayout::Xbox360)] {
        assert_eq!(token.parse(), Ok(layout));
        assert_eq!(layout.to_string(), token);
    }
    for invalid in ["centered", "xbox", "wide", "0", ""] {
        assert!(invalid.parse::<HudLayout>().is_err(), "{invalid:?}");
    }
}

#[test]
fn hud_layout_layers_resolve_independently_and_bad_values_fall_through() {
    let file = file::parse("hud_layout = 'classic'\nhud = false\nminimap = 'rotating'", "test");
    let env = env::read(|key| (key == env::HUD_LAYOUT).then(|| "xbox360".into()));
    let cli = Partial { hud_layout: Some(HudLayout::Pc), ..Partial::default() };
    assert_eq!(Settings::from(file).hud_layout, HudLayout::Classic);
    assert_eq!(Settings::from(env.or(file)).hud_layout, HudLayout::Xbox360);
    let resolved = Settings::from(cli.or(env).or(file));
    assert_eq!(resolved.hud_layout, HudLayout::Pc);
    assert!(!resolved.hud);
    assert_eq!(resolved.minimap, MinimapMode::Rotating);
    let invalid_env = env::read(|key| (key == env::HUD_LAYOUT).then(|| "wide".into()));
    assert_eq!(Settings::from(invalid_env.or(file)).hud_layout, HudLayout::Classic);
    for invalid in ["hud_layout = 'wide'", "hud_layout = true", "hud_layout = 1"] {
        assert_eq!(file::parse(invalid, "test").hud_layout, None);
    }
}

#[test]
fn hud_layout_writes_only_its_own_key_and_round_trips_every_mode() {
    let existing = "game_dir = 'D:/NFS'\nfuture_option = 42\nminimap = 'off'\n";
    for layout in [HudLayout::Pc, HudLayout::Classic, HudLayout::Xbox360] {
        let changes = Partial { hud_layout: Some(layout), ..Partial::default() };
        let text = write::merge(existing, &changes).unwrap();
        let table: toml::Table = text.parse().unwrap();
        assert_eq!(table["game_dir"].as_str(), Some("D:/NFS"));
        assert_eq!(table["future_option"].as_integer(), Some(42));
        let parsed = file::parse(&text, "test");
        assert_eq!(parsed.hud_layout, Some(layout));
        assert_eq!(parsed.minimap, Some(MinimapMode::Off));
    }
    let text = write::merge(existing, &Partial::default()).unwrap();
    assert_eq!(file::parse(&text, "test").hud_layout, None);
    assert!(write::merge("broken = [", &Partial { hud_layout: Some(HudLayout::Pc), ..Partial::default() }).is_err());
}

#[test]
fn hud_layout_cli_is_optional_validated_and_reaches_the_top_layer() {
    let parse = |flags: &[&str]| {
        let cli = Cli::try_parse_from(["nfsmw", "play"].into_iter().chain(flags.iter().copied())).unwrap();
        let Command::Play { view, .. } = cli.command else { unreachable!() };
        view.settings_layer()
    };
    assert_eq!(parse(&[]).hud_layout, None);
    for layout in [HudLayout::Pc, HudLayout::Classic, HudLayout::Xbox360] {
        let cli = parse(&["--hud-layout", &layout.to_string()]);
        let env = env::read(|key| (key == env::HUD_LAYOUT).then(|| "classic".into()));
        assert_eq!(Settings::from(cli.or(env)).hud_layout, layout);
    }
    assert!(Cli::try_parse_from(["nfsmw", "play", "--hud-layout", "wide"]).is_err());
}
