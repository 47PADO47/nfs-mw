//! The 2D UI layer: textured, clipped triangles drawn over the scene.
//!
//! This is what immediate-mode UI libraries (egui) and the games' own menus produce, so a console, a
//! metrics overlay and front-end screens all draw through it. Positions are in points; the layer says
//! how many pixels one point is.

/// One UI vertex.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex {
    /// Position in points from the top-left corner of the window.
    pub position: [f32; 2],
    pub uv: [f32; 2],
    /// RGBA with **premultiplied** alpha, in the same (gamma) space as the texture.
    pub color_rgba: [u8; 4],
}

/// Names a UI texture. The caller picks the number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UiTextureId(pub u64);

/// Triangles drawn with one texture inside one clip rectangle.
#[derive(Debug, Clone)]
pub struct UiMesh {
    pub vertices: Vec<UiVertex>,
    /// Indices into `vertices`, three per triangle.
    pub indices: Vec<u32>,
    pub texture: UiTextureId,
    /// Clip rectangle in points: `[min_x, min_y, max_x, max_y]`.
    pub clip: [f32; 4],
}

/// Everything the UI draws in one frame.
#[derive(Debug, Clone)]
pub struct UiLayer {
    /// Pixels per point (the display's scale factor, times any user zoom).
    pub pixels_per_point: f32,
    /// Drawn in order, later ones on top.
    pub meshes: Vec<UiMesh>,
}

impl Default for UiLayer {
    fn default() -> Self {
        Self { pixels_per_point: 1.0, meshes: Vec::new() }
    }
}

/// Creates, replaces or partly updates a UI texture (RGBA8, not sRGB-converted).
#[derive(Debug, Clone, Copy)]
pub struct UiTexturePatch<'a> {
    pub id: UiTextureId,
    /// `None` replaces the whole texture (and resizes it); `Some` writes a region of an existing one.
    pub offset: Option<[u32; 2]>,
    pub size: [u32; 2],
    /// `size[0] * size[1] * 4` bytes, tightly packed.
    pub rgba: &'a [u8],
}
