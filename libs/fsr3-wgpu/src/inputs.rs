//! What the application hands the upscaler each frame.

use glam::{UVec2, Vec2};

/// The textures and per-frame values of one upscale.
///
/// All textures are sampled with integer loads, so any of the usual formats works:
///
/// - `color`: the scene at render resolution, `Rgba16Float` (HDR) or `Rgba8Unorm` (LDR), linear. For
///   best results it is not tone mapped (see [`crate::Fsr3Config::hdr`]), without jitter-sensitive post
///   effects such as bloom or film grain applied.
/// - `depth`: `Depth32Float` or a 32-bit float colour texture holding the depth buffer, at render
///   resolution, laid out as [`crate::Fsr3Config::depth`] says.
/// - `motion_vectors`: two-channel (the first two channels are used) float vectors per pixel, the
///   previous position minus the current position, so that `uv + motion = previous uv`. The vectors are
///   multiplied by `motion_vector_scale` to get pixels.
///
/// Every texture needs the `TEXTURE_BINDING` usage.
pub struct Fsr3Inputs<'a> {
    /// The jittered scene colour at render resolution.
    pub color: &'a wgpu::TextureView,
    /// The depth buffer at render resolution.
    pub depth: &'a wgpu::TextureView,
    /// The motion vectors, at render resolution (or the output resolution if
    /// [`crate::MotionVectorLayout::display_resolution`] is set).
    pub motion_vectors: &'a wgpu::TextureView,
    /// A 1x1 texture whose first channel is the exposure to multiply the colour with. Used only when
    /// [`crate::Fsr3Config::auto_exposure`] is off; a value of 0 (or no texture) means 1.
    pub exposure: Option<&'a wgpu::TextureView>,
    /// A render-resolution mask of pixels whose history should not be trusted (particles, reflections
    /// of moving objects): 0 trusts the history, 1 rejects it. One channel, any filterable-or-not float
    /// format.
    pub reactive: Option<&'a wgpu::TextureView>,
    /// A render-resolution mask of pixels with transparency or composition (a window in front of
    /// something): like `reactive`, it makes the upscaler lean on the current frame. May have another
    /// size; it is sampled bilinearly.
    pub transparency_and_composition: Option<&'a wgpu::TextureView>,
    /// The size of `color`, `depth` and the (render-resolution) motion vectors.
    pub render_size: UVec2,
    /// The sub-pixel jitter applied to this frame's projection, in render pixels (see [`crate::jitter`]).
    pub jitter: Vec2,
    /// Pixels per motion vector unit, at the motion vector texture's resolution. `(1, 1)` for vectors
    /// already in pixels; see [`motion_vector_scale_ndc`] for vectors in normalised device coordinates.
    pub motion_vector_scale: Vec2,
    /// Seconds since the previous frame; clamped to `[0, 1]`. Only auto exposure uses it.
    pub delta_time: f32,
    /// The pre-exposure the application already multiplied the colour with, or 0 for none. Changes
    /// between frames are compensated in the history.
    pub pre_exposure: f32,
    /// RCAS sharpness in `[0, 1]`, or `None` to skip the sharpening pass. AMD's default is 0.8.
    pub sharpness: Option<f32>,
    /// The distance of the near plane, in view-space units.
    pub camera_near: f32,
    /// The distance of the far plane, ignored for an infinite far plane.
    pub camera_far: f32,
    /// The vertical field of view in radians.
    pub camera_fov_y: f32,
    /// How many metres one view-space unit is; 0 means 1. Distance thresholds in the upscaler are in
    /// metres, so a world that is not in metres should set this.
    pub view_space_to_meters: f32,
    /// The camera moved discontinuously (a cut, a teleport, a new scene): drop the history.
    pub reset: bool,
}

/// Where the upscaled image goes.
pub struct Fsr3Outputs<'a> {
    /// A storage texture of [`crate::Fsr3Config::output_format`] with the `STORAGE_BINDING` usage.
    pub output: &'a wgpu::TextureView,
    /// The size of `output`.
    pub size: UVec2,
}

/// The `motion_vector_scale` for motion vectors in normalised device coordinates (the difference
/// between the previous and the current clip-space xy, y up): half the target size, with y flipped.
/// `target_size` is the render size, or the output size for display-resolution vectors.
pub fn motion_vector_scale_ndc(target_size: UVec2) -> Vec2 {
    Vec2::new(0.5 * target_size.x as f32, -0.5 * target_size.y as f32)
}

/// The `motion_vector_scale` for motion vectors already in pixels (previous minus current, y down).
pub const MOTION_VECTOR_SCALE_PIXELS: Vec2 = Vec2::ONE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ndc_vectors_scale_to_half_the_size_with_y_flipped() {
        assert_eq!(motion_vector_scale_ndc(UVec2::new(1920, 1080)), Vec2::new(960.0, -540.0));
    }
}
