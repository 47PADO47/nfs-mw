use glam::Vec3;

use crate::SceneType;
use crate::placement::{render_to_world, scene_to_world, world_to_scene};

const QUARTER_TURN: u16 = 0x4000;

fn near(actual: Vec3, expected: Vec3) {
    assert!((actual - expected).length() < 1e-5, "{actual} is not {expected}");
}

#[test]
fn the_render_axes_map_to_the_world_axes() {
    let map = render_to_world();
    near(map.transform_point3(Vec3::X), Vec3::Z);
    near(map.transform_point3(Vec3::Y), Vec3::NEG_X);
    near(map.transform_point3(Vec3::Z), Vec3::Y);
}

#[test]
fn type_0_with_a_quarter_turn_heading_has_no_turn_and_no_shift() {
    // The turn is the heading minus a quarter turn, so a quarter-turn heading leaves the scene unturned.
    let map = scene_to_world(SceneType::Type0, Vec3::ZERO, QUARTER_TURN);
    near(map.transform_point3(Vec3::X), render_to_world().transform_point3(Vec3::X));
}

#[test]
fn type_0_shifts_the_scene_to_the_marker_origin() {
    // The origin is a render-frame point (10, 20, 30); render (10, 20, 30) is world (-20, 30, 10).
    let origin = Vec3::new(10.0, 20.0, 30.0);
    let map = scene_to_world(SceneType::Type0, origin, QUARTER_TURN);
    near(map.transform_point3(Vec3::ZERO), Vec3::new(-20.0, 30.0, 10.0));
}

#[test]
fn type_0_turns_by_the_heading_minus_a_quarter_turn() {
    // Heading 0 is a quarter turn back: the scene's +x is turned to render (0, -1, 0), which is world (1, 0, 0).
    let map = scene_to_world(SceneType::Type0, Vec3::ZERO, 0);
    near(map.transform_point3(Vec3::X), Vec3::X);
}

#[test]
fn type_4_ignores_the_marker_and_is_the_render_to_world_map() {
    let marker = Vec3::new(10.0, 20.0, 30.0);
    let point = Vec3::new(1.0, 2.0, 3.0);
    let map = scene_to_world(SceneType::Type4, marker, 1234);
    near(map.transform_point3(point), Vec3::new(-2.0, 3.0, 1.0));
    near(map.transform_point3(point), render_to_world().transform_point3(point));
}

#[test]
fn type_1_is_placed_like_type_4() {
    let marker = Vec3::new(5.0, -5.0, 2.0);
    let as_one = scene_to_world(SceneType::Type1, marker, 999);
    let as_four = scene_to_world(SceneType::Type4, marker, 999);
    assert_eq!(as_one, as_four);
}

#[test]
fn the_inverse_takes_world_points_back_to_the_scene() {
    let origin = Vec3::new(3.0, -7.0, 0.5);
    let point = Vec3::new(1.2, -0.4, 2.0);
    for scene_type in [SceneType::Type0, SceneType::Type2, SceneType::Type3, SceneType::Type4] {
        let world = scene_to_world(scene_type, origin, 0x1234).transform_point3(point);
        let back = world_to_scene(scene_type, origin, 0x1234).transform_point3(world);
        near(back, point);
    }
}
