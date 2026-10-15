use super::super::{follow::Follow, projection};
use super::{body, view};
use glam::Vec3;

#[test]
fn whole_native_reverse() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(-30.0f32, 0.0f32, 0.0f32);
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
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.000_063_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 1 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_388_3_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.000_248_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_356_8_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.000_547_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 3 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_308_7_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.000_952_7_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_247_8_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.001_457_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 5 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_177_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.002_053_7_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 6 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_098_8_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.002_735_6_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 7 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_015_6_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
}

#[test]
fn whole_native_highspeed() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(100.0f32, 0.0f32, 0.0f32);
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
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.001_751_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 1 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_166_7_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.006_824_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 2 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.868_528_5_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.014_918_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 3 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_578_7_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.025_749_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 4 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.866_398_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.039_051_5_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 5 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.865_054_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.054_574_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 6 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.863_603_6_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.072_080_6_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 7 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.862_094_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
}

#[test]
fn whole_native_zoom() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    body.zoom = 0.6f32;
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
    assert!((actual.eye - Vec3::new(-6.003_162_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 1 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.868_831_4_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15282.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.012_410_6_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_250_2_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15258.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.027_350_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 3 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.864_851_6_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15222.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.047_602_7_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.861_807_5_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15168.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.072_803_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 5 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.858_267_8_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15106.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.102_604_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 6 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.854_362_6_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15036.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.136_671_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 7 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.850_203_9_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 14954.0f32, "frame 7 fov");
}
