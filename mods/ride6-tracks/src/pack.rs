//! The exporter's track pack (`docs/formats/ride6-track-pack.md`): `pack.json` and `ground.r6m`, read into plain data.

use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

/// The magic and version of `ground.r6m`.
const MAGIC: &[u8; 4] = b"R6TM";
const VERSION: u32 = 1;

/// A drawn material: one picture (or a flat colour), where it tiles, and the surface under a wheel.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Material {
    pub name: String,
    /// The stem of the picture in `tex/`, or `None` for the flat `color`.
    pub texture: Option<String>,
    pub color: [u8; 3],
    /// The `simsurface` name: `asphalt`, `concrete`, `grass`, `dirt`, `sand`, `gravel`, `stone` or `metal`.
    pub surface: String,
}

/// The track spline: the lap as a polyline.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[serde(default)]
pub struct Spline {
    pub closed: bool,
    pub length_m: f32,
    /// Render space, about every 2 m.
    pub points: Vec<[f32; 3]>,
}

/// The starting grid and the lap.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
#[serde(default)]
pub struct Lap {
    /// `[x, y, z, heading degrees]` of each grid slot, render space, in grid order.
    pub grid: Vec<[f32; 4]>,
    pub spline: Spline,
}

#[derive(Debug, Deserialize)]
struct Document {
    format: u32,
    track: String,
    materials: Vec<Material>,
    #[serde(default)]
    lap: Lap,
}

/// One triangle group of one material.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub material: usize,
    /// The group is ground a wheel can touch (collision).
    pub collide: bool,
    /// Render space, metres.
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Unit vectors, as the pack stores them (`/ 127`).
    pub normals: Vec<[i8; 3]>,
    pub indices: Vec<u32>,
}

/// A whole track pack.
#[derive(Debug, Clone, PartialEq)]
pub struct Pack {
    /// The track's name in the install (`Okayama`).
    pub track: String,
    pub materials: Vec<Material>,
    pub groups: Vec<Group>,
    pub lap: Lap,
}

impl Pack {
    /// The triangles of all groups.
    pub fn triangles(&self) -> usize {
        self.groups.iter().map(|group| group.indices.len() / 3).sum()
    }
}

/// Reads the pack in `dir`.
pub fn read(dir: &Path) -> Result<Pack> {
    let text = std::fs::read_to_string(dir.join("pack.json"))
        .with_context(|| format!("reading {}/pack.json", dir.display()))?;
    let document: Document = serde_json::from_str(&text).context("parsing pack.json")?;
    ensure!(document.format == 1, "pack.json is format {}, this mod reads format 1", document.format);
    let bytes =
        std::fs::read(dir.join("ground.r6m")).with_context(|| format!("reading {}/ground.r6m", dir.display()))?;
    let groups = read_groups(&bytes, document.materials.len())?;
    Ok(Pack { track: document.track, materials: document.materials, groups, lap: document.lap })
}

/// The groups of the bytes of a `ground.r6m` file, each naming one of `materials` materials.
pub fn read_groups(bytes: &[u8], materials: usize) -> Result<Vec<Group>> {
    ensure!(bytes.len() >= 12 && &bytes[..4] == MAGIC, "ground.r6m does not start with R6TM");
    let word = |at: usize| -> Result<u32> {
        let slice = bytes.get(at..at + 4).context("ground.r6m is cut short")?;
        Ok(u32::from_le_bytes(slice.try_into().expect("four bytes")))
    };
    if word(4)? != VERSION {
        bail!("ground.r6m is version {}, this mod reads version {VERSION}", word(4)?);
    }
    let count = word(8)? as usize;
    let mut cursor = 12 + 16 * count;
    let mut groups = Vec::with_capacity(count);
    for i in 0..count {
        let header = 12 + 16 * i;
        let (material, vertices, indices, flags) =
            (word(header)? as usize, word(header + 4)? as usize, word(header + 8)? as usize, word(header + 12)?);
        ensure!(material < materials, "group {i} names material {material} of {materials}");
        ensure!(indices.is_multiple_of(3), "group {i} has {indices} indices");
        let positions = floats(bytes, &mut cursor, vertices * 3)?;
        let uvs = floats(bytes, &mut cursor, vertices * 2)?;
        let normal_bytes = bytes.get(cursor..cursor + vertices * 4).context("ground.r6m is cut short")?;
        cursor += vertices * 4;
        let index_bytes = bytes.get(cursor..cursor + indices * 4).context("ground.r6m is cut short")?;
        cursor += indices * 4;
        let indices: Vec<u32> = index_bytes.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
        ensure!(indices.iter().all(|&i| (i as usize) < vertices), "group {i} has an index past its vertices");
        groups.push(Group {
            material,
            collide: flags & 1 != 0,
            positions: positions.as_chunks::<3>().0.to_vec(),
            uvs: uvs.as_chunks::<2>().0.to_vec(),
            normals: normal_bytes.as_chunks::<4>().0.iter().map(|n| [n[0] as i8, n[1] as i8, n[2] as i8]).collect(),
            indices,
        });
    }
    Ok(groups)
}

fn floats(bytes: &[u8], cursor: &mut usize, count: usize) -> Result<Vec<f32>> {
    let slice = bytes.get(*cursor..*cursor + count * 4).context("ground.r6m is cut short")?;
    *cursor += count * 4;
    Ok(slice.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect())
}

/// The bytes of a `ground.r6m` for tests and tools: the same layout the exporter writes.
pub fn write_groups(groups: &[Group]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(groups.len() as u32).to_le_bytes());
    for group in groups {
        for word in
            [group.material as u32, group.positions.len() as u32, group.indices.len() as u32, u32::from(group.collide)]
        {
            out.extend_from_slice(&word.to_le_bytes());
        }
    }
    for group in groups {
        group.positions.iter().flatten().for_each(|v| out.extend_from_slice(&v.to_le_bytes()));
        group.uvs.iter().flatten().for_each(|v| out.extend_from_slice(&v.to_le_bytes()));
        for n in &group.normals {
            out.extend_from_slice(&[n[0] as u8, n[1] as u8, n[2] as u8, 0]);
        }
        group.indices.iter().for_each(|i| out.extend_from_slice(&i.to_le_bytes()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(material: usize, collide: bool) -> Group {
        Group {
            material,
            collide,
            positions: vec![[0.0, 0.0, 1.0], [10.0, 0.0, 1.0], [10.0, 10.0, 1.0], [0.0, 10.0, 1.0]],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            normals: vec![[0, 0, 127]; 4],
            indices: vec![0, 1, 2, 0, 2, 3],
        }
    }

    #[test]
    fn groups_round_trip_through_the_file_layout() {
        let groups = vec![quad(0, true), quad(1, false)];
        let bytes = write_groups(&groups);
        assert_eq!(read_groups(&bytes, 2).unwrap(), groups);
    }

    #[test]
    fn a_bad_file_is_refused_with_a_reason() {
        assert!(read_groups(b"nope", 1).is_err());
        let bytes = write_groups(&[quad(3, true)]);
        assert!(read_groups(&bytes, 2).unwrap_err().to_string().contains("material 3"));
        assert!(read_groups(&bytes[..bytes.len() - 4], 4).is_err());
        let mut wrong = write_groups(&[quad(0, true)]);
        wrong[4] = 9;
        assert!(read_groups(&wrong, 1).unwrap_err().to_string().contains("version 9"));
    }
}
