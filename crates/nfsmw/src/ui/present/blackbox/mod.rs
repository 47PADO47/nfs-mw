//! The presenter for `blackbox-render`: quads and glyphs of the tree as premultiplied UI meshes, and the
//! textures they need as UI texture patches.

mod fallback;
mod mirror;
mod raster_cache;

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};

use blackbox_feng::{NodeKind, UiNode, UiTree, font::TextStyle};
use blackbox_gfx::{UiLayer, UiMesh, UiTextureComposition, UiTextureId, UiVertex};
use blackbox_ui::justify;
use glam::{Vec3, Vec4};

use self::mirror::{MIRROR_BEZEL, faded_alpha, mirror_picture};
use self::raster_cache::RasterCache;
use super::{Screen, mask, repeat};
use crate::gui::{OwnedPatch, UiOutput};
use crate::ui::UiAssets;
use crate::ui::assets::Image;
use crate::ui::raster_text::{self, UNITS_ASPECT, UNITS_PER_SCALE};

/// UI texture ids of the HUD carry this bit so they never meet egui's small ones.
const HUD_TEXTURE_BIT: u64 = 1 << 62;
/// Ids of masked multi images (one slot each, rewritten when the mask turns) carry this bit too.
const MASKED_TEXTURE_BIT: u64 = 1 << 61;
/// A wrapped filter border is a different upload from the ordinary clamped texture.
const REPEATED_TEXTURE_BIT: u64 = 1 << 60;
/// Mask rotations are rebuilt at this resolution, in steps per degree.
const MASK_STEPS_PER_DEGREE: f32 = 4.0;
/// The one-pixel white texture an untextured panel draws its colour through (`BlackboxPresenter::solid`).
const SOLID_TEXTURE: UiTextureId = UiTextureId(1 << 59);
/// The first id of the strings rasterised in the Windows font (`raster_text`); each string takes the next.
const RASTER_TEXTURE_BIT: u64 = 1 << 58;
/// Straight source pixels for GPU masking must stay separate from ordinary premultiplied uploads.
const MASK_SOURCE_BIT: u64 = 1 << 57;
/// How far Remastered's rasterised text leans, as a fraction of its height. The upright key (Custom) does not lean.
const RASTER_ITALIC: f32 = 0.18;
/// The rear-view mirror's picture, uploaded by the app (`app::mirror`). Drawn under the bezel.
pub const MIRROR_TEXTURE: UiTextureId = UiTextureId(0x4D49_5252_4F52);

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
    /// Straight pictures/masks uploaded once; scrolling/rotation updates only small GPU commands.
    mask_sources: HashSet<u32>,
    repeated: HashMap<u32, ([u32; 2], bool)>,
    solid_uploaded: bool,
    /// Strings rasterised in the Windows font: the texture and the image's width over height, by the string. Bounded:
    /// strings that stop being drawn have their textures freed (`raster_cache.rs`).
    raster: RasterCache,
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
fn send(id: UiTextureId, mut image: Image, out: &mut UiOutput) {
    mask::premultiply(&mut image);
    if image.blend == 2 {
        for px in image.rgba.as_chunks_mut::<4>().0 {
            px[3] = 0;
        }
    }
    out.patches.push(OwnedPatch { id, offset: None, size: [image.width, image.height], rgba: image.rgba });
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
        (self.uploaded.len(), self.masked.len(), self.mask_sources.len())
    }

    /// Builds the HUD meshes for `tree` and prepends them to the frame's UI layer (egui's panels stay on top);
    /// uploads the textures it has not uploaded yet.
    pub fn present(&mut self, tree: &UiTree, assets: &UiAssets, screen: Screen, out: &mut UiOutput) {
        let _trace = blackbox_trace::scope("HUD: mesh / texture presentation");
        self.raster.tick(&mut out.freed);
        out.layer.get_or_insert_with(|| UiLayer { pixels_per_point: screen.pixels_per_point, ..Default::default() });
        let origin = [screen.width * 0.5, screen.height * 0.5];
        let scale = screen.scale();
        let mut builder = MeshBuilder { meshes: Vec::new(), clip: [0.0, 0.0, screen.width, screen.height] };
        let mut mirror_drawn = false;
        for &i in &tree.draw_order {
            let node = &tree.nodes[i];
            match &node.kind {
                NodeKind::Image { texture, uv, mask, mask_rotation, mask_uv } => {
                    if node.name_hash == MIRROR_BEZEL && !mirror_drawn {
                        mirror_drawn = true;
                        mirror_picture(tree, &mut builder, scale, origin);
                    }
                    if mask.is_none() && self.repeated(node, *texture, *uv, assets, out, &mut builder, scale, origin) {
                        continue;
                    }
                    let drawn = match mask {
                        Some(mask) => self.masked(node, (*texture, *mask), *mask_rotation, *mask_uv, assets, out),
                        None if *texture == 0 => Some(self.solid(out)),
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
        let layer = out.layer.get_or_insert_with(|| UiLayer {
            compositions: Vec::new(),
            pixels_per_point: screen.pixels_per_point,
            meshes: Vec::new(),
        });
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

    /// The white texture an untextured image draws its colour through. Uploaded once.
    fn solid(&mut self, out: &mut UiOutput) -> (UiTextureId, bool) {
        if !self.solid_uploaded {
            send(SOLID_TEXTURE, Image { width: 1, height: 1, rgba: vec![255; 4], blend: 1 }, out);
            self.solid_uploaded = true;
        }
        (SOLID_TEXTURE, false)
    }

    /// Draws `text` in the Windows font: the string is rasterised once and drawn as a texture, centred in the node's box
    /// and leaning by `RASTER_ITALIC`. The box keeps the string's proportions, so the text is never stretched.
    #[allow(clippy::too_many_arguments)]
    fn raster_text(
        &mut self,
        node: &UiNode,
        text: &str,
        font_key: u32,
        justification: u32,
        lean_fraction: f32,
        out: &mut UiOutput,
        builder: &mut MeshBuilder,
        scale: f32,
        origin: [f32; 2],
    ) {
        let _trace = blackbox_trace::scope("HUD: text raster / cache");
        // Each font key has its own look (the Remastered text is wider and heavier than the Custom text), so the keys keep
        // apart in the cache.
        let Some(face) = raster_text::face(font_key) else { return };
        let (id, aspect) = match self.raster.get(text, font_key) {
            Some(entry) => entry,
            None => {
                let rendered = match face.mono {
                    true => raster_text::render_mono(text, face.look),
                    false => raster_text::render(text, face.look),
                };
                let Some(image) = rendered else { return };
                let aspect = image.width as f32 / image.height as f32;
                let id = self.raster.insert(text, font_key, aspect, &mut out.freed);
                send(id, image, out);
                (id, aspect)
            }
        };
        // The node's scale is the text's size: its height in canvas units is the scale times `UNITS_PER_SCALE`.
        // Its origin is the centre of the text, as the layout places it.
        let height = node.world.y_axis.y * UNITS_PER_SCALE;
        if height <= 0.0 {
            return;
        }
        let width = height * aspect * UNITS_ASPECT;
        // The node's point is the text's left, centre or right edge, as its justification says, so the text is drawn
        // from that edge.
        let shift = if justification & justify::H_RIGHT != 0 {
            -width / 2.0
        } else if justification & justify::H_CENTRE != 0 {
            0.0
        } else {
            width / 2.0
        };
        let centre = node.world.w_axis;
        let (cx, cy) = (centre.x + shift, centre.y);
        let lean = lean_fraction * height / 2.0;
        let corners = [
            Vec3::new(cx - width / 2.0 + lean, cy - height / 2.0, 0.0),
            Vec3::new(cx + width / 2.0 + lean, cy - height / 2.0, 0.0),
            Vec3::new(cx + width / 2.0 - lean, cy + height / 2.0, 0.0),
            Vec3::new(cx - width / 2.0 - lean, cy + height / 2.0, 0.0),
        ];
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        builder.quad(id, corners, uvs, vertex_colour(node.world_colour, false), scale, origin);
    }

    /// Upload straight source pixels once, without changing RGB or the mask's authored alpha.
    fn mask_source(&mut self, key: u32, assets: &UiAssets, out: &mut UiOutput) -> Option<UiTextureId> {
        let id = UiTextureId(HUD_TEXTURE_BIT | MASK_SOURCE_BIT | u64::from(key));
        if self.mask_sources.contains(&key) {
            return Some(id);
        }
        let image = assets.image(key)?;
        if image.width == 0 || image.height == 0 {
            return None;
        }
        out.patches.push(OwnedPatch { id, offset: None, size: [image.width, image.height], rgba: image.rgba });
        self.mask_sources.insert(key);
        Some(id)
    }

    #[allow(clippy::too_many_arguments)]
    fn repeated(
        &mut self,
        node: &UiNode,
        texture: u32,
        uv: [f32; 4],
        assets: &UiAssets,
        out: &mut UiOutput,
        builder: &mut MeshBuilder,
        scale: f32,
        origin: [f32; 2],
    ) -> bool {
        let Some(tiles) = repeat::tiles(uv) else { return false };
        let key = assets.resolve(texture);
        let id = UiTextureId(HUD_TEXTURE_BIT | REPEATED_TEXTURE_BIT | u64::from(key));
        let (size, additive) = match self.repeated.entry(key) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(e) => {
                let Some(image) = assets.image(key) else { return false };
                if image.width == 0 || image.height == 0 {
                    return false;
                }
                let metadata = ([image.width, image.height], image.blend == 2);
                send(id, repeat::pad(&image), out);
                *e.insert(metadata)
            }
        };
        let corner = |x, y| (node.world * Vec4::new(x - 0.5, y - 0.5, 0.0, 1.0)).truncate();
        let colour = vertex_colour(node.world_colour, additive);
        for tile in tiles {
            let [x0, y0, x1, y1] = tile.rect;
            let corners = [corner(x0, y0), corner(x1, y0), corner(x1, y1), corner(x0, y1)];
            let [u0, v0, u1, v1] = repeat::padded_uv(tile.uv, size);
            builder.quad(id, corners, [[u0, v0], [u1, v0], [u1, v1], [u0, v1]], colour, scale, origin);
        }
        true
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
        let _trace = blackbox_trace::scope("HUD: masked texture cache");
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
        let picture = self.mask_source(base, assets, out);
        let mask_image = self.mask_source(mask, assets, out);
        let (Some(picture), Some(mask_image)) = (picture, mask_image) else {
            return self.ensure(texture, assets, out);
        };
        let degrees = step as f32 / MASK_STEPS_PER_DEGREE;
        out.layer.as_mut()?.compositions.push(UiTextureComposition {
            output: id,
            picture,
            mask: mask_image,
            pivot: [rotation[0], rotation[1]],
            degrees,
            window,
            additive,
        });
        if let Some(slot) = self.masked.get_mut(&key) {
            slot.built = Some(built);
        }
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
        // The existing mirror capture compensation applies only to its bezel. Other images,
        // including the standalone detector backing, keep their authored opacity.
        let mut colour = node.world_colour;
        if node.name_hash == MIRROR_BEZEL {
            colour[3] = faded_alpha(colour[3]);
        }
        builder.quad(id, corners, uvs, vertex_colour(colour, additive), scale, origin);
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
        if let Some(face) = raster_text::face(font_key) {
            let lean = if face.upright { 0.0 } else { RASTER_ITALIC };
            if let Some(text) = node.text.as_deref() {
                self.raster_text(node, text, font_key, style.justification, lean, out, builder, scale, origin);
            }
            return;
        }
        let (Some(font), Some(text)) = (assets.font(font_key), node.text.as_deref()) else { return };
        if !font.draws(text) {
            let font_scale = assets.font_scale(font.hash);
            self.fallback_text(node, text, font, style, font_scale, out, builder, scale, origin);
            return;
        }
        let Some((id, additive)) = self.ensure(font.texture_hash, assets, out) else { return };
        let size = assets.texture_size(assets.resolve(font.texture_hash)).unwrap_or((256, 256));
        let layout = font.layout_scaled(text, style, size, assets.font_scale(font.hash));
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
    fn additive_upload_preserves_texture_and_mask_coverage() {
        let mut out = UiOutput::default();
        let image = Image {
            width: 3,
            height: 1,
            rgba: vec![255, 255, 255, 0, 240, 120, 60, 128, 255, 255, 255, 255],
            blend: 2,
        };
        send(UiTextureId(7), image, &mut out);
        assert_eq!(out.patches[0].rgba, [0, 0, 0, 0, 120, 60, 30, 0, 255, 255, 255, 0]);
    }

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
