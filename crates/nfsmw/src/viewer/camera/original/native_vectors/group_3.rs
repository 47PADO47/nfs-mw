use super::super::{follow::Follow, projection};
use super::{body, view};
use glam::Vec3;

#[test]
fn whole_native_acceleration() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.acceleration = Vec3::new(10.0f32, 0.0f32, 0.0f32);
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
    assert!((actual.eye - Vec3::new(-6.000_666_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 1 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(-0.000_497_617_1_f32, 0.0f32, 1.869_388_3_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(8.543_261e-6_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 1 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.002_775_7_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(-0.002_048_130_6_f32, 0.0f32, 1.869_356_8_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(3.596_728_5e-5_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 2 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.006_394_4_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 3 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(-0.004_692_978_7_f32, 0.0f32, 1.869_308_7_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!((actual.up - Vec3::new(8.446_423e-5_f32, 0.0f32, 1.0f32)).length() < 3e-4, "frame 3 up: {:?}", actual.up);
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(-6.011_422_6_f32, 0.0f32, 2.0f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(-0.008_339_295_f32, 0.0f32, 1.869_247_8_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.000_153_949_54_f32, 0.0f32, 1.0f32)).length() < 3e-4,
        "frame 4 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.017_764_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 5 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(-0.013_015_057_f32, 0.0f32, 1.869_177_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.000_246_265_03_f32, 0.0f32, 1.0f32)).length() < 3e-4,
        "frame 5 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.025_327_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 6 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(-0.018_676_158_f32, 0.0f32, 1.869_098_8_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.000_362_072_78_f32, 0.0f32, 1.0f32)).length() < 3e-4,
        "frame 6 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(3, &view(3), body, 0.016_666_668_f32));
    assert!((actual.eye - Vec3::new(-6.034_024_f32, 0.0f32, 2.0f32)).length() < 3e-4, "frame 7 eye: {:?}", actual.eye);
    assert!(
        (actual.target - Vec3::new(-0.025_081_104_f32, 0.0f32, 1.869_015_6_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.000_498_515_3_f32, 0.0f32, 1.0f32)).length() < 3e-4,
        "frame 7 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
}

#[test]
fn whole_native_bodypitch() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(30.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(0.980_066_6_f32, 0.0f32, -0.198_669_33_f32);
    body.up = Vec3::new(0.197_676_81_f32, -0.099_833_414_f32, 0.975_170_3_f32);
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
fn whole_native_hoodbank() {
    let mut follow = Follow::default();
    let mut body = body();
    follow.update(3, &view(3), body, 1.0 / 60.0);
    body.velocity = Vec3::new(20.0f32, 0.0f32, 0.0f32);
    body.forward = Vec3::new(0.980_066_6_f32, 0.0f32, -0.198_669_33_f32);
    body.up = Vec3::new(0.189_796_06_f32, -0.295_520_22_f32, 0.936_293_36_f32);
    body.speed = body.velocity.length();
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.522_520_66_f32, -0.354_624_27_f32, 1.063_800_2_f32)).length() < 3e-4,
        "frame 0 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.490_981_6_f32, -0.336_553_93_f32, 0.807_878_8_f32)).length() < 3e-4,
        "frame 0 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.189_796_06_f32, -0.295_520_22_f32, 0.936_293_36_f32)).length() < 3e-4,
        "frame 0 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 0 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.522_520_66_f32, -0.354_624_27_f32, 1.063_800_2_f32)).length() < 3e-4,
        "frame 1 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.490_981_6_f32, -0.336_553_93_f32, 0.807_878_8_f32)).length() < 3e-4,
        "frame 1 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.189_796_06_f32, -0.295_520_22_f32, 0.936_293_36_f32)).length() < 3e-4,
        "frame 1 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 1 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.522_520_66_f32, -0.354_624_27_f32, 1.063_800_2_f32)).length() < 3e-4,
        "frame 2 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.490_981_6_f32, -0.336_553_93_f32, 0.807_878_8_f32)).length() < 3e-4,
        "frame 2 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.189_796_06_f32, -0.295_520_22_f32, 0.936_293_36_f32)).length() < 3e-4,
        "frame 2 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 2 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.522_520_66_f32, -0.354_624_27_f32, 1.063_800_2_f32)).length() < 3e-4,
        "frame 3 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.490_981_6_f32, -0.336_553_93_f32, 0.807_878_8_f32)).length() < 3e-4,
        "frame 3 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.189_796_06_f32, -0.295_520_22_f32, 0.936_293_36_f32)).length() < 3e-4,
        "frame 3 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 3 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.522_520_66_f32, -0.354_624_27_f32, 1.063_800_2_f32)).length() < 3e-4,
        "frame 4 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.490_981_6_f32, -0.336_553_93_f32, 0.807_878_8_f32)).length() < 3e-4,
        "frame 4 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.189_796_06_f32, -0.295_520_22_f32, 0.936_293_36_f32)).length() < 3e-4,
        "frame 4 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 4 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.522_520_66_f32, -0.354_624_27_f32, 1.063_800_2_f32)).length() < 3e-4,
        "frame 5 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.490_981_6_f32, -0.336_553_93_f32, 0.807_878_8_f32)).length() < 3e-4,
        "frame 5 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.189_796_06_f32, -0.295_520_22_f32, 0.936_293_36_f32)).length() < 3e-4,
        "frame 5 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 5 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.522_520_66_f32, -0.354_624_27_f32, 1.063_800_2_f32)).length() < 3e-4,
        "frame 6 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.490_981_6_f32, -0.336_553_93_f32, 0.807_878_8_f32)).length() < 3e-4,
        "frame 6 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.189_796_06_f32, -0.295_520_22_f32, 0.936_293_36_f32)).length() < 3e-4,
        "frame 6 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 6 fov");
    let actual = projection::finalize_sample(follow.update(1, &view(1), body, 0.016_666_668_f32));
    assert!(
        (actual.eye - Vec3::new(0.522_520_66_f32, -0.354_624_27_f32, 1.063_800_2_f32)).length() < 3e-4,
        "frame 7 eye: {:?}",
        actual.eye
    );
    assert!(
        (actual.target - Vec3::new(1.490_981_6_f32, -0.336_553_93_f32, 0.807_878_8_f32)).length() < 3e-4,
        "frame 7 target: {:?}",
        actual.target
    );
    assert!(
        (actual.up - Vec3::new(0.189_796_06_f32, -0.295_520_22_f32, 0.936_293_36_f32)).length() < 3e-4,
        "frame 7 up: {:?}",
        actual.up
    );
    assert_eq!(actual.fov_bangle, 15290.0f32, "frame 7 fov");
}
