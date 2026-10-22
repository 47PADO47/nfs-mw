use glam::Vec3;

use super::*;

fn angle(pitch: f32, yaw: f32, distance: f32, roll: f32, fov: f32, target: [f32; 3]) -> CameraAngle {
    CameraAngle { pitch, yaw, distance, roll, fov, target, decay: 9.0, duration: 3.15, order: 1 }
}

/// The safe house's entry and final poses (`FE_ATTRIB.bin`, `docs/specs/menu-backdrop-looks.md` section 4.3).
fn safe_house() -> CameraMove {
    let entry = angle(94.4, -58.0, 9.0, -13.1, 55.0, [2.3, -0.3, 1.0]);
    let goal = angle(97.3, 110.0, 4.25, 16.0, 65.0, [-0.2, -0.5, 0.3]);
    CameraMove::between(&entry, &goal)
}

fn at(mut camera: CameraMove, seconds: f32) -> CameraPose {
    camera.advance(seconds);
    camera.pose()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

#[test]
fn the_move_starts_on_the_start_pose() {
    let pose = safe_house().pose();
    assert!(near(pose.pitch, 94.4) && near(pose.distance, 9.0) && near(pose.fov, 55.0), "{pose:?}");
    assert!(near(pose.yaw, 302.0), "yaw -58 is 302 once wrapped: {pose:?}");
    assert!(near(pose.roll, 346.9), "{pose:?}");
}

#[test]
fn the_move_ends_on_the_goal_and_stays_there() {
    for seconds in [3.15, 4.0, 60.0] {
        let pose = at(safe_house(), seconds);
        assert_eq!(pose, CameraPose { target: pose.target, ..crate::frontend::camera_pose::SAFE_HOUSE }, "{seconds}");
        assert!((pose.target - Vec3::new(-0.2, -0.5, 0.3)).length() < 1e-5);
    }
}

#[test]
fn the_curve_is_the_decaying_swing_of_the_original() {
    // k = 9, n = 1, t = 0.5: 1 - e^-2.25 cos(0.375 pi).
    let s = safe_house().curve(0.5);
    assert!((s - 0.959_667).abs() < 1e-4, "{s}");
    assert!(near(safe_house().curve(0.0), 0.0));
    // An order of one swings past the goal once: the curve rises above 1 before it settles.
    let peak = (1..100).map(|i| safe_house().curve(i as f32 / 100.0)).fold(f32::MIN, f32::max);
    assert!(peak > 1.0 && peak < 1.1, "{peak}");
}

#[test]
fn every_value_follows_the_curve() {
    // Half way through the 3.15 s, s is the curve at t = 0.5; the field of view and the target blend with it too.
    let s = safe_house().curve(0.5);
    let pose = at(safe_house(), 3.15 / 2.0);
    assert!(near(pose.fov, 55.0 + 10.0 * s), "{pose:?}");
    assert!(near(pose.distance, 9.0 + (4.25 - 9.0) * s), "{pose:?}");
    assert!(near(pose.pitch, 94.4 + (97.3 - 94.4) * s), "{pose:?}");
    assert!(near(pose.target.x, 2.3 + (-0.2 - 2.3) * s), "{pose:?}");
}

#[test]
fn a_move_takes_the_short_way_round_a_turn() {
    let from = angle(90.0, 350.0, 5.0, 0.0, 55.0, [0.0; 3]);
    let to = angle(90.0, 10.0, 5.0, 0.0, 55.0, [0.0; 3]);
    let camera = CameraMove::between(&from, &to);
    // The goal becomes 370, so half way through (s = the curve at 0.5) the yaw is 350 + 20 s, wrapped past 360.
    let s = camera.curve(0.5);
    let mid = at(camera, 3.15 / 2.0);
    assert!(near(mid.yaw, wrap_turn(350.0 + 20.0 * s)), "{mid:?}");
    assert!(mid.yaw < 30.0 || mid.yaw > 340.0, "the yaw stays near north, not half a turn away: {mid:?}");
}

#[test]
fn a_retarget_starts_from_where_the_camera_is() {
    let mut camera = safe_house();
    camera.advance(1.0);
    let before = camera.pose();
    let back = angle(94.4, -58.0, 9.0, -13.1, 55.0, [2.3, -0.3, 1.0]);
    camera.retarget(&back);
    assert_eq!(camera.pose(), before, "the new move begins on the pose the old one reached");
    camera.advance(60.0);
    let end = camera.pose();
    assert!(
        near(end.distance, 9.0) && near(end.fov, 55.0) && near(end.yaw, 302.0),
        "and ends on the new goal: {end:?}"
    );
}

#[test]
fn the_nearest_turn_picks_the_copy_closest_to_the_start() {
    assert!(near(nearest_turn(350.0, 10.0), 370.0));
    assert!(near(nearest_turn(10.0, 350.0), -10.0));
    assert!(near(nearest_turn(-58.0, 110.0), 110.0));
    assert!(near(nearest_turn(100.0, 110.0), 110.0));
}
