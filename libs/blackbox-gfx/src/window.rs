//! View-dependent architectural glass, using an authored 2D reflection texture.

use crate::TextureHandle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowMaterialDesc {
    pub diffuse: TextureHandle,
    pub reflection: TextureHandle,
}
