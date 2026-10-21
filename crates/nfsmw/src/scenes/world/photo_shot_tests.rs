use blackbox_chunk::ice::Key;

use super::*;

fn key(time: f32, eye: [[f32; 3]; 2], look: [[f32; 3]; 2], space: Space) -> Key {
    Key {
        kind: 12,
        smooth: false,
        cubic_eye: false,
        cubic_look: false,
        space_eye: space,
        space_look: space,
        time,
        tangent_length: [1.0, 1.0],
        eye,
        look,
        dutch: [0.0, 0.0],
        lens: [24.0, 24.0],
        near_clip: [-1.0, -1.0],
        aperture: [0, 0],
        letterbox: [0, 0],
        sim_speed: [100, 100],
    }
}

/// `SprintFinish` as the install has it, in car space: the camera moves in from the front-right and holds there.
fn sprint_finish() -> Track {
    Track {
        name: "SprintFinish".into(),
        start: 0.0,
        length: 3.0,
        keys: vec![
            key(
                0.0,
                [[1.49, -1.35, 1.69], [3.98, -2.41, 0.89]],
                [[1.04, -1.15, 1.58], [3.55, -2.16, 0.87]],
                Space::Car,
            ),
            key(0.212, [[3.98, -2.41, 0.89]; 2], [[3.55, -2.16, 0.87]; 2], Space::Car),
        ],
    }
}

#[test]
fn a_car_space_point_follows_the_car() {
    // The car stands at (10, 20, 0) turned a quarter turn to the left, so its nose points along +y.
    let car = Mat4::from_rotation_translation(
        glam::Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
        Vec3::new(10.0, 20.0, 0.0),
    );
    let ahead = place(&car, Space::Car, [4.0, 0.0, 1.0]).unwrap();
    assert!((ahead - Vec3::new(10.0, 24.0, 1.0)).length() < 1e-4, "{ahead}");
    let left = place(&car, Space::Car, [0.0, 2.0, 0.0]).unwrap();
    assert!((left - Vec3::new(8.0, 20.0, 0.0)).length() < 1e-4, "+y is the car's left: {left}");
}

#[test]
fn hybrid_and_world_points_keep_the_worlds_axes() {
    let car = Mat4::from_rotation_translation(glam::Quat::from_rotation_z(1.0), Vec3::new(10.0, 20.0, 0.0));
    assert_eq!(place(&car, Space::Hybrid, [1.0, 2.0, 3.0]), Some(Vec3::new(11.0, 22.0, 3.0)));
    assert_eq!(place(&car, Space::World, [1.0, 2.0, 3.0]), Some(Vec3::new(1.0, 2.0, 3.0)));
    assert_eq!(place(&car, Space::Scene, [1.0, 2.0, 3.0]), None, "a replay camera has no scene to stand in");
}

#[test]
fn the_photo_is_the_camera_at_the_end_of_its_track() {
    let shot = PhotoShot::new(sprint_finish(), Mat4::IDENTITY);
    let (eye, target, up, _) = shot.at_rest().unwrap();
    assert!((eye - Vec3::new(3.98, -2.41, 0.89)).length() < 1e-4, "the held key, not the cut's start: {eye}");
    assert!((target - Vec3::new(3.55, -2.16, 0.87)).length() < 1e-4);
    assert!((up - Vec3::Z).length() < 0.1);
}

#[test]
fn the_camera_in_front_of_the_car_looks_back_at_it() {
    let car = Mat4::from_translation(Vec3::new(100.0, 50.0, 10.0));
    let shot = PhotoShot::new(sprint_finish(), car);
    let (eye, target, _, _) = shot.at_rest().unwrap();
    let nose = Vec3::new(100.0, 50.0, 10.0);
    assert!(eye.x > nose.x + 3.0, "the eye is ahead of the car: {eye}");
    let toward_car = (nose - eye).normalize();
    let looking = (target - eye).normalize();
    assert!(
        looking.dot(toward_car) > 0.97,
        "the look direction heads for the car, not past it: {looking} vs {toward_car}"
    );
}

#[test]
fn a_track_with_no_key_gives_no_camera() {
    let shot =
        PhotoShot::new(Track { name: "empty".into(), start: 0.0, length: 1.0, keys: Vec::new() }, Mat4::IDENTITY);
    assert!(shot.at_rest().is_none());
    assert!(shot.lens(16.0 / 9.0).is_none());
}

#[test]
fn the_lens_stands_at_the_eye() {
    let car = Mat4::from_translation(Vec3::new(100.0, 50.0, 10.0));
    let shot = PhotoShot::new(sprint_finish(), car);
    let lens = shot.lens(16.0 / 9.0).unwrap();
    let (eye, ..) = shot.at_rest().unwrap();
    assert!((lens.position - eye).length() < 0.05, "{} vs {eye}", lens.position);
}
