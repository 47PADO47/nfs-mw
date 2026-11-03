//! Everything the writer refuses, checked before a byte is written.

use std::collections::HashMap;

use blackbox_hash::bstring_hash;

use super::error::{GroupProblem, WriteError};
use super::plan::plan;
use super::source::{GroupSource, SolidSource};
use crate::layout::SolidLayout;

/// The effect with 44-byte vertices (a second UV set), seen once in the install (`SKYDOME_XENON`).
const SECOND_UV_EFFECT: u32 = 19;
/// `NumTextureTableEntries` and `NumLightMaterials` are bytes.
const MAX_LISTED: usize = 255;
/// Indices are 16-bit, so one vertex buffer holds at most this many vertices.
const MAX_BUFFER_VERTICES: usize = 0x1_0000;

pub(super) fn validate(solids: &[SolidSource], layout: &SolidLayout) -> Result<(), WriteError> {
    if solids.is_empty() {
        return Err(WriteError::NoSolids);
    }
    let mut seen: HashMap<u32, &str> = HashMap::new();
    for solid in solids {
        if solid.name.is_empty() || solid.name.contains('\0') {
            return Err(WriteError::BadName { name: solid.name.clone() });
        }
        let hash = bstring_hash(&solid.name);
        if let Some(first) = seen.insert(hash, &solid.name) {
            return Err(WriteError::DuplicateHash { first: first.to_owned(), second: solid.name.clone(), hash });
        }
        validate_solid(solid, layout)?;
    }
    Ok(())
}

fn validate_solid(solid: &SolidSource, layout: &SolidLayout) -> Result<(), WriteError> {
    let too_many = |what, count, max| WriteError::TooMany { solid: solid.name.clone(), what, count, max };
    if solid.groups.is_empty() {
        return Err(WriteError::NoGroups { solid: solid.name.clone() });
    }
    if solid.texture_hashes.len() > MAX_LISTED {
        return Err(too_many("textures", solid.texture_hashes.len(), MAX_LISTED));
    }
    if solid.light_material_hashes.len() > MAX_LISTED {
        return Err(too_many("light materials", solid.light_material_hashes.len(), MAX_LISTED));
    }
    let triangles: usize = solid.groups.iter().map(|g| g.indices.len() / 3).sum();
    if triangles > usize::from(u16::MAX) {
        return Err(too_many("triangles", triangles, usize::from(u16::MAX)));
    }
    for (i, group) in solid.groups.iter().enumerate() {
        let problem = group_problem(group, solid.light_material_hashes.len(), layout);
        if let Some(problem) = problem {
            return Err(WriteError::Group { solid: solid.name.clone(), group: i, problem });
        }
    }
    for run in plan(&solid.groups).runs {
        let vertices: usize = solid.groups[run.clone()].iter().map(|g| g.vertices.len()).sum();
        if vertices > MAX_BUFFER_VERTICES {
            let problem = GroupProblem::BufferTooLarge { vertices };
            return Err(WriteError::Group { solid: solid.name.clone(), group: run.start, problem });
        }
    }
    Ok(())
}

fn group_problem(group: &GroupSource, lights: usize, layout: &SolidLayout) -> Option<GroupProblem> {
    if group.vertices.is_empty() {
        return Some(GroupProblem::NoVertices);
    }
    if group.indices.is_empty() {
        return Some(GroupProblem::NoIndices);
    }
    if group.indices.len() % 3 != 0 {
        return Some(GroupProblem::PartialTriangle(group.indices.len()));
    }
    let wide = layout.skinned_effects.contains(&group.effect_id) || group.effect_id == SECOND_UV_EFFECT;
    if wide {
        return Some(GroupProblem::WideEffect(group.effect_id));
    }
    let wanted = if layout.tangent_effects.contains(&group.effect_id) { group.vertices.len() } else { 0 };
    if group.tangents.len() != wanted {
        let (effect_id, tangents, vertices) = (group.effect_id, group.tangents.len(), group.vertices.len());
        return Some(GroupProblem::Tangents { effect_id, tangents, vertices });
    }
    if group.light_material != 0xFF && usize::from(group.light_material) >= lights {
        return Some(GroupProblem::LightMaterialOutOfRange { light_material: group.light_material, listed: lights });
    }
    let index = group.indices.iter().copied().find(|&i| usize::from(i) >= group.vertices.len())?;
    Some(GroupProblem::IndexOutOfRange { index, vertices: group.vertices.len() })
}
