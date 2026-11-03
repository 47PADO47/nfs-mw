//! Readers of the intermediate files the user's exporter writes (see `docs/formats/ride6-rider.md`): the animation
//! container `R6RA`, the skinned mesh `R6RM`, and the skeleton JSON. Everything is in Unreal's axes and centimetres.

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

/// A cursor over little-endian bytes.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).context("length overflow")?;
        let slice = self.bytes.get(self.at..end).context("the file is cut short")?;
        self.at = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }

    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn floats<const N: usize>(&mut self) -> Result<[f32; N]> {
        let mut out = [0.0; N];
        for v in &mut out {
            *v = self.f32()?;
        }
        Ok(out)
    }

    fn string(&mut self) -> Result<String> {
        let n = usize::from(self.u16()?);
        Ok(String::from_utf8_lossy(self.take(n)?).into_owned())
    }

    /// A count whose items take at least `item` bytes each, checked against what is left.
    fn count(&mut self, item: usize) -> Result<usize> {
        let n = self.u32()? as usize;
        ensure!(n.saturating_mul(item) <= self.bytes.len() - self.at, "a count runs past the end of the file");
        Ok(n)
    }
}

/// One animation sequence, sampled at every frame of the source.
pub struct RawClip {
    pub name: String,
    pub length: f32,
    pub frames: usize,
    pub rate_scale: f32,
    /// 0 none, 1 additive in local space, 2 additive in mesh space.
    pub additive: u8,
    pub base_name: String,
    pub base_frame: i32,
    pub root_motion: bool,
    pub notifies: Vec<(String, f32)>,
    /// One scale per bone, constant over the clip.
    pub scales: Vec<[f32; 3]>,
    /// `frames * bones` entries of rotation (x, y, z, w) then translation (x, y, z).
    pub keys: Vec<[f32; 7]>,
}

pub struct RawAnims {
    pub bone_count: usize,
    pub clips: Vec<RawClip>,
}

/// Reads an `R6RA` file.
pub fn parse_anims(bytes: &[u8]) -> Result<RawAnims> {
    let mut c = Cursor { bytes, at: 0 };
    ensure!(c.take(4)? == b"R6RA", "not an R6RA file");
    ensure!(c.u32()? == 1, "unknown R6RA version");
    let bone_count = c.u32()? as usize;
    let clip_count = c.u32()? as usize;
    let mut clips = Vec::new();
    for _ in 0..clip_count {
        let name = c.string()?;
        let length = c.f32()?;
        let frames = c.u32()? as usize;
        let rate_scale = c.f32()?;
        let additive = c.u8()?;
        let base_name = c.string()?;
        let base_frame = c.i32()?;
        let root_motion = c.u8()? != 0;
        let mut notifies = Vec::new();
        for _ in 0..c.count(6)? {
            notifies.push((c.string()?, c.f32()?));
        }
        ensure!(frames > 0 && bone_count > 0, "clip {name} is empty");
        let scales = (0..bone_count).map(|_| c.floats::<3>()).collect::<Result<Vec<_>>>()?;
        let keys = (0..frames * bone_count).map(|_| c.floats::<7>()).collect::<Result<Vec<_>>>()?;
        clips.push(RawClip {
            name,
            length,
            frames,
            rate_scale,
            additive,
            base_name,
            base_frame,
            root_motion,
            notifies,
            scales,
            keys,
        });
    }
    Ok(RawAnims { bone_count, clips })
}

pub struct RawBone {
    pub name: String,
    pub parent: i32,
    pub rotation: [f32; 4],
    pub translation: [f32; 3],
    pub scale: [f32; 3],
}

pub struct RawVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub influences: Vec<(u16, f32)>,
}

pub struct RawSection {
    pub material: i32,
    pub first_index: u32,
    pub faces: u32,
}

pub struct RawLod {
    pub screen_size: f32,
    pub sections: Vec<RawSection>,
    pub vertices: Vec<RawVertex>,
    pub indices: Vec<u32>,
}

pub struct RawMesh {
    pub name: String,
    pub bones: Vec<RawBone>,
    /// (slot name, material asset name)
    pub materials: Vec<(String, String)>,
    pub lods: Vec<RawLod>,
}

/// Reads an `R6RM` file.
pub fn parse_mesh(bytes: &[u8]) -> Result<RawMesh> {
    let mut c = Cursor { bytes, at: 0 };
    ensure!(c.take(4)? == b"R6RM", "not an R6RM file");
    ensure!(c.u32()? == 1, "unknown R6RM version");
    let name = c.string()?;
    let mut bones = Vec::new();
    for _ in 0..c.count(2 + 4 + 40)? {
        bones.push(RawBone {
            name: c.string()?,
            parent: c.i32()?,
            rotation: c.floats::<4>()?,
            translation: c.floats::<3>()?,
            scale: c.floats::<3>()?,
        });
    }
    let mut materials = Vec::new();
    for _ in 0..c.count(4)? {
        materials.push((c.string()?, c.string()?));
    }
    let mut lods = Vec::new();
    for _ in 0..c.count(16)? {
        let screen_size = c.f32()?;
        let vertex_count = c.count(33)?;
        let index_count = c.count(4)?;
        let section_count = c.count(12)?;
        let mut sections = Vec::new();
        for _ in 0..section_count {
            sections.push(RawSection { material: c.i32()?, first_index: c.u32()?, faces: c.u32()? });
        }
        let mut vertices = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            let position = c.floats::<3>()?;
            let normal = c.floats::<3>()?;
            let uv = c.floats::<2>()?;
            let used = c.u8()?;
            let mut influences = Vec::new();
            for _ in 0..used {
                influences.push((c.u16()?, c.f32()?));
            }
            vertices.push(RawVertex { position, normal, uv, influences });
        }
        let indices = (0..index_count).map(|_| c.u32()).collect::<Result<Vec<_>>>()?;
        lods.push(RawLod { screen_size, sections, vertices, indices });
    }
    Ok(RawMesh { name, bones, materials, lods })
}

/// A bone of a skeleton JSON file.
#[derive(Deserialize)]
pub struct JsonBone {
    pub name: String,
    pub parent: i32,
    pub t: [f32; 3],
    pub q: [f32; 4],
    pub s: [f32; 3],
}

/// A socket of a skeleton JSON file; `r` is an Unreal rotator (pitch, yaw, roll) in degrees.
#[derive(Deserialize)]
pub struct JsonSocket {
    pub name: String,
    pub bone: String,
    pub t: [f32; 3],
    pub r: [f32; 3],
    pub s: [f32; 3],
}

#[derive(Deserialize)]
pub struct JsonSkeleton {
    pub bones: Vec<JsonBone>,
    pub sockets: Vec<JsonSocket>,
}

pub fn parse_skeleton(bytes: &[u8]) -> Result<JsonSkeleton> {
    let skeleton: JsonSkeleton = serde_json::from_slice(bytes).context("the skeleton JSON")?;
    if skeleton.bones.is_empty() {
        bail!("the skeleton has no bones");
    }
    Ok(skeleton)
}
