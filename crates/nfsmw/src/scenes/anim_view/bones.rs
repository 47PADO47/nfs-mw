//! The maths of the stick figure: where each joint sits, the box that joins a bone to its parent, the time a clip
//! loops at, and the unit cube every box is stretched from.

use blackbox_gfx::Vertex;
use glam::{Mat4, Quat, Vec3};

/// The joint of every bone: the origin of its world matrix.
pub fn joints(world: &[Mat4]) -> Vec<Vec3> {
    world.iter().map(|matrix| matrix.w_axis.truncate()).collect()
}

/// The box of every bone that has a parent, from the parent's joint to its own, `thickness` across. A root has no
/// box, and neither does a bone whose joint sits on its parent's.
pub fn sticks(parents: &[Option<usize>], joints: &[Vec3], thickness: f32) -> Vec<Option<Mat4>> {
    parents
        .iter()
        .enumerate()
        .map(|(bone, parent)| {
            let parent = (*parent)?;
            segment(joints[parent], joints[bone], thickness)
        })
        .collect()
}

/// The transform that stretches the unit cube ([`cube`]) from `from` to `to`: its Z axis runs between the two
/// points, its ends sit on them, and it is `thickness` wide. `None` when the points are the same (or not numbers).
pub fn segment(from: Vec3, to: Vec3, thickness: f32) -> Option<Mat4> {
    let delta = to - from;
    let length = delta.length();
    if length.is_nan() || length <= f32::EPSILON {
        return None;
    }
    let rotation = Quat::from_rotation_arc(Vec3::Z, delta / length);
    let scale = Vec3::new(thickness, thickness, length);
    Some(Mat4::from_scale_rotation_translation(scale, rotation, (from + to) * 0.5))
}

/// The time within a clip of `duration` seconds that `secs` lands on: the clip loops, so the time wraps round. A clip
/// with no length, or a time that is not finite, shows its start.
pub fn looped(secs: f32, duration: f32) -> f32 {
    if !secs.is_finite() || !duration.is_finite() || duration <= 0.0 {
        return 0.0;
    }
    secs.rem_euclid(duration)
}

/// A unit cube centred on the origin, as 24 vertices (four per face, each with its face normal) and 36 indices. Every
/// triangle is counter-clockwise seen from outside, the same winding as the scenes' floors.
pub fn cube(color: [u8; 4]) -> (Vec<Vertex>, Vec<u16>) {
    // Each face: its outward normal, then two in-plane axes whose cross product is that normal.
    const FACES: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::Y, Vec3::X),
        (Vec3::X, Vec3::Y, Vec3::Z),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
    ];
    const CORNERS: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (normal, u, v) in FACES {
        let first = vertices.len() as u16;
        for (a, b) in CORNERS {
            let position = 0.5 * (normal + a * u + b * v);
            vertices.push(Vertex {
                position: position.to_array(),
                normal: normal.to_array(),
                color_bgra: color,
                uv: [0.0, 0.0],
            });
        }
        indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }
    (vertices, indices)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A root at height 1, and its child two metres further up its own Y axis after a quarter turn about Z: the
    /// child's joint is at (0, 1, 1), so its stick runs from (0, 0, 1) along +Y.
    fn two_bone_chain() -> (Vec<Option<usize>>, Vec<Mat4>) {
        let root = Mat4::from_translation(Vec3::new(0.0, 0.0, 1.0));
        let child = root * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2) * Mat4::from_translation(Vec3::X);
        (vec![None, Some(0)], vec![root, child])
    }

    #[test]
    fn a_two_bone_chain_gives_one_stick_from_the_root_joint_to_the_child_joint() {
        let (parents, world) = two_bone_chain();
        let joints = joints(&world);
        assert!((joints[0] - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-5);
        assert!((joints[1] - Vec3::new(0.0, 1.0, 1.0)).length() < 1e-5);
        let sticks = sticks(&parents, &joints, 0.1);
        assert!(sticks[0].is_none(), "the root has no parent, so no box");
        let stick = sticks[1].expect("the child has a box");
        let near_from = stick.transform_point3(Vec3::new(0.0, 0.0, -0.5));
        let near_to = stick.transform_point3(Vec3::new(0.0, 0.0, 0.5));
        assert!((near_from - joints[0]).length() < 1e-5, "the box starts at the parent's joint: {near_from}");
        assert!((near_to - joints[1]).length() < 1e-5, "the box ends at the bone's joint: {near_to}");
    }

    #[test]
    fn the_box_is_as_thick_as_asked_across_the_bone_and_stops_at_its_joints() {
        let from = Vec3::new(1.0, 2.0, 3.0);
        let to = Vec3::new(4.0, 2.0, 3.0);
        let stick = segment(from, to, 0.2).expect("a bone of length three");
        let mid = (from + to) * 0.5;
        let axis = (to - from).normalize();
        for side in [Vec3::X, Vec3::Y] {
            let corner = stick.transform_point3(side * 0.5);
            let across = corner - mid;
            assert!(across.dot(axis).abs() < 1e-5, "the cross-section is across the bone: {across}");
            assert!((across.length() - 0.1).abs() < 1e-5, "half the thickness from the axis: {across}");
        }
        let spans =
            stick.transform_point3(Vec3::new(0.0, 0.0, 0.5)) - stick.transform_point3(Vec3::new(0.0, 0.0, -0.5));
        assert!((spans.length() - 3.0).abs() < 1e-5, "the box is as long as the bone");
    }

    #[test]
    fn a_bone_with_no_length_has_no_box() {
        let point = Vec3::new(5.0, -1.0, 0.5);
        assert!(segment(point, point, 0.1).is_none());
        assert!(segment(point, Vec3::new(f32::NAN, 0.0, 0.0), 0.1).is_none());
    }

    #[test]
    fn a_clip_loops_over_its_length() {
        assert!((looped(2.5, 2.0) - 0.5).abs() < 1e-6);
        assert!((looped(-0.5, 2.0) - 1.5).abs() < 1e-6);
        assert!((looped(1.0, 2.0) - 1.0).abs() < 1e-6);
        assert_eq!(looped(3.0, 0.0), 0.0, "no length shows the start");
        assert_eq!(looped(f32::NAN, 2.0), 0.0);
    }

    #[test]
    fn every_face_of_the_cube_points_outwards() {
        let (vertices, indices) = cube([0, 0, 0, 255]);
        assert_eq!(vertices.len(), 24);
        assert_eq!(indices.len(), 36);
        for triangle in indices.as_chunks::<3>().0 {
            let [a, b, c] = triangle.map(|i| Vec3::from_array(vertices[usize::from(i)].position));
            let normal = Vec3::from_array(vertices[usize::from(triangle[0])].normal);
            let winding = (b - a).cross(c - a);
            assert!(winding.dot(normal) > 0.0, "a triangle faces its normal");
            assert!((normal.length() - 1.0).abs() < 1e-6);
        }
    }
}
