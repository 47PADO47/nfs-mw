//! The presenter for `blackbox-render`: quads and glyphs of the tree as premultiplied UI meshes, and the
//! textures they need as UI texture patches.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use blackbox_feng::{NodeKind, UiNode, UiTree, font::TextStyle};
use blackbox_render::{UiLayer, UiMesh, UiTextureId, UiVertex};
use glam::{Vec3, Vec4};

use super::{Screen, mask};
use crate::gui::{OwnedPatch, UiOutput};
use crate::ui::UiAssets;
use crate::ui::assets::Image;

/// UI texture ids of the HUD carry this bit so they never meet egui's small ones.
const HUD_TEXTURE_BIT: u64 = 1 << 62;
/// Ids of masked multi images (one slot each, rewritten when the mask turns) carry this bit too.
const MASKED_TEXTURE_BIT: u64 = 1 << 61;
/// Mask rotations are rebuilt at this resolution, in steps per degree.
const MASK_STEPS_PER_DEGREE: f32 = 4.0;

/// A multi image drawn through its mask: the slot is the object's (and its mask's), whatever picture it shows.
#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct MaskedKey {
    guid: u32,
    mask: u32,
}

/// What the pixels of a slot were built from.
#[derive(PartialEq, Clone, Copy)]
struct Built {
    /// The picture, as the texture that supplied it.
    base: u32,
    /// The rotation in `MASK_STEPS_PER_DEGREE` steps.
    step: i32,
    /// The bits of the mask's pivot and of its window.
    pivot: [u32; 2],
    window: [u32; 4],
}

struct MaskedSlot {
    id: UiTextureId,
    built: Option<Built>,
}

#[derive(Default)]
pub struct BlackboxPresenter {
    uploaded: HashSet<u32>,
    masked: HashMap<MaskedKey, MaskedSlot>,
    /// Pictures and masks decoded for composing, by the texture that supplied them (the minimap composes four
    /// of them every frame).
    decoded: HashMap<u32, Image>,
}

fn id_of(hash: u32) -> UiTextureId {
    UiTextureId(HUD_TEXTURE_BIT | hash as u64)
}

/// Premultiplied vertex colour from straight RGBA; additive textures get zero alpha.
fn vertex_colour(c: [u8; 4], additive: bool) -> [u8; 4] {
    let a = c[3] as u32;
    let p = |v: u8| ((v as u32 * a + 127) / 255) as u8;
    [p(c[0]), p(c[1]), p(c[2]), if additive { 0 } else { c[3] }]
}

/// Queues a texture for upload: premultiplied alpha, and zero alpha for an additive texture (its colour is added).
fn send(id: UiTextureId, image: Image, out: &mut UiOutput) {
    let mut rgba = image.rgba;
    for px in rgba.as_chunks_mut::<4>().0 {
        if image.blend == 2 {
            px[3] = 0;
            continue;
        }
        let a = px[3] as u32;
        for c in &mut px[..3] {
            *c = ((*c as u32 * a + 127) / 255) as u8;
        }
    }
    out.patches.push(OwnedPatch { id, offset: None, size: [image.width, image.height], rgba });
}

struct MeshBuilder {
    meshes: Vec<UiMesh>,
    clip: [f32; 4],
}

impl MeshBuilder {
    fn quad(
        &mut self,
        texture: UiTextureId,
        corners: [Vec3; 4],
        uvs: [[f32; 2]; 4],
        colour: [u8; 4],
        scale: f32,
        origin: [f32; 2],
    ) {
        let needs_new = self.meshes.last().is_none_or(|m| m.texture != texture);
        if needs_new {
            self.meshes.push(UiMesh { vertices: Vec::new(), indices: Vec::new(), texture, clip: self.clip });
        }
        let mesh = self.meshes.last_mut().expect("a mesh was just pushed");
        let base = mesh.vertices.len() as u32;
        for (c, uv) in corners.iter().zip(uvs) {
            mesh.vertices.push(UiVertex {
                position: [origin[0] + c.x * scale, origin[1] + c.y * scale],
                uv,
                color_rgba: colour,
            });
        }
        mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

impl BlackboxPresenter {
    #[cfg(test)]
    pub(crate) fn cache_sizes(&self) -> (usize, usize, usize) {
        (self.uploaded.len(), self.masked.len(), self.decoded.len())
    }

    /// Builds the HUD meshes for `tree` and prepends them to the frame's UI layer (egui's panels stay on top);
    /// uploads the textures it has not uploaded yet.
    pub fn present(&mut self, tree: &UiTree, assets: &UiAssets, screen: Screen, out: &mut UiOutput) {
        let origin = [screen.width * 0.5, screen.height * 0.5];
        let scale = screen.scale();
        let mut builder = MeshBuilder { meshes: Vec::new(), clip: [0.0, 0.0, screen.width, screen.height] };
        for &i in &tree.draw_order {
            let node = &tree.nodes[i];
            match &node.kind {
                NodeKind::Image { texture, uv, mask, mask_rotation, mask_uv } => {
                    let drawn = match mask {
                        Some(mask) => self.masked(node, (*texture, *mask), *mask_rotation, *mask_uv, assets, out),
                        None => self.ensure(*texture, assets, out),
                    };
                    self.image(node, drawn, *uv, &mut builder, scale, origin);
                }
                NodeKind::Text { font, justification, leading, max_width } => {
                    let style = TextStyle { justification: *justification, leading: *leading, max_width: *max_width };
                    self.text(node, *font, style, assets, out, &mut builder, scale, origin);
                }
                NodeKind::Group | NodeKind::Other => {}
            }
        }
        let layer =
            out.layer.get_or_insert_with(|| UiLayer { pixels_per_point: screen.pixels_per_point, meshes: Vec::new() });
        let mut meshes = builder.meshes;
        meshes.append(&mut layer.meshes);
        layer.meshes = meshes;
    }

    /// Makes sure a texture is on the GPU side; returns its UI id and whether it is additive.
    fn ensure(&mut self, key: u32, assets: &UiAssets, out: &mut UiOutput) -> Option<(UiTextureId, bool)> {
        let resolved = assets.resolve(key);
        let additive = assets.texture(resolved).is_some_and(|t| t.alpha_blend == 2);
        if self.uploaded.contains(&resolved) {
            return Some((id_of(resolved), additive));
        }
        let image = assets.image(resolved)?;
        send(id_of(resolved), image, out);
        self.uploaded.insert(resolved);
        Some((id_of(resolved), additive))
    }

    /// A texture decoded for composing, kept for the next frame.
    fn decode(&mut self, key: u32, assets: &UiAssets) -> Option<&Image> {
        match self.decoded.entry(key) {
            Entry::Occupied(e) => Some(e.into_mut()),
            Entry::Vacant(e) => Some(e.insert(assets.image(key)?)),
        }
    }

    /// The texture of a multi image: the picture through its mask, rebuilt into the node's own slot when the
    /// mask has turned or its window has moved. Without the mask texture the picture is drawn whole.
    fn masked(
        &mut self,
        node: &UiNode,
        (texture, mask): (u32, u32),
        rotation: [f32; 3],
        window: [f32; 4],
        assets: &UiAssets,
        out: &mut UiOutput,
    ) -> Option<(UiTextureId, bool)> {
        let (base, mask) = (assets.resolve(texture), assets.resolve(mask));
        let additive = assets.texture(base).is_some_and(|t| t.alpha_blend == 2);
        let key = MaskedKey { guid: node.guid, mask };
        let next_slot = self.masked.len() as u64;
        let slot = self.masked.entry(key).or_insert_with(|| MaskedSlot {
            id: UiTextureId(HUD_TEXTURE_BIT | MASKED_TEXTURE_BIT | next_slot),
            built: None,
        });
        let step = (rotation[2] * MASK_STEPS_PER_DEGREE).round() as i32;
        let built = Built {
            base,
            step,
            pivot: [rotation[0].to_bits(), rotation[1].to_bits()],
            window: window.map(f32::to_bits),
        };
        let id = slot.id;
        if slot.built == Some(built) {
            return Some((id, additive));
        }
        self.decode(base, assets);
        self.decode(mask, assets);
        let (Some(picture), Some(mask_image)) = (self.decoded.get(&base), self.decoded.get(&mask)) else {
            return self.ensure(texture, assets, out);
        };
        let degrees = step as f32 / MASK_STEPS_PER_DEGREE;
        let mut composed = mask::compose(picture, mask_image, [rotation[0], rotation[1]], degrees, window);
        if additive {
            // An additive picture adds its colour: where the mask hides it the colour has to go too.
            mask::premultiply(&mut composed);
        }
        if let Some(slot) = self.masked.get_mut(&key) {
            slot.built = Some(built);
        }
        send(id, composed, out);
        Some((id, additive))
    }

    fn image(
        &mut self,
        node: &UiNode,
        drawn: Option<(UiTextureId, bool)>,
        uv: [f32; 4],
        builder: &mut MeshBuilder,
        scale: f32,
        origin: [f32; 2],
    ) {
        let Some((id, additive)) = drawn else { return };
        let corner = |x: f32, y: f32| (node.world * Vec4::new(x, y, 0.0, 1.0)).truncate();
        let corners = [corner(-0.5, -0.5), corner(0.5, -0.5), corner(0.5, 0.5), corner(-0.5, 0.5)];
        let uvs = [[uv[0], uv[1]], [uv[2], uv[1]], [uv[2], uv[3]], [uv[0], uv[3]]];
        builder.quad(id, corners, uvs, vertex_colour(node.world_colour, additive), scale, origin);
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        node: &UiNode,
        font_key: u32,
        style: TextStyle,
        assets: &UiAssets,
        out: &mut UiOutput,
        builder: &mut MeshBuilder,
        scale: f32,
        origin: [f32; 2],
    ) {
        let (Some(font), Some(text)) = (assets.font(font_key), node.text.as_deref()) else { return };
        let Some((id, additive)) = self.ensure(font.texture_hash, assets, out) else { return };
        let size = assets.texture_size(assets.resolve(font.texture_hash)).unwrap_or((256, 256));
        let layout = font.layout(text, style, size);
        // A line wider than the maximum width is squeezed unless the string wraps.
        let squeeze =
            if style.max_width != 0 && style.justification & 0x10 == 0 && layout.width > style.max_width as f32 {
                style.max_width as f32 / layout.width
            } else {
                1.0
            };
        let colour = vertex_colour(node.world_colour, additive);
        for q in &layout.quads {
            let p = |x: f32, y: f32| (node.world * Vec4::new(x * squeeze, y, 0.0, 1.0)).truncate();
            let corners = [p(q.x0, q.y0), p(q.x1, q.y0), p(q.x1, q.y1), p(q.x0, q.y1)];
            let uvs = [[q.u0, q.v0], [q.u1, q.v0], [q.u1, q.v1], [q.u0, q.v1]];
            builder.quad(id, corners, uvs, colour, scale, origin);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_premultiply_and_additive_drops_alpha() {
        assert_eq!(vertex_colour([255, 128, 0, 128], false), [128, 64, 0, 128]);
        assert_eq!(vertex_colour([255, 255, 255, 255], false), [255, 255, 255, 255]);
        assert_eq!(vertex_colour([200, 100, 50, 128], true), [100, 50, 25, 0]);
        assert_eq!(vertex_colour([200, 100, 50, 0], true), [0, 0, 0, 0]);
    }

    #[test]
    fn texture_ids_stay_clear_of_egui() {
        assert!(id_of(1).0 & HUD_TEXTURE_BIT != 0);
        assert_ne!(id_of(1), id_of(2));
    }

    #[test]
    fn quads_with_one_texture_share_a_mesh() {
        let mut b = MeshBuilder { meshes: Vec::new(), clip: [0.0; 4] };
        let c = [Vec3::ZERO; 4];
        let uv = [[0.0; 2]; 4];
        b.quad(UiTextureId(1), c, uv, [255; 4], 1.0, [0.0; 2]);
        b.quad(UiTextureId(1), c, uv, [255; 4], 1.0, [0.0; 2]);
        b.quad(UiTextureId(2), c, uv, [255; 4], 1.0, [0.0; 2]);
        assert_eq!(b.meshes.len(), 2);
        assert_eq!(b.meshes[0].indices.len(), 12);
        assert_eq!(b.meshes[0].vertices.len(), 8);
    }
}
