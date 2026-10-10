use blackbox_gfx::{RayTracing, Setting, resolve};

use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

fn view_layer(flags: &[&str]) -> Partial {
    let cli = Cli::try_parse_from(["nfsmw", "view-world"].into_iter().chain(flags.iter().copied())).unwrap();
    let Some(Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
    view.settings_layer()
}

#[test]
fn ray_tracing_is_off_by_default_and_asks_for_nothing() {
    let s = Settings::from(Partial::default());
    assert_eq!(s.ray_tracing, RayTracingLevel::Off);
    assert_eq!(s.graphics().ray_tracing, RayTracing::Off);
}

#[test]
fn names_round_trip_strictly_and_map_to_the_renderer_levels() {
    for (text, level, mode) in [
        ("off", RayTracingLevel::Off, RayTracing::Off),
        ("low", RayTracingLevel::Low, RayTracing::Low),
        ("medium", RayTracingLevel::Medium, RayTracing::Medium),
        ("high", RayTracingLevel::High, RayTracing::High),
    ] {
        assert_eq!(text.parse::<RayTracingLevel>().unwrap(), level);
        assert_eq!(level.to_string(), text);
        assert_eq!(level.level(), mode);
    }
    for text in ["", "on", "ultra", "High", " low", "true"] {
        assert!(text.parse::<RayTracingLevel>().is_err(), "{text:?}");
    }
}

#[test]
fn layers_resolve_in_order_and_bad_values_fall_through() {
    let file = file::parse("ray_tracing = 'low'", "test");
    let env = env::read(|n| (n == env::RAY_TRACING).then(|| "medium".to_owned()));
    assert_eq!(Settings::from(file).ray_tracing, RayTracingLevel::Low);
    assert_eq!(Settings::from(env.or(file)).ray_tracing, RayTracingLevel::Medium, "environment over file");
    let cli = view_layer(&["--ray-tracing", "high"]);
    assert_eq!(Settings::from(cli.or(env).or(file)).ray_tracing, RayTracingLevel::High, "command line first");
    let off = view_layer(&["--ray-tracing", "off"]);
    assert_eq!(Settings::from(off.or(env).or(file)).ray_tracing, RayTracingLevel::Off);
    let bad = env::read(|n| (n == env::RAY_TRACING).then(|| "path".to_owned()));
    assert_eq!(Settings::from(bad.or(file)).ray_tracing, RayTracingLevel::Low);
    for text in ["ray_tracing = 'path'", "ray_tracing = true", "ray_tracing = 1"] {
        assert_eq!(file::parse(text, "test").ray_tracing, None, "{text}");
    }
    assert!(Cli::try_parse_from(["nfsmw", "view-world", "--ray-tracing", "path"]).is_err());
    assert_eq!(view_layer(&[]).ray_tracing, None);
}

#[test]
fn the_choice_round_trips_through_the_config_file() {
    let changes = Partial { ray_tracing: Some(RayTracingLevel::High), ..Partial::default() };
    let text = write::merge("future_option = 5", &changes).unwrap();
    assert_eq!(file::parse(&text, "test"), changes);
    assert_eq!(text.parse::<toml::Table>().unwrap()["ray_tracing"].as_str(), Some("high"));
}

#[test]
fn the_native_renderer_runs_it_as_off_and_keeps_the_stored_value() {
    let s = Settings::from(Partial { ray_tracing: Some(RayTracingLevel::Medium), ..Partial::default() });
    let native = resolve(&s.graphics(), &test_caps::native());
    assert_eq!(native.effective.ray_tracing, RayTracing::Off);
    assert!(native.downgrade_of(Setting::RayTracing).is_some());
    assert_eq!(s.ray_tracing, RayTracingLevel::Medium, "only the effective value changes");
    let full = resolve(&s.graphics(), &test_caps::full());
    assert_eq!(full.effective.ray_tracing, RayTracing::Medium);
    assert!(full.is_exact());
}
