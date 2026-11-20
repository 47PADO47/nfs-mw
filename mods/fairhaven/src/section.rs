//! One section file of the game, made in memory: the solids, the texture pack, the collision pack and the scenery section of a
//! unit, one after the other, each on a 0x800 boundary as the sections of the install's stream file are (the readers align by the
//! offset in the file, and a map pack's file is read from offset 0: `docs/specs/mod-world-content.md`).

use anyhow::{Context, Result};
use blackbox_chunk::ChunkWriter;
use blackbox_collision::{SoupOptions, SoupReport, SoupTriangle, write_soup_pack};
use blackbox_scenery::{ScenerySection, layout::MOST_WANTED, write_section};
use blackbox_solid::SolidSource;
use blackbox_tpk::{PackForm, TextureSource, WriteOptions, write_tpk_with};

/// The boundary the packs start on.
const ALIGN: usize = 0x800;

/// A section file and what its collision came to.
pub struct Section {
    pub bytes: Vec<u8>,
    pub collision: SoupReport,
}

/// The bytes of section `number`: `solids`, a texture pack called `name` holding `textures`, the collision pack of `ground` and
/// the `scenery` section. A part with nothing in it is left out.
pub fn build(
    name: &str,
    number: u32,
    solids: &[SolidSource],
    textures: &[TextureSource],
    ground: &[SoupTriangle],
    scenery: &ScenerySection,
) -> Result<Section> {
    let mut out = ChunkWriter::new();
    if !solids.is_empty() {
        let geometry =
            blackbox_solid::write_geometry_pack(solids).with_context(|| format!("writing the solids of {name}"))?;
        out.append(&geometry);
        out.pad_to(ALIGN);
    }
    if !textures.is_empty() {
        let options = WriteOptions { form: PackForm::Plain, filename: None };
        let pack =
            write_tpk_with(name, textures, &options).with_context(|| format!("writing the textures of {name}"))?;
        out.append(&pack);
        out.pad_to(ALIGN);
    }
    let mut collision = SoupReport::default();
    if !ground.is_empty() {
        collision = write_soup_pack(&mut out, number, ground, &SoupOptions::default())
            .with_context(|| format!("writing the collision of {name}"))?;
        out.pad_to(ALIGN);
    }
    write_section(&mut out, scenery, &MOST_WANTED);
    Ok(Section { bytes: out.finish().context("assembling the section")?, collision })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::{Group, build as build_solids};
    use crate::textures::{Look, name_of};
    use blackbox_scenery::build::{Placement, section as scenery_section};

    fn flat_texture(name: &str) -> TextureSource {
        TextureSource::new(name, 4, 4, blackbox_tpk::PixelFormat::Dxt1, 1, vec![0, 0, 0, 0, 0, 0, 0, 0])
    }

    #[test]
    fn a_section_reads_back_as_solids_textures_collision_and_scenery() {
        let texture = name_of(0xAB, Look { cut_out: false, two_sided: false });
        let group = Group {
            texture: texture.clone(),
            part: crate::mesh::Part {
                positions: vec![[0.0, 1.0, 0.0], [10.0, 1.0, 0.0], [0.0, 1.0, 10.0]],
                uvs: vec![[0.0, 0.0]; 3],
                indices: vec![0, 2, 1],
            },
        };
        let built = build_solids("M0A", vec![group]).remove(0);
        let placement = Placement {
            name: built.source.name.clone(),
            solids: [blackbox_hash::bstring_hash(&built.source.name), 0, 0, 0],
            local_min: [-6.0, -6.0, -1.0],
            local_max: [6.0, 6.0, 1.0],
            radius: built.radius,
            position: built.centre,
            rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            exclude_flags: 0x60,
        };
        let scenery = scenery_section(2700, &[placement]).unwrap();
        let ground =
            [SoupTriangle { points: [[0.0, 1.0, 0.0], [0.0, 1.0, 10.0], [-10.0, 1.0, 0.0]], surface: 0x19DB_2F1E }];
        let section = build("U1", 2700, &[built.source], &[flat_texture(&texture)], &ground, &scenery).unwrap();
        assert_eq!(section.collision.triangles, 1);
        let bytes = &section.bytes;
        assert_eq!(blackbox_solid::read_solids(bytes).unwrap().len(), 1);
        assert_eq!(blackbox_tpk::read_texture_packs(bytes).unwrap()[0].textures.len(), 1);
        assert_eq!(blackbox_collision::read_collision_packs(bytes).unwrap()[0].section, 2700);
        let read = blackbox_scenery::read_scenery_sections(bytes, &MOST_WANTED).unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!(read[0].instances.len(), 1);
        assert_eq!(read[0].infos[0].name, "M0A0");
    }

    #[test]
    fn a_unit_with_nothing_but_a_scenery_section_still_writes() {
        let scenery = scenery_section(2701, &[]).unwrap();
        let section = build("U2", 2701, &[], &[], &[], &scenery).unwrap();
        assert!(blackbox_solid::read_solids(&section.bytes).unwrap().is_empty());
        assert_eq!(blackbox_scenery::read_scenery_sections(&section.bytes, &MOST_WANTED).unwrap().len(), 1);
    }
}
