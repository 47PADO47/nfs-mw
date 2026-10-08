//! The retained tree a host draws: one node per object with its transform, colour, kind and draw order.
//! Rules: `docs/specs/feng-runtime.md` section 6. Nothing here knows about rendering.

use glam::{Mat4, Quat, Vec3};

use crate::package::{ObjectKind, ResourceKind};
use crate::runtime::{PackageId, Runtime};

/// What a node draws.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    Group,
    /// A textured quad: the unit square scaled by the size, textured with `texture` (a key in the texture
    /// packs; 0 = none) over the UV rectangle `[u0, v0, u1, v1]`. `mask` is the second texture of a multi image.
    Image {
        texture: u32,
        uv: [f32; 4],
        mask: Option<u32>,
    },
    /// A string. `font` is the font key (the resource handle of its `.ffn`).
    Text {
        font: u32,
        justification: u32,
        leading: i32,
        max_width: i32,
    },
    /// An object this runtime does not draw (movies, models).
    Other,
}

/// One object.
#[derive(Clone, Debug)]
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
    /// parent context, position, pivot, rotation and, for leaves, size.
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

/// Per-channel product used for the colour of nested objects: `a * b / 255`, exact for 255.
fn mul_channel(a: u8, b: u8) -> u8 {
    ((a as u32 * (b as u32 + 1)) >> 8).min(255) as u8
}

fn mul_colour(a: [u8; 4], b: [u8; 4]) -> [u8; 4] {
    [mul_channel(a[0], b[0]), mul_channel(a[1], b[1]), mul_channel(a[2], b[2]), mul_channel(a[3], b[3])]
}

impl Runtime {
    /// Builds the tree of a package from its current state.
    pub fn tree(&self, id: PackageId) -> UiTree {
        let Some(p) = self.running(id) else { return UiTree::default() };
        let def = &p.def;
        let mut nodes: Vec<Option<UiNode>> = vec![None; def.objects.len()];
        // Parents are visited before children: walk the update order, which is depth first.
        for &i in &p.order {
            let (def_obj, state) = (&def.objects[i], &p.objects[i]);
            let parent = def_obj
                .parent
                .and_then(|g| p.by_guid.get(&g).copied())
                .filter(|pi| *pi != i && def.objects[*pi].kind == ObjectKind::Group);
            let (ctx, ctx_colour, parent_visible) = match parent.and_then(|pi| nodes[pi].as_ref()) {
                Some(pn) => (pn.world, pn.world_colour, pn.visible),
                None => (Mat4::IDENTITY, [255; 4], true),
            };
            let data = &state.data;
            let (pos, pivot, rot, size) = (data.position(), data.pivot(), data.rotation(), data.size());
            let placed =
                ctx * Mat4::from_translation(pos + pivot) * Mat4::from_quat(rot) * Mat4::from_translation(-pivot);
            let is_group = def_obj.kind == ObjectKind::Group;
            let world = if is_group { placed } else { placed * Mat4::from_scale(size) };
            let colour = data.colour_rgba();
            let world_colour = mul_colour(ctx_colour, colour);
            let z = ctx.transform_point3(pivot + pos).z;
            let visible = parent_visible && colour[3] != 0 && !state.hidden && def_obj.flags & 8 == 0;

            let resource = def_obj.resource.and_then(|r| def.resources.get(r));
            let kind = match def_obj.kind {
                ObjectKind::Group => NodeKind::Group,
                ObjectKind::String => NodeKind::Text {
                    font: resource.filter(|r| r.kind == ResourceKind::Font).map_or(0, |r| r.handle),
                    justification: def_obj.string.as_ref().map_or(0, |s| s.justification),
                    leading: def_obj.string.as_ref().map_or(0, |s| s.leading),
                    max_width: def_obj.string.as_ref().map_or(0, |s| s.max_width),
                },
                k if k.is_image() => NodeKind::Image {
                    texture: state.texture.or_else(|| resource.map(|r| r.handle)).unwrap_or(0),
                    uv: data.uv(),
                    mask: def_obj.multi.and_then(|m| (m.textures[0] != 0).then_some(m.textures[0])),
                },
                _ => NodeKind::Other,
            };
            let text = def_obj.string.as_ref().map(|s| {
                if let Some(t) = &state.text {
                    t.clone()
                } else if def_obj.flags & 2 == 0 && state.label != 0 {
                    self.resolve_label(state.label).unwrap_or_else(|| s.text.clone())
                } else {
                    s.text.clone()
                }
            });
            nodes[i] = Some(UiNode {
                index: i,
                guid: def_obj.guid,
                name_hash: def_obj.name_hash,
                parent,
                kind,
                text,
                local_position: pos,
                position: ctx.transform_point3(pos),
                local_pivot: pivot,
                local_rotation: rot,
                size,
                colour,
                world_colour,
                world,
                z,
                visible,
                clip: None,
            });
        }
        let nodes: Vec<UiNode> = nodes.into_iter().flatten().collect();
        let mut draw_order: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.visible && n.z > 0.0 && !matches!(n.kind, NodeKind::Group | NodeKind::Other))
            .map(|(k, _)| k)
            .collect();
        // Farthest first; the sort is stable, so objects later in the list draw over earlier ones.
        draw_order.sort_by(|a, b| nodes[*b].z.total_cmp(&nodes[*a].z));
        UiTree { nodes, draw_order }
    }
}

#[cfg(test)]
mod tests;
