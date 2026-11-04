//! Blended skinning of a NIS character's rigid parts on the CPU (`docs/specs/nis-player-2.md`, section 9.2). Each vertex
//! moves with up to three bones by its blend weights, as the original's skinned-character shader does, so a vertex that
//! two parts share lands in one place and the parts' seams stay closed.

use blackbox_solid::{RigidPart, SkinVertex, Vertex};
use glam::{Mat4, Vec3};

/// Writes the posed vertices of `part` into `out` (resized to the part's vertex count): each vertex blended over its
/// bones, `sum(weight * skinning[bone] * bind_position) / sum(weight)`, and the normal the same way with the matrices'
/// rotation, then normalised. Colour and UV are kept. A vertex without weights keeps its bind position, and a bone beyond
/// `skinning` counts as the identity. The result is in the rig's frame: the part is drawn with the actor's placement
/// alone.
pub fn skin_part(part: &RigidPart, skin: &[SkinVertex], skinning: &[Mat4], out: &mut Vec<Vertex>) {
    out.clear();
    out.extend_from_slice(&part.vertices);
    for (vertex, &source) in out.iter_mut().zip(&part.sources) {
        let Some(influence) = skin.get(source as usize) else { continue };
        let position = Vec3::from_array(vertex.position);
        let normal = Vec3::from_array(vertex.normal);
        let (mut placed, mut turned, mut total) = (Vec3::ZERO, Vec3::ZERO, 0.0);
        for (&weight, &bone) in influence.weights.iter().zip(&influence.bones) {
            if weight <= 0.0 {
                continue;
            }
            let matrix = skinning.get(usize::from(bone)).copied().unwrap_or(Mat4::IDENTITY);
            placed += matrix.transform_point3(position) * weight;
            turned += matrix.transform_vector3(normal) * weight;
            total += weight;
        }
        if total <= 0.0 {
            continue;
        }
        vertex.position = (placed / total).to_array();
        vertex.normal = turned.normalize_or(normal).to_array();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(position: [f32; 3]) -> Vertex {
        Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [255; 4], uv: [0.25, 0.5] }
    }

    fn part(positions: &[[f32; 3]]) -> RigidPart {
        RigidPart {
            bone: 0,
            vertices: positions.iter().copied().map(vertex).collect(),
            sources: (0..positions.len() as u32).collect(),
            indices: vec![],
            runs: vec![],
        }
    }

    fn influence(weights: [f32; 3], bones: [u8; 3]) -> SkinVertex {
        SkinVertex { weights, bones }
    }

    #[test]
    fn a_vertex_moves_with_the_weighted_mean_of_its_bones() {
        let part = part(&[[1.0, 0.0, 0.0]]);
        let skin = [influence([0.25, 0.75, 0.0], [0, 1, 0])];
        let skinning = [Mat4::IDENTITY, Mat4::from_translation(Vec3::new(0.0, 4.0, 0.0))];
        let mut out = Vec::new();
        skin_part(&part, &skin, &skinning, &mut out);
        assert_eq!(out[0].position, [1.0, 3.0, 0.0]);
        assert_eq!((out[0].uv, out[0].color_bgra), ([0.25, 0.5], [255; 4]), "colour and UV are kept");
    }

    #[test]
    fn two_parts_sharing_a_vertex_put_it_in_one_place() {
        // The rigid split copies a shared vertex into both parts; the rigid draw moved each copy with its own bone.
        let skin = [influence([0.5, 0.5, 0.0], [0, 1, 0])];
        let skinning = [Mat4::from_rotation_z(0.6), Mat4::from_translation(Vec3::new(0.0, 0.0, 2.0))];
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let (upper, lower) =
            (RigidPart { bone: 0, ..part(&[[1.0, 1.0, 0.0]]) }, RigidPart { bone: 1, ..part(&[[1.0, 1.0, 0.0]]) });
        skin_part(&upper, &skin, &skinning, &mut a);
        skin_part(&lower, &skin, &skinning, &mut b);
        assert_eq!(a[0].position, b[0].position);
    }

    #[test]
    fn normals_turn_with_the_bones_and_stay_unit_length() {
        let part = part(&[[0.0, 0.0, 0.0]]);
        let skin = [influence([1.0, 0.0, 0.0], [1, 0, 0])];
        let skinning = [Mat4::IDENTITY, Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2)];
        let mut out = Vec::new();
        skin_part(&part, &skin, &skinning, &mut out);
        let normal = Vec3::from_array(out[0].normal);
        assert!((normal - Vec3::new(0.0, -1.0, 0.0)).length() < 1e-5, "{normal}");
    }

    #[test]
    fn a_vertex_without_weights_or_with_a_missing_bone_is_kept_sane() {
        let part = part(&[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
        let skin = [influence([0.0; 3], [0; 3]), influence([1.0, 0.0, 0.0], [9, 0, 0])];
        let mut out = Vec::new();
        skin_part(&part, &skin, &[Mat4::from_translation(Vec3::X)], &mut out);
        assert_eq!(out[0].position, [1.0, 2.0, 3.0], "no weight: the bind position");
        assert_eq!(out[1].position, [4.0, 5.0, 6.0], "a bone the rig lacks: the identity");
    }
}
