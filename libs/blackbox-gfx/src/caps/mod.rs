//! What a renderer can do, and how a request is mapped onto it.

mod resolve;
#[cfg(test)]
mod resolve_tests;
mod set;
mod setting;

pub use resolve::{Downgrade, Note, Resolved, resolve};
pub use set::{AaSet, EnumSet, RestartSet, SetMember, TonemapSet, UpscalerSet};
pub use setting::Setting;

use crate::{Antialiasing, GraphicsApi, GraphicsSettings, MAX_RENDER_SCALE, MIN_RENDER_SCALE, Tonemap, Upscaler};

/// The denoiser a renderer has for ray-traced lighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Denoiser {
    /// NVIDIA DLSS Ray Reconstruction.
    DlssRr,
}

/// Whether and how well a renderer ray-traces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RtSupport {
    /// No hardware ray queries, or the renderer does not do ray tracing.
    #[default]
    None,
    /// Ray-traced lighting works; without a denoiser it is noisy.
    Available { denoiser: Option<Denoiser> },
}

impl RtSupport {
    pub fn is_available(self) -> bool {
        matches!(self, Self::Available { .. })
    }
}

/// What a backend can run, fixed once it is created. [`resolve`] maps a request onto it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Capabilities {
    /// Which renderer this is ("blackbox", "bevy"), for messages.
    pub renderer: &'static str,
    pub api: GraphicsApi,
    /// Whether DXT (BC1 to BC3) textures upload without CPU decoding.
    pub compressed_bc: bool,
    /// Whether the scene can be drawn in an HDR format (bloom and tone mapping need it).
    pub hdr_targets: bool,
    pub antialiasing: AaSet,
    pub upscalers: UpscalerSet,
    pub tonemaps: TonemapSet,
    pub bloom: bool,
    pub ray_tracing: RtSupport,
    /// Smallest and largest render scale. A temporal upscaler's quality mode must fit in it too.
    pub render_scale: (f32, f32),
    /// Settings that need a restart to change on this backend.
    pub restart_required: RestartSet,
}

impl Capabilities {
    /// The least a renderer can be: no anti-aliasing, upscaling, tone mapping, bloom or ray tracing,
    /// the render scale fixed at 1.0, and nothing needing a restart. Build real capabilities from it.
    pub fn baseline(renderer: &'static str, api: GraphicsApi) -> Self {
        Self {
            renderer,
            api,
            compressed_bc: false,
            hdr_targets: false,
            antialiasing: AaSet::of(&[Antialiasing::Off]),
            upscalers: UpscalerSet::of(&[Upscaler::Off]),
            tonemaps: TonemapSet::of(&[Tonemap::Off]),
            bloom: false,
            ray_tracing: RtSupport::None,
            render_scale: (1.0, 1.0),
            restart_required: RestartSet::empty(),
        }
    }

    /// The widest render scale range the settings types allow.
    pub const fn full_render_scale() -> (f32, f32) {
        (MIN_RENDER_SCALE, MAX_RENDER_SCALE)
    }

    /// The settings that changed from `old` to `new` and need a restart on this backend.
    pub fn restart_needed(&self, old: &GraphicsSettings, new: &GraphicsSettings) -> RestartSet {
        old.changed_settings(new).intersection(self.restart_required)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RayTracing;

    #[test]
    fn the_baseline_offers_only_off() {
        let caps = Capabilities::baseline("test", GraphicsApi::Vulkan);
        assert_eq!(caps.antialiasing.len(), 1);
        assert!(caps.upscalers.contains(Upscaler::Off) && caps.upscalers.len() == 1);
        assert!(caps.tonemaps.contains(Tonemap::Off) && caps.tonemaps.len() == 1);
        assert!(!caps.ray_tracing.is_available() && !caps.bloom);
        assert_eq!(caps.render_scale, (1.0, 1.0));
    }

    #[test]
    fn restart_needed_is_the_changed_restart_settings() {
        let mut caps = Capabilities::baseline("test", GraphicsApi::Auto);
        caps.restart_required = RestartSet::of(&[Setting::RayTracing]);
        let old = GraphicsSettings::default();
        let mut new = old;
        new.upscaler = Upscaler::Fsr1;
        assert!(caps.restart_needed(&old, &new).is_empty());
        new.ray_tracing = RayTracing::High;
        assert_eq!(caps.restart_needed(&old, &new), RestartSet::of(&[Setting::RayTracing]));
    }

    #[test]
    fn the_full_render_scale_matches_the_settings_range() {
        assert_eq!(Capabilities::full_render_scale(), (0.25, 2.0));
    }
}
