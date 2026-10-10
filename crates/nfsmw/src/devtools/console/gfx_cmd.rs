//! `gfx`: which renderer and GPU is drawing, what it can do, and what the graphics settings come to on it.

use blackbox_gfx::{BackendInfo, Capabilities, GraphicsSettings, Resolved, RtSupport, Setting};

use crate::settings::{RendererKind, Settings};

/// The report for the console. `resolved` is what the renderer made of the settings' current request;
/// `bevy_built` is whether this build has the Bevy renderer at all.
pub fn report(
    settings: &Settings,
    info: &BackendInfo,
    caps: &Capabilities,
    resolved: &Resolved,
    bevy_built: bool,
) -> String {
    let mut lines = vec![renderer_line(settings, info, bevy_built)];
    lines.push(format!("graphics api: {} (requested backend: {})", info.api, settings.backend));
    lines.push(format!("adapter:      {}", adapter(info)));
    lines.push("capabilities:".to_owned());
    lines.extend(capability_lines(caps));
    lines.push("settings (requested -> effective):".to_owned());
    lines.extend(setting_lines(&settings.graphics(), resolved));
    for note in &resolved.notes {
        lines.push(format!("note: {note}"));
    }
    lines.join("\n")
}

fn renderer_line(settings: &Settings, info: &BackendInfo, bevy_built: bool) -> String {
    let active = info.renderer;
    if settings.renderer.to_string() == active {
        return format!("renderer:     {active}");
    }
    let why = match settings.renderer == RendererKind::Bevy && !bevy_built {
        true => "this build has no bevy renderer",
        false => "applies after a restart",
    };
    format!("renderer:     {active} (requested {}: {why})", settings.renderer)
}

fn adapter(info: &BackendInfo) -> String {
    match info.driver.is_empty() {
        true => info.adapter.clone(),
        false => format!("{} (driver {})", info.adapter, info.driver),
    }
}

fn names(it: impl Iterator<Item = &'static str>) -> String {
    it.collect::<Vec<_>>().join(", ")
}

fn capability_lines(caps: &Capabilities) -> Vec<String> {
    let ray_tracing = match caps.ray_tracing {
        RtSupport::None => "no".to_owned(),
        RtSupport::Available { denoiser: Some(_) } => "yes, with a denoiser".to_owned(),
        RtSupport::Available { denoiser: None } => "yes, without a denoiser (noisy)".to_owned(),
    };
    let restart = match caps.restart_required.is_empty() {
        true => "nothing".to_owned(),
        false => names(caps.restart_required.iter().map(Setting::name)),
    };
    let yes_no = |b: bool| if b { "yes" } else { "no" };
    vec![
        format!("  anti-aliasing: {}", names(caps.antialiasing.iter().map(|a| a.name()))),
        format!("  upscalers:     {}", names(caps.upscalers.iter().map(|u| u.name()))),
        format!("  tone mapping:  {}", names(caps.tonemaps.iter().map(|t| t.name()))),
        format!("  bloom:         {}", yes_no(caps.bloom)),
        format!("  ray tracing:   {ray_tracing}"),
        format!("  render scale:  {:.2} to {:.2}", caps.render_scale.0, caps.render_scale.1),
        format!("  BC textures:   {}", yes_no(caps.compressed_bc)),
        format!("  needs restart: {restart}"),
    ]
}

/// The value of one setting in a request, as text.
fn value_of(setting: Setting, g: &GraphicsSettings) -> String {
    match setting {
        Setting::Antialiasing => g.post.antialiasing.name().to_owned(),
        Setting::Upscaler => g.upscaler.name().to_owned(),
        Setting::UpscaleQuality => g.upscale_quality.name().to_owned(),
        Setting::RenderScale => format!("{:.2}", g.render_scale),
        Setting::Tonemap => g.post.tonemap.name().to_owned(),
        Setting::Bloom if g.post.bloom_intensity > 0.0 => format!("{:.2}", g.post.bloom_intensity),
        Setting::Bloom => "off".to_owned(),
        Setting::RayTracing => g.ray_tracing.name().to_owned(),
    }
}

fn setting_lines(asked: &GraphicsSettings, resolved: &Resolved) -> Vec<String> {
    Setting::ALL
        .into_iter()
        .map(|setting| {
            let (requested, effective) = (value_of(setting, asked), value_of(setting, &resolved.effective));
            let line = format!("  {:<16}{requested:<9} -> {effective:<9}", setting.name());
            match (resolved.downgrade_of(setting), setting) {
                (Some(downgrade), _) => format!("{line} {}", downgrade.reason),
                (None, Setting::RenderScale) if requested != effective => {
                    format!("{line} set by the upscale quality")
                }
                (None, _) => line.trim_end().to_owned(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use blackbox_gfx::{GraphicsApi, resolve};

    use super::*;
    use crate::settings::{Partial, PostAa, RayTracingLevel, RendererKind, UpscaleMode, test_caps};

    fn info(renderer: &'static str) -> BackendInfo {
        BackendInfo { renderer, api: GraphicsApi::Vulkan, adapter: "Test GPU".into(), driver: "1.2.3".into() }
    }

    fn gfx(partial: Partial, caps: &Capabilities) -> String {
        let settings = Settings::from(partial);
        report(&settings, &info(caps.renderer), caps, &resolve(&settings.graphics(), caps), false)
    }

    #[test]
    fn the_defaults_on_the_native_renderer_are_all_honoured() {
        let text = gfx(Partial::default(), &test_caps::native());
        assert!(text.starts_with("renderer:     blackbox\n"), "{text}");
        for line in [
            "graphics api: vulkan (requested backend: auto)",
            "adapter:      Test GPU (driver 1.2.3)",
            "  upscalers:     off, bilinear, fsr1",
            "  anti-aliasing: off, fxaa",
            "  ray tracing:   no",
            "  needs restart: nothing",
            "  upscaler        fsr1      -> fsr1",
        ] {
            assert!(text.lines().any(|l| l == line), "missing {line:?} in\n{text}");
        }
        assert!(!text.contains("not supported"), "{text}");
    }

    #[test]
    fn requests_the_native_renderer_cannot_run_show_their_downgrades_and_reasons() {
        let asked = Partial {
            post_aa: Some(PostAa::Taa),
            upscaler: Some(UpscaleMode::Dlss),
            ray_tracing: Some(RayTracingLevel::High),
            ..Partial::default()
        };
        let text = gfx(asked, &test_caps::native());
        let want = [
            "  antialiasing    taa       -> fxaa      not supported by the blackbox renderer on vulkan",
            "  upscaler        dlss      -> fsr1      not supported by the blackbox renderer on vulkan",
            "  render scale    1.00      -> 0.67      set by the upscale quality",
            "  ray tracing     high      -> off       not supported by the blackbox renderer on vulkan",
        ];
        for line in want {
            assert!(text.lines().any(|l| l.trim_end() == line.trim_end()), "missing {line:?} in\n{text}");
        }
    }

    #[test]
    fn a_full_renderer_lists_what_it_can_do_and_its_restart_rule() {
        let text = gfx(Partial { renderer: Some(RendererKind::Bevy), ..Partial::default() }, &test_caps::full());
        assert!(text.starts_with("renderer:     bevy\n"), "{text}");
        assert!(text.contains("  upscalers:     off, bilinear, fsr1, fsr3, fsr4, dlss"), "{text}");
        assert!(text.contains("  ray tracing:   yes, with a denoiser"), "{text}");
        assert!(text.contains("  needs restart: ray tracing"), "{text}");
    }

    #[test]
    fn a_requested_renderer_that_is_not_running_says_why() {
        let settings = Settings::from(Partial { renderer: Some(RendererKind::Bevy), ..Partial::default() });
        let caps = test_caps::native();
        let resolved = resolve(&settings.graphics(), &caps);
        let not_built = report(&settings, &info("blackbox"), &caps, &resolved, false);
        assert!(
            not_built.starts_with("renderer:     blackbox (requested bevy: this build has no bevy renderer)"),
            "{not_built}"
        );
        let built = report(&settings, &info("blackbox"), &caps, &resolved, true);
        assert!(built.starts_with("renderer:     blackbox (requested bevy: applies after a restart)"), "{built}");
    }
}
