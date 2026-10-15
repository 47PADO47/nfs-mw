use super::super::{follow::Follow, projection};
use super::{body, view};
use glam::Vec3;

#[test]
fn whole_native_hoodupside() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, -0.239_249_33_f32, -0.970_958_2_f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.300_760_57_f32, 0.287_099_2_f32, 1.165_149_8_f32)).length() < 3e-4,
        "frame 0 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.300_760_5_f32, 0.272_469_7_f32, 1.105_778_1_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.0f32, -0.239_249_33_f32, -0.970_958_2_f32)).length() < 3e-4,
        "frame 0 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.300_760_57_f32, 0.287_099_2_f32, 1.165_149_8_f32)).length() < 3e-4,
        "frame 1 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.300_760_5_f32, 0.272_469_7_f32, 1.105_778_1_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.0f32, -0.239_249_33_f32, -0.970_958_2_f32)).length() < 3e-4,
        "frame 1 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.300_760_57_f32, 0.287_099_2_f32, 1.165_149_8_f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.300_760_5_f32, 0.272_469_7_f32, 1.105_778_1_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.0f32, -0.239_249_33_f32, -0.970_958_2_f32)).length() < 3e-4,
        "frame 2 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.300_760_57_f32, 0.287_099_2_f32, 1.165_149_8_f32)).length() < 3e-4,
        "frame 3 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.300_760_5_f32, 0.272_469_7_f32, 1.105_778_1_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.0f32, -0.239_249_33_f32, -0.970_958_2_f32)).length() < 3e-4,
        "frame 3 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.300_760_57_f32, 0.287_099_2_f32, 1.165_149_8_f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.300_760_5_f32, 0.272_469_7_f32, 1.105_778_1_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.0f32, -0.239_249_33_f32, -0.970_958_2_f32)).length() < 3e-4,
        "frame 4 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.300_760_57_f32, 0.287_099_2_f32, 1.165_149_8_f32)).length() < 3e-4,
        "frame 5 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.300_760_5_f32, 0.272_469_7_f32, 1.105_778_1_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.0f32, -0.239_249_33_f32, -0.970_958_2_f32)).length() < 3e-4,
        "frame 5 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.300_760_57_f32, 0.287_099_2_f32, 1.165_149_8_f32)).length() < 3e-4,
        "frame 6 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.300_760_5_f32, 0.272_469_7_f32, 1.105_778_1_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.0f32, -0.239_249_33_f32, -0.970_958_2_f32)).length() < 3e-4,
        "frame 6 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.300_760_57_f32, 0.287_099_2_f32, 1.165_149_8_f32)).length() < 3e-4,
        "frame 7 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.300_760_5_f32, 0.272_469_7_f32, 1.105_778_1_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.0f32, -0.239_249_33_f32, -0.970_958_2_f32)).length() < 3e-4,
        "frame 7 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
}

#[test]
fn whole_native_outside_switch() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
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
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(2, &view(2), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.000_01_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 1 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_397_8_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(4, &view(4), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.000_036_2_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_393_1_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(5, &view(5), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.000_078_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 3 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_386_1_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(6, &view(6), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.000_134_5_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_377_3_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.000_212_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 5 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_365_6_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(2, &view(2), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.000_305_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 6 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_352_5_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.000_412_5_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 7 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_338_3_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
}
