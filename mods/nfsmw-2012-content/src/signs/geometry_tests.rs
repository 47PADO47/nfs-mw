//! The geometry of the signs and lights on authored meshes. No game data.

use super::*;
use crate::signs::table::{Box3, Front, Hanging};

fn mesh(positions: Vec<[f32; 3]>, triangles: Vec<[u32; 3]>) -> Mesh {
    let uvs = vec![[0.0, 0.0]; positions.len()];
    Mesh { picture: Picture::Atlas(1), cut_out: false, positions, uvs, triangles }
}

/// A quad in the x-y plane (a sign plate), counter-clockwise seen from +z.
fn plate(width: f32, height: f32, z: f32) -> Mesh {
    let (w, h) = (0.5 * width, height);
    mesh(vec![[-w, 0.0, z], [w, 0.0, z], [w, h, z], [-w, h, z]], vec![[0, 1, 2], [0, 2, 3]])
}

/// The unit normal of the first triangle, by its winding.
fn normal(mesh: &Mesh) -> [f32; 3] {
    let [a, b, c] = mesh.triangles[0].map(|i| mesh.positions[i as usize]);
    let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
    let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    n.map(|value| value / length)
}

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|axis| (a[axis] - b[axis]).abs() < 1e-5)
}

#[test]
fn a_sign_that_faces_either_way_ends_up_facing_minus_y() {
    // Its front normal is +z in the 2012 axes (the plate is counter-clockwise from +z) ...
    let front_plus = plate(1.0, 2.0, 0.0);
    assert!(close(normal(&front_plus), [0.0, 0.0, 1.0]));
    let a = face_minus_y(&to_old_axes(&[front_plus]), Front::PlusZ);
    assert!(close(normal(&a[0]), [0.0, -1.0, 0.0]), "{:?}", normal(&a[0]));
    // ... and a plate wound the other way has its front towards -z.
    let mut back = plate(1.0, 2.0, 0.0);
    back.triangles.iter_mut().for_each(|t| t.swap(1, 2));
    assert!(close(normal(&back), [0.0, 0.0, -1.0]));
    let b = face_minus_y(&to_old_axes(&[back]), Front::MinusZ);
    assert!(close(normal(&b[0]), [0.0, -1.0, 0.0]), "{:?}", normal(&b[0]));
}

#[test]
fn the_axes_become_x_forward_y_left_z_up_without_turning_the_model_inside_out() {
    let moved = to_old_axes(&[mesh(vec![[1.0, 2.0, 3.0], [0.0, 0.0, 0.0], [0.0, 1.0, 0.0]], vec![[0, 1, 2]])]);
    assert_eq!(moved[0].positions[0], [3.0, 1.0, 2.0]);
    assert_eq!(moved[0].triangles, [[0, 1, 2]]);
}

#[test]
fn a_sign_is_scaled_evenly_to_the_old_height_and_stands_on_its_floor() {
    let old = Box3 { low: [-0.5, -0.05, -0.3], high: [0.5, 0.09, 3.2] };
    let model = to_old_axes(&[plate(0.6, 2.3, 0.0)]);
    let fitted = stand_in(&face_minus_y(&model, Front::PlusZ), &old, 1.15);
    let b = bounds(&fitted).unwrap();
    assert!((b.high[2] - b.low[2] - 3.5).abs() < 1e-4, "as tall as the old: {b:?}");
    assert!((b.low[2] + 0.3).abs() < 1e-4, "on the old floor");
    assert!(b.high[0] - b.low[0] <= 1.15 * 1.0 + 1e-4, "no wider than the plate share allows: {b:?}");
    // A very wide sign is held to the width instead.
    let wide = stand_in(&face_minus_y(&to_old_axes(&[plate(4.0, 2.3, 0.0)]), Front::PlusZ), &old, 1.15);
    let w = bounds(&wide).unwrap();
    assert!((w.high[0] - w.low[0] - 1.15).abs() < 1e-4, "{w:?}");
    assert!(w.high[2] - w.low[2] < 3.5);
}

#[test]
fn a_model_that_sticks_out_is_squeezed_in_and_one_that_fits_is_left_alone() {
    let old = Box3 { low: [-0.25, -1.0, 0.0], high: [0.25, 0.25, 8.0] };
    let fits = vec![mesh(vec![[-0.2, -0.9, 0.0], [0.2, 0.2, 7.9], [0.0, 0.0, 1.0]], vec![[0, 1, 2]])];
    assert_eq!(squeeze_into(&fits, &old, 1.25), fits);
    // 1.5 m thick on x where the old is 0.5 wide.
    let thick = vec![mesh(vec![[-0.75, -0.9, 0.0], [0.75, 0.2, 7.9], [0.0, 0.0, 1.0]], vec![[0, 1, 2]])];
    let squeezed = squeeze_into(&thick, &old, 1.25);
    let b = bounds(&squeezed).unwrap();
    assert!(b.low[0] >= -0.25 - 0.0625 - 0.05 && b.high[0] <= 0.25 + 0.0625 + 0.05, "{b:?}");
    assert_eq!((b.low[2], b.high[2]), (0.0, 7.9), "the other axes are untouched");
}

#[test]
fn a_model_is_stretched_to_fill_the_old_box() {
    let old = Box3 { low: [-0.25, -7.0, -0.05], high: [0.25, 0.25, 8.7] };
    let model = mesh(vec![[-0.2, -6.0, 0.0], [0.2, 0.2, 6.0], [0.0, -3.0, 3.0]], vec![[0, 1, 2]]);
    let b = bounds(&fill(&[model], &old)).unwrap();
    assert!(close(b.low, old.low) && close(b.high, old.high), "{b:?}");
}

/// A mast arm in the 2012 axes: a pole 6 m tall (x and z near 0), an arm 5 m long at the top, and two signal heads hanging from it.
fn arm_with_heads() -> Mesh {
    let mut positions = Vec::new();
    let mut triangles = Vec::new();
    let mut piece = |points: [[f32; 3]; 3]| {
        let at = positions.len() as u32;
        positions.extend(points);
        triangles.push([at, at + 1, at + 2]);
    };
    piece([[0.0, 0.0, 0.0], [0.1, 6.0, 0.0], [0.0, 3.0, 0.1]]); // the pole, to the ground
    piece([[0.0, 5.5, 0.0], [-5.0, 5.8, 0.0], [-2.5, 5.6, 0.1]]); // the arm, metres long
    piece([[-2.0, 4.6, 0.0], [-1.8, 5.5, 0.0], [-1.9, 5.0, 0.3]]); // a head, a metre or less, high up
    piece([[-4.0, 4.6, 0.0], [-3.8, 5.5, 0.0], [-3.9, 5.0, 0.3]]); // another
    piece([[-0.4, 1.9, 0.0], [-0.1, 2.3, 0.0], [-0.2, 2.1, 0.2]]); // the pedestrian head, small and above 1.5 m
    mesh(positions, triangles)
}

#[test]
fn the_cut_takes_the_heads_and_keeps_the_pole_and_the_arm() {
    let cut = cut_heads(&arm_with_heads());
    assert_eq!(cut.arm.triangles.len(), 2, "pole and arm");
    assert_eq!(cut.taken.len(), 3, "two signal heads and the pedestrian head");
    let b = bounds(std::slice::from_ref(&cut.arm)).unwrap();
    assert_eq!((b.low[0], b.high[1]), (-5.0, 6.0));
    for head in &cut.taken {
        assert!(bounds(std::slice::from_ref(head)).unwrap().low[1] > HEAD_FLOOR);
    }
}

#[test]
fn surfaces_that_share_a_position_are_one_surface() {
    let two_touching = mesh(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
        vec![[0, 1, 2], [3, 4, 5]],
    );
    assert_eq!(surfaces(&two_touching).len(), 1, "the corner at x = 1 is doubled in the buffer but is one place");
    let apart = mesh(
        vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [5.0, 0.0, 0.0], [6.0, 0.0, 0.0], [5.0, 1.0, 0.0]],
        vec![[0, 1, 2], [3, 4, 5]],
    );
    assert_eq!(surfaces(&apart).len(), 2);
}

/// A signal head in the 2012 axes: a housing 0.3 wide, 0.9 tall and 0.2 deep (a box of triangles) with its back at z = 0.05, and a lens
/// visor in front of it. Facing +z.
fn head() -> Vec<Mesh> {
    let housing = {
        let mut points = Vec::new();
        for y in [2.15, 3.05] {
            for (x, z) in [(-0.15, 0.05), (0.15, 0.05), (0.15, 0.25), (-0.15, 0.25)] {
                points.push([x, y, z]);
            }
        }
        let mut tris = Vec::new();
        for side in 0..4u32 {
            let next = (side + 1) % 4;
            tris.push([side, next, 4 + next]);
            tris.push([side, 4 + next, 4 + side]);
        }
        mesh(points, tris)
    };
    let visor = mesh(vec![[-0.1, 2.8, 0.25], [0.1, 2.8, 0.25], [0.0, 3.0, 0.43]], vec![[0, 1, 2]]);
    vec![housing, visor]
}

#[test]
fn a_head_hangs_with_its_middle_lens_where_the_world_hangs_its_own_and_faces_forward() {
    let hanging = Hanging { centre: -0.86, pitch: 0.35 };
    let (hung, red) = hang(&head(), hanging, -0.1).unwrap();
    let b = bounds(&hung).unwrap();
    // The housing's middle (the amber lens) is at `centre`; the scale is pitch over the 2012 lens spacing.
    let scale = 0.35 / LENS_PITCH;
    let housing = bounds(&hung[..1]).unwrap();
    assert!((0.5 * (housing.low[2] + housing.high[2]) + 0.86).abs() < 1e-4, "{housing:?}");
    assert!(((housing.high[2] - housing.low[2]) - 0.9 * scale).abs() < 1e-4);
    assert!((housing.low[0] + 0.1).abs() < 1e-5, "the back of the housing is at the asked for x");
    assert!(b.high[0] > housing.high[0], "the lens visor is in front, towards +x");
    assert!(b.low[1] < 0.0 && b.high[1] > 0.0, "centred across");
    assert_eq!(red[0], b.high[0], "the red flare sits on the front of the visors");
    assert!((red[2] - (-0.86 + 0.35)).abs() < 1e-5, "red is a pitch above the middle");
}

#[test]
fn a_rod_is_a_closed_shape_with_its_faces_out() {
    let rod = rod(0.0, -0.5, 0.04, [0.0; 3]);
    let b = bounds(std::slice::from_ref(&rod)).unwrap();
    assert_eq!((b.low[2], b.high[2]), (-0.5, 0.0));
    let faces_out = rod.triangles.iter().all(|t| {
        let [a, b, c] = t.map(|i| rod.positions[i as usize]);
        let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - a[0], c[1] - a[1]]);
        let n = [u[1] * (c[2] - a[2]) - (b[2] - a[2]) * v[1], (b[2] - a[2]) * v[0] - u[0] * (c[2] - a[2])];
        // Horizontal part of the face normal against the direction from the axis to the face.
        n[0] * (a[0] + b[0] + c[0]) + n[1] * (a[1] + b[1] + c[1]) > 0.0
    });
    assert!(faces_out, "every side faces away from the rod's axis");
}
