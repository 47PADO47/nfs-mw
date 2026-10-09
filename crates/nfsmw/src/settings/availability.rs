//! What the running renderer offers of each graphics setting, for the console (`set` refuses what it cannot do)
//! and the menus (rows cycle only through what it can run).
//!
//! A value stored in the config file for a renderer you do not run now is left alone (the player may switch
//! renderer later); only what is changed while the game runs is checked.

use blackbox_gfx::{Antialiasing, Capabilities, GraphicsSettings, RayTracing, Setting, Tonemap, Upscaler};

use super::Settings;

/// What a setting asks for, whether the renderer can do it, and the values it can do.
struct Asked {
    value: String,
    possible: bool,
    available: Vec<&'static str>,
}

fn asked(key: &str, settings: &Settings, caps: &Capabilities) -> Option<Asked> {
    let g = settings.graphics();
    Some(match key {
        "post_aa" => Asked {
            value: g.post.antialiasing.name().to_owned(),
            possible: g.post.antialiasing == Antialiasing::Off || caps.antialiasing.contains(g.post.antialiasing),
            available: caps.antialiasing.iter().map(Antialiasing::name).collect(),
        },
        "upscaler" => Asked {
            value: g.upscaler.name().to_owned(),
            possible: g.upscaler == Upscaler::Off || caps.upscalers.contains(g.upscaler),
            available: caps.upscalers.iter().map(Upscaler::name).collect(),
        },
        "post_tonemap" => Asked {
            value: g.post.tonemap.name().to_owned(),
            possible: g.post.tonemap == Tonemap::Off || caps.tonemaps.contains(g.post.tonemap),
            available: caps.tonemaps.iter().map(Tonemap::name).collect(),
        },
        "post_bloom" => Asked {
            value: settings.post_bloom.to_string(),
            possible: g.post.bloom_intensity <= 0.0 || caps.bloom,
            available: vec!["off"],
        },
        "ray_tracing" => Asked {
            value: g.ray_tracing.name().to_owned(),
            possible: !g.ray_tracing.is_on() || caps.ray_tracing.is_available(),
            available: match caps.ray_tracing.is_available() {
                true => RayTracing::ALL.iter().map(|r| r.name()).collect(),
                false => vec!["off"],
            },
        },
        _ => return None,
    })
}

/// Whether `settings`, just changed at `key`, ask the renderer for something it cannot do. The error text names
/// the value, the renderer and what it offers.
pub fn check(key: &str, settings: &Settings, caps: &Capabilities) -> Result<(), String> {
    let Some(asked) = asked(key, settings, caps) else { return Ok(()) };
    if asked.possible {
        return Ok(());
    }
    Err(format!(
        "{} is not available with the {} renderer (available: {})",
        asked.value,
        caps.renderer,
        asked.available.join(", ")
    ))
}

/// Whether going from `before` to `after` needs a restart on this renderer. Ray tracing only does when it is
/// switched on or off, not when its quality changes.
pub fn needs_restart(caps: &Capabilities, before: &GraphicsSettings, after: &GraphicsSettings) -> bool {
    let mut restart = caps.restart_needed(before, after);
    if before.ray_tracing.is_on() == after.ray_tracing.is_on() {
        restart.remove(Setting::RayTracing);
    }
    !restart.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Partial;
    use crate::settings::{PostAa, PostBloom, PostTonemap, RayTracingLevel, UpscaleMode, test_caps};

    fn settings() -> Settings {
        Settings::from(Partial::default())
    }

    #[test]
    fn an_upscaler_the_native_renderer_lacks_is_refused_with_what_it_has() {
        let caps = test_caps::native();
        let s = Settings { upscaler: UpscaleMode::Dlss, ..settings() };
        assert_eq!(
            check("upscaler", &s, &caps).unwrap_err(),
            "dlss is not available with the blackbox renderer (available: off, bilinear, fsr1)"
        );
        let ok = Settings { upscaler: UpscaleMode::Bilinear, ..s };
        assert!(check("upscaler", &ok, &caps).is_ok());
    }

    #[test]
    fn the_other_native_limits_are_refused_the_same_way() {
        let caps = test_caps::native();
        let taa = Settings { post_aa: PostAa::Taa, ..settings() };
        let smaa = Settings { post_aa: PostAa::Smaa, ..settings() };
        let rt = Settings { ray_tracing: RayTracingLevel::Low, ..settings() };
        let fsr4 = Settings { upscaler: UpscaleMode::Fsr4, ..settings() };
        for (key, s, error) in [
            ("post_aa", taa, "taa is not available with the blackbox renderer (available: off, fxaa)"),
            ("post_aa", smaa, "smaa is not available with the blackbox renderer (available: off, fxaa)"),
            ("ray_tracing", rt, "low is not available with the blackbox renderer (available: off)"),
            ("upscaler", fsr4, "fsr4 is not available with the blackbox renderer (available: off, bilinear, fsr1)"),
        ] {
            assert_eq!(check(key, &s, &caps).unwrap_err(), error);
        }
        let fine = Settings {
            post_aa: PostAa::Fxaa,
            post_bloom: PostBloom::High,
            post_tonemap: PostTonemap::Aces,
            ..settings()
        };
        for key in ["post_aa", "post_bloom", "post_tonemap", "ray_tracing"] {
            assert!(check(key, &fine, &caps).is_ok(), "{key}");
        }
    }

    #[test]
    fn a_renderer_without_bloom_or_tone_mapping_refuses_them() {
        let caps = blackbox_gfx::Capabilities::baseline("tiny", blackbox_gfx::GraphicsApi::Gl);
        let s = Settings { post_bloom: PostBloom::Low, post_tonemap: PostTonemap::Aces, ..settings() };
        assert_eq!(
            check("post_bloom", &s, &caps).unwrap_err(),
            "low is not available with the tiny renderer (available: off)"
        );
        assert_eq!(
            check("post_tonemap", &s, &caps).unwrap_err(),
            "aces is not available with the tiny renderer (available: off)"
        );
    }

    #[test]
    fn a_full_renderer_accepts_everything_and_other_keys_are_never_refused() {
        let caps = test_caps::full();
        let s = Settings {
            upscaler: UpscaleMode::Dlss,
            post_aa: PostAa::Taa,
            ray_tracing: RayTracingLevel::High,
            ..settings()
        };
        for key in ["upscaler", "post_aa", "ray_tracing", "upscale_quality"] {
            assert!(check(key, &s, &caps).is_ok(), "{key}");
        }
        let native = test_caps::native();
        assert!(check("upscale_quality", &s, &native).is_ok(), "only used by the temporal upscalers, never refused");
        assert!(check("vsync", &s, &native).is_ok());
    }

    #[test]
    fn only_switching_ray_tracing_on_or_off_needs_a_restart() {
        let (caps, native) = (test_caps::full(), test_caps::native());
        let graphics = |text: &str| {
            Settings::from(Partial { ray_tracing: Some(text.parse().unwrap()), ..Partial::default() }).graphics()
        };
        assert!(needs_restart(&caps, &graphics("off"), &graphics("low")));
        assert!(!needs_restart(&caps, &graphics("low"), &graphics("high")));
        assert!(needs_restart(&caps, &graphics("high"), &graphics("off")));
        assert!(!needs_restart(&native, &graphics("off"), &graphics("high")), "the native renderer never asks");
    }
}
