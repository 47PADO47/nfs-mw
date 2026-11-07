//! A piece of gear's mesh as a `BBSK` pack of its own: the exporter's `R6RM` (all levels of detail, RIDE 6's material slots)
//! becomes one level of detail whose sections say what they are drawn with (`docs/formats/ride6-rider-gear.md`, "The runtime
//! files"). The pack's skeleton is the mesh's own bones: the game finds each by name in the rider's skeleton.

use anyhow::{Context, Result, ensure};
use blackbox_skeleton::{Bone, MAX_INFLUENCES, Pack, Section, Skeleton, SkinVertex};

use super::super::convert;
use super::super::raw::{self, RawMesh};

/// A section drawn with the picture of the item.
pub const MAIN: &str = "main";
/// A section drawn dark grey: the goggle band of a motocross helmet, which has no picture of its own.
pub const GOGGLES: &str = "goggles";
/// The triangles of a body that are the hands: drawn with the colour of the gloves.
pub const HANDS: &str = "hands";

/// The bones a vertex must follow, by their weight, for its triangles to be hands.
const HAND_SHARE: f32 = 0.5;
/// The start of the names of the hand and finger bones of the rider's skeleton.
const HAND_BONES: [&str; 6] = ["hand_", "index_", "middle_", "ring_", "pinky_", "thumb_"];

/// What a mesh is a piece of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Body,
    Helmet,
    Visor,
    Boots,
}

impl Role {
    /// The name the mesh has in its pack. The helmet and the visor take the names of the stock rider's, which the bikes'
    /// camera tables leave out of the picture from inside the helmet (`camera::HIDDEN_MESHES`).
    pub fn mesh_name(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Helmet => crate::camera::HIDDEN_MESHES[0],
            Self::Visor => crate::camera::HIDDEN_MESHES[1],
            Self::Boots => "boots",
        }
    }
}

/// What a section of RIDE 6's mesh is drawn with, by the slot and the material it names; `None` for a stray placeholder that
/// is not drawn.
fn section_role(slot: &str, asset: &str) -> Option<&'static str> {
    let names = format!("{slot} {asset}").to_ascii_lowercase();
    if ["lambert", "default_material", "worldgrid"].iter().any(|stray| names.contains(stray)) {
        return None;
    }
    if names.contains("goggle") {
        return Some(GOGGLES);
    }
    Some(MAIN)
}

/// Whether a vertex follows the hand and finger bones for at least half of its weight.
fn is_hand(vertex: &SkinVertex, hand_bones: &[bool]) -> bool {
    let total = vertex.influences.total();
    if total <= 0.0 {
        return false;
    }
    let hand: f32 = (0..MAX_INFLUENCES)
        .filter(|&slot| hand_bones.get(usize::from(vertex.influences.bones[slot])).copied().unwrap_or(false))
        .map(|slot| vertex.influences.weights[slot])
        .sum();
    hand >= HAND_SHARE * total
}

/// The index of `name` in `names`, added at the end when it is not there.
fn position_or_push(names: &mut Vec<&'static str>, name: &'static str) -> usize {
    if let Some(at) = names.iter().position(|known| *known == name) {
        return at;
    }
    names.push(name);
    names.len() - 1
}

/// The pack of a mesh: its first level of detail, its sections given their roles, stray sections dropped, and for a body the
/// triangles of the hands moved into a section of their own.
pub fn pack(mut raw_mesh: RawMesh, role: Role) -> Result<Pack> {
    raw_mesh.lods.truncate(1);
    ensure!(!raw_mesh.lods.is_empty(), "the mesh has no level of detail");
    let bones: Vec<Bone> = raw_mesh
        .bones
        .iter()
        .map(|b| Bone {
            name: b.name.clone(),
            parent: usize::try_from(b.parent).ok(),
            rest: convert::transform(b.rotation, b.translation, b.scale),
        })
        .collect();
    let hand_bones: Vec<bool> =
        raw_mesh.bones.iter().map(|b| HAND_BONES.iter().any(|prefix| b.name.starts_with(prefix))).collect();
    let skeleton = Skeleton::new(bones)?;
    let roles: Vec<Option<&'static str>> =
        raw_mesh.materials.iter().map(|(slot, asset)| section_role(slot, asset)).collect();
    let mut mesh = convert::mesh(&raw_mesh, &skeleton)?;
    let lod = mesh.lods.first_mut().context("the mesh has no level of detail")?;
    ensure!(
        lod.indices.iter().all(|&i| (i as usize) < lod.vertices.len()),
        "an index of the mesh is past its vertices"
    );
    let mut names: Vec<&'static str> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut hands: Vec<u32> = Vec::new();
    for section in &lod.sections {
        let Some(name) = roles.get(section.material).copied().unwrap_or(Some(MAIN)) else { continue };
        let first = section.first_index as usize;
        let triangles = lod
            .indices
            .get(first..first + section.index_count as usize)
            .context("a section runs past the indices of the mesh")?;
        let start = indices.len();
        for triangle in triangles.chunks_exact(3) {
            let hand = role == Role::Body && triangle.iter().all(|&i| is_hand(&lod.vertices[i as usize], &hand_bones));
            match hand {
                true => hands.extend_from_slice(triangle),
                false => indices.extend_from_slice(triangle),
            }
        }
        let material = position_or_push(&mut names, name);
        sections.push(Section { material, first_index: start as u32, index_count: (indices.len() - start) as u32 });
    }
    if !hands.is_empty() {
        let material = position_or_push(&mut names, HANDS);
        sections.push(Section { material, first_index: indices.len() as u32, index_count: hands.len() as u32 });
        indices.extend(hands);
    }
    sections.retain(|section| section.index_count > 0);
    ensure!(!sections.is_empty(), "the mesh has nothing to draw");
    lod.indices = indices;
    lod.sections = sections;
    mesh.materials = names.iter().map(|name| (*name).to_owned()).collect();
    mesh.name = role.mesh_name().to_owned();
    Ok(Pack { skeleton, sockets: Vec::new(), clips: Vec::new(), meshes: vec![mesh] })
}

/// The bytes of the pack of the `R6RM` file `bytes`.
pub fn pack_bytes(bytes: &[u8], role: Role) -> Result<Vec<u8>> {
    Ok(pack(raw::parse_mesh(bytes)?, role)?.to_bytes())
}
