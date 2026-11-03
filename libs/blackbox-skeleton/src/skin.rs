//! Linear-blend skinning on the CPU.

use glam::{Mat4, Vec3};

use crate::error::{Error, Result};
use crate::transform::Transform;

/// The most bones that can pull on one vertex.
pub const MAX_INFLUENCES: usize = 8;

/// Up to [`MAX_INFLUENCES`] `(bone, weight)` pairs; unused slots have weight 0. The bone is an index into the mesh's
/// own bone list (the one `bone_map` and `inverse_bind` of [`crate::pack::SkinnedMesh`] are indexed by).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Influences {
    pub bones: [u16; MAX_INFLUENCES],
    pub weights: [f32; MAX_INFLUENCES],
}

impl Influences {
    /// Builds the slots from a list; extra pairs beyond [`MAX_INFLUENCES`] are dropped.
    #[must_use]
    pub fn from_pairs(pairs: &[(u16, f32)]) -> Self {
        let mut out = Self::default();
        for (slot, &(bone, weight)) in pairs.iter().take(MAX_INFLUENCES).enumerate() {
            out.bones[slot] = bone;
            out.weights[slot] = weight;
        }
        out
    }

    /// The sum of the weights.
    #[must_use]
    pub fn total(&self) -> f32 {
        self.weights.iter().sum()
    }
}

/// A vertex in the bind pose.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SkinVertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub uv: [f32; 2],
    pub influences: Influences,
}

/// The inverse of each bone's bind-pose component transform, as matrices. `parents[i]` is the mesh bone `i`'s parent
/// (must precede it) and `rest[i]` its bind transform relative to that parent.
#[must_use]
pub fn inverse_bind_from_rest(parents: &[Option<usize>], rest: &[Transform]) -> Vec<Mat4> {
    let mut world: Vec<Mat4> = Vec::with_capacity(rest.len());
    for (i, local) in rest.iter().enumerate() {
        let matrix = match parents.get(i).copied().flatten() {
            Some(parent) => world[parent] * local.to_mat4(),
            None => local.to_mat4(),
        };
        world.push(matrix);
    }
    world.iter().map(Mat4::inverse).collect()
}

/// One skinning matrix per mesh bone: `component[bone_map[i]] * inverse_bind[i]`. A `bone_map` entry of `None` (the
/// mesh bone does not exist in the skeleton) gives the identity.
pub fn skin_matrices(
    component: &[Transform],
    bone_map: &[Option<usize>],
    inverse_bind: &[Mat4],
    out: &mut Vec<Mat4>,
) -> Result<()> {
    if bone_map.len() != inverse_bind.len() {
        return Err(Error::CountMismatch { expected: bone_map.len(), got: inverse_bind.len() });
    }
    out.clear();
    for (mapped, inverse) in bone_map.iter().zip(inverse_bind) {
        let Some(skeleton_bone) = *mapped else {
            out.push(Mat4::IDENTITY);
            continue;
        };
        let transform = component.get(skeleton_bone).ok_or(Error::BadBone(skeleton_bone))?;
        out.push(transform.to_mat4() * *inverse);
    }
    Ok(())
}

/// Skins every vertex. `positions` and `normals` are cleared and filled, one entry per vertex. A vertex whose
/// weights sum to nothing keeps its bind position. Normals are rotated by the blended matrix and renormalised, which
/// is exact while the bones have a uniform scale.
pub fn skin_vertices(vertices: &[SkinVertex], matrices: &[Mat4], positions: &mut Vec<Vec3>, normals: &mut Vec<Vec3>) {
    positions.clear();
    normals.clear();
    for vertex in vertices {
        let total = vertex.influences.total();
        if total <= 0.0 {
            positions.push(vertex.position);
            normals.push(vertex.normal);
            continue;
        }
        let mut blended = Mat4::ZERO;
        for slot in 0..MAX_INFLUENCES {
            let weight = vertex.influences.weights[slot];
            if weight == 0.0 {
                continue;
            }
            let matrix = matrices.get(usize::from(vertex.influences.bones[slot])).copied().unwrap_or(Mat4::IDENTITY);
            blended += matrix * (weight / total);
        }
        positions.push(blended.transform_point3(vertex.position));
        normals.push(blended.transform_vector3(vertex.normal).normalize_or_zero());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::tests::arm;
    use glam::Quat;

    fn vertex_on(bones: &[(u16, f32)], at: Vec3) -> SkinVertex {
        SkinVertex { position: at, normal: Vec3::Y, uv: [0.0; 2], influences: Influences::from_pairs(bones) }
    }

    fn mesh_matrices(local: &[Transform]) -> Vec<Mat4> {
        let skel = arm();
        let parents: Vec<Option<usize>> = skel.bones().iter().map(|b| b.parent).collect();
        let rest: Vec<Transform> = skel.bones().iter().map(|b| b.rest).collect();
        let inverse = inverse_bind_from_rest(&parents, &rest);
        let mut component = Vec::new();
        skel.component_transforms(local, &mut component).unwrap();
        let map: Vec<Option<usize>> = (0..4).map(Some).collect();
        let mut out = Vec::new();
        skin_matrices(&component, &map, &inverse, &mut out).unwrap();
        out
    }

    #[test]
    fn rest_pose_leaves_vertices_alone() {
        let skel = arm();
        let m = mesh_matrices(&skel.rest_pose().local);
        let v = vertex_on(&[(2, 0.5), (3, 0.5)], Vec3::new(0.3, 2.5, 0.1));
        let (mut p, mut n) = (Vec::new(), Vec::new());
        skin_vertices(&[v], &m, &mut p, &mut n);
        assert!(p[0].distance(v.position) < 1e-5);
        assert!(n[0].distance(Vec3::Y) < 1e-5);
    }

    #[test]
    fn rotating_a_bone_swings_its_vertices_about_the_joint() {
        let skel = arm();
        let mut pose = skel.rest_pose();
        pose.local[1].rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        let m = mesh_matrices(&pose.local);
        // A vertex rigidly on the elbow (bone 2) at y = 2 swings about the shoulder at y = 1: it ends at x = -1, y = 1.
        let v = vertex_on(&[(2, 1.0)], Vec3::new(0.0, 2.0, 0.0));
        let (mut p, mut n) = (Vec::new(), Vec::new());
        skin_vertices(&[v], &m, &mut p, &mut n);
        assert!(p[0].distance(Vec3::new(-1.0, 1.0, 0.0)) < 1e-5, "{:?}", p[0]);
        assert!(n[0].distance(Vec3::NEG_X) < 1e-5);
    }

    #[test]
    fn weights_are_normalised_and_unweighted_vertices_stay() {
        let skel = arm();
        let mut pose = skel.rest_pose();
        pose.local[1].translation.x = 2.0;
        let m = mesh_matrices(&pose.local);
        // Weights 1 and 1 behave like 0.5 and 0.5: bone 0 does not move, bone 1 moves by 2 in x.
        let v = vertex_on(&[(0, 1.0), (1, 1.0)], Vec3::new(0.0, 1.0, 0.0));
        let bare = vertex_on(&[], Vec3::new(5.0, 5.0, 5.0));
        let (mut p, mut n) = (Vec::new(), Vec::new());
        skin_vertices(&[v, bare], &m, &mut p, &mut n);
        assert!(p[0].distance(Vec3::new(1.0, 1.0, 0.0)) < 1e-5);
        assert_eq!(p[1], Vec3::new(5.0, 5.0, 5.0));
    }

    #[test]
    fn unmapped_bone_is_identity_and_bad_index_errors() {
        let mut out = Vec::new();
        let component = vec![Transform::IDENTITY];
        skin_matrices(&component, &[None], &[Mat4::IDENTITY], &mut out).unwrap();
        assert_eq!(out, vec![Mat4::IDENTITY]);
        assert_eq!(skin_matrices(&component, &[Some(4)], &[Mat4::IDENTITY], &mut out), Err(Error::BadBone(4)));
        assert!(skin_matrices(&component, &[None], &[], &mut out).is_err());
    }
}
