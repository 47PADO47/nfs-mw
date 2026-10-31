//! From Unreal's axes and centimetres to the car frame (x forward, y left, z up, metres), and from the raw exports to
//! the generic runtime's [`Pack`].
//!
//! Unreal is left-handed with x forward, y right, z up. Negating y gives the car frame. A reflection reverses the
//! sense of rotation, so a quaternion `(x, y, z, w)` becomes `(-x, y, -z, w)`, and the winding of triangles flips.

use anyhow::{Context, Result, bail};
use blackbox_skeleton::{
    Bone, Channel, Clip, Influences, MeshLod, Notify, Pack, Section, Skeleton, SkinVertex, SkinnedMesh, Socket, Track,
    Transform, inverse_bind_from_rest,
};
use glam::{Quat, Vec3};

use super::raw::{JsonSkeleton, RawAnims, RawClip, RawMesh};

/// Centimetres to metres.
const SCALE: f32 = 0.01;
/// Values closer than this count as equal when a channel is checked for being constant.
const TOLERANCE: f32 = 1e-5;

/// A position from Unreal's frame in centimetres to the car frame in metres.
#[must_use]
pub fn position(p: [f32; 3]) -> Vec3 {
    Vec3::new(p[0], -p[1], p[2]) * SCALE
}

/// A direction (normal) from Unreal's frame to the car frame.
#[must_use]
pub fn direction(d: [f32; 3]) -> Vec3 {
    Vec3::new(d[0], -d[1], d[2])
}

/// A rotation from Unreal's frame to the car frame.
#[must_use]
pub fn rotation(q: [f32; 4]) -> Quat {
    Quat::from_xyzw(-q[0], q[1], -q[2], q[3]).normalize()
}

/// Unreal's `FRotator` (pitch, yaw, roll in degrees) as a quaternion in Unreal's frame, as `FRotator::Quaternion`.
#[must_use]
pub fn rotator_quat(pitch: f32, yaw: f32, roll: f32) -> [f32; 4] {
    let half = |d: f32| (d.to_radians() * 0.5).sin_cos();
    let (sp, cp) = half(pitch);
    let (sy, cy) = half(yaw);
    let (sr, cr) = half(roll);
    [
        cr * sp * sy - sr * cp * cy,
        -cr * sp * cy - sr * cp * sy,
        cr * cp * sy - sr * sp * cy,
        cr * cp * cy + sr * sp * sy,
    ]
}

#[must_use]
pub fn transform(q: [f32; 4], t: [f32; 3], s: [f32; 3]) -> Transform {
    Transform::new(rotation(q), position(t), Vec3::from_array(s))
}

/// The skeleton and its sockets.
pub fn skeleton(json: &JsonSkeleton) -> Result<(Skeleton, Vec<Socket>)> {
    let bones = json
        .bones
        .iter()
        .map(|b| Bone { name: b.name.clone(), parent: usize::try_from(b.parent).ok(), rest: transform(b.q, b.t, b.s) })
        .collect();
    let skeleton = Skeleton::new(bones)?;
    let mut sockets = Vec::new();
    for socket in &json.sockets {
        let bone =
            skeleton.find(&socket.bone).with_context(|| format!("socket {} is on a missing bone", socket.name))?;
        let q = rotator_quat(socket.r[0], socket.r[1], socket.r[2]);
        sockets.push(Socket { name: socket.name.clone(), bone, transform: transform(q, socket.t, socket.s) });
    }
    Ok((skeleton, sockets))
}

fn channel<T: Copy + PartialEq>(values: Vec<T>, rest: T, close: impl Fn(T, T) -> bool) -> Channel<T> {
    if values.iter().all(|&v| close(v, rest)) {
        return Channel::None;
    }
    if values.iter().all(|&v| close(v, values[0])) {
        return Channel::Constant(values[0]);
    }
    Channel::Uniform(values)
}

/// A clip of full poses (additive clips are turned into deltas by [`clips`]).
fn full_clip(raw: &RawClip, skeleton: &Skeleton) -> Clip {
    let bones = skeleton.len();
    let mut tracks = Vec::new();
    for (bone, rest) in skeleton.bones().iter().map(|b| b.rest).enumerate() {
        let keys = (0..raw.frames).map(|f| raw.keys[f * bones + bone]);
        let rotations: Vec<Quat> = keys.clone().map(|k| rotation([k[0], k[1], k[2], k[3]])).collect();
        let translations: Vec<Vec3> = keys.map(|k| position([k[4], k[5], k[6]])).collect();
        let scale = Vec3::from_array(raw.scales[bone]);
        let rotation = channel(rotations, rest.rotation, |a, b| a.angle_between(b) < TOLERANCE);
        let translation = channel(translations, rest.translation, |a, b| a.distance(b) < TOLERANCE);
        let scale = if (scale - rest.scale).length() < TOLERANCE { Channel::None } else { Channel::Constant(scale) };
        if matches!((&rotation, &translation, &scale), (Channel::None, Channel::None, Channel::None)) {
            continue;
        }
        tracks.push(Track { bone, rotation, translation, scale });
    }
    Clip {
        name: raw.name.clone(),
        length: raw.length,
        tracks,
        additive: false,
        root_motion: raw.root_motion,
        notifies: raw.notifies.iter().map(|(name, time)| Notify { name: name.clone(), time: *time }).collect(),
    }
}

/// All clips. A clip the source marks additive was exported as full poses (base plus difference); it becomes a clip
/// of differences against the frame of the base clip the source names.
pub fn clips(raw: &RawAnims, skeleton: &Skeleton) -> Result<Vec<Clip>> {
    if raw.bone_count != skeleton.len() {
        bail!("the animations have {} bones, the skeleton {}", raw.bone_count, skeleton.len());
    }
    let full: Vec<Clip> = raw.clips.iter().map(|c| full_clip(c, skeleton)).collect();
    let mut out = Vec::with_capacity(full.len());
    for (raw_clip, clip) in raw.clips.iter().zip(&full) {
        if raw_clip.additive == 0 {
            out.push(clip.clone());
            continue;
        }
        let base_clip =
            full.iter().zip(&raw.clips).find(|(_, r)| r.name == raw_clip.base_name).map(|(c, r)| (c, r.frames));
        let base_pose = match base_clip {
            Some((base, frames)) => {
                let step = if frames > 1 { base.length / (frames - 1) as f32 } else { 0.0 };
                base.sample(skeleton, step * raw_clip.base_frame.max(0) as f32, false)
            }
            None => skeleton.rest_pose(),
        };
        out.push(clip.to_additive(skeleton, &base_pose, raw_clip.frames));
    }
    Ok(out)
}

/// A skinned mesh, bones matched to the skeleton by name, triangles re-wound for the reflection.
pub fn mesh(raw: &RawMesh, skeleton: &Skeleton) -> Result<SkinnedMesh> {
    let parents: Vec<Option<usize>> = raw.bones.iter().map(|b| usize::try_from(b.parent).ok()).collect();
    let rest: Vec<Transform> = raw.bones.iter().map(|b| transform(b.rotation, b.translation, b.scale)).collect();
    let bone_map = raw.bones.iter().map(|b| skeleton.find(&b.name)).collect();
    let mut lods = Vec::new();
    for lod in &raw.lods {
        let vertices = lod
            .vertices
            .iter()
            .map(|v| SkinVertex {
                position: position(v.position),
                normal: direction(v.normal).normalize_or_zero(),
                uv: v.uv,
                influences: Influences::from_pairs(&v.influences),
            })
            .collect();
        let mut indices = lod.indices.clone();
        indices.as_chunks_mut::<3>().0.iter_mut().for_each(|t| t.swap(1, 2));
        let sections = lod
            .sections
            .iter()
            .map(|s| Section {
                material: s.material.max(0) as usize,
                first_index: s.first_index,
                index_count: s.faces * 3,
            })
            .collect();
        lods.push(MeshLod { screen_size: lod.screen_size, vertices, indices, sections });
    }
    Ok(SkinnedMesh {
        name: raw.name.clone(),
        bone_map,
        inverse_bind: inverse_bind_from_rest(&parents, &rest),
        materials: raw.materials.iter().map(|(slot, _)| slot.clone()).collect(),
        lods,
    })
}

/// Joins the parts into one pack.
pub fn pack(skeleton: Skeleton, sockets: Vec<Socket>, clips: Vec<Clip>, meshes: Vec<SkinnedMesh>) -> Pack {
    Pack { skeleton, sockets, clips, meshes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_unreal_yaw_to_the_right_is_a_turn_to_the_right_in_the_car_frame() {
        // Unreal yaw +90: forward (+x) turns to the right (+y in Unreal).
        let q = rotation(rotator_quat(0.0, 90.0, 0.0));
        let turned = q * Vec3::X;
        // In the car frame right is -y.
        assert!(turned.distance(Vec3::NEG_Y) < 1e-5, "{turned:?}");
    }

    #[test]
    fn positions_scale_and_mirror() {
        assert!(position([100.0, 50.0, 200.0]).distance(Vec3::new(1.0, -0.5, 2.0)) < 1e-6);
        assert!(direction([0.0, 1.0, 0.0]).distance(Vec3::NEG_Y) < 1e-6);
    }

    #[test]
    fn rotation_conversion_commutes_with_the_point_conversion() {
        let q = rotator_quat(20.0, -35.0, 70.0);
        let p = [30.0, -12.0, 8.0];
        // Rotate in Unreal's frame by the Unreal quaternion (standard sandwich), then convert...
        let uq = Quat::from_xyzw(q[0], q[1], q[2], q[3]);
        let rotated = uq * Vec3::from_array(p);
        let a = position(rotated.to_array());
        // ...equals converting first and rotating by the converted quaternion.
        let b = rotation(q) * position(p);
        assert!(a.distance(b) < 1e-5, "{a:?} {b:?}");
    }

    #[test]
    fn channel_compaction() {
        let rest = 1.0_f32;
        let close = |a: f32, b: f32| (a - b).abs() < 1e-6;
        assert_eq!(channel(vec![1.0, 1.0], rest, close), Channel::None);
        assert_eq!(channel(vec![2.0, 2.0], rest, close), Channel::Constant(2.0));
        assert_eq!(channel(vec![2.0, 3.0], rest, close), Channel::Uniform(vec![2.0, 3.0]));
    }
}
