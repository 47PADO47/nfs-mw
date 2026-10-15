use super::super::{follow::Follow, projection};
use super::{body, view};
use glam::Vec3;

#[test]
fn whole_native_startup_0() {
    let mut follow = Follow::default();
    let mut body = body();
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(0, &view(0), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(0.3_f32, 0.0f32, 1.2_f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.3_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
}

#[test]
fn whole_native_startup_1() {
    let mut follow = Follow::default();
    let mut body = body();
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(0.3_f32, 0.0f32, 1.2_f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.3_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
}

#[test]
fn whole_native_startup_2() {
    let mut follow = Follow::default();
    let mut body = body();
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(2, &view(2), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
}

#[test]
fn whole_native_startup_3() {
    let mut follow = Follow::default();
    let mut body = body();
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
}

#[test]
fn whole_native_startup_4() {
    let mut follow = Follow::default();
    let mut body = body();
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(4, &view(4), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
}

#[test]
fn whole_native_startup_5() {
    let mut follow = Follow::default();
    let mut body = body();
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(5, &view(5), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
}

#[test]
fn whole_native_startup_6() {
    let mut follow = Follow::default();
    let mut body = body();
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(6, &view(6), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
}

#[test]
fn whole_native_turn() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(20.0f32, 10.0f32, 0.0f32);
    body.forward = Vec3::new(0.877_582_55_f32, 0.479_425_55_f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-5.999_991_f32, -0.019_052_8_f32, 1.999_999_9_f32)).length() < 3e-4,
        "frame 1 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_395_5_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-5.999_653_f32, -0.071_958_66_f32, 1.999_999_9_f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_384_3_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-5.998_258_f32, -0.152_103_98_f32, 1.999_999_9_f32)).length() < 3e-4,
        "frame 3 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_367_1_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-5.994_957_f32, -0.253_764_03_f32, 2.0f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_345_4_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-5.988_960_7_f32, -0.371_916_74_f32, 2.0f32)).length() < 3e-4,
        "frame 5 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_32_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-5.979_657_6_f32, -0.502_123_f32, 2.0f32)).length() < 3e-4,
        "frame 6 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_291_9_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-5.966_662_f32, -0.640_458_05_f32, 2.0f32)).length() < 3e-4,
        "frame 7 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_261_9_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
}
