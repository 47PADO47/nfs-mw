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
