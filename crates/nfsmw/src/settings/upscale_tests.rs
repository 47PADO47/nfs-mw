use super::*;
use crate::cli::{Cli, Command};
use blackbox_render::Upscaler;
use clap::Parser;

#[test]
fn defaults_are_native_and_fsr1() {
    let s = Settings::from(Partial::default());
    assert_eq!(s.render_scale.percent(), 100);
    assert_eq!(s.render_scale.factor(), 1.0);
    assert_eq!(s.upscaler, UpscaleMode::Fsr1);
    assert_eq!(s.upscale_sharpness, Percent(80));
}

#[test]
fn render_scale_reads_percent_or_factor_and_rejects_out_of_range() {
    for (text, percent) in
        [("50", 50), ("67%", 67), (" 100 ", 100), ("0.67", 67), ("1.5", 150), ("1.0", 100), ("200", 200)]
    {
        assert_eq!(text.parse::<RenderScale>().unwrap().percent(), percent, "{text:?}");
    }
    for text in ["49", "201", "0", "1", "-50", "0.4", "2.5", "half", "", "nan", "inf", "1e9"] {
        assert!(text.parse::<RenderScale>().is_err(), "{text:?}");
    }
    assert_eq!(RenderScale::new(75).unwrap().to_string(), "75");
    assert_eq!(RenderScale::new(75).unwrap().factor(), 0.75);
    assert!(RenderScale::new(49).is_none() && RenderScale::new(201).is_none());
}

#[test]
fn upscaler_has_a_strict_round_trip() {
    for (text, mode) in [("off", UpscaleMode::Off), ("bilinear", UpscaleMode::Bilinear), ("fsr1", UpscaleMode::Fsr1)] {
        assert_eq!(text.parse::<UpscaleMode>().unwrap(), mode);
        assert_eq!(mode.to_string(), text);
    }
    assert_eq!("FSR".parse::<UpscaleMode>().unwrap(), UpscaleMode::Fsr1);
    for text in ["fsr2", "dlss", "on", "", "1"] {
        assert!(text.parse::<UpscaleMode>().is_err(), "{text:?}");
    }
    assert_eq!(UpscaleMode::Off.upscaler(), None);
    assert_eq!(UpscaleMode::Bilinear.upscaler(), Some(Upscaler::Bilinear));
    assert_eq!(UpscaleMode::Fsr1.upscaler(), Some(Upscaler::Fsr1));
}

#[test]
fn layers_resolve_in_order_and_bad_values_fall_through() {
    let file = file::parse("render_scale = 67\nupscaler = 'bilinear'\nupscale_sharpness = 30", "test");
    assert_eq!(file.render_scale.map(RenderScale::percent), Some(67));
    assert_eq!(file.upscaler, Some(UpscaleMode::Bilinear));
    assert_eq!(file.upscale_sharpness, Some(Percent(30)));
    let env = env::read(|name| match name {
        env::RENDER_SCALE => Some("50%".to_owned()),
        env::UPSCALER => Some("fsr1".to_owned()),
        env::UPSCALE_SHARPNESS => Some("lots".to_owned()),
        _ => None,
    });
    let s = Settings::from(env.or(file));
    assert_eq!((s.render_scale.percent(), s.upscaler), (50, UpscaleMode::Fsr1), "environment over file");
    assert_eq!(s.upscale_sharpness, Percent(30), "a bad environment value falls through to the file");
    let cli = Partial { render_scale: RenderScale::new(200), ..Partial::default() };
    assert_eq!(Settings::from(cli.or(env).or(file)).render_scale.percent(), 200, "command line over environment");
    for text in
        ["render_scale = 10", "render_scale = true", "upscaler = 3", "upscaler = 'dlss'", "upscale_sharpness = 300"]
    {
        let p = file::parse(text, "test");
        assert_eq!((p.render_scale, p.upscaler, p.upscale_sharpness), (None, None, None), "{text}");
    }
    assert_eq!(file::parse("render_scale = 0.75", "test").render_scale.map(RenderScale::percent), Some(75));
}

#[test]
fn what_is_written_reads_back() {
    let changes = Partial {
        render_scale: RenderScale::new(59),
        upscaler: Some(UpscaleMode::Off),
        upscale_sharpness: Some(Percent(0)),
        ..Partial::default()
    };
    assert_eq!(file::parse(&write::merge("future = 1", &changes).unwrap(), "test"), changes);
}

#[test]
fn cli_flags_are_strict_and_set_the_top_layer() {
    let parse = |flags: &[&str]| {
        let cli = Cli::try_parse_from(["nfsmw", "view-world"].into_iter().chain(flags.iter().copied())).unwrap();
        let Some(Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
        view.settings_layer()
    };
    let none = parse(&[]);
    assert_eq!((none.render_scale, none.upscaler, none.upscale_sharpness), (None, None, None));
    let layer = parse(&["--render-scale", "67", "--upscaler", "bilinear", "--upscale-sharpness", "10"]);
    assert_eq!(layer.render_scale.map(RenderScale::percent), Some(67));
    assert_eq!(layer.upscaler, Some(UpscaleMode::Bilinear));
    assert_eq!(layer.upscale_sharpness, Some(Percent(10)));
    for flags in [["--render-scale", "10"], ["--upscaler", "dlss"], ["--upscale-sharpness", "101"]] {
        assert!(Cli::try_parse_from(["nfsmw", "view-world"].into_iter().chain(flags)).is_err(), "{flags:?}");
    }
}
