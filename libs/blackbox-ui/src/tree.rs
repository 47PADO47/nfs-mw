//! The neutral draw tree: the flat output a presenter draws. One node per object, with its transform, colour,
//! kind and draw order. Moved here from `blackbox-feng` (task T3, ADR 0008); the fields are unchanged.
//! Nothing here knows about rendering or about where the nodes came from.

use glam::{Mat4, Quat, Vec3};

/// What a node draws.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum NodeKind {
    #[default]
    Group,
    /// A textured quad: the unit square scaled by the size, textured with `texture` (a key in the texture
    /// packs; 0 = none) over the UV rectangle `[u0, v0, u1, v1]`. `mask` is the second texture of a multi image;
    /// `mask_rotation` is its pivot (`x`, `y`, fractions of the texture) and rotation (degrees), which the game
    /// changes to fill a gauge (`[0.5, 0.5, 0.0]` for any other image). `mask_uv` is the part of the mask texture
    /// that covers the image, `[u0, v0, u1, v1]`: the whole texture for a gauge, a window that moves for a
    /// scrolling map (the unit square for any other image).
    Image { texture: u32, uv: [f32; 4], mask: Option<u32>, mask_rotation: [f32; 3], mask_uv: [f32; 4] },
    /// A string. `font` is the font key (the resource handle of its `.ffn`).
    Text { font: u32, justification: u32, leading: i32, max_width: i32 },
    /// An object this runtime does not draw (movies, models).
    Other,
}

/// One object.
#[derive(Clone, Debug, Default)]
pub struct UiNode {
    /// Index of the object in the package.
    pub index: usize,
    pub guid: u32,
    pub name_hash: u32,
    pub parent: Option<usize>,
    pub kind: NodeKind,
    /// The text of a string node, resolved: the host text, else the language table, else the stored text.
    pub text: Option<String>,
    pub local_position: Vec3,
    /// Where the object position lands on the screen (the parent context applied to it): what button
    /// navigation measures.
    pub position: Vec3,
    pub local_pivot: Vec3,
    pub local_rotation: Quat,
    pub size: Vec3,
    /// The object colour `[r, g, b, a]`.
    pub colour: [u8; 4],
    /// The colour multiplied through the ancestors.
    pub world_colour: [u8; 4],
    /// Where the unit quad (or the string origin) lands on the 640 x 480 screen with its origin at the centre:
    /// parent context, position, pivot, rotation and size (including scaled groups).
    pub world: Mat4,
    /// The sort depth: larger is farther.
    pub z: f32,
    /// False when the node or an ancestor is transparent, hidden or off-screen in depth.
    pub visible: bool,
    /// Always `None`: the engine clip path is unused.
    pub clip: Option<[f32; 4]>,
}

/// All nodes of a package, in file order (parents first), plus the order to draw them in.
#[derive(Clone, Debug, Default)]
pub struct UiTree {
    pub nodes: Vec<UiNode>,
    /// Indices into `nodes` of the visible drawable nodes, farthest first (stable: later objects draw on top).
    pub draw_order: Vec<usize>,
}
