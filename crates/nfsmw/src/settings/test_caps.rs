//! Capabilities for tests, so settings, console and menu code can be tried against both kinds of renderer
//! without a GPU: the native renderer's frozen feature set, and a fake renderer that can do everything.

use blackbox_gfx::{
    AaSet, Antialiasing, Capabilities, GraphicsApi, RestartSet, RtSupport, TonemapSet, Upscaler, UpscalerSet,
};

/// What the native `blackbox` renderer reports on Vulkan: FXAA, bloom, tone mapping, bilinear and FSR 1.
pub fn native() -> Capabilities {
    Capabilities {
        renderer: "blackbox",
        api: GraphicsApi::Vulkan,
        compressed_bc: true,
        hdr_targets: true,
        antialiasing: AaSet::of(&[Antialiasing::Off, Antialiasing::Fxaa]),
        upscalers: UpscalerSet::of(&[Upscaler::Off, Upscaler::Bilinear, Upscaler::Fsr1]),
        tonemaps: TonemapSet::all(),
        bloom: true,
        ray_tracing: RtSupport::None,
        render_scale: Capabilities::full_render_scale(),
        restart_required: RestartSet::empty(),
    }
}
