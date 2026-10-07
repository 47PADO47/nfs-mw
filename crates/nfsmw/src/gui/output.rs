//! egui's output as renderer-neutral data: texture patches and a [`UiLayer`] of clipped meshes.

use bevy_ecs::prelude::*;
use blackbox_render::{UiLayer, UiMesh, UiTextureId, UiTexturePatch, UiVertex};
use egui::epaint::{ImageData, Primitive};

/// A texture upload waiting for the renderer.
pub struct OwnedPatch {
    pub id: UiTextureId,
    pub offset: Option<[u32; 2]>,
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}

impl OwnedPatch {
    pub fn as_patch(&self) -> UiTexturePatch<'_> {
        UiTexturePatch { id: self.id, offset: self.offset, size: self.size, rgba: &self.rgba }
    }
}

/// What the UI pass produced, until the render bridge drains it.
#[derive(Resource, Default)]
pub struct UiOutput {
    pub patches: Vec<OwnedPatch>,
    pub freed: Vec<UiTextureId>,
    /// The newest layer; `None` when nothing changed since the bridge took it.
    pub layer: Option<UiLayer>,
}

fn texture_id(id: egui::TextureId) -> UiTextureId {
    match id {
        egui::TextureId::Managed(n) => UiTextureId(n),
        // User textures are not used; keep their ids apart from the managed ones.
        egui::TextureId::User(n) => UiTextureId(n | (1 << 63)),
    }
}

pub(super) fn convert(ctx: &egui::Context, full: egui::FullOutput, out: &mut UiOutput) {
    for (id, deltas) in &full.textures_delta.set {
        for delta in deltas {
            let ImageData::Color(image) = &delta.image;
            let rgba: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
            out.patches.push(OwnedPatch {
                id: texture_id(*id),
                offset: delta.pos.map(|p| [p[0] as u32, p[1] as u32]),
                size: [image.size[0] as u32, image.size[1] as u32],
                rgba,
            });
        }
    }
    out.freed.extend(full.textures_delta.free.iter().map(|id| texture_id(*id)));

    let pixels_per_point = full.pixels_per_point;
    let meshes = ctx
        .tessellate(full.shapes, pixels_per_point)
        .into_iter()
        .filter_map(|clipped| {
            let Primitive::Mesh(mesh) = clipped.primitive else { return None };
            let vertices = mesh
                .vertices
                .iter()
                .map(|v| UiVertex {
                    position: [v.pos.x, v.pos.y],
                    uv: [v.uv.x, v.uv.y],
                    color_rgba: v.color.to_array(),
                })
                .collect();
            let r = clipped.clip_rect;
            Some(UiMesh {
                vertices,
                indices: mesh.indices,
                texture: texture_id(mesh.texture_id),
                clip: [r.min.x, r.min.y, r.max.x, r.max.y],
            })
        })
        .collect();
    out.layer = Some(UiLayer { pixels_per_point, meshes });
}
