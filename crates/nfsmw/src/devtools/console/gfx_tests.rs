//! `set` and `gfx` through the console's own entry points, against a renderer with given capabilities.

use blackbox_gfx::{Capabilities, FrameParams, Instance, RenderBackend};

use super::{exec, parse};
use crate::app::Host;
use crate::app::test_backend::NullBackend;
use crate::settings::{Partial, Settings, test_caps};
use crate::viewer::Scene;

struct Empty;

impl Scene for Empty {
    fn title(&self) -> String {
        String::new()
    }
    fn init(&mut self, _: &mut dyn RenderBackend) -> anyhow::Result<()> {
        Ok(())
    }
    fn update(&mut self, _: &mut dyn RenderBackend, _: &crate::input::ActionState, _: f32) {}
    fn frame(&mut self, _: f32) -> (FrameParams, &[Instance]) {
        unreachable!()
    }
}

fn host(settings: &Settings, caps: Capabilities) -> Host {
    let mut host = Host::new(Box::new(Empty), settings, None);
    host.renderer = Some(Box::new(NullBackend::new(caps)));
    host
}

fn set(settings: &mut Settings, host: &mut Host, line: &str) -> Result<String, String> {
    let Some(parse::Command::Set { key, value }) = parse::parse(line).unwrap() else { panic!("not a set: {line}") };
    exec::set_live(settings, host, &key, &value)
}

#[test]
fn an_unavailable_value_is_refused_with_what_the_native_renderer_has() {
    let mut settings = Settings::from(Partial::default());
    let mut host = host(&settings, test_caps::native());
    let before = settings;
    assert_eq!(
        set(&mut settings, &mut host, "set upscaler dlss").unwrap_err(),
        "dlss is not available with the blackbox renderer (available: off, bilinear, fsr1)"
    );
    assert_eq!(
        set(&mut settings, &mut host, "post_aa taa").unwrap_err(),
        "taa is not available with the blackbox renderer (available: off, fxaa)"
    );
    assert_eq!(
        set(&mut settings, &mut host, "ray_tracing low").unwrap_err(),
        "low is not available with the blackbox renderer (available: off)"
    );
    assert_eq!(settings, before, "a refused value leaves the setting unchanged");
    assert_eq!(set(&mut settings, &mut host, "set upscaler bilinear").unwrap(), "upscaler = bilinear");
    assert_eq!(set(&mut settings, &mut host, "post_aa fxaa").unwrap(), "post_aa = fxaa");
}

#[test]
fn a_full_renderer_takes_the_values_and_ray_tracing_asks_for_a_restart_only_when_switched() {
    let mut settings = Settings::from(Partial::default());
    let mut host = host(&settings, test_caps::full());
    assert_eq!(set(&mut settings, &mut host, "set upscaler dlss").unwrap(), "upscaler = dlss");
    assert_eq!(set(&mut settings, &mut host, "post_aa taa").unwrap(), "post_aa = taa");
    assert_eq!(set(&mut settings, &mut host, "ray_tracing low").unwrap(), "ray_tracing = low (applies after restart)");
    assert_eq!(set(&mut settings, &mut host, "ray_tracing high").unwrap(), "ray_tracing = high");
    assert_eq!(set(&mut settings, &mut host, "ray_tracing off").unwrap(), "ray_tracing = off (applies after restart)");
}

#[test]
fn the_renderer_key_says_when_it_applies_and_when_the_build_cannot_run_it() {
    let mut settings = Settings::from(Partial::default());
    let mut host = host(&settings, test_caps::native());
    let text = set(&mut settings, &mut host, "renderer bevy").unwrap();
    assert!(text.starts_with("renderer = bevy (applies after restart)"), "{text}");
    // The note exists exactly in builds without the renderer-bevy feature.
    assert_eq!(text.contains("this build has no bevy renderer"), !cfg!(feature = "renderer-bevy"), "{text}");
    assert_eq!(
        set(&mut settings, &mut host, "renderer blackbox").unwrap(),
        "renderer = blackbox (applies after restart)"
    );
}

#[test]
fn gfx_reports_the_requested_and_effective_settings_of_the_running_renderer() {
    let mut settings = Settings::from(Partial {
        upscaler: Some(crate::settings::UpscaleMode::Dlss),
        post_aa: Some(crate::settings::PostAa::Taa),
        ..Partial::default()
    });
    let mut host = host(&settings, test_caps::native());
    let text = exec::gfx(&settings, &mut host).unwrap();
    assert!(text.contains("renderer:     blackbox\n"), "{text}");
    assert!(
        text.contains("  upscaler        dlss      -> fsr1      not supported by the blackbox renderer on vulkan"),
        "{text}"
    );
    assert!(
        text.contains("  antialiasing    taa       -> fxaa      not supported by the blackbox renderer on vulkan"),
        "{text}"
    );
    let applied = host.graphics.resolved().expect("gfx applies the request first");
    assert_eq!(applied.downgrades.len(), 2, "{:?}", applied.downgrades);
    assert_eq!(settings.upscaler, crate::settings::UpscaleMode::Dlss, "the stored request is untouched");
    set(&mut settings, &mut host, "set upscaler fsr1").unwrap();
    assert!(!exec::gfx(&settings, &mut host).unwrap().contains("upscaler        dlss"));
}

#[test]
fn gfx_is_a_built_in_command() {
    assert_eq!(parse::parse("gfx").unwrap(), Some(parse::Command::Gfx));
    assert!(parse::BUILT_IN.iter().any(|(usage, _)| *usage == "gfx"));
    assert_eq!(parse::complete("gf", &[]), ["gfx"]);
}
