//! Sparse screen-space displacement, composed before UI and excluded from scene captures.
use crate::TextureHandle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistortionRegion {
    /// Top-left and size in normalized screen coordinates, Y down.
    pub rect: [f32; 4],
    /// Source UV origin and signed extent. Negative extents flip the source axis.
    pub uv_rect: [f32; 4],
    /// Scene UV = sampled displacement GB * zw + xy.
    pub sample_transform: [f32; 4],
    pub opacity: f32,
    pub brightness: f32,
}

#[derive(Debug, Clone)]
pub struct ScreenDistortion {
    pub texture: TextureHandle,
    pub regions: Vec<DistortionRegion>,
}
