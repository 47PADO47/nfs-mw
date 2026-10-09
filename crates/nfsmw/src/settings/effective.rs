//! The settings as a given renderer runs them, for showing in the menus.
//!
//! The stored settings are what the player asked for and what the config file keeps (a value for a renderer you do
//! not run now stays there). A menu row should say what is happening, so it reads this view instead: `taa` on the
//! native renderer reads `fxaa`, `dlss` reads `fsr1`.

use blackbox_gfx::{Antialiasing, Capabilities, RayTracing, Tonemap, Upscaler, resolve};

use super::{PostAa, PostBloom, PostTonemap, RayTracingLevel, Settings, UpscaleMode};

impl Settings {
    /// These settings with every value the renderer cannot run replaced by what it runs instead
    /// (`blackbox_gfx::resolve`). Only the capability-limited settings change; the rest is as stored.
    pub fn effective(&self, caps: &Capabilities) -> Settings {
        let run = resolve(&self.graphics(), caps).effective;
        Settings {
            post_aa: aa(run.post.antialiasing),
            upscaler: upscaler(run.upscaler),
            post_tonemap: tonemap(run.post.tonemap),
            post_bloom: if run.post.bloom_intensity > 0.0 { self.post_bloom } else { PostBloom::Off },
            ray_tracing: ray_tracing(run.ray_tracing),
            ..*self
        }
    }
}

fn aa(a: Antialiasing) -> PostAa {
    match a {
        Antialiasing::Off => PostAa::Off,
        Antialiasing::Fxaa => PostAa::Fxaa,
        Antialiasing::Smaa => PostAa::Smaa,
        Antialiasing::Taa => PostAa::Taa,
    }
}

fn upscaler(u: Upscaler) -> UpscaleMode {
    match u {
        Upscaler::Off => UpscaleMode::Off,
        Upscaler::Bilinear => UpscaleMode::Bilinear,
        Upscaler::Fsr1 => UpscaleMode::Fsr1,
        Upscaler::Fsr3 => UpscaleMode::Fsr3,
        Upscaler::Fsr4 => UpscaleMode::Fsr4,
        Upscaler::Dlss => UpscaleMode::Dlss,
    }
}

fn tonemap(t: Tonemap) -> PostTonemap {
    match t {
        Tonemap::Off => PostTonemap::Off,
        Tonemap::Aces => PostTonemap::Aces,
    }
}

fn ray_tracing(r: RayTracing) -> RayTracingLevel {
    match r {
        RayTracing::Off => RayTracingLevel::Off,
        RayTracing::Low => RayTracingLevel::Low,
        RayTracing::Medium => RayTracingLevel::Medium,
        RayTracing::High => RayTracingLevel::High,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Partial, test_caps};

    fn asked() -> Settings {
        Settings::from(Partial {
            post_aa: Some(PostAa::Taa),
            upscaler: Some(UpscaleMode::Dlss),
            ray_tracing: Some(RayTracingLevel::High),
            post_bloom: Some(PostBloom::Medium),
            post_tonemap: Some(PostTonemap::Aces),
            ..Partial::default()
        })
    }

    #[test]
    fn the_native_renderer_shows_what_it_runs_and_the_stored_values_stay() {
        let stored = asked();
        let shown = stored.effective(&test_caps::native());
        assert_eq!(
            (shown.post_aa, shown.upscaler, shown.ray_tracing),
            (PostAa::Fxaa, UpscaleMode::Fsr1, RayTracingLevel::Off)
        );
        assert_eq!((shown.post_bloom, shown.post_tonemap), (PostBloom::Medium, PostTonemap::Aces), "it can do these");
        assert_eq!(
            (stored.post_aa, stored.upscaler, stored.ray_tracing),
            (PostAa::Taa, UpscaleMode::Dlss, RayTracingLevel::High)
        );
        assert_eq!(
            Settings {
                post_aa: PostAa::Fxaa,
                upscaler: UpscaleMode::Fsr1,
                ray_tracing: RayTracingLevel::Off,
                ..stored
            },
            shown
        );
    }

    #[test]
    fn a_full_renderer_shows_the_settings_as_they_are() {
        // A temporal upscaler replaces anti-aliasing, which is the one change a full renderer makes.
        let shown = asked().effective(&test_caps::full());
        assert_eq!((shown.upscaler, shown.ray_tracing), (UpscaleMode::Dlss, RayTracingLevel::High));
        assert_eq!(shown.post_aa, PostAa::Off, "dlss does its own anti-aliasing");
        let plain = Settings { upscaler: UpscaleMode::Fsr1, ..asked() };
        assert_eq!(plain.effective(&test_caps::full()), plain);
    }

    #[test]
    fn a_renderer_without_bloom_shows_it_off() {
        let caps = blackbox_gfx::Capabilities::baseline("tiny", blackbox_gfx::GraphicsApi::Gl);
        let shown =
            Settings::from(Partial { post_bloom: Some(PostBloom::High), ..Partial::default() }).effective(&caps);
        assert_eq!(shown.post_bloom, PostBloom::Off);
    }
}
