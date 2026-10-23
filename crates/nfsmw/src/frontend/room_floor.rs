//! Verify the authored receiver before exposing a live image through its alpha.
use super::stage::Stage;
use blackbox_solid::Solid;
use glam::{Mat4, Vec3};

const FLOOR_TEXTURE: u32 = 0xC0C5_19D1;

pub(super) fn floor_texture(solids: &[Solid], stage: Stage) -> Option<u32> {
    if stage != Stage::SafeHouse {
        return None;
    }
    let mut found = false;
    for solid in solids {
        let world = Mat4::from_cols_array(&solid.transform);
        for group in &solid.groups {
            if group.diffuse_texture(solid) != Some(FLOOR_TEXTURE) {
                continue;
            }
            let begin = group.first_index as usize;
            let end = begin.checked_add(group.num_indices as usize)?;
            let indices = solid.indices.get(begin..end)?;
            if indices.is_empty() {
                return None;
            }
            for &index in indices {
                let vertex = solid.vertices.get((group.base_vertex as usize).checked_add(usize::from(index))?)?;
                let position = world.transform_point3(Vec3::from_array(vertex.position));
                if !position.is_finite() || position.z.abs() > 1e-4 {
                    return None;
                }
            }
            found |= solid.name == "BACKDROP_CRIB";
        }
    }
    found.then_some(FLOOR_TEXTURE)
}
