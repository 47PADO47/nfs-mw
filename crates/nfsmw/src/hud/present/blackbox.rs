//! The presenter for `blackbox-render`: quads and glyphs of the tree as premultiplied UI meshes, and the
//! textures they need as UI texture patches.

use std::collections::HashSet;

use blackbox_feng::{NodeKind, UiNode, UiTree, font::TextStyle};
use blackbox_render::{UiLayer, UiMesh, UiTextureId, UiVertex};
use glam::{Vec3, Vec4};

use super::Screen;
use crate::gui::{OwnedPatch, UiOutput};
use crate::hud::assets::HudAssets;

/// UI texture ids of the HUD carry this bit so they never meet egui's small ones.
const HUD_TEXTURE_BIT: u64 = 1 << 62;

#[derive(Default)]
pub struct BlackboxPresenter {
    uploaded: HashSet<u32>,
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
    /// Builds the HUD meshes for `tree` and prepends them to the frame's UI layer (egui's panels stay on top);
    /// uploads the textures it has not uploaded yet.
    pub fn present(&mut self, tree: &UiTree, assets: &HudAssets, screen: Screen, out: &mut UiOutput) {
        let origin = [screen.width * 0.5, screen.height * 0.5];
        let scale = screen.scale();
        let mut builder = MeshBuilder { meshes: Vec::new(), clip: [0.0, 0.0, screen.width, screen.height] };
        for &i in &tree.draw_order {
            let node = &tree.nodes[i];
            match &node.kind {
                NodeKind::Image { texture, uv, .. } => {
                    self.image(node, *texture, *uv, assets, out, &mut builder, scale, origin);
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
    fn ensure(&mut self, key: u32, assets: &HudAssets, out: &mut UiOutput) -> Option<(UiTextureId, bool)> {
        let resolved = assets.resolve(key);
        let additive = assets.texture(resolved).is_some_and(|t| t.alpha_blend == 2);
        if self.uploaded.contains(&resolved) {
            return Some((id_of(resolved), additive));
        }
        let image = assets.image(resolved)?;
        let mut rgba = image.rgba;
        for px in rgba.as_chunks_mut::<4>().0 {
            if image.blend == 2 {
                px[3] = 0;
            } else {
                let a = px[3] as u32;
                for c in &mut px[..3] {
                    *c = ((*c as u32 * a + 127) / 255) as u8;
                }
            }
        }
        out.patches.push(OwnedPatch { id: id_of(resolved), offset: None, size: [image.width, image.height], rgba });
        self.uploaded.insert(resolved);
        Some((id_of(resolved), additive))
    }

    #[allow(clippy::too_many_arguments)]
    fn image(
        &mut self,
        node: &UiNode,
        texture: u32,
        uv: [f32; 4],
        assets: &HudAssets,
        out: &mut UiOutput,
        builder: &mut MeshBuilder,
        scale: f32,
        origin: [f32; 2],
    ) {
        let Some((id, additive)) = self.ensure(texture, assets, out) else { return };
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
        assets: &HudAssets,
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
