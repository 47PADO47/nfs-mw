//! The outlet search on synthetic silencers: tubes of a known axis and end, not any bike's mesh.

use super::*;

type Mesh = (Vec<[f32; 3]>, Vec<[u32; 3]>);

const RINGS: usize = 8;
const SEGMENTS: usize = 12;

/// A closed tube from `from` to `to`: rings of vertices along it, a fan at each end.
fn tube(from: V, to: V, radius: f64) -> Mesh {
    let axis = unit(sub(to, from)).unwrap();
    let (b, c) = across(axis);
    let mut points: Vec<[f32; 3]> = Vec::new();
    for ring in 0..RINGS {
        let t = ring as f64 / (RINGS - 1) as f64;
        let middle: V = std::array::from_fn(|k| from[k] + (to[k] - from[k]) * t);
        for s in 0..SEGMENTS {
            let a = std::f64::consts::TAU * s as f64 / SEGMENTS as f64;
            let p: V = std::array::from_fn(|k| middle[k] + radius * (a.cos() * b[k] + a.sin() * c[k]));
            points.push(p.map(|v| v as f32));
        }
    }
    let (start, end) = (points.len() as u32, points.len() as u32 + 1);
    points.push(from.map(|v| v as f32));
    points.push(to.map(|v| v as f32));
    let mut triangles = Vec::new();
    let at = |ring: usize, s: usize| (ring * SEGMENTS + s % SEGMENTS) as u32;
    for ring in 0..RINGS - 1 {
        for s in 0..SEGMENTS {
            triangles.push([at(ring, s), at(ring, s + 1), at(ring + 1, s)]);
            triangles.push([at(ring, s + 1), at(ring + 1, s + 1), at(ring + 1, s)]);
        }
    }
    for s in 0..SEGMENTS {
        triangles.push([start, at(0, s + 1), at(0, s)]);
        triangles.push([end, at(RINGS - 1, s), at(RINGS - 1, s + 1)]);
    }
    (points, triangles)
}

/// The meshes as one.
fn join(meshes: Vec<Mesh>) -> Mesh {
    let (mut points, mut triangles) = (Vec::new(), Vec::new());
    for (p, t) in meshes {
        let base = points.len() as u32;
        points.extend(p);
        triangles.extend(t.into_iter().map(|[a, b, c]| [a + base, b + base, c + base]));
    }
    (points, triangles)
}

fn near(a: [f32; 3], b: V, tolerance: f64) -> bool {
    (0..3).all(|k| (f64::from(a[k]) - b[k]).abs() < tolerance)
}

fn found(mesh: &Mesh) -> Vec<Pipe> {
    pipes(&mesh.0, &mesh.1)
}

#[test]
fn an_upswept_silencer_on_the_right_has_its_outlet_at_the_rear_end_shooting_along_the_axis() {
    let found = found(&tube([-0.35, -0.15, 0.5], [-0.75, -0.15, 0.75], 0.06));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].side, Side::Right);
    assert!(near(found[0].at, [-0.75, -0.15, 0.75], 1e-4), "{:?}", found[0].at);
    let axis = unit([-0.4, 0.0, 0.25]).unwrap();
    assert!(near(found[0].dir, axis, 1e-3), "{:?}", found[0].dir);
}

#[test]
fn the_order_of_the_mesh_does_not_decide_which_end_is_the_outlet() {
    let found = found(&tube([-0.75, -0.15, 0.75], [-0.35, -0.15, 0.5], 0.06));
    assert!(near(found[0].at, [-0.75, -0.15, 0.75], 1e-4));
    assert!(found[0].dir[0] < -0.8, "backwards: {:?}", found[0].dir);
}

#[test]
fn two_silencers_make_two_pipes_named_by_their_side() {
    let mesh =
        join(vec![tube([-0.3, 0.2, 0.4], [-0.8, 0.2, 0.4], 0.05), tube([-0.3, -0.2, 0.4], [-0.8, -0.2, 0.4], 0.05)]);
    let found = found(&mesh);
    assert_eq!(found.iter().map(|p| p.side).collect::<Vec<_>>(), vec![Side::Left, Side::Right]);
    assert!(near(found[0].at, [-0.8, 0.2, 0.4], 1e-4) && near(found[1].at, [-0.8, -0.2, 0.4], 1e-4));
    let names: Vec<u32> = markers(&found).iter().map(|m| m.name_hash).collect();
    assert_eq!(names, vec![bstring_hash("LEFT_EXHAUST"), bstring_hash("RIGHT_EXHAUST")]);
}

#[test]
fn a_silencer_on_the_centre_plane_is_the_exhaust_marker() {
    let found = found(&tube([-0.3, 0.01, 0.2], [-0.8, 0.01, 0.2], 0.05));
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].side, Side::Centre);
    assert_eq!(marker(&found[0]).name_hash, bstring_hash("EXHAUST"));
}

#[test]
fn a_small_fitting_does_not_pull_the_outlet() {
    let can = tube([-0.35, -0.15, 0.5], [-0.75, -0.15, 0.75], 0.06);
    let bolt = tube([-1.2, -0.15, 0.5], [-1.22, -0.15, 0.5], 0.004);
    let found = found(&join(vec![can, bolt]));
    assert_eq!(found.len(), 1, "the bolt is not a pipe of its own");
    assert!(near(found[0].at, [-0.75, -0.15, 0.75], 1e-4), "{:?}", found[0].at);
}

#[test]
fn a_can_shorter_than_it_is_wide_shoots_straight_back() {
    let found = found(&tube([-0.5, 0.2, 0.4], [-0.6, 0.2, 0.4], 0.1));
    assert!(near(found[0].dir, BACKWARD, 1e-6));
    assert!((f64::from(found[0].at[0]) + 0.6).abs() < 1e-4);
}

#[test]
fn a_squat_pot_beside_a_real_can_is_not_an_outlet() {
    let can = tube([-0.35, -0.15, 0.5], [-0.75, -0.15, 0.75], 0.06);
    let pot = tube([-0.2, 0.0, 0.3], [-0.4, 0.0, 0.3], 0.15);
    let found = found(&join(vec![can, pot]));
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].side, Side::Right);
}

#[test]
fn an_upright_stack_shoots_up_and_a_crosswise_one_shoots_out() {
    let stack = found(&tube([-0.5, -0.2, 0.3], [-0.5, -0.2, 0.8], 0.04));
    assert!(near(stack[0].dir, [0.0, 0.0, 1.0], 1e-6));
    assert!(near(stack[0].at, [-0.5, -0.2, 0.8], 1e-3), "{:?}", stack[0].at);
    let side = found(&tube([-0.5, -0.2, 0.3], [-0.5, -0.7, 0.3], 0.04));
    assert_eq!(side[0].side, Side::Right);
    assert!(near(side[0].dir, [0.0, -1.0, 0.0], 1e-6), "{:?}", side[0].dir);
}

#[test]
fn nothing_to_find_in_an_empty_or_broken_mesh() {
    assert!(pipes(&[], &[]).is_empty());
    assert!(pipes(&[[0.0; 3]; 3], &[[0, 1, 2]]).is_empty(), "a triangle with no area");
    assert!(pipes(&[[0.0; 3], [1.0, 0.0, 0.0]], &[[0, 1, 7]]).is_empty(), "an index outside the points");
}

#[test]
fn vertices_within_a_millimetre_are_one_vertex() {
    let ids = welded(&[[0.0, 0.0, 0.0], [0.0004, 0.0, 0.0], [0.002, 0.0, 0.0]]);
    assert_eq!(ids[0], ids[1]);
    assert_ne!(ids[0], ids[2]);
}

#[test]
fn a_marker_is_a_right_handed_frame_whose_z_axis_is_the_flame() {
    let pipe = Pipe { side: Side::Left, at: [-0.7, 0.1, 0.6], dir: [-0.8, 0.0, 0.6] };
    let m = marker(&pipe).matrix;
    let row = |n: usize| [f64::from(m[n * 4]), f64::from(m[n * 4 + 1]), f64::from(m[n * 4 + 2])];
    let (x, y, z) = (row(0), row(1), row(2));
    assert!(near(pipe.dir, z, 1e-6));
    assert!(dot(x, y).abs() < 1e-6 && dot(x, z).abs() < 1e-6 && dot(y, z).abs() < 1e-6);
    assert!(near([x[0] as f32, x[1] as f32, x[2] as f32], cross(y, z), 1e-6), "x = y cross z");
    assert!(x[2] > 0.0, "the x axis points up");
    assert_eq!(&m[12..16], &[-0.7, 0.1, 0.6, 1.0]);
}

#[test]
fn the_marker_of_a_pipe_that_shoots_up_has_a_forward_x_axis() {
    let up = Pipe { side: Side::Right, at: [0.0; 3], dir: [0.0, 0.0, 1.0] };
    let m = marker(&up).matrix;
    assert_eq!(&m[0..3], &[1.0, 0.0, 0.0]);
    assert_eq!(&m[8..11], &[0.0, 0.0, 1.0]);
}
