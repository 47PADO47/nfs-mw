//! A binary pack of everything one animated character needs: skeleton, sockets, clips and skinned meshes. The layout
//! is documented in `docs/formats/ride6-rider.md` (section "Runtime pack").

use glam::{Mat4, Vec3};

use crate::bytes::{Reader, Writer};
use crate::clip::Clip;
use crate::clip_io::{get_clip, put_clip};
use crate::error::{Error, Result};
use crate::skeleton::{Bone, Skeleton};
use crate::skin::{Influences, MAX_INFLUENCES, SkinVertex};
use crate::transform::Transform;

const MAGIC: &[u8; 4] = b"BBSK";
const VERSION: u32 = 1;

/// A named attachment point on a bone, with a transform relative to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Socket {
    pub name: String,
    pub bone: usize,
    pub transform: Transform,
}

/// A run of indices drawn with one material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    pub material: usize,
    pub first_index: u32,
    pub index_count: u32,
}

/// One level of detail of a mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshLod {
    /// The screen size below which the next level takes over (informational).
    pub screen_size: f32,
    pub vertices: Vec<SkinVertex>,
    /// Triangle list into `vertices`.
    pub indices: Vec<u32>,
    pub sections: Vec<Section>,
}

/// A mesh skinned to some bones of the skeleton.
#[derive(Clone, Debug, PartialEq)]
pub struct SkinnedMesh {
    pub name: String,
    /// For each of the mesh's own bones, the skeleton bone it follows (`None`: it follows nothing).
    pub bone_map: Vec<Option<usize>>,
    /// The inverse bind-pose matrix of each of the mesh's own bones.
    pub inverse_bind: Vec<Mat4>,
    pub materials: Vec<String>,
    pub lods: Vec<MeshLod>,
}

/// The whole character.
#[derive(Clone, Debug, PartialEq)]
pub struct Pack {
    pub skeleton: Skeleton,
    pub sockets: Vec<Socket>,
    pub clips: Vec<Clip>,
    pub meshes: Vec<SkinnedMesh>,
}

impl Pack {
    /// The index of the clip with this name.
    #[must_use]
    pub fn clip_index(&self, name: &str) -> Option<usize> {
        self.clips.iter().position(|c| c.name == name)
    }

    #[must_use]
    pub fn socket(&self, name: &str) -> Option<&Socket> {
        self.sockets.iter().find(|s| s.name == name)
    }

    /// Serialises the pack.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::default();
        w.bytes.extend_from_slice(MAGIC);
        w.u32(VERSION);
        w.count(self.skeleton.len());
        for bone in self.skeleton.bones() {
            w.str(&bone.name);
            w.bone(bone.parent);
            w.transform(&bone.rest);
        }
        w.count(self.sockets.len());
        for socket in &self.sockets {
            w.str(&socket.name);
            w.count(socket.bone);
            w.transform(&socket.transform);
        }
        w.count(self.clips.len());
        self.clips.iter().for_each(|c| put_clip(&mut w, c));
        w.count(self.meshes.len());
        self.meshes.iter().for_each(|m| put_mesh(&mut w, m));
        w.bytes
    }

    /// Reads a pack, checking that every bone index it holds is inside the skeleton.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.get(..4) != Some(MAGIC.as_slice()) {
            return Err(Error::BadHeader("wrong magic"));
        }
        let mut r = Reader::new(&bytes[4..]);
        if r.u32()? != VERSION {
            return Err(Error::BadHeader("unknown version"));
        }
        let mut bones = Vec::new();
        for _ in 0..r.count(2 + 4 + 40)? {
            let name = r.str()?;
            let parent = r.bone()?;
            bones.push(Bone { name, parent, rest: r.transform()? });
        }
        let skeleton = Skeleton::new(bones)?;
        let mut sockets = Vec::new();
        for _ in 0..r.count(2 + 4 + 40)? {
            sockets.push(Socket { name: r.str()?, bone: r.u32()? as usize, transform: r.transform()? });
        }
        let clips = (0..r.count(11)?).map(|_| get_clip(&mut r)).collect::<Result<Vec<_>>>()?;
        let meshes = (0..r.count(10)?).map(|_| get_mesh(&mut r)).collect::<Result<Vec<_>>>()?;
        if !r.is_done() {
            return Err(Error::BadValue("trailing bytes"));
        }
        let pack = Self { skeleton, sockets, clips, meshes };
        pack.check_bones()?;
        Ok(pack)
    }

    fn check_bones(&self) -> Result<()> {
        let n = self.skeleton.len();
        let bad = |bone: usize| if bone < n { Ok(()) } else { Err(Error::BadBone(bone)) };
        self.sockets.iter().try_for_each(|s| bad(s.bone))?;
        self.clips.iter().flat_map(|c| &c.tracks).try_for_each(|t| bad(t.bone))?;
        let mapped = self.meshes.iter().flat_map(|m| m.bone_map.iter().flatten());
        mapped.copied().try_for_each(bad)
    }
}

fn put_mesh(w: &mut Writer, mesh: &SkinnedMesh) {
    w.str(&mesh.name);
    w.count(mesh.bone_map.len());
    mesh.bone_map.iter().for_each(|&b| w.bone(b));
    mesh.inverse_bind.iter().for_each(|m| w.mat4(m));
    w.count(mesh.materials.len());
    mesh.materials.iter().for_each(|m| w.str(m));
    w.count(mesh.lods.len());
    for lod in &mesh.lods {
        w.f32(lod.screen_size);
        w.count(lod.vertices.len());
        for v in &lod.vertices {
            w.vec3(v.position);
            w.vec3(v.normal);
            v.uv.iter().for_each(|&c| w.f32(c));
            let used = (0..MAX_INFLUENCES).filter(|&s| v.influences.weights[s] != 0.0).count();
            w.u8(used as u8);
            for s in (0..MAX_INFLUENCES).filter(|&s| v.influences.weights[s] != 0.0) {
                w.u16(v.influences.bones[s]);
                w.f32(v.influences.weights[s]);
            }
        }
        w.count(lod.indices.len());
        lod.indices.iter().for_each(|&i| w.u32(i));
        w.count(lod.sections.len());
        for s in &lod.sections {
            w.count(s.material);
            w.u32(s.first_index);
            w.u32(s.index_count);
        }
    }
}

fn get_mesh(r: &mut Reader) -> Result<SkinnedMesh> {
    let name = r.str()?;
    let bones = r.count(4 + 64)?;
    let bone_map = (0..bones).map(|_| r.bone()).collect::<Result<Vec<_>>>()?;
    let inverse_bind = (0..bones).map(|_| r.mat4()).collect::<Result<Vec<_>>>()?;
    let materials = (0..r.count(2)?).map(|_| r.str()).collect::<Result<Vec<_>>>()?;
    let mut lods = Vec::new();
    for _ in 0..r.count(16)? {
        let screen_size = r.f32()?;
        let mut vertices = Vec::new();
        for _ in 0..r.count(33)? {
            let position = r.vec3()?;
            let normal: Vec3 = r.vec3()?;
            let uv = [r.f32()?, r.f32()?];
            let used = usize::from(r.u8()?);
            if used > MAX_INFLUENCES {
                return Err(Error::BadValue("too many influences"));
            }
            let mut pairs = Vec::with_capacity(used);
            for _ in 0..used {
                let bone = r.u16()?;
                if usize::from(bone) >= bones {
                    return Err(Error::BadValue("influence names a missing mesh bone"));
                }
                pairs.push((bone, r.f32()?));
            }
            vertices.push(SkinVertex { position, normal, uv, influences: Influences::from_pairs(&pairs) });
        }
        let indices = (0..r.count(4)?).map(|_| r.u32()).collect::<Result<Vec<_>>>()?;
        if indices.iter().any(|&i| i as usize >= vertices.len()) {
            return Err(Error::BadValue("index past the vertices"));
        }
        let sections = (0..r.count(12)?)
            .map(|_| Ok(Section { material: r.u32()? as usize, first_index: r.u32()?, index_count: r.u32()? }))
            .collect::<Result<Vec<_>>>()?;
        lods.push(MeshLod { screen_size, vertices, indices, sections });
    }
    Ok(SkinnedMesh { name, bone_map, inverse_bind, materials, lods })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{Channel, Track};
    use crate::skeleton::tests::arm;
    use glam::Quat;

    fn pack() -> Pack {
        let skeleton = arm();
        let clip = Clip {
            name: "idle".into(),
            length: 1.0,
            tracks: vec![Track {
                bone: 2,
                rotation: Channel::Constant(Quat::from_rotation_x(0.2)),
                translation: Channel::None,
                scale: Channel::None,
            }],
            additive: false,
            root_motion: false,
            notifies: vec![],
        };
        let mesh = SkinnedMesh {
            name: "sleeve".into(),
            bone_map: vec![Some(1), None],
            inverse_bind: vec![Mat4::IDENTITY, Mat4::from_translation(Vec3::Y)],
            materials: vec!["cloth".into()],
            lods: vec![MeshLod {
                screen_size: 1.0,
                vertices: vec![
                    SkinVertex {
                        position: Vec3::X,
                        normal: Vec3::Y,
                        uv: [0.25, 0.5],
                        influences: Influences::from_pairs(&[(0, 0.75), (1, 0.25)]),
                    };
                    3
                ],
                indices: vec![0, 1, 2],
                sections: vec![Section { material: 0, first_index: 0, index_count: 3 }],
            }],
        };
        Pack {
            sockets: vec![Socket {
                name: "seat".into(),
                bone: 0,
                transform: Transform::from_rt(Quat::IDENTITY, Vec3::Z),
            }],
            skeleton,
            clips: vec![clip],
            meshes: vec![mesh],
        }
    }

    #[test]
    fn pack_round_trips() {
        let p = pack();
        let back = Pack::parse(&p.to_bytes()).unwrap();
        assert_eq!(back, p);
        assert_eq!(back.clip_index("idle"), Some(0));
        assert!(back.socket("seat").is_some());
    }

    #[test]
    fn bad_input_is_rejected_without_panicking() {
        let bytes = pack().to_bytes();
        assert!(Pack::parse(b"nope").is_err());
        for cut in (0..bytes.len()).step_by(7) {
            assert!(Pack::parse(&bytes[..cut]).is_err(), "cut {cut}");
        }
        let mut wrong_version = bytes.clone();
        wrong_version[4] = 9;
        assert_eq!(Pack::parse(&wrong_version), Err(Error::BadHeader("unknown version")));
    }

    #[test]
    fn a_track_on_a_missing_bone_is_rejected() {
        let mut p = pack();
        p.clips[0].tracks[0].bone = 40;
        assert_eq!(Pack::parse(&p.to_bytes()), Err(Error::BadBone(40)));
    }
}
