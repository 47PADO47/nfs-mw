use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

fn view_layer(flags: &[&str]) -> Partial {
    let cli = Cli::try_parse_from(["nfsmw", "view-world"].into_iter().chain(flags.iter().copied())).unwrap();
    let Some(Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
    view.settings_layer()
}

#[test]
fn the_default_is_the_native_renderer() {
    assert_eq!(Settings::from(Partial::default()).renderer, RendererKind::Blackbox);
}

#[test]
fn names_round_trip_strictly() {
    for (text, kind) in [("blackbox", RendererKind::Blackbox), ("bevy", RendererKind::Bevy)] {
        assert_eq!(text.parse::<RendererKind>().unwrap(), kind);
        assert_eq!(kind.to_string(), text);
    }
    for text in ["", "Bevy", " bevy", "native", "wgpu", "1"] {
        assert!(text.parse::<RendererKind>().is_err(), "{text:?}");
    }
    assert_eq!("vulkan".parse::<RendererKind>().unwrap_err(), "expected blackbox or bevy, got \"vulkan\"");
}

#[test]
fn layers_resolve_in_order_and_bad_values_fall_through() {
    let file = file::parse("renderer = 'bevy'", "test");
    let env_bevy = env::read(|name| (name == env::RENDERER).then(|| "bevy".to_owned()));
    let env_native = env::read(|name| (name == env::RENDERER).then(|| "blackbox".to_owned()));
    assert_eq!(Settings::from(file).renderer, RendererKind::Bevy, "file over default");
    assert_eq!(Settings::from(env_native.or(file)).renderer, RendererKind::Blackbox, "environment over file");
    let cli = view_layer(&["--renderer", "bevy"]);
    assert_eq!(Settings::from(cli.or(env_native).or(Partial::default())).renderer, RendererKind::Bevy);
    assert_eq!(
        Settings::from(view_layer(&["--renderer", "blackbox"]).or(env_bevy).or(file)).renderer,
        RendererKind::Blackbox
    );
    let bad_env = env::read(|name| (name == env::RENDERER).then(|| "wgpu".to_owned()));
    assert_eq!(Settings::from(bad_env.or(file)).renderer, RendererKind::Bevy, "a bad environment value is ignored");
    for text in ["renderer = 'wgpu'", "renderer = true", "renderer = 2"] {
        assert_eq!(file::parse(text, "test").renderer, None, "{text}");
    }
    assert!(Cli::try_parse_from(["nfsmw", "view-world", "--renderer", "wgpu"]).is_err());
    assert_eq!(view_layer(&[]).renderer, None);
}

#[test]
fn the_choice_round_trips_through_the_config_file_without_dropping_other_keys() {
    let changes = Partial { renderer: Some(RendererKind::Bevy), ..Partial::default() };
    let text = write::merge("future_option = 5", &changes).unwrap();
    assert_eq!(file::parse(&text, "test"), changes);
    let table = text.parse::<toml::Table>().unwrap();
    assert_eq!((table["renderer"].as_str(), table["future_option"].as_integer()), (Some("bevy"), Some(5)));
}
