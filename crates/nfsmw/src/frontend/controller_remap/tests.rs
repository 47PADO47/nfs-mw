use super::*;
use crate::input::Action;

#[test]
fn batch12_capture_requires_accept_release_and_uses_only_its_selected_pad() {
    let a = Entity::from_raw_u32(1).unwrap();
    let b = Entity::from_raw_u32(2).unwrap();
    let mut capture = Capture::new(Control::button(Action::Throttle), 0);
    let held = Physical { buttons: vec![GamepadButton::South], ..Default::default() };
    assert_eq!(capture.sample(a, &held), None);
    assert_eq!(capture.sample(a, &held), None, "holding Accept cannot bind itself");
    assert_eq!(capture.sample(b, &Physical::default()), None);
    assert!(!capture.armed, "another controller cannot arm the capture");
    assert_eq!(capture.sample(a, &Physical::default()), None);
    assert!(capture.armed);
    assert_eq!(capture.sample(a, &held), Some((Source::PadButton(GamepadButton::South), 1.0)));
}

#[test]
fn batch12_capture_waits_for_stick_and_trigger_neutral_and_accepts_deliberate_motion() {
    let entity = Entity::from_raw_u32(1).unwrap();
    let mut capture = Capture::new(Control::axis(Action::Steer, -1.0), 1);
    let axis = Physical { axes: vec![(GamepadAxis::LeftStickX, -0.8)], ..Default::default() };
    assert_eq!(capture.sample(entity, &axis), None);
    assert_eq!(capture.sample(entity, &Physical::default()), None);
    assert_eq!(
        capture.sample(entity, &Physical { axes: vec![(GamepadAxis::LeftStickX, 0.1)], ..Default::default() }),
        None
    );
    assert_eq!(capture.sample(entity, &axis), Some((Source::PadAxis(GamepadAxis::LeftStickX), -0.8)));
    let trigger = Physical {
        triggers: vec![(GamepadButton::RightTrigger2, 0.75)],
        buttons: vec![GamepadButton::RightTrigger2],
        ..Default::default()
    };
    assert_eq!(trigger.candidate(), Some((Source::PadTrigger(GamepadButton::RightTrigger2), 0.75)));
    assert!(!trigger.neutral());
}

#[test]
fn batch12_capture_escape_cancels_and_reconnection_requires_neutral_again() {
    let entity = Entity::from_raw_u32(1).unwrap();
    let neutral = Physical::default();
    let mut capture = Capture::new(Control::button(Action::Handbrake), 0);
    assert_eq!(capture.step(Some(entity), Some(&neutral), false), Outcome::Waiting);
    assert!(capture.armed);
    assert_eq!(capture.step(Some(entity), Some(&neutral), true), Outcome::Cancelled);
    assert_eq!(capture.step(None, None, false), Outcome::Waiting);
    assert!(!capture.armed);
    let held = Physical { buttons: vec![GamepadButton::South], ..Default::default() };
    assert_eq!(capture.step(Some(entity), Some(&held), false), Outcome::Waiting);
    assert!(!capture.armed, "reconnecting while holding a control cannot bind it");
}
