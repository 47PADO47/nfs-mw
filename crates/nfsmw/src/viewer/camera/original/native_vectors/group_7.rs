use super::super::{follow::Follow, projection};
use super::{body, view};
use glam::Vec3;

#[test]
fn whole_native_vertical_stock() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.acceleration = Vec3::new(0.0f32, 0.0f32, 30.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
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
    assert!(
        (actual.eye - Vec3::new(-6.003_309_2_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 1 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.868_893_3_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.011_384_5_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.868_165_5_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.021_694_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 3 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_755_7_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.032_552_2_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_604_3_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.042_906_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 5 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_573_5_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.052_164_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 6 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_579_6_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.060_057_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 7 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_589_2_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.066_527_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 8 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_594_7_f32)).length() < 3e-4,
        "frame 8 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 8 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 8 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.071_65_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 9 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_596_9_f32)).length() < 3e-4,
        "frame 9 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 9 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 9 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.075_573_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 10 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_597_3_f32)).length() < 3e-4,
        "frame 10 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 10 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 10 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.078_478_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 11 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_597_2_f32)).length() < 3e-4,
        "frame 11 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 11 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 11 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.080_549_2_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 12 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_597_1_f32)).length() < 3e-4,
        "frame 12 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 12 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 12 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.081_963_5_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 13 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_597_f32)).length() < 3e-4,
        "frame 13 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 13 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 13 fov");
}

#[test]
fn whole_native_gear() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
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
    assert!(
        (actual.eye - Vec3::new(-5.891_247_3_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 1 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.868_893_3_f32)).length() < 3e-4,
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
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-5.862_703_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 2 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.868_165_5_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-5.912_099_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 3 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_755_7_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.040_998_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 4 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_604_3_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.144_427_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 5 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_573_5_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.221_625_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 6 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_579_6_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.190_146_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 7 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_589_2_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.159_951_7_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 8 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_594_7_f32)).length() < 3e-4,
        "frame 8 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 8 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 8 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.131_152_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 9 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_596_9_f32)).length() < 3e-4,
        "frame 9 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 9 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 9 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(-6.112_432_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 10 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_597_3_f32)).length() < 3e-4,
        "frame 10 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 10 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 10 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.095_497_6_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 11 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_597_2_f32)).length() < 3e-4,
        "frame 11 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 11 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 11 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.080_549_2_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 12 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_597_1_f32)).length() < 3e-4,
        "frame 12 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 12 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 12 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(-6.081_963_5_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 13 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.867_597_f32)).length() < 3e-4,
        "frame 13 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 13 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 13 fov");
}
