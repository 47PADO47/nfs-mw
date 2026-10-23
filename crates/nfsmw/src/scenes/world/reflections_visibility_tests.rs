use super::*;
use blackbox_gfx::reflection::ALL_FACES;
use blackbox_gfx::{FrameParams, InstanceKey, MeshHandle, Projection};
use blackbox_scene::Aabb;
use glam::{Mat4, Vec3};

fn cameras(origin: Vec3) -> [visibility::Camera; 6] {
    let frame = FrameParams {
        view: Mat4::IDENTITY,
        projection: Projection::Identity,
        camera_position: origin,
        light_dir: Vec3::NEG_Z,
        clear_color: [0.0; 3],
        fog: None,
        scattering: None,
        surface_light: None,
        camera_cut: true,
        shadow_tuning: None,
    };
    std::array::from_fn(|face| {
        let f = blackbox_gfx::reflection::face_frame(origin, face, &frame);
        visibility::Camera {
            view_proj: f.view_proj(),
            position: origin.to_array(),
            forward: f.view.inverse().transform_vector3(Vec3::NEG_Z).to_array(),
            fov_y_radians: std::f32::consts::FRAC_PI_2,
            light_dir: [0.0, 0.0, -1.0],
        }
    })
}

fn placement(id: u32, at: Vec3) -> Placed {
    Placed {
        lods: [Some(MeshHandle::from_raw(1)), Some(MeshHandle::from_raw(2)), None, Some(MeshHandle::from_raw(4))],
        detailed: None,
        transform: Mat4::from_translation(at),
        bounds: Aabb { min: at - Vec3::splat(2.0), max: at + Vec3::splat(2.0) },
        radius: 2.0,
        flags: 0,
        group: 0,
        prop: None,
        prop_id: 0,
        key: InstanceKey::new(1, id),
        original_shadow: false,
        casts_sun_shadow: true,
        spawner: None,
        parts: None,
        curve_lamp: None,
        signal: None,
    }
}

fn scene() -> Vec<Placed> {
    let mut placed = Vec::new();
    for x in -12..=12 {
        for y in -12..=12 {
            let id = placed.len() as u32 + 1;
            let mut p = placement(id, Vec3::new(x as f32 * 20.0, y as f32 * 20.0, (id % 9) as f32 - 4.0));
            p.group = u16::from(id.is_multiple_of(19));
            p.flags = match id % 7 {
                0 => blackbox_scenery::INSTANCE_TURNABLE,
                _ => 0,
            };
            placed.push(p);
        }
    }
    placed
}

fn digest(face: &[Instance]) -> Vec<(usize, InstanceKey, [f32; 16])> {
    let mut values: Vec<_> = face.iter().map(|i| (i.mesh.raw(), i.key, i.transform.to_cols_array())).collect();
    values.sort_by_key(|i| (i.0, i.1));
    values
}

fn reference(
    placed: &[Placed],
    cameras: &[visibility::Camera; 6],
    props: &PropWorld,
    height: f32,
) -> [Vec<Instance>; 6] {
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    std::array::from_fn(|face| {
        let mut out = Vec::new();
        visibility::collect_for_height(placed.iter(), &cameras[face], rules, props, height, &mut out);
        out
    })
}

#[test]
fn six_face_selection_matches_the_previous_collectors_at_every_capture_size() {
    let placed = scene();
    let props = PropWorld::default();
    let mut scratch = Scratch::default();
    let mut faces = std::array::from_fn(|_| Vec::new());
    let mut selected_instances = 0;
    for height in [16.0, 64.0, 128.0, 256.0, 480.0, 512.0, 1024.0] {
        for origin in [Vec3::ZERO, Vec3::new(45.0, -19.0, 4.0), Vec3::new(-300.0, 220.0, -10.0)] {
            let cameras = cameras(origin);
            collect(
                placed.iter(),
                &cameras,
                &blackbox_scenery::layout::MOST_WANTED.lod,
                &props,
                height,
                None,
                ALL_FACES,
                &mut faces,
                &mut scratch,
            );
            let expected = reference(&placed, &cameras, &props, height);
            selected_instances += faces.iter().map(Vec::len).sum::<usize>();
            for face in 0..6 {
                assert_eq!(
                    digest(&faces[face]),
                    digest(&expected[face]),
                    "height {height}, face {face}, origin {origin}"
                );
            }
        }
    }
    assert!(selected_instances > 100, "the comparison must include visible geometry as well as fully culled views");
}

#[test]
fn the_next_capture_uses_current_geometry_and_traverses_each_object_once() {
    let mut placed = [placement(1, Vec3::X * 10.0)];
    let props = PropWorld::default();
    let cameras = cameras(Vec3::ZERO);
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let mut faces = std::array::from_fn(|_| Vec::new());
    let mut scratch = Scratch::default();
    let mut visits = 0;
    collect(
        placed.iter().inspect(|_| visits += 1),
        &cameras,
        rules,
        &props,
        480.0,
        None,
        ALL_FACES,
        &mut faces,
        &mut scratch,
    );
    assert_eq!(visits, 1);
    let before = faces.each_ref().map(|face| digest(face));
    placed[0] = placement(1, -Vec3::X * 10.0);
    collect(placed.iter(), &cameras, rules, &props, 480.0, None, ALL_FACES, &mut faces, &mut scratch);
    assert_ne!(faces.each_ref().map(|face| digest(face)), before, "no prior-frame visibility is reused");
    for (actual, expected) in faces.iter().zip(reference(&placed, &cameras, &props, 480.0)) {
        assert_eq!(digest(actual), digest(&expected));
    }
}

#[test]
fn hidden_props_are_removed_from_all_faces_on_the_next_frame() {
    let mut placed = [placement(1, Vec3::X * 10.0)];
    placed[0].prop_id = 77;
    let mut props = PropWorld::default();
    let cameras = cameras(Vec3::ZERO);
    let mut faces = std::array::from_fn(|_| Vec::new());
    let mut scratch = Scratch::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    collect(placed.iter(), &cameras, rules, &props, 480.0, None, ALL_FACES, &mut faces, &mut scratch);
    assert!(faces.iter().any(|f| !f.is_empty()));
    props.knock(77);
    collect(placed.iter(), &cameras, rules, &props, 480.0, None, ALL_FACES, &mut faces, &mut scratch);
    assert!(faces.iter().all(Vec::is_empty));
}

#[test]
fn only_the_drawn_faces_are_selected_and_the_others_keep_their_lists() {
    let placed = scene();
    let props = PropWorld::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let cameras = cameras(Vec3::new(45.0, -19.0, 4.0));
    let mut scratch = Scratch::default();
    let mut all = std::array::from_fn(|_| Vec::new());
    collect(placed.iter(), &cameras, rules, &props, 480.0, None, ALL_FACES, &mut all, &mut scratch);
    assert!(all.iter().filter(|face| !face.is_empty()).count() >= 2, "the comparison needs several visible faces");

    let marker = Instance::keyed(MeshHandle::from_raw(9_999), Mat4::IDENTITY, InstanceKey::new(9, 9));
    let mut faces: [Vec<Instance>; 6] = std::array::from_fn(|_| vec![marker]);
    let drawn = 0b00_0101;
    collect(placed.iter(), &cameras, rules, &props, 480.0, None, drawn, &mut faces, &mut scratch);
    for face in 0..6 {
        match (drawn >> face) & 1 {
            1 => assert_eq!(digest(&faces[face]), digest(&all[face]), "face {face} is selected as in a full capture"),
            _ => assert_eq!(digest(&faces[face]), digest(&[marker]), "face {face} keeps its previous list"),
        }
    }
}

#[test]
#[ignore = "CPU benchmark; run explicitly with --nocapture"]
fn benchmark_six_face_scenery_selection() {
    use std::{hint::black_box, time::Instant};
    let placed: Vec<_> = (0..12).flat_map(|_| scene()).collect();
    let cameras = cameras(Vec3::ZERO);
    let props = PropWorld::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let mut faces = std::array::from_fn(|_| Vec::new());
    let mut scratch = Scratch::default();
    let mut legacy: [Vec<Instance>; 6] = std::array::from_fn(|_| Vec::new());
    let repeats = 100;
    let start = Instant::now();
    for _ in 0..repeats {
        for face in 0..6 {
            visibility::collect_for_height(
                black_box(placed.iter()),
                &cameras[face],
                rules,
                &props,
                1024.0,
                &mut legacy[face],
            );
            visibility::sort_by_mesh(&mut legacy[face]);
        }
        black_box(&legacy);
    }
    let old = start.elapsed();
    let start = Instant::now();
    for _ in 0..repeats {
        collect(black_box(placed.iter()), &cameras, rules, &props, 1024.0, None, ALL_FACES, &mut faces, &mut scratch);
        for face in &mut faces {
            visibility::sort_by_mesh(face);
        }
        black_box(&faces);
    }
    let new = start.elapsed();
    for face in 0..6 {
        assert_eq!(digest(&legacy[face]), digest(&faces[face]));
    }
    eprintln!(
        "reflection selection {} objects: old {:.3} ms, new {:.3} ms per capture; {:.2}x",
        placed.len(),
        old.as_secs_f64() * 1000.0 / f64::from(repeats),
        new.as_secs_f64() * 1000.0 / f64::from(repeats),
        old.as_secs_f64() / new.as_secs_f64()
    );
}

#[test]
fn distance_tests_current_bounds_instead_of_discarding_a_near_wall_by_its_origin() {
    let mut wall = placement(1, Vec3::new(110.0, 0.0, 0.0));
    wall.bounds = Aabb::new([90.0, -20.0, -5.0], [130.0, 20.0, 10.0]);
    wall.radius = 30.0;
    let cameras = cameras(Vec3::ZERO);
    let props = PropWorld::default();
    let mut faces = std::array::from_fn(|_| Vec::new());
    let mut scratch = Scratch::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    collect([&wall].into_iter(), &cameras, rules, &props, 1024.0, Some(100.0), ALL_FACES, &mut faces, &mut scratch);
    assert!(faces.iter().any(|face| !face.is_empty()));
    collect([&wall].into_iter(), &cameras, rules, &props, 1024.0, Some(80.0), ALL_FACES, &mut faces, &mut scratch);
    assert!(faces.iter().all(Vec::is_empty));
}
