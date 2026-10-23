//! Resolve real D-pad bindings through the same pad and wheel paths as the frontend plugin.

use super::*;
use crate::frontend::plugin_keys::{menu_wheel_steps, pad_mask};
use crate::input::{ActionState, Bindings, Snapshot};
use bevy_input::gamepad::GamepadButton;

fn selected_row(h: &Harness) -> u32 {
    let focused = h.screens.focused_button().expect("the selected setting has focus");
    let tree = h.screens.trees().pop().unwrap();
    tree.nodes.iter().find(|node| node.index == focused.index).unwrap().name_hash
}

#[test]
#[ignore = "requires installed menu packages; set NFSMW_GAME_DIR"]
fn short_dpad_taps_move_one_settings_row_each_without_extra_scroll() {
    let mut h =
        Harness::open(screen::OPTIONS, Args { category: Category::Audio, ..Args::default() }).expect("installed menus");
    h.wait(1.0);
    let bindings = Bindings::default();
    let mut actions = ActionState::default();
    let mut wheel = 0.0;
    for (button, rows) in [(GamepadButton::DPadDown, [2, 3, 4, 5, 1]), (GamepadButton::DPadUp, [5, 4, 3, 2, 1])] {
        for row in rows {
            for held in [true, true, true, false, false, false] {
                let mut snapshot = Snapshot { dt: FRAME, ..Snapshot::default() };
                if held {
                    snapshot.pad_buttons.insert(button);
                }
                actions.update(&bindings, &snapshot, false);
                let notches = menu_wheel_steps(&actions, &mut wheel, true);
                if notches != 0 {
                    h.scroll(notches);
                }
                h.run(pad_mask(&actions), 1);
            }
            assert_eq!(
                selected_row(&h),
                blackbox_feng::fe_hash_upper(&format!("OPTION_NAME_{row}")),
                "{button:?} must move exactly one row per short tap"
            );
        }
    }
}
