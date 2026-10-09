//! Capability-aware validation of `set`: a value the running renderer cannot do is refused, with what it can do.
//!
//! A value stored in the config file for a renderer you do not run now is left alone (the player may switch
//! renderer later); this only guards what is changed while the game runs.

use blackbox_gfx::{Antialiasing, Capabilities, GraphicsSettings, Setting, Tonemap, Upscaler};

use crate::settings::Settings;

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
                true => blackbox_gfx::RayTracing::ALL.iter().map(|r| r.name()).collect(),
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
    use crate::devtools::console::settings_cmd;
    use crate::settings::{Partial, test_caps};

    fn set(settings: &mut Settings, caps: &Capabilities, key: &str, value: &str) -> Result<String, String> {
        let before = *settings;
        let text = settings_cmd::set(settings, key, value)?;
        if let Err(e) = check(key, settings, caps) {
            *settings = before;
            return Err(e);
        }
        Ok(text)
    }

    #[test]
    fn an_upscaler_the_native_renderer_lacks_is_refused_with_what_it_has() {
        let (mut s, caps) = (Settings::from(Partial::default()), test_caps::native());
        let before = s;
        assert_eq!(
            set(&mut s, &caps, "upscaler", "dlss").unwrap_err(),
            "dlss is not available with the blackbox renderer (available: off, bilinear, fsr1)"
        );
        assert_eq!(s, before, "the value is unchanged");
        assert!(set(&mut s, &caps, "upscaler", "bilinear").is_ok());
    }

    #[test]
    fn the_other_native_limits_are_refused_the_same_way() {
        let (mut s, caps) = (Settings::from(Partial::default()), test_caps::native());
        let before = s;
        for (key, value, error) in [
            ("post_aa", "taa", "taa is not available with the blackbox renderer (available: off, fxaa)"),
            ("post_aa", "smaa", "smaa is not available with the blackbox renderer (available: off, fxaa)"),
            ("ray_tracing", "low", "low is not available with the blackbox renderer (available: off)"),
            ("upscaler", "fsr4", "fsr4 is not available with the blackbox renderer (available: off, bilinear, fsr1)"),
        ] {
            assert_eq!(set(&mut s, &caps, key, value).unwrap_err(), error);
        }
        assert_eq!(s, before);
        for (key, value) in
            [("post_aa", "fxaa"), ("post_bloom", "high"), ("post_tonemap", "aces"), ("ray_tracing", "off")]
        {
            assert!(set(&mut s, &caps, key, value).is_ok(), "{key} {value}");
        }
    }

    #[test]
    fn a_full_renderer_accepts_everything_and_other_keys_are_never_refused() {
        let (mut s, caps) = (Settings::from(Partial::default()), test_caps::full());
        for (key, value) in
            [("upscaler", "dlss"), ("post_aa", "taa"), ("ray_tracing", "high"), ("upscale_quality", "balanced")]
        {
            assert!(set(&mut s, &caps, key, value).is_ok(), "{key} {value}");
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
