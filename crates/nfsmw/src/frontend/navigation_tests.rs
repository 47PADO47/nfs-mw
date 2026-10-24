//! The D-pad already navigates the menu mask and must not also contribute mouse-wheel notches.

use super::*;
use crate::input::Snapshot;
use bevy_input::gamepad::GamepadButton;

#[test]
fn repeated_dpad_taps_do_not_accumulate_extra_menu_wheel_steps() {
    let bindings = Bindings::default();
    for (button, bit) in [(GamepadButton::DPadUp, pad::UP), (GamepadButton::DPadDown, pad::DOWN)] {
        let mut actions = ActionState::default();
        let mut wheel = 0.0;
        let mut viewer_zoom = 0.0;
        // Each tap is below the native repeat delay. Together they previously accumulated extra wheel moves.
        for _ in 0..10 {
            for held in [true, true, true, false, false, false] {
                let mut snapshot = Snapshot { dt: 1.0 / 60.0, ..Snapshot::default() };
                if held {
                    snapshot.pad_buttons.insert(button);
                }
                actions.update(&bindings, &snapshot, false);
                viewer_zoom += actions.value(Action::Zoom);
                assert_eq!(pad_mask(&actions), if held { bit } else { 0 });
                assert_eq!(menu_wheel_steps(&actions, &mut wheel, true), 0);
            }
        }
        assert!(viewer_zoom.abs() > 2.0, "viewer D-pad zoom remains available");
        assert_eq!(wheel, 0.0, "pad taps must not leave a fractional mouse-wheel carry");
    }
}

#[test]
fn physical_menu_wheel_retains_fractional_lines_during_dpad_navigation() {
    let bindings = Bindings::default();
    let mut actions = ActionState::default();
    let mut wheel = 0.0;
    for (scroll, expected) in [(0.4, 0), (0.7, 1), (-0.4, 0), (-0.8, -1)] {
        let mut snapshot = Snapshot { dt: 0.2, scroll, ..Snapshot::default() };
        snapshot.pad_buttons.insert(GamepadButton::DPadDown);
        actions.update(&bindings, &snapshot, false);
        assert_eq!(menu_wheel_steps(&actions, &mut wheel, true), expected);
        assert!(actions.pressed(Action::MenuDown));
    }
    assert_eq!(menu_wheel_steps(&actions, &mut wheel, false), 0);
    assert_eq!(wheel, 0.0, "leaving the menu clears fractional wheel state");
    actions.update(&bindings, &Snapshot { scroll: 1.0, ..Snapshot::default() }, true);
    assert_eq!(menu_wheel_steps(&actions, &mut wheel, true), 0, "UI capture consumes wheel movement");
}
