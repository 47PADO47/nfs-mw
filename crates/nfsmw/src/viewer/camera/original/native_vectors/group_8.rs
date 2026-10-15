use super::super::{follow::Follow, projection};
use super::{body, view};
use glam::Vec3;

#[test]
fn whole_native_gear_slow() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(10.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    body.gear = 2;
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 1 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    body.velocity = Vec3::new(10.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    body.gear = 3;
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 2 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 3 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 4 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 5 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 6 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 7 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 8 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 8 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 8 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 8 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 9 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 9 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 9 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 9 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 10 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 10 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 10 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 10 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 11 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 11 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 11 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 11 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 12 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 12 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 12 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 12 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 13 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 13 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 13 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 13 fov");
}

#[test]
fn whole_native_gear_inside() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    body.gear = 2;
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 0 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 1 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    body.gear = 3;
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 3 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 5 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 6 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 7 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 8 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 8 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 8 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 8 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 9 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 9 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 9 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 9 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 10 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 10 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 10 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 10 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 11 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 11 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 11 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 11 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 12 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 12 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 12 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 12 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.304_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 13 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.304_140_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 13 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 13 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 13 fov");
}
