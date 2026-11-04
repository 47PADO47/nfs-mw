use super::*;
use glam::Vec4;

fn assert_bounds(area: &HintArea, min: Vec2, max: Vec2) {
    assert!((area.min - min).abs().max_element() < 1e-5, "min: {:?} != {min:?}", area.min);
    assert!((area.max - max).abs().max_element() < 1e-5, "max: {:?} != {max:?}", area.max);
}

#[test]
fn rotated_hint_encloses_all_four_corners() {
    let area = hint_area(Action::MenuBack, Mat4::from_rotation_z(std::f32::consts::FRAC_PI_4), -2.0, 2.0, 1.0);
    let extent = 3.0 / 2.0_f32.sqrt();
    assert_bounds(&area, Vec2::splat(-extent), Vec2::splat(extent));
    assert_eq!(area.action, Action::MenuBack);
    assert!(area.covers(Vec2::new(2.0, 0.0)));
    assert!(!area.covers(Vec2::new(2.2, 0.0)));
}

#[test]
fn sheared_hint_encloses_all_four_corners() {
    let world = Mat4::from_cols(Vec4::X, Vec4::new(-1.0, 1.0, 0.0, 0.0), Vec4::Z, Vec4::W);
    let area = hint_area(Action::MenuAccept, world, -2.0, 2.0, 1.0);
    assert_bounds(&area, Vec2::new(-3.0, -1.0), Vec2::new(3.0, 1.0));
    assert_eq!(area.action, Action::MenuAccept);
    assert!(area.covers(Vec2::new(3.0, 1.0)));
    assert!(!area.covers(Vec2::new(3.01, 0.0)));
}

#[test]
fn translation_and_mirror_preserve_bounds_and_hit_behavior() {
    let translated = hint_area(Action::MenuBack, Mat4::from_translation(Vec3::new(10.0, -4.0, 0.0)), -2.0, 2.0, 1.0);
    assert_bounds(&translated, Vec2::new(8.0, -5.0), Vec2::new(12.0, -3.0));
    assert!(translated.covers(Vec2::new(8.0, -5.0)));
    assert!(!translated.covers(Vec2::new(7.99, -5.0)));
    let mirror = hint_area(Action::MenuAccept, Mat4::from_scale(Vec3::new(-2.0, 3.0, 1.0)), -2.0, 2.0, 1.0);
    assert_bounds(&mirror, Vec2::new(-4.0, -3.0), Vec2::new(4.0, 3.0));
    assert_eq!(mirror.action, Action::MenuAccept);
    assert!(mirror.covers(Vec2::new(4.0, 3.0)));
    assert!(!mirror.covers(Vec2::new(4.01, 0.0)));
}

#[test]
fn the_pursuit_result_screens_hint_continue_with_its_key_and_no_back() {
    let bindings = Bindings::default();
    for device in [InputDevice::Keyboard, InputDevice::Xbox] {
        for name in [screen::MILESTONE_REWARDS, screen::POST_RACE_PURSUIT] {
            let hints = hint_set(name, Footer::Standard, &bindings, device);
            assert_eq!((hints[0].1, hints[0].2), (HintLabel::Continue, Some(Action::MenuAccept)), "{name}");
            assert!(!hints[0].0.is_empty(), "{name}: Continue has its key or pad glyph");
            assert!(hints.iter().all(|(_, label, _)| *label != HintLabel::Back), "{name} has no Back");
        }
    }
    let pursuit = hint_set(screen::POST_RACE_PURSUIT, Footer::Standard, &bindings, InputDevice::Keyboard);
    assert_eq!(pursuit.get(1).map(|hint| hint.2), Some(Some(Action::MenuButton2)), "the infractions page");
}
