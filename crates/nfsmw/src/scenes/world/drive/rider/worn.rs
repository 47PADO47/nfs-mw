//! The rider's meshes and pictures on the renderer for the gear he wears: the parts planned from `parts.json` and the gear
//! (`gear.rs`), each part's pictures decoded and uploaded, its mesh created.

use std::collections::HashMap;

use anyhow::{Context, Result};
use blackbox_gfx::{MeshDesc, RenderBackend, TextureHandle};
use blackbox_skeleton::Pack;
use game_install::GameDir;
use glam::Vec3;

use super::{GpuPart, fill, gear, parts};
use crate::garage::rider_gear::{Catalogue, Choice};

/// What is on the renderer for a rider.
pub(super) struct Worn {
    pub parts: Vec<GpuPart>,
    pub textures: Vec<TextureHandle>,
}

/// One picture of the rider: a file of the rider's folder, or one of the gear's.
fn load_picture(renderer: &mut dyn RenderBackend, dir: &GameDir, folder: &str, picture: &str) -> Result<TextureHandle> {
    if gear::is_gear_picture(picture) {
        return gear::upload(renderer, dir, picture);
    }
    let path = format!("{folder}/textures/{picture}");
    let bytes = dir.read(&path).with_context(|| format!("reading {path}"))?;
    let image = parts::decode_png(&bytes)?;
    Ok(parts::upload_picture(renderer, picture, &image))
}

/// Puts `wearing` (the catalogue and the choice; `None`: his own gear) on the rider whose pack is `pack`: the meshes of the gear
/// are added to the pack, the parts are planned from `stock` (the entries of his `parts.json`) and the gear, and uploaded.
pub(super) fn wear(
    renderer: &mut dyn RenderBackend,
    dir: &GameDir,
    folder: &str,
    pack: &mut Pack,
    stock: &[parts::Entry],
    wearing: Option<(&Catalogue, &Choice)>,
) -> Worn {
    let dressing = match wearing {
        Some((catalogue, choice)) => gear::dress(dir, pack, stock, catalogue, choice),
        None => gear::Dressing { entries: stock.to_vec(), hidden: Vec::new() },
    };
    let (plans, skipped) = parts::plan_without(pack, &dressing.entries, &dressing.hidden);
    skipped.iter().for_each(|why| log::warn!("rider: {why}"));
    let mut textures: HashMap<String, TextureHandle> = HashMap::new();
    for picture in plans.iter().flat_map(|p| &p.draws).filter_map(|d| d.picture.as_deref()) {
        if textures.contains_key(picture) {
            continue;
        }
        match load_picture(renderer, dir, folder, picture) {
            Ok(handle) => {
                textures.insert(picture.to_owned(), handle);
            }
            Err(error) => log::warn!("rider: the picture {picture} is not drawn: {error:#}"),
        }
    }
    let lookup = |name: &str| textures.get(name).copied();
    let mut worn_parts = Vec::new();
    for plan in plans {
        let mesh = &pack.meshes[plan.mesh];
        let lod = &mesh.lods[plan.level];
        let (positions, normals): (Vec<Vec3>, Vec<Vec3>) = lod.vertices.iter().map(|v| (v.position, v.normal)).unzip();
        let mut vertices = Vec::new();
        fill(&mut vertices, &lod.vertices, &positions, &normals);
        let draws = parts::draw_ranges(&plan, &lookup);
        let label = format!("rider: {}", mesh.name);
        let handle = renderer.create_mesh(&MeshDesc {
            label: &label,
            vertices: &vertices,
            tangents: None,
            secondary_uvs: None,
            indices: &plan.indices,
            draws: draws.clone(),
        });
        worn_parts.push(GpuPart {
            mesh: plan.mesh,
            level: plan.level,
            handle,
            label,
            vertices,
            indices: plan.indices,
            draws,
        });
    }
    Worn { parts: worn_parts, textures: textures.into_values().collect() }
}
