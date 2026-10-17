//! Authored multilayer sky materials, independent of any game's texture names.

use crate::TextureHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SkyLayerKind {
    Base,
    Cloud2,
    Cloud3,
}

#[derive(Debug, Clone, Copy)]
pub struct SkyMaterialDesc {
    pub kind: SkyLayerKind,
    pub diffuse: TextureHandle,
    pub miscellaneous: TextureHandle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyState {
    pub cloud_intensity: [f32; 4],
    pub scroll: f32,
}

impl Default for SkyState {
    fn default() -> Self {
        Self { cloud_intensity: [1.0; 4], scroll: 0.0 }
    }
}
