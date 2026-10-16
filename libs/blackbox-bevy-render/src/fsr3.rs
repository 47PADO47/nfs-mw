//! FSR 3.1 temporal upscaler on the Bevy backend (PR 13b, `feat/gfx-fsr3`), behind the `fsr3` feature.
//!
//! The upscaler itself is `libs/fsr3-wgpu`. This module wires it into Bevy's 3D graph in place of TAA:
//!
//! - `component`: the main-world components an FSR 3 camera carries (prepasses, jitter, `Hdr`, the temporal mip bias,
//!   and the `Fsr3Upscale` marker with the camera-cut reset). Put on or taken off every frame by `apply/mod.rs`.
//! - `sizes`: the render size, the jitter, the motion-vector scale and the projection's near plane and field of view,
//!   as plain functions.
//! - `node`: the render-world systems. The jitter goes with view preparation; the upscale runs in
//!   `Core3dSystems::EarlyPostProcess` where TAA runs, and blits its display-size output into the view target.
//!
//! The render size comes from the same quality table as the other temporal upscalers
//! (`blackbox_gfx::UpscaleQuality::render_scale`, resolved into `render_scale` by `caps::resolve`), and the render-scale
//! pass draws the screen camera at that size (`post::scale::apply_resolution_override`). `docs/upscaling.md` has the
//! status.

mod component;
#[cfg(test)]
mod gpu_tests;
mod node;
mod sizes;
#[cfg(test)]
mod tests;

pub use component::set;

use bevy_app::App;
use bevy_render::RenderApp;
use bevy_render::renderer::RenderDevice;
use blackbox_gfx::{Upscaler, UpscalerSet};

/// Offer FSR 3 in the caps when the device can run it (`support`).
pub fn offer(upscalers: &mut UpscalerSet, supported: bool) {
    if !supported {
        return;
    }
    upscalers.insert(Upscaler::Fsr3);
}

/// Whether the device can run the upscaler. Its storage formats are the ones WebGPU guarantees (`R32Float` read-write,
/// `Rg32Float` and `Rgba16Float` write-only), so the check that is left is the storage limits its passes use: four
/// storage textures and eight storage buffers per stage, wgpu's defaults.
pub fn support(device: &RenderDevice) -> bool {
    let limits = device.limits();
    limits.max_storage_textures_per_shader_stage >= 4 && limits.max_storage_buffers_per_shader_stage >= 8
}

/// Add the FSR 3 systems to the render world: the `Fsr3Upscale` extraction from the main world, the jitter, and the
/// upscale. Called from `BlackboxBevyRenderPlugin::build` after the render plugin is added.
pub fn install(app: &mut App) {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    node::add_systems(render_app);
}
