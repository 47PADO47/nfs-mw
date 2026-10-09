use super::*;
use crate::cli::{Cli, Command};
use clap::Parser;

fn settings(cli: Partial, env: Partial, file: Partial) -> Settings {
    Settings::from(cli.or(env).or(file))
}

fn preset(p: GraphicsPreset) -> Partial {
    Partial { graphics_preset: Some(p), ..Partial::default() }
}

#[test]
fn the_default_is_custom_and_leaves_every_default_alone() {
    let s = Settings::from(Partial::default());
    assert_eq!(s.graphics_preset, GraphicsPreset::Custom);
    assert_eq!(settings(preset(GraphicsPreset::Custom), Partial::default(), Partial::default()), s);
    assert_eq!(GraphicsPreset::Custom.layer(), Partial::default());
}

#[test]
fn names_round_trip_strictly() {
    for (text, p) in [
        ("custom", GraphicsPreset::Custom),
        ("low", GraphicsPreset::Low),
        ("medium", GraphicsPreset::Medium),
        ("high", GraphicsPreset::High),
        ("ultra", GraphicsPreset::Ultra),
    ] {
        assert_eq!(text.parse::<GraphicsPreset>().unwrap(), p);
        assert_eq!(p.to_string(), text);
    }
    for text in ["extreme", "Low", " low", "", "1", "ULTRA"] {
        assert!(text.parse::<GraphicsPreset>().is_err(), "{text:?}");
    }
}

#[test]
fn low_is_the_cheap_set_and_high_the_pretty_one() {
    let low = Settings::from(preset(GraphicsPreset::Low));
    assert_eq!(low.graphics_preset, GraphicsPreset::Low);
    assert_eq!(low.car_shading, CarShading::Simple);
    assert_eq!((low.post_tonemap, low.post_bloom, low.post_aa), (PostTonemap::Off, PostBloom::Off, PostAa::Off));
    assert_eq!((low.render_scale.percent(), low.upscaler), (75, UpscaleMode::Bilinear));
    assert_eq!(low.smoke_quality, SmokeQuality::Standard);
    assert!(!low.collision_sparks && !low.speed_trails);
    let high = Settings::from(preset(GraphicsPreset::High));
    assert_eq!((high.car_shading, high.post_bloom, high.post_aa), (CarShading::Glossy, PostBloom::Low, PostAa::Fxaa));
    assert_eq!((high.smoke_quality, high.collision_sparks, high.speed_trails), (SmokeQuality::High, true, false));
    let medium = Settings::from(preset(GraphicsPreset::Medium));
    assert_eq!((medium.post_aa, medium.post_bloom, medium.render_scale.percent()), (PostAa::Fxaa, PostBloom::Off, 100));
}

#[test]
fn every_preset_sets_the_same_keys() {
    let keys = |p: GraphicsPreset| {
        let l = p.layer();
        [
            l.car_shading.is_some(),
            l.post_tonemap.is_some(),
            l.post_bloom.is_some(),
            l.post_aa.is_some(),
            l.render_scale.is_some(),
            l.upscaler.is_some(),
            l.tire_smoke.is_some(),
            l.skid_marks.is_some(),
            l.smoke_quality.is_some(),
            l.collision_sparks.is_some(),
            l.speed_trails.is_some(),
            l.ray_tracing.is_some(),
        ]
    };
    for p in [GraphicsPreset::Low, GraphicsPreset::Medium, GraphicsPreset::High, GraphicsPreset::Ultra] {
        assert!(keys(p).iter().all(|set| *set), "{p}");
        assert_eq!(Settings::from(preset(p)).preset_keys(), p.layer(), "{p} reads back as itself");
    }
}

#[test]
fn explicit_settings_beat_the_preset_in_every_layer() {
    // Preset from the command line; a single key from the file and another from the environment.
    let cli = preset(GraphicsPreset::Low);
    let env = env::read(|name| (name == env::CAR_SHADING).then(|| "glossy".to_owned()));
    let file = file::parse("render_scale = 90\npost_aa = 'fxaa'", "test");
    let s = settings(cli, env, file);
    assert_eq!(s.car_shading, CarShading::Glossy, "environment over a command line preset");
    assert_eq!(s.render_scale.percent(), 90, "file over a command line preset");
    assert_eq!(s.post_aa, PostAa::Fxaa);
    assert_eq!(s.upscaler, UpscaleMode::Bilinear, "keys nobody set come from the preset");
    assert_eq!(s.post_bloom, PostBloom::Off);
    assert_eq!(s.graphics_preset, GraphicsPreset::Custom, "overridden, so no longer the preset");
}

#[test]
fn the_preset_itself_resolves_cli_then_environment_then_file() {
    let env = env::read(|name| (name == env::GRAPHICS_PRESET).then(|| "medium".to_owned()));
    let file = file::parse("graphics_preset = 'high'", "test");
    assert_eq!(settings(Partial::default(), Partial::default(), file).graphics_preset, GraphicsPreset::High);
    assert_eq!(settings(Partial::default(), env, file).graphics_preset, GraphicsPreset::Medium);
    assert_eq!(settings(preset(GraphicsPreset::Low), env, file).graphics_preset, GraphicsPreset::Low);
    let bad = env::read(|name| (name == env::GRAPHICS_PRESET).then(|| "extreme".to_owned()));
    assert_eq!(
        settings(Partial::default(), bad, file).graphics_preset,
        GraphicsPreset::High,
        "bad values fall through"
    );
    for text in ["graphics_preset = 'extreme'", "graphics_preset = 2"] {
        assert_eq!(file::parse(text, "test").graphics_preset, None);
    }
}

#[test]
fn an_explicit_value_equal_to_the_preset_keeps_the_preset() {
    let file = file::parse("graphics_preset = 'low'\ncar_shading = 'simple'", "test");
    assert_eq!(Settings::from(file).graphics_preset, GraphicsPreset::Low);
}

#[test]
fn applying_a_preset_replaces_its_keys_and_changing_one_makes_it_custom() {
    let mut s = Settings::from(Partial::default());
    s.speed_trails = true;
    s.apply_preset(GraphicsPreset::High);
    assert_eq!((s.graphics_preset, s.post_bloom, s.speed_trails), (GraphicsPreset::High, PostBloom::Low, false));
    assert!(!s.settle_preset());
    s.post_bloom = PostBloom::High;
    assert!(s.settle_preset());
    assert_eq!(s.graphics_preset, GraphicsPreset::Custom);
    assert!(!s.settle_preset(), "already custom");
    s.apply_preset(GraphicsPreset::Custom);
    assert_eq!(s.post_bloom, PostBloom::High, "custom changes nothing");
}

#[test]
fn the_preset_is_written_and_read_back() {
    let changes = Partial {
        graphics_preset: Some(GraphicsPreset::Low),
        render_scale: RenderScale::new(75),
        ..Partial::default()
    };
    assert_eq!(file::parse(&write::merge("", &changes).unwrap(), "test"), changes);
}

#[test]
fn the_command_line_takes_a_preset() {
    let parse = |flags: &[&str]| {
        let cli = Cli::try_parse_from(["nfsmw", "view-world"].into_iter().chain(flags.iter().copied())).unwrap();
        let Some(Command::ViewWorld { view, .. }) = cli.command else { unreachable!() };
        view.settings_layer()
    };
    assert_eq!(parse(&[]).graphics_preset, None);
    assert_eq!(parse(&["--graphics-preset", "low"]).graphics_preset, Some(GraphicsPreset::Low));
    let s = Settings::from(parse(&["--graphics-preset", "low", "--render-scale", "100"]));
    assert_eq!((s.render_scale.percent(), s.car_shading), (100, CarShading::Simple));
    assert_eq!(parse(&["--graphics-preset", "ultra"]).graphics_preset, Some(GraphicsPreset::Ultra));
    assert!(Cli::try_parse_from(["nfsmw", "view-world", "--graphics-preset", "extreme"]).is_err());
}

#[test]
fn ultra_is_high_plus_temporal_anti_aliasing_and_ray_tracing() {
    let (high, ultra) = (Settings::from(preset(GraphicsPreset::High)), Settings::from(preset(GraphicsPreset::Ultra)));
    assert_eq!(ultra.graphics_preset, GraphicsPreset::Ultra);
    assert_eq!((ultra.post_aa, ultra.ray_tracing), (PostAa::Taa, RayTracingLevel::Medium));
    assert_eq!((high.post_aa, high.ray_tracing), (PostAa::Fxaa, RayTracingLevel::Off));
    let same = |s: &Settings| (s.car_shading, s.post_bloom, s.post_tonemap, s.smoke_quality, s.collision_sparks);
    assert_eq!(same(&high), same(&ultra));
    assert_eq!((ultra.upscaler, ultra.render_scale.percent()), (UpscaleMode::Fsr1, 100), "no temporal upscaler");
}

#[test]
fn a_preset_resolves_per_renderer_and_ultra_is_high_on_the_native_one() {
    use blackbox_gfx::{Antialiasing, RayTracing, Setting, resolve};
    let effective = |p: GraphicsPreset, caps| resolve(&Settings::from(preset(p)).graphics(), &caps);
    for p in [GraphicsPreset::Low, GraphicsPreset::Medium, GraphicsPreset::High] {
        assert!(effective(p, test_caps::native()).is_exact(), "{p} is plain on the native renderer");
        assert!(effective(p, test_caps::full()).is_exact(), "{p}");
    }
    let low = effective(GraphicsPreset::Low, test_caps::native()).effective;
    assert_eq!((low.render_scale, low.post.antialiasing), (0.75, Antialiasing::Off));

    let native = effective(GraphicsPreset::Ultra, test_caps::native());
    let high = effective(GraphicsPreset::High, test_caps::native());
    assert_eq!(native.effective, high.effective, "ultra is high where the features are missing");
    assert!(native.downgrade_of(Setting::Antialiasing).is_some() && native.downgrade_of(Setting::RayTracing).is_some());
    assert_eq!(native.downgrades.len(), 2, "{:?}", native.downgrades);

    let full = effective(GraphicsPreset::Ultra, test_caps::full());
    assert!(full.is_exact(), "{:?}", full.downgrades);
    assert_eq!((full.effective.post.antialiasing, full.effective.ray_tracing), (Antialiasing::Taa, RayTracing::Medium));
}
