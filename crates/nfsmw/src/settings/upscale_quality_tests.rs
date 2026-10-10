use blackbox_gfx::UpscaleQuality as Mode;

use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

fn view_layer(flags: &[&str]) -> Partial {
    let cli = Cli::try_parse_from(["nfsmw", "view-world"].into_iter().chain(flags.iter().copied())).unwrap();
    let Some(Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
    view.settings_layer()
}

#[test]
fn the_default_is_quality() {
    let s = Settings::from(Partial::default());
    assert_eq!(s.upscale_quality, UpscaleQuality::Quality);
    assert_eq!(s.graphics().upscale_quality, Mode::Quality);
}

#[test]
fn names_round_trip_strictly_and_map_to_the_renderer_modes() {
    let all = [
        ("auto", UpscaleQuality::Auto, Mode::Auto),
        ("native", UpscaleQuality::Native, Mode::Native),
        ("quality", UpscaleQuality::Quality, Mode::Quality),
        ("balanced", UpscaleQuality::Balanced, Mode::Balanced),
        ("performance", UpscaleQuality::Performance, Mode::Performance),
        ("ultra_performance", UpscaleQuality::UltraPerformance, Mode::UltraPerformance),
    ];
    for (text, quality, mode) in all {
        assert_eq!(text.parse::<UpscaleQuality>().unwrap(), quality);
        assert_eq!(quality.to_string(), text);
        assert_eq!(quality.mode(), mode);
        assert_eq!(mode.name(), text, "the setting names are the renderer's names");
    }
    for text in ["", "Quality", "ultra-performance", "ultra performance", "high", "1"] {
        assert!(text.parse::<UpscaleQuality>().is_err(), "{text:?}");
    }
}

#[test]
fn layers_resolve_in_order_and_bad_values_fall_through() {
    let file = file::parse("upscale_quality = 'balanced'", "test");
    let env = env::read(|n| (n == env::UPSCALE_QUALITY).then(|| "performance".to_owned()));
    assert_eq!(Settings::from(file).upscale_quality, UpscaleQuality::Balanced);
    assert_eq!(Settings::from(env.or(file)).upscale_quality, UpscaleQuality::Performance, "environment over file");
    let cli = view_layer(&["--upscale-quality", "native"]);
    assert_eq!(Settings::from(cli.or(env).or(file)).upscale_quality, UpscaleQuality::Native, "command line first");
    let bad = env::read(|n| (n == env::UPSCALE_QUALITY).then(|| "extreme".to_owned()));
    assert_eq!(Settings::from(bad.or(file)).upscale_quality, UpscaleQuality::Balanced);
    for text in ["upscale_quality = 'extreme'", "upscale_quality = 2", "upscale_quality = true"] {
        assert_eq!(file::parse(text, "test").upscale_quality, None, "{text}");
    }
    assert!(Cli::try_parse_from(["nfsmw", "view-world", "--upscale-quality", "extreme"]).is_err());
    assert_eq!(view_layer(&[]).upscale_quality, None);
}

#[test]
fn the_choice_round_trips_through_the_config_file() {
    let changes = Partial { upscale_quality: Some(UpscaleQuality::UltraPerformance), ..Partial::default() };
    let text = write::merge("future_option = 5", &changes).unwrap();
    assert_eq!(file::parse(&text, "test"), changes);
    assert_eq!(text.parse::<toml::Table>().unwrap()["upscale_quality"].as_str(), Some("ultra_performance"));
}
