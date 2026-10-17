//! The numbers FSR 3 takes from the view, as plain functions with no Bevy world behind them: the render size,
//! the jitter, the motion-vector scale, the projection's near plane and field of view, and the sharpness.
//!
//! Sizes are `[u32; 2]` and offsets `[f32; 2]` at this boundary, because Bevy's math types (glam 0.33) and the
//! upscaler's (glam 0.34, the workspace's) are different types even though they have the same shape.

use bevy_math::Mat4;
use blackbox_gfx::clamp_upscale_sharpness;
use glam::UVec2;

/// The size the scene is drawn at for a display of `display` pixels at `render_scale`. The quality modes
/// (`UpscaleQuality::render_scale`) reach here through the resolver, so this is the shared table, not a second
/// one. At 1.0 the scene is drawn at the display size and nothing is upscaled.
pub fn render_size(display: [u32; 2], render_scale: f32) -> [u32; 2] {
    crate::post::scale::render_size(display, render_scale).map_or(display, |(width, height)| [width, height])
}

/// The sub-pixel jitter of frame `frame` for a scene drawn at `render` and shown at `display`, in render pixels
/// (`fsr3_wgpu::jitter::offset_for_sizes`: a Halton(2, 3) sequence, its length set by the ratio). The same value is
/// the camera's `TemporalJitter::offset`, which Bevy turns into the projection offset, and is passed to the upscaler.
pub fn jitter(frame: u32, render: [u32; 2], display: [u32; 2]) -> [f32; 2] {
    fsr3_wgpu::jitter::offset_for_sizes(frame, UVec2::from_array(render), UVec2::from_array(display)).to_array()
}

/// The `motion_vector_scale` that turns Bevy's motion vectors into the upscaler's.
///
/// Bevy writes `current - previous` in UV units with y down (`bevy_pbr`'s `prepass.wesl`, `* vec2(0.5, -0.5)` of the
/// NDC difference; TAA reads it as `history_uv = uv - motion`). The upscaler wants `previous - current` in pixels, so
/// both the sign and the size change: the scale is minus the render size.
pub fn motion_vector_scale(render: [u32; 2]) -> [f32; 2] {
    [-(render[0] as f32), -(render[1] as f32)]
}

/// The sharpening the upscaler applies: `None` when the setting is zero (the pass is skipped), else the
/// setting clamped to 0..=1 (AMD's default is 0.8).
pub fn sharpness(setting: f32) -> Option<f32> {
    let sharpness = clamp_upscale_sharpness(setting);
    (sharpness > 0.0).then_some(sharpness)
}

/// The near plane and the vertical field of view, in radians, of a perspective projection, read back from its matrix.
///
/// Bevy's perspective is infinite reverse-Z (`Mat4::perspective_infinite_reverse_rh`): `w_axis.z` is the near plane
/// and `y_axis.y` is `1 / tan(fov_y / 2)`. The jitter only touches `z_axis.xy`, so it does not change either value.
/// `None` for an orthographic projection (`w_axis.w == 1`), which the upscaler does not take.
pub fn near_and_fov_y(clip_from_view: Mat4) -> Option<(f32, f32)> {
    if clip_from_view.w_axis.w == 1.0 {
        return None;
    }
    let near = clip_from_view.w_axis.z;
    let focal = clip_from_view.y_axis.y;
    if !(near.is_finite() && near > 0.0 && focal.is_finite() && focal > 0.0) {
        return None;
    }
    Some((near, 2.0 * (1.0 / focal).atan()))
}
