//! Capabilities for tests, so settings, console and menu code can be tried against both kinds of renderer
//! without a GPU: the native renderer's frozen feature set, and a fake renderer that can do everything.

use blackbox_gfx::{
    AaSet, Antialiasing, Capabilities, Denoiser, GraphicsApi, RestartSet, RtSupport, Setting, TonemapSet, Upscaler,
    UpscalerSet,
};

/// A fake renderer with every method, every upscaler and ray tracing with a denoiser (like a full Bevy build on
/// an NVIDIA card). Ray tracing needs a restart to switch.
pub fn full() -> Capabilities {
    Capabilities {
        renderer: "bevy",
        soft_shadows: true,
        antialiasing: AaSet::all(),
        upscalers: UpscalerSet::all(),
        ray_tracing: RtSupport::Available { denoiser: Some(Denoiser::DlssRr) },
        restart_required: RestartSet::of(&[Setting::RayTracing]),
        ..native()
    }
}

/// What the native `blackbox` renderer reports on Vulkan with the storage limits of FSR 3 missing: FXAA, bloom, tone
/// mapping, bilinear and FSR 1. [`native_fsr3`] is the same renderer on a device that has them.
pub fn native() -> Capabilities {
    Capabilities {
        renderer: "blackbox",
        api: GraphicsApi::Vulkan,
        compressed_bc: true,
        hdr_targets: true,
        antialiasing: AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa, Antialiasing::Smaa]),
        upscalers: UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]),
        tonemaps: TonemapSet::all(),
        bloom: true,
        scene_treatment: true,
        radial_blur: true,
        frame_generation: false,
        local_reflections: true,
        ambient_occlusion: true,
        soft_shadows: false,
        ray_tracing: RtSupport::None,
        render_scale: Capabilities::full_render_scale(),
        restart_required: RestartSet::empty(),
    }
}

/// The native renderer on a device that can run the FSR 3 upscaler (any Vulkan or DirectX 12 GPU of this decade):
/// [`native`] plus the temporal method.
pub fn native_fsr3() -> Capabilities {
    let mut caps = native();
    caps.upscalers.insert(Upscaler::Fsr3);
    caps
}
