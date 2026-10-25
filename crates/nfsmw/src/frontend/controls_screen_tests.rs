//! The controls list: the packages' buttons, the rows a list look adds for the HUD's actions, the focus and the key names.

use super::*;

#[test]
fn every_button_belongs_to_one_row_and_slot_in_both_packages() {
    for layout in [&controls_layout::OPTIONS, &controls_layout::PAUSE] {
        for row in 0..ROW_COUNT {
            assert_eq!(cell_of_guid(layout, layout.primary[row]), Some(Cell { row, slot: 0 }));
            assert_eq!(cell_of_guid(layout, layout.secondary[row]), Some(Cell { row, slot: 1 }));
        }
    }
}

#[test]
fn each_package_routes_its_text_objects_by_row() {
    for layout in [&controls_layout::OPTIONS, &controls_layout::PAUSE] {
        for row in 0..ROW_COUNT {
            assert_ne!(layout.primary_text[row], layout.secondary_text[row]);
            assert_ne!(layout.title[row], layout.primary_text[row]);
        }
    }
}

#[test]
fn the_pause_list_is_the_one_used_in_game() {
    let logic = ControlsLogic::new("Pause_PC_Controller.fng", Args::default());
    assert!(std::ptr::eq(logic.layout, &controls_layout::PAUSE));
    let logic = ControlsLogic::new("Options_PC_Controller.fng", Args::default());
    assert!(std::ptr::eq(logic.layout, &controls_layout::OPTIONS));
}

#[test]
fn the_two_steering_rows_are_the_two_directions_of_steer() {
    assert_eq!(ROWS[2], Control::axis(Action::Steer, -1.0));
    assert_eq!(ROWS[3], Control::axis(Action::Steer, 1.0));
}

#[test]
fn the_arrows_move_the_focus_and_stop_at_the_ends() {
    let top = Cell { row: 0, slot: 0 };
    assert_eq!(step(top, PAD_UP, ROW_COUNT), Some(top));
    assert_eq!(step(top, PAD_RIGHT, ROW_COUNT), Some(Cell { row: 0, slot: 1 }));
    let bottom = Cell { row: ROW_COUNT - 1, slot: 1 };
    assert_eq!(step(bottom, PAD_DOWN, ROW_COUNT), Some(bottom));
    assert_eq!(step(bottom, PAD_LEFT, ROW_COUNT), Some(Cell { row: ROW_COUNT - 1, slot: 0 }));
    assert_eq!(step(top, BUTTON_PRESSED, ROW_COUNT), None);
}

#[test]
fn a_key_shows_its_name_without_the_key_prefix() {
    assert_eq!(key_name(KeyCode::KeyW), "W");
    assert_eq!(key_name(KeyCode::Space), "SPACE");
    assert_eq!(key_name(KeyCode::ArrowDown), "DOWN");
    assert_eq!(key_name(KeyCode::ControlLeft), "L CTRL");
    assert_eq!(key_name(KeyCode::ShiftRight), "R SHIFT");
    assert_eq!(key_name(KeyCode::Escape), "ESCAPE");
    assert_eq!(key_name(KeyCode::Digit1), "1");
}

#[test]
fn a_slot_waiting_for_its_key_says_so() {
    let texts = key_texts(&Bindings(Vec::new()), Some((ROWS[1], 1)));
    assert_eq!(texts[1], [KEY_PLACEHOLDER.to_owned(), waiting_text()]);
    assert_eq!(texts[0][1], KEY_PLACEHOLDER, "the others are as they were");
}

#[test]
fn an_unbound_control_shows_the_placeholder() {
    let texts = key_texts(&Bindings(Vec::new()), None);
    assert_eq!(texts.len(), ROW_COUNT + HUD_ROWS.len(), "the package's rows, then the HUD's");
    assert!(texts.iter().all(|t| t[0] == KEY_PLACEHOLDER && t[1] == KEY_PLACEHOLDER));
}
