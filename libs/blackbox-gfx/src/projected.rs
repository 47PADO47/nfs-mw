//! A caller-selected 2D scene sampled in the main view's normalized screen coordinates.
use crate::{FrameParams, Instance};

/// The caller supplies authored capture geometry, transforms and view-specific visibility.
/// This does not infer or reflect a camera about an estimated ground plane.
/// The image has no HUD, post effects or temporal accumulation; `None` releases it.
pub struct ProjectedReflectionScene<'a> {
    pub frame: FrameParams,
    pub size: [u32; 2],
    pub instances: &'a [Instance],
    /// Two separable four-tap image-filter passes at the capture's actual texel size.
    pub blur: bool,
}

/// Transparent, car/object-only planar capture. The caller reflects the camera and supplies
/// the source-world plane whose positive half-space is visible. Receiver textures keep their
/// authored geometry and diffuse alpha; its complement exposes the captured image.
pub struct PlanarReflectionScene<'a> {
    pub frame: FrameParams,
    pub size: [u32; 2],
    pub instances: &'a [Instance],
    pub clip_plane: [f32; 4],
    pub receivers: &'a [crate::TextureHandle],
}
