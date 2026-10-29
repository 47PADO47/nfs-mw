use super::super::attach::tests::SAMPLE;
use super::*;
use blackbox_skeleton::{Bone, Influences, MeshLod, SkinVertex, SkinnedMesh};
use glam::Quat;

fn flat(z: f32) -> Vec<[f32; 2]> {
    vec![[-0.5, z], [0.1, z]]
}

#[test]
fn the_profile_is_read_by_linear_interpolation_and_ends_where_it_was_measured() {
    let surface = [[-0.4, 0.80], [-0.2, 0.84], [0.0, 0.90]];
    assert_eq!(height(&surface, -0.4), Some(0.80));
    assert!((height(&surface, -0.3).unwrap() - 0.82).abs() < 1e-6);
    assert!((height(&surface, -0.1).unwrap() - 0.87).abs() < 1e-6);
    assert_eq!(height(&surface, 0.0), Some(0.90));
    assert_eq!(height(&surface, -0.41), None, "behind the profile");
    assert_eq!(height(&surface, 0.01), None, "ahead of it");
    assert_eq!(height(&[], 0.0), None);
    assert_eq!(height(&[[0.0, 1.0]], 0.0), None, "one sample is no profile");
}

/// A pelvis (0) whose frame is the bike's own (the identity seat), at `z` above the ground, and the seat points 0.12 and 0.10 m
/// below it, at x = -0.1 and x = -0.05, and one in front of the profile.
fn seat(z: f32, sink: f32) -> (Seat, Vec<Transform>) {
    let points = vec![Vec3::new(-0.1, 0.0, -0.12), Vec3::new(-0.05, 0.05, -0.10), Vec3::new(0.5, 0.0, -0.30)];
    let component = vec![Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, z))];
    (Seat::from_points(0, points, flat(0.84), sink), component)
}

#[test]
fn the_body_is_lowered_until_the_lowest_point_sinks_into_the_seat() {
    let (seat, component) = seat(1.0, 0.02);
    // The lowest point is at 0.88, 4 cm above the seat: to sink 2 cm into it the body drops 6 cm.
    assert!((seat.clearance(&component, &Mat4::IDENTITY).unwrap() - 0.04).abs() < 1e-6);
    assert!((seat.drop(&component, &Mat4::IDENTITY).unwrap() - 0.06).abs() < 1e-6);
    // Dropped by that much, the lowest point is exactly `sink` below the surface.
    let lowered: Vec<Transform> =
        component.iter().map(|t| Transform::from_rt(t.rotation, t.translation - Vec3::Z * 0.06)).collect();
    assert!((seat.clearance(&lowered, &Mat4::IDENTITY).unwrap() + 0.02).abs() < 1e-6);
    // A rider already deeper than that is raised.
    let (low, component) = (seat.clone(), vec![Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.93))]);
    // His lowest point is at 0.81, 3 cm under the surface: 1 cm deeper than the sink of 2 cm, so he comes up 1 cm.
    assert!((low.drop(&component, &Mat4::IDENTITY).unwrap() + 0.01).abs() < 1e-6);
}

#[test]
fn a_point_ahead_of_the_profile_does_not_count_and_nothing_over_it_is_no_answer() {
    let (seat, component) = seat(1.0, 0.0);
    // The point at x = 0.5 is 0.70 high, far under the seat's height, but off the profile: it does not pull the rider down.
    assert!((seat.clearance(&component, &Mat4::IDENTITY).unwrap() - 0.04).abs() < 1e-6);
    // Moved back along the bike, nothing is over the surface.
    let back = Mat4::from_translation(Vec3::new(-2.0, 0.0, 0.0));
    assert_eq!(seat.clearance(&component, &back), None);
    assert_eq!(seat.drop(&component, &back), None);
}

#[test]
fn the_bike_frame_matrix_carries_the_riders_space_onto_the_surface() {
    // The rider's root sits on the seat socket, turned a quarter about z (he faces the bike's +x): his pelvis is at the socket.
    let socket = Transform::from_rt(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2), Vec3::new(-0.3, 0.0, 0.9));
    let to_bike = Mat4::from_rotation_translation(socket.rotation, socket.translation);
    // In the rider's space his pelvis is 0.1 above the root and the seat point 0.12 below it, 0.1 behind (his -y is the bike's -x...
    // he faces +y of his own space, which the quarter turn takes onto the bike's... +x is his -y).
    let component = vec![Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, 0.1))];
    let seat = Seat::from_points(0, vec![Vec3::new(0.0, 0.1, -0.12)], vec![[-0.5, 0.80], [0.1, 0.80]], 0.0);
    // His own +y goes to the bike's -x (the quarter turn about z takes +y to -x): the point is at x = -0.3 - 0.1, z = 0.88.
    let clearance = seat.clearance(&component, &to_bike).unwrap();
    assert!((clearance - 0.08).abs() < 1e-5, "{clearance}");
}

#[test]
fn the_shift_is_held_to_its_bound() {
    let (seat, component) = seat(2.0, 0.0);
    assert_eq!(seat.drop(&component, &Mat4::IDENTITY), Some(MAX_SHIFT_M));
}

/// A rider of a pelvis (1) and a thigh (2) under a root (0), and a mesh bound to them.
fn pack() -> Pack {
    let bone = |name: &str, parent: Option<usize>, z: f32| Bone {
        name: name.into(),
        parent,
        rest: Transform::from_rt(Quat::IDENTITY, Vec3::new(0.0, 0.0, z)),
    };
    let skeleton =
        Skeleton::new(vec![bone("root", None, 0.0), bone("pelvis", Some(0), 1.0), bone("thigh", Some(1), -0.1)])
            .unwrap();
    let vertex = |position: Vec3, pairs: &[(u16, f32)]| SkinVertex {
        position,
        normal: Vec3::Z,
        uv: [0.0; 2],
        influences: Influences::from_pairs(pairs),
    };
    // The mesh's own bones are (pelvis, thigh); their bind places are 1.0 and 0.9 up. Bind positions are in component space.
    let lod = MeshLod {
        screen_size: 1.0,
        vertices: vec![
            vertex(Vec3::new(0.0, 0.0, 0.88), &[(0, 1.0)]),
            vertex(Vec3::new(0.0, 0.0, 0.5), &[(1, 1.0)]),
            vertex(Vec3::new(0.1, 0.0, 0.9), &[(0, 0.5), (1, 0.5)]),
            vertex(Vec3::new(0.2, 0.0, 0.9), &[(0, 0.2), (1, 0.8)]),
        ],
        indices: vec![0, 1, 2],
        sections: Vec::new(),
    };
    let mesh = SkinnedMesh {
        name: "suit".into(),
        bone_map: vec![Some(1), Some(2)],
        inverse_bind: vec![
            Mat4::from_translation(Vec3::new(0.0, 0.0, -1.0)),
            Mat4::from_translation(Vec3::new(0.0, 0.0, -0.9)),
        ],
        materials: Vec::new(),
        lods: vec![lod],
    };
    Pack { skeleton, sockets: Vec::new(), clips: Vec::new(), meshes: vec![mesh] }
}

fn attach_with(seat: &str, surface: &str) -> Attach {
    let mut doc: serde_json::Value = serde_json::from_str(SAMPLE).unwrap();
    doc["seat"] = serde_json::from_str(seat).unwrap();
    doc["seat_surface"] = serde_json::from_str(surface).unwrap();
    Attach::parse(&serde_json::to_vec(&doc).unwrap()).unwrap()
}

#[test]
fn the_points_are_the_vertices_that_mostly_follow_the_seat_bones_in_the_anchors_frame() {
    let attach = attach_with(r#"{"bones": ["pelvis"], "sink": 0.02}"#, "[[-0.5, 0.8], [0.5, 0.8]]");
    let seat = Seat::new(&attach, &pack()).unwrap();
    // Vertex 0 (all pelvis) and vertex 2 (half pelvis) count; the thigh vertices 1 and 3 do not (3 is 20% pelvis).
    assert_eq!(seat.points.len(), 2);
    assert!(seat.points[0].distance(Vec3::new(0.0, 0.0, -0.12)) < 1e-6, "{:?}", seat.points[0]);
    assert!(seat.points[1].distance(Vec3::new(0.1, 0.0, -0.10)) < 1e-6);
    assert_eq!(seat.anchor, 1);
    assert_eq!(seat.sink, 0.02);
}

#[test]
fn a_bike_with_no_surface_or_a_file_with_no_seat_has_no_seat() {
    let seat = r#"{"bones": ["pelvis"], "sink": 0.02}"#;
    assert!(Seat::new(&attach_with(seat, "[]"), &pack()).is_none(), "no measured surface");
    assert!(Seat::new(&attach_with(seat, "[[0.0, 0.8]]"), &pack()).is_none(), "one sample is no surface");
    let nameless = attach_with(r#"{"bones": ["tail"], "sink": 0.02}"#, "[[-0.5, 0.8], [0.5, 0.8]]");
    assert!(Seat::new(&nameless, &pack()).is_none(), "a bone the rider does not have");
    let plain: Attach = Attach::parse(SAMPLE.as_bytes()).unwrap();
    assert!(plain.seat.is_none() && Seat::new(&plain, &pack()).is_none());
}
