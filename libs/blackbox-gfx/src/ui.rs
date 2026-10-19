//! The 2D UI layer: textured, clipped triangles drawn over the scene.
//!
//! This is what immediate-mode UI libraries (egui) and the games' own menus produce, so a console, a
//! metrics overlay and front-end screens all draw through it. Positions are in points; the layer says
//! how many pixels one point is.

use crate::UiTextureId;

/// One UI vertex.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiVertex {
    /// Position in points from the top-left corner of the window.
    pub position: [f32; 2],
    pub uv: [f32; 2],
    /// RGBA with **premultiplied** alpha, in the same (gamma) space as the texture.
    /// Nonzero RGB with zero alpha encodes additive colour, not transparency.
    pub color_rgba: [u8; 4],
}

/// Triangles drawn with one texture inside one clip rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct UiMesh {
    pub vertices: Vec<UiVertex>,
    /// Indices into `vertices`, three per triangle.
    pub indices: Vec<u32>,
    pub texture: UiTextureId,
    /// Clip rectangle in points: `[min_x, min_y, max_x, max_y]`.
    pub clip: [f32; 4],
}

/// Everything the UI draws in one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct UiLayer {
    /// Pixels per point (the display's scale factor, times any user zoom).
    pub pixels_per_point: f32,
    /// Drawn in order, later ones on top.
    pub meshes: Vec<UiMesh>,
    /// Rebuild these persistent textures before drawing. Sources contain straight RGBA8;
    /// outputs contain gamma-space premultiplied RGBA8, ready for the ordinary UI shader.
    pub compositions: Vec<UiTextureComposition>,
}

/// A picture through a bilinearly sampled, rotated/scrolled alpha mask, at the picture's resolution.
/// Sources are independent straight textures, not other composition outputs; both IDs must
/// differ from `output`. Outside the mask texture coverage is zero.
/// The result persists until replaced/freed, so unchanged masks need no command or pixel upload.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiTextureComposition {
    pub output: UiTextureId,
    pub picture: UiTextureId,
    pub mask: UiTextureId,
    /// Rotation pivot in fractions of the mask's size.
    pub pivot: [f32; 2],
    pub degrees: f32,
    /// Mask UV rectangle seen by the picture, `[u0, v0, u1, v1]`.
    pub window: [f32; 4],
    /// Zero output alpha after premultiplication to retain additive UI blending/coverage.
    pub additive: bool,
}

impl Default for UiLayer {
    fn default() -> Self {
        Self { pixels_per_point: 1.0, meshes: Vec::new(), compositions: Vec::new() }
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
