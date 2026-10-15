use super::super::{follow::Follow, projection};
use super::{body, view};
use glam::Vec3;

#[test]
fn whole_native_worldpulse_inside() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.acceleration = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    body.collision_damping = 0.8f32;
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.217_521_12_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 0 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.254_140_9_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.acceleration = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    body.collision_damping = 0.4f32;
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.183_930_9_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 1 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.378_300_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.163_720_95_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.502_460_7_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.178_827_38_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 3 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.409_654_4_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.193_933_8_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.316_847_9_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.198_629_04_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 5 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.288_002_6_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.203_324_27_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 6 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.259_157_4_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.203_732_57_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 7 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.256_649_1_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.204_140_84_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 8 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.254_140_9_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 8 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 8 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 8 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.194_035_87_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 9 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.316_220_9_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 9 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 9 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 9 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.183_930_9_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 10 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.378_300_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 10 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 10 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 10 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.191_484_11_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 11 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.331_897_6_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 11 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 11 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 11 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.199_037_31_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 12 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.285_494_3_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 12 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 12 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 12 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(0.201_384_95_f32, 0.0f32, 1.2_f32)).length() < 3e-4,
        "frame 13 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.271_071_8_f32, 0.0f32, 1.138_852_5_f32)).length() < 3e-4,
        "frame 13 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 13 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 13 fov");
}

#[test]
fn whole_native_worldpulse_lookback() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.acceleration = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    body.look_back = true;
    body.collision_damping = 0.8f32;
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(6.0f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 0 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.0f32, 0.0f32, 1.869_399_7_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.0f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 0 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.acceleration = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(1.0f32, 0.0f32, -0.0f32);
    body.up = Vec3::new(0.0f32, 0.0f32, 1.0f32);
    body.speed = body.velocity.length();
    body.look_back = true;
    body.collision_damping = 0.4f32;
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(5.444_984_4_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 1 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(0.365_290_76_f32, 0.0f32, 1.868_893_3_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.006_356_242_6_f32, 0.0f32, 1.0f32)).length() < 3e-4,
        "frame 1 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(4.931_377_4_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 2 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.032_053_2_f32, 0.0f32, 1.868_165_5_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.021_721_68_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(5.504_477_5_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 3 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.353_664_f32, 0.0f32, 1.867_755_7_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.040_753_856_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(6.104_334_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 4 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.412_571_1_f32, 0.0f32, 1.867_604_3_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.059_212_58_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 4 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(6.383_842_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 5 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.418_113_f32, 0.0f32, 1.867_573_5_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.076_962_21_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 5 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(6.655_313_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 6 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.385_523_1_f32, 0.0f32, 1.867_579_6_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.093_196_97_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 6 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(6.771_67_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 7 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.375_815_7_f32, 0.0f32, 1.867_589_2_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.105_714_865_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 7 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(6.869_839_7_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 8 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.368_449_8_f32, 0.0f32, 1.867_594_7_f32)).length() < 3e-4,
        "frame 8 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.114_312_92_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 8 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 8 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(6.593_446_3_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 9 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.434_157_5_f32, 0.0f32, 1.867_596_9_f32)).length() < 3e-4,
        "frame 9 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.119_653_374_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 9 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 9 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(6.296_290_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 10 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.501_247_4_f32, 0.0f32, 1.867_597_3_f32)).length() < 3e-4,
        "frame 10 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.122_630_2_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 10 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 10 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(6.590_468_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 11 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.450_705_9_f32, 0.0f32, 1.867_597_2_f32)).length() < 3e-4,
        "frame 11 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.124_063_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 11 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 11 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!((actual.eye - Vec3::new(6.876_841_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 12 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(1.400_256_4_f32, 0.0f32, 1.867_597_1_f32)).length() < 3e-4,
        "frame 12 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.124_584_59_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 12 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 12 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.125f32));
    assert!(
        (actual.eye - Vec3::new(6.976_192_5_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 13 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.384_609_2_f32, 0.0f32, 1.867_597_f32)).length() < 3e-4,
        "frame 13 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(0.124_630_11_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 13 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 13 fov");
}
