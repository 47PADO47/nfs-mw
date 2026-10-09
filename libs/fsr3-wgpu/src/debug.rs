//! Read access to the internal images, for debug overlays and tests.

use wgpu::{Buffer, Texture};

use crate::{context::Fsr3Context, resources::Resources};

/// An internal image of the upscaler that [`Fsr3Context::debug_texture`] can hand out. They are
/// `COPY_SRC` and sampleable, and describe the last dispatch; their formats and meaning may change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DebugTexture {
    /// `Rg32Float` at render size: the motion vector of the nearest pixel of each 3x3 block, in uv.
    DilatedMotionVectors,
    /// `R32Float` at render size: the nearest depth of each 3x3 block, as in the depth input.
    DilatedDepth,
    /// `R32Float` at render size: the farthest depth of each 3x3 block, in metres.
    FarthestDepth,
    /// `R32Float` at half render size: the mean of each 2x2 block of the farthest depth, in metres.
    FarthestDepthMip1,
    /// `Rgba16Float` at render size: reactive, disocclusion, shading change and accumulation.
    ReactiveMasks,
    /// `R32Float` at half render size: the shading change estimate.
    ShadingChange,
    /// `R32Float` at render size: the luma instability factor.
    LumaInstability,
    /// `R32Float` at render size: the frames of history, 0 to 1.
    Accumulation,
    /// `R32Float` at render size: the luma of the colour input.
    Luma,
    /// `Rgba16Float` at output size: the accumulated image, in the alpha channel with its lock.
    History,
}

impl Fsr3Context {
    /// An internal image as the last dispatch left it, or `None` before the first dispatch. The
    /// commands of that dispatch must have been submitted before the image is read.
    pub fn debug_texture(&self, which: DebugTexture) -> Option<&Texture> {
        let r: &Resources = self.resources()?;
        let last = self.frames_dispatched().checked_sub(1)?;
        let view = match which {
            DebugTexture::DilatedMotionVectors => &r.dilated_motion_vectors,
            DebugTexture::DilatedDepth => &r.dilated_depth,
            DebugTexture::FarthestDepth => &r.farthest_depth,
            DebugTexture::FarthestDepthMip1 => &r.farthest_depth_mip1,
            DebugTexture::ReactiveMasks => &r.reactive_masks,
            DebugTexture::ShadingChange => &r.shading_change,
            DebugTexture::LumaInstability => &r.luma_instability,
            DebugTexture::Accumulation => r.accumulation.current(last),
            DebugTexture::Luma => r.luma.current(last),
            DebugTexture::History => r.history.current(last),
        };
        Some(view.texture())
    }

    /// The frame info buffer of the last dispatch: four `f32`s, the exposure, the smoothed average log
    /// luma, the average luma and a zero. `COPY_SRC`.
    pub fn debug_frame_info(&self) -> Option<&Buffer> {
        self.resources().map(|r| &r.frame_info)
    }
}
