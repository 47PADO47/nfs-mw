use super::super::Camera;
use super::super::chase::{ChaseCamera, Followed};
use super::*;

fn view(fov: f32, height: f32, lag: f32) -> View {
    View {
        fov: [fov; 2],
        height: [height; 2],
        lag: [lag; 2],
        angle: [5.0; 2],
        stiffness: [0.5; 2],
        lat_offset: [0.0; 2],
        tilting: [true; 2],
        selectable: [true; 2],
    }
}

/// A car's seven views in the order bumper, hood, close, far, superfar, drift, pursuit.
fn car_views() -> Views {
    let lags = [0.5, 0.0, -5.0, -6.5, -9.0, -7.0, -6.0];
    let heights = [0.9, 1.1, 1.5, 1.8, 2.6, 1.9, 1.7];
    Views { views: std::array::from_fn(|i| view(90.0, heights[i], lags[i])) }
}

fn rig(wheelbase: f32) -> BikeCamera {
    BikeCamera::derived(wheelbase, wheelbase * 0.5)
}

const LEVEL: Attitude = Attitude { yaw: 0.0, pitch: 0.0, roll: 0.0 };

#[test]
fn the_chase_views_scale_with_the_length_of_the_bike() {
    let (small, long) = (views(&rig(1.2), 0.6, &car_views()), views(&rig(1.8), 0.6, &car_views()));
    for index in 2..7 {
        let (a, b) = (small.views[index], long.views[index]);
        assert!(((-b.lag[0]) / (-a.lag[0]) - 1.5).abs() < 1e-4, "view {index}: {a:?} {b:?}");
    }
    let (small, long) = (chase_fit(&rig(1.2)), chase_fit(&rig(1.8)));
    assert!((long.distance / small.distance - 1.5).abs() < 1e-4);
    assert!((long.look_height / small.look_height - 1.5).abs() < 1e-4);
}

#[test]
fn the_views_keep_the_order_of_the_cars_and_sit_at_the_bikes_size_not_the_cars() {
    let rig = rig(1.45);
    let out = views(&rig, 0.67, &car_views());
    let behind = |index: usize| -out.views[index].lag[0];
    assert!((behind(2) - rig.chase_near.distance).abs() < 1e-4 && (behind(3) - rig.chase_far.distance).abs() < 1e-4);
    assert!(behind(2) < behind(3) && behind(3) < behind(4), "close, far, superfar");
    assert!(behind(3) < 6.5, "far is nearer than a car's far view: {}", behind(3));
    // The camera height above the ground is the chase's own, whatever the anchor's height.
    assert!((out.views[3].height[0] + 0.67 - rig.chase_far.height).abs() < 1e-4);
    // The controller's other numbers are the car's.
    assert_eq!(out.views[3].stiffness, car_views().views[3].stiffness);
}

#[test]
fn the_chase_views_look_down_at_the_bike_from_above_its_look_point() {
    let rig = rig(1.45);
    for view in &views(&rig, 0.67, &car_views()).views[2..] {
        assert!(view.angle[0] > 0.0 && view.angle[0] < 30.0, "{view:?}");
    }
}

#[test]
fn a_field_of_view_given_at_16_by_9_becomes_the_4_by_3_number_of_the_original_optics() {
    assert!((original_fov(85.56559) - 69.53).abs() < 0.1, "{}", original_fov(85.56559));
    assert!(original_fov(90.0) < 90.0);
    assert!((vertical_fov(91.49284) - 60.0).abs() < 1e-3, "{}", vertical_fov(91.49284));
}

#[test]
fn the_attitude_reads_heading_pitch_and_roll_of_a_rotation() {
    let turned = Quat::from_rotation_z(0.5) * Quat::from_rotation_y(-0.2) * Quat::from_rotation_x(0.3);
    let attitude = Attitude::of(turned);
    assert!((attitude.yaw - 0.5).abs() < 1e-4, "{attitude:?}");
    assert!((attitude.pitch - 0.2).abs() < 1e-4, "nose up is positive: {attitude:?}");
    assert!((attitude.roll - 0.3).abs() < 1e-2, "leaning right is positive: {attitude:?}");
    assert_eq!(Attitude::of(Quat::IDENTITY), LEVEL);
}

fn helmet_mount() -> Mount {
    Mount {
        look_back: false,
        pitch_deg: -10.0,
        fov: 91.5,
        near: 0.1,
        pitch_factor: 0.25,
        roll_factor: 0.0,
        follow_roll_max_deg: 5.0,
    }
}

#[test]
fn the_helmet_camera_follows_the_lean_only_up_to_its_limit_and_the_front_camera_by_its_share() {
    let helmet = helmet_mount();
    assert_eq!(helmet.roll(0.0), 0.0);
    assert!((helmet.roll(0.05) - 0.05).abs() < 1e-6, "a small lean is followed whole");
    assert!((helmet.roll(0.8) - 5f32.to_radians()).abs() < 1e-6, "a hard lean is followed to the limit");
    assert!((helmet.roll(-0.8) + 5f32.to_radians()).abs() < 1e-6);
    let front = Mount { roll_factor: 0.8, follow_roll_max_deg: 0.0, ..helmet };
    assert!((front.roll(0.5) - 0.4).abs() < 1e-6);
}

#[test]
fn the_helmet_camera_is_at_the_eye_and_looks_ahead_and_down_by_its_tilt() {
    let eye = Vec3::new(10.0, 20.0, 1.3);
    let lens = lens(eye, LEVEL, &helmet_mount(), 16.0 / 9.0);
    assert_eq!(lens.position, eye);
    assert!(lens.forward.x > 0.98 && lens.forward.z < 0.0);
    assert!((lens.forward.z.asin().to_degrees() + 10.0).abs() < 1e-3);
    assert!((lens.fov_y_degrees - vertical_fov(91.5)).abs() < 1e-4);
}

#[test]
fn a_wheelie_tips_the_helmet_camera_up_by_its_share_of_the_pitch() {
    let level = lens(Vec3::ZERO, LEVEL, &helmet_mount(), 1.0).forward;
    let wheelie = Attitude { pitch: 0.8, ..LEVEL };
    let up = lens(Vec3::ZERO, wheelie, &helmet_mount(), 1.0).forward;
    assert!((up.z.asin() - level.z.asin() - 0.2).abs() < 1e-4, "a quarter of 0.8 rad");
}

#[test]
fn a_camera_that_looks_back_looks_along_the_bike_the_other_way_and_the_heading_turns_it() {
    let mount = Mount { look_back: true, pitch_deg: 0.0, ..helmet_mount() };
    let at = |yaw: f32| lens(Vec3::ZERO, Attitude { yaw, ..LEVEL }, &mount, 1.0).forward;
    assert!(at(0.0).distance(-Vec3::X) < 1e-5);
    assert!(at(std::f32::consts::FRAC_PI_2).distance(-Vec3::Y) < 1e-5);
    let front = Front { look_back: true, ..rig(1.45).front };
    assert!(!Mount::front(&front, true).look_back, "the look-back control turns it round");
    assert!(Mount::front(&front, false).look_back);
}

#[test]
fn the_front_camera_is_ahead_of_the_front_wheel_and_looks_back_at_the_bike() {
    let rig = rig(1.45);
    let hub = 1.45 * 0.5;
    assert!(rig.front.position[0] > hub + 0.3, "ahead of the front tyre: {:?}", rig.front.position);
    let mount = Mount::front(&rig.front, false);
    let eye = Vec3::from_array(rig.front.position);
    let shot = lens(eye, LEVEL, &mount, 16.0 / 9.0);
    assert!(shot.forward.x < -0.9, "it looks at the bike: {:?}", shot.forward);
    // The bike's middle is in front of the lens.
    let to_bike = (Vec3::new(0.0, 0.0, 0.7) - shot.position).normalize();
    assert!(shot.forward.dot(to_bike) > 0.9);
}

#[test]
fn a_bike_rolled_to_the_right_tips_a_bolted_camera_up_to_the_right() {
    let mount = Mount { roll_factor: 1.0, follow_roll_max_deg: 0.0, pitch_deg: 0.0, ..helmet_mount() };
    let shot = lens(Vec3::ZERO, Attitude { roll: 0.5, ..LEVEL }, &mount, 1.0);
    // The camera's up in the world: tipped to the right (-y) when the bike leans right.
    let up = shot.view.inverse().transform_vector3(Vec3::Y);
    assert!(up.y < -0.3 && up.z > 0.8, "{up:?}");
}

/// A chase camera with `fit` that has followed a stopped vehicle at (100, 50, 0) heading along +x for one frame.
fn chase_behind_a_stopped_vehicle(fit: Fit) -> ChaseCamera {
    let mut camera = ChaseCamera::default();
    camera.fit = fit;
    camera
        .update(Followed { position: Vec3::new(100.0, 50.0, 0.0), heading: 0.0, speed: 0.0 }, 1.0 / 60.0, |_, _| None);
    camera
}

#[test]
fn a_car_keeps_the_chase_camera_it_had() {
    let camera = chase_behind_a_stopped_vehicle(Fit::default());
    assert!(
        (camera.position.x - (100.0 - 5.6)).abs() < 1e-4 && (camera.position.z - 1.9).abs() < 1e-4,
        "{:?}",
        camera.position
    );
    assert!(camera.forward().x > 0.9);
}

#[test]
fn the_chase_camera_of_a_bike_sits_at_the_near_chases_distance_and_height_and_looks_at_its_target() {
    let rig = rig(1.45);
    let camera = chase_behind_a_stopped_vehicle(chase_fit(&rig));
    assert!((camera.position.x - (100.0 - rig.chase_near.distance)).abs() < 1e-3, "{:?}", camera.position);
    assert!((camera.position.z - rig.chase_near.height).abs() < 1e-3, "{:?}", camera.position);
    // The line of sight crosses the vehicle's own place at the look height: it looks at the bike, not above it.
    let sight = camera.lens(16.0 / 9.0).forward;
    let height_at_the_bike = camera.position.z + sight.z * (100.0 - camera.position.x) / sight.x;
    assert!((height_at_the_bike - rig.chase_near.target_height).abs() < 1e-3, "{height_at_the_bike}");
    assert!(camera.position.x > 100.0 - 5.6, "closer than a car's camera");
}

#[test]
fn the_obstacle_clearance_of_the_chase_camera_scales_with_the_fit() {
    let fit = chase_fit(&rig(1.2));
    let mut camera = ChaseCamera::default();
    camera.fit = fit;
    // Something stands right behind the vehicle: the camera comes in no closer than the fitted minimum, not a car's 2.8 m.
    camera.update(Followed { position: Vec3::ZERO, heading: 0.0, speed: 0.0 }, 1.0, |_, _| Some(0.0));
    let behind = -camera.position.x;
    assert!(behind < 2.8 && behind > 1.0, "{behind}");
}
