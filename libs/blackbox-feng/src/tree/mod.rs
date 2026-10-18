//! The retained tree a host draws: one node per object with its transform, colour, kind and draw order.
//! Rules: `docs/specs/feng-runtime.md` section 6. Nothing here knows about rendering.

use blackbox_ui::DrawList;
use glam::Mat4;

use crate::package::{ObjectKind, ResourceKind, word};
use crate::runtime::{PackageId, Runtime};

// The neutral tree types live in `blackbox-ui` (task T3). They are re-exported here so every existing
// `blackbox_feng::{UiTree, UiNode, NodeKind}` and `blackbox_feng::tree::*` path keeps resolving.
pub use blackbox_ui::{NodeKind, UiNode, UiTree};

/// Whether a node goes into a package tree's `draw_order`: visible, in front of the screen plane, and not a group or an
/// object this runtime does not draw. The composer uses the same rule for the trees it builds (`ui/theme/compose.rs`).
pub fn is_drawn(node: &UiNode) -> bool {
    node.visible && node.z > 0.0 && !matches!(node.kind, NodeKind::Group | NodeKind::Other)
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
            let world = placed * Mat4::from_scale(size);
            let colour = data.colour_rgba();
            let world_colour = mul_colour(ctx_colour, colour);
            let z = ctx.transform_point3(pivot + pos).z;
            let visible = parent_visible && colour[3] != 0 && !state.hidden;

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
                    mask_rotation: match def_obj.multi {
                        Some(_) => {
                            let at = |i: usize| data.f32_at(word::MULTI_PIVOT_ROT + i);
                            [at(0), at(1), at(2)]
                        }
                        None => [0.5, 0.5, 0.0],
                    },
                    mask_uv: match def_obj.multi {
                        Some(_) => data.multi_uv(0),
                        None => [0.0, 0.0, 1.0, 1.0],
                    },
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
        let mut list = DrawList::new();
        for node in nodes.into_iter().flatten() {
            list.push(node);
        }
        // The drawn nodes are the visible ones in front of the screen plane that are not groups or unsupported
        // objects. `into_tree` sorts them farthest first, stable: objects later in the list draw over earlier ones.
        list.into_tree(is_drawn)
    }
}

#[cfg(test)]
mod tests;
