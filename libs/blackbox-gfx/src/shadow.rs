//! Parameters for a fitted perspective shadow camera, supplied by the caller.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowProjection {
    /// Near/far distances of the source-view fitting frustum.
    pub frustum: [f32; 2],
    /// Raster depth-bias units and slope scale for the selected depth format.
    pub depth_bias: [f32; 2],
    pub near: f32,
    pub width_scale: f32,
    pub height_scale: f32,
    pub far_scale: f32,
    pub offset: [f32; 2],
    /// Column-major transform applied after the fitted perspective projection.
    pub post_transform: [[f32; 4]; 4],
}
