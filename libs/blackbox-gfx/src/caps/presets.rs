//! Preset capabilities for tests and for mock builds, so the resolver and the menus can be exercised without a GPU.

use crate::{GraphicsApi, Upscaler};

use super::{AaSet, Capabilities, Denoiser, RestartSet, RtSupport, Setting, TonemapSet, UpscalerSet};

impl Capabilities {
    /// A renderer with DLSS Super Resolution and Ray Reconstruction, as on an NVIDIA RTX GPU under Vulkan.
    ///
    /// It has everything the native renderer has, plus TAA and SMAA, Off, Bilinear, FSR 1 and DLSS (no FSR 3 or
    /// FSR 4, so a DLSS request falls back straight to FSR 1). Ray tracing runs with DLSS-RR as its denoiser, and
    /// it starts at launch, so it needs a restart. The tests use it, and the Bevy backend's `dlss-mock` path
    /// reports it when the DLSS SDK is not installed.
    pub fn mock_dlss(renderer: &'static str, api: GraphicsApi) -> Self {
        Self {
            compressed_bc: true,
            hdr_targets: true,
            antialiasing: AaSet::all(),
            upscalers: UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1, Upscaler::Dlss]),
            tonemaps: TonemapSet::all(),
            bloom: true,
            scene_treatment: true,
            local_reflections: true,
            ambient_occlusion: true,
            soft_shadows: true,
            ray_tracing: RtSupport::Available { denoiser: Some(Denoiser::DlssRr) },
            render_scale: Self::full_render_scale(),
            restart_required: RestartSet::of(&[Setting::RayTracing]),
            ..Self::baseline(renderer, api)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_dlss_offers_dlss_and_ray_tracing_with_dlss_rr() {
        let caps = Capabilities::mock_dlss("bevy", GraphicsApi::Vulkan);
        assert_eq!((caps.renderer, caps.api), ("bevy", GraphicsApi::Vulkan));
        assert!(caps.upscalers.contains(Upscaler::Dlss));
        assert!(!caps.upscalers.contains(Upscaler::Fsr3) && !caps.upscalers.contains(Upscaler::Fsr4));
        assert_eq!(caps.ray_tracing, RtSupport::Available { denoiser: Some(Denoiser::DlssRr) });
        assert!(caps.antialiasing.contains(crate::Antialiasing::Taa));
    }

    #[test]
    fn mock_dlss_restarts_for_ray_tracing_and_nothing_else() {
        let caps = Capabilities::mock_dlss("bevy", GraphicsApi::Vulkan);
        assert_eq!(caps.restart_required, RestartSet::of(&[Setting::RayTracing]));
        let off = crate::GraphicsSettings::default();
        let low = crate::GraphicsSettings { ray_tracing: crate::RayTracing::Low, ..off };
        assert_eq!(caps.restart_needed(&off, &low), RestartSet::of(&[Setting::RayTracing]));
        let upscaled = crate::GraphicsSettings { upscaler: Upscaler::Dlss, ..off };
        assert!(caps.restart_needed(&off, &upscaled).is_empty(), "the upscaler applies at once");
    }
}
