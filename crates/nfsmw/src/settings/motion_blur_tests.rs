use super::motion_blur::{MOTION_BLUR_STRENGTH_RANGE, MotionBlur, parse_strength};
use super::{Partial, Settings};

#[test]
fn the_defaults_keep_the_curve_as_it_is() {
    let s = Settings::from(Partial::default());
    assert!(s.motion_blur);
    assert_eq!(s.motion_blur_strength, 1.0);
    assert_eq!(MotionBlur::from(&s).radial_blur(0.6), 0.6, "nothing changes by default");
}

#[test]
fn the_switch_and_strength_come_from_the_layers() {
    let cli = Partial { motion_blur: Some(false), motion_blur_strength: Some(0.5), ..Partial::default() };
    let s = Settings::from(cli);
    assert!(!s.motion_blur);
    assert_eq!(s.motion_blur_strength, 0.5);
}

#[test]
fn the_strength_is_clamped_to_its_range() {
    let (lo, hi) = MOTION_BLUR_STRENGTH_RANGE;
    let high = Settings::from(Partial { motion_blur_strength: Some(9.0), ..Partial::default() });
    let low = Settings::from(Partial { motion_blur_strength: Some(-1.0), ..Partial::default() });
    assert_eq!((high.motion_blur_strength, low.motion_blur_strength), (hi, lo));
}

#[test]
fn off_draws_no_blur_whatever_the_strength() {
    let off = MotionBlur { enabled: false, strength: 2.0 };
    assert_eq!(off.radial_blur(1.0), 0.0);
}

#[test]
fn strength_scales_the_curve_and_zero_draws_none() {
    let half = MotionBlur { enabled: true, strength: 0.5 };
    assert!((half.radial_blur(0.8) - 0.4).abs() < 1e-6);
    let none = MotionBlur { enabled: true, strength: 0.0 };
    assert_eq!(none.radial_blur(1.0), 0.0);
    let double = MotionBlur { enabled: true, strength: 2.0 };
    assert_eq!(double.radial_blur(0.8), 1.0, "clamped to the full blur");
}

#[test]
fn the_config_file_and_the_console_read_the_same_values() {
    let file = super::file::parse("motion_blur = false\nmotion_blur_strength = 0.5\n", "test");
    assert_eq!((file.motion_blur, file.motion_blur_strength), (Some(false), Some(0.5)));
    let whole = super::file::parse("motion_blur_strength = 2\n", "test");
    assert_eq!(whole.motion_blur_strength, Some(2.0), "a whole number is a number too");
    assert_eq!(super::file::parse("motion_blur = 1\n", "test").motion_blur, None);
    assert_eq!(parse_strength(" 1.5 "), Ok(1.5));
    assert!(parse_strength("3").is_err(), "out of range is refused at the console");
    assert!(parse_strength("loud").is_err());
}
