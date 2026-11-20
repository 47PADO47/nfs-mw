//! One section file of the game: the solids, the texture pack and the collision pack of a cell, one after the other, each on
//! a 0x800 boundary as the sections of the install's stream file are (the readers align by the offset in the file, and a
//! map pack's file is read from offset 0: `docs/specs/mod-world-content.md`).

use anyhow::{Context, Result};
use blackbox_chunk::ChunkWriter;
use blackbox_collision::{SoupOptions, SoupReport, SoupTriangle, write_soup_pack};
use blackbox_solid::SolidSource;
use blackbox_tpk::{PackForm, TextureSource, WriteOptions, write_tpk_with};

/// The boundary the packs start on.
const ALIGN: usize = 0x800;

/// A section file and what its collision came to.
pub struct Section {
    pub bytes: Vec<u8>,
    pub collision: SoupReport,
}

/// The bytes of section `number`: `solids`, a texture pack called `name` holding `textures`, and the collision pack of
/// `ground` (left out when there is none).
pub fn build(
    name: &str,
    number: u32,
    solids: &[SolidSource],
    textures: &[TextureSource],
    ground: &[SoupTriangle],
) -> Result<Section> {
    let mut out = ChunkWriter::new();
    let geometry =
        blackbox_solid::write_geometry_pack(solids).with_context(|| format!("writing the solids of {name}"))?;
    out.append(&geometry);
    out.pad_to(ALIGN);
    let options = WriteOptions { form: PackForm::Plain, filename: None };
    let pack = write_tpk_with(name, textures, &options).with_context(|| format!("writing the textures of {name}"))?;
    out.append(&pack);
    let mut collision = SoupReport::default();
    if !ground.is_empty() {
        out.pad_to(ALIGN);
        collision = write_soup_pack(&mut out, number, ground, &SoupOptions::default())
            .with_context(|| format!("writing the collision of {name}"))?;
    }
    Ok(Section { bytes: out.finish().context("assembling the section")?, collision })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cells::Part;
    use crate::model::solid_of;
    use crate::textures::flat_colour;

    fn part() -> Part {
        Part {
            material: 0,
            positions: vec![[0.0, 0.0, 1.0], [10.0, 0.0, 1.0], [0.0, 10.0, 1.0]],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            normals: vec![[0, 0, 127]; 3],
            indices: vec![0, 1, 2],
        }
    }

    #[test]
    fn a_section_reads_back_as_solids_textures_and_collision() {
        let texture = flat_colour([10, 120, 10]);
        let placed = solid_of("G000_0", &part(), &texture.name);
        let ground =
            [SoupTriangle { points: [[0.0, 1.0, 0.0], [0.0, 1.0, 10.0], [-10.0, 1.0, 0.0]], surface: 0x772F_B736 }];
        let section = build("G000", 2700, &[placed.source], &[texture], &ground).unwrap();
        assert_eq!(section.collision.triangles, 1);
        let bytes = &section.bytes;
        let solids = blackbox_solid::read_solids(bytes).unwrap();
        assert_eq!(solids.len(), 1);
        assert_eq!(solids[0].name, "G000_0");
        let packs = blackbox_tpk::read_texture_packs(bytes).unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].textures.len(), 1);
        assert_eq!(packs[0].textures[0].name_hash, blackbox_hash::bstring_hash("C_0A780A"));
        let collision = blackbox_collision::read_collision_packs(bytes).unwrap();
        assert_eq!(collision.len(), 1);
        assert_eq!(collision[0].section, 2700);
    }

    #[test]
    fn a_section_without_ground_has_no_collision_pack() {
        let texture = flat_colour([1, 1, 1]);
        let placed = solid_of("G001_0", &part(), &texture.name);
        let section = build("G001", 2701, &[placed.source], &[texture], &[]).unwrap();
        assert!(blackbox_collision::read_collision_packs(&section.bytes).unwrap().is_empty());
    }
}
