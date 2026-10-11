use blackbox_gfx::{Setting, resolve};

use super::*;
use crate::settings::test_caps;
use crate::settings::{Partial, PostAa, UpscaleMode};

fn request(aa: PostAa, upscaler: UpscaleMode) -> GraphicsSettings {
    Settings::from(Partial { post_aa: Some(aa), upscaler: Some(upscaler), ..Partial::default() }).graphics()
}

/// Sends through `resolve` against `caps`, counting how often the "renderer" is called.
fn send<'a>(
    caps: &'a blackbox_gfx::Capabilities,
    calls: &'a mut u32,
) -> impl FnOnce(&GraphicsSettings) -> Resolved + 'a {
    move |request| {
        *calls += 1;
        resolve(request, caps)
    }
}

#[test]
fn a_request_the_renderer_already_has_is_not_sent_again() {
    let (mut applied, caps, mut calls) = (Applied::default(), test_caps::native(), 0);
    let ask = request(PostAa::Off, UpscaleMode::Fsr1);
    assert!(applied.apply_with(ask, send(&caps, &mut calls)));
    assert!(!applied.apply_with(ask, send(&caps, &mut calls)));
    assert_eq!(calls, 1);
    assert!(applied.apply_with(request(PostAa::Fxaa, UpscaleMode::Fsr1), send(&caps, &mut calls)));
    assert_eq!(calls, 2);
}

#[test]
fn the_resolved_settings_are_kept_for_the_console() {
    let (mut applied, caps, mut calls) = (Applied::default(), test_caps::native(), 0);
    assert!(applied.resolved().is_none(), "nothing before the first request");
    applied.apply_with(request(PostAa::Taa, UpscaleMode::Dlss), send(&caps, &mut calls));
    let resolved = applied.resolved().expect("kept");
    assert!(
        resolved.downgrade_of(Setting::Antialiasing).is_some() && resolved.downgrade_of(Setting::Upscaler).is_some()
    );
    assert_eq!(resolved.effective.upscaler, blackbox_gfx::Upscaler::Fsr1);
}

#[test]
fn downgrades_are_logged_once_and_again_only_when_they_change() {
    let caps = test_caps::native();
    let taa = resolve(&request(PostAa::Taa, UpscaleMode::Fsr1), &caps);
    let first = change_lines(None, &taa);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].0, Level::Warn);
    assert_eq!(first[0].1, "antialiasing: taa -> fxaa (not supported by the blackbox renderer on vulkan)");
    assert!(change_lines(Some(&taa), &taa).is_empty(), "the same downgrade is not logged twice");

    let both = resolve(&request(PostAa::Taa, UpscaleMode::Dlss), &caps);
    assert_eq!(change_lines(Some(&taa), &both).len(), 2, "a new downgrade lists them all");

    let exact = resolve(&request(PostAa::Fxaa, UpscaleMode::Fsr1), &caps);
    assert_eq!(change_lines(Some(&both), &exact), [(Level::Info, "every requested setting is in effect".to_owned())]);
    assert!(change_lines(None, &exact).is_empty(), "a request that is honoured from the start says nothing");
}

#[test]
fn notes_are_logged_at_info_level() {
    let noisy = test_caps::full();
    let asked =
        Settings::from(Partial { ray_tracing: Some(crate::settings::RayTracingLevel::Low), ..Partial::default() });
    let no_denoiser =
        blackbox_gfx::Capabilities { ray_tracing: blackbox_gfx::RtSupport::Available { denoiser: None }, ..noisy };
    let resolved = resolve(&asked.graphics(), &no_denoiser);
    let lines = change_lines(None, &resolved);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].0, Level::Info);
    assert!(lines[0].1.contains("noisy"), "{}", lines[0].1);
}
