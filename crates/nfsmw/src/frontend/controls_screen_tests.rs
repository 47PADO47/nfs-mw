//! The controls list: the packages' buttons, the rows a list look adds for the HUD's actions, the focus and the key names.

use std::collections::HashMap;

use blackbox_feng::{PackageId, Runtime};

use super::*;
use crate::frontend::logic::Way;
use crate::settings::{Partial, Settings, UiStyle};
use crate::ui::theme::remastered::menu::layout::MAX_ROWS;

const PACKAGE: &str = "Options_PC_Controller.fng";

/// Runs `call` with a context under the look `style`, with no package loaded and the key names `key_texts`, and returns
/// the commands it sent.
fn with_cx(style: UiStyle, key_texts: &[[String; 2]], call: impl FnOnce(&mut Cx)) -> Vec<Command> {
    let mut rt = Runtime::new();
    let assets = UiAssets::empty();
    let mut settings = Settings::from(Partial::default());
    settings.ui_style = style;
    let mut changed = Partial::default();
    let (mut commands, mut memory) = (Vec::new(), HashMap::new());
    let mut cx = Cx {
        rt: &mut rt,
        package: PackageId(1),
        assets: &assets,
        settings: &mut settings,
        changed: &mut changed,
        caps: crate::settings::test_caps::native(),
        commands: &mut commands,
        memory: &mut memory,
        key_texts,
        world_map: None,
        mods: None,
        name: PACKAGE,
    };
    call(&mut cx);
    commands
}

/// The slot the commands ask a key for.
fn remap(commands: &[Command]) -> Option<(Control, usize)> {
    commands.iter().find_map(|command| match command {
        Command::Remap(control, slot) => Some((*control, *slot)),
        _ => None,
    })
}

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

#[test]
fn the_list_looks_add_the_hud_rows_after_the_packages() {
    let texts = key_texts(&Bindings::default(), None);
    for style in [UiStyle::Custom, UiStyle::Remastered] {
        let mut logic = ControlsLogic::new(PACKAGE, Args::default());
        with_cx(style, &texts, |cx| {
            logic.start(cx);
            logic.tick(cx, 0.0);
        });
        let list = logic.list(&UiAssets::empty()).expect("a list look publishes the list");
        assert_eq!(list.entries.len(), ROW_COUNT + HUD_ROWS.len(), "{style:?}");
        for (i, control) in HUD_ROWS.iter().enumerate() {
            let entry = &list.entries[ROW_COUNT + i];
            assert_eq!(Some(entry.label.clone()), hud_labels::label(control.action), "{style:?}, {control:?}");
            let MenuValue::Keys { primary, secondary, .. } = &entry.value else {
                panic!("{control:?} is not a key row")
            };
            assert_eq!([primary.clone(), secondary.clone()], texts[ROW_COUNT + i], "{style:?}, {control:?}");
        }
        assert_ne!(texts[ROW_COUNT][0], KEY_PLACEHOLDER, "Map mode shows its default key");
    }
}

#[test]
fn the_original_look_keeps_the_packages_eight_rows() {
    let texts = key_texts(&Bindings::default(), None);
    let mut logic = ControlsLogic::new(PACKAGE, Args::default());
    with_cx(UiStyle::Original, &texts, |cx| {
        logic.start(cx);
        logic.tick(cx, 0.0);
    });
    assert!(logic.list(&UiAssets::empty()).is_none(), "the package draws the list");
    assert_eq!(row_count(logic.vertical), ROW_COUNT);
    assert_eq!(control(ROW_COUNT), Some(HUD_ROWS[0]), "the HUD rows follow only where a list look counts them");
}

#[test]
fn the_list_scrolls_to_the_last_hud_row_and_rebinding_it_changes_the_bindings() {
    let mut bindings = Bindings::default();
    let texts = key_texts(&bindings, None);
    let last = ROW_COUNT + HUD_ROWS.len() - 1;
    let mut logic = ControlsLogic::new(PACKAGE, Args::default());
    let commands = with_cx(UiStyle::Custom, &texts, |cx| {
        logic.start(cx);
        for _ in 0..last + 3 {
            logic.message(cx, PAD_DOWN);
            logic.tick(cx, 0.0);
        }
        logic.message(cx, PAD_RIGHT);
        logic.message(cx, BUTTON_RELEASED);
    });
    let list = logic.list(&UiAssets::empty()).unwrap();
    assert_eq!(list.selected, last, "the focus stops on the last row");
    assert_eq!(list.first, last + 1 - MAX_ROWS, "the window follows the focus");
    let (control, slot) = remap(&commands).expect("the press asks for a key");
    assert_eq!((control, slot), (Control::button(Action::DismissNotification), 1));
    // The frontend puts the next key pressed in that slot (`plugin_keys::remember_key`).
    bindings.set_keyboard_slot(control, slot, KeyCode::KeyG);
    assert_eq!(bindings.keyboard_keys(control), vec![KeyCode::KeyX, KeyCode::KeyG]);
    assert_eq!(key_texts(&bindings, None)[last], ["X".to_owned(), "G".to_owned()], "the row shows the new key");
}

#[test]
fn a_click_on_a_hud_row_asks_for_the_key_of_the_half_it_landed_on() {
    let texts = key_texts(&Bindings::default(), None);
    let mut logic = ControlsLogic::new(PACKAGE, Args::default());
    let commands = with_cx(UiStyle::Custom, &texts, |cx| {
        logic.start(cx);
        logic.pick_row(cx, ROW_COUNT + 2, Pick::Press(Way::Stay, 0.2));
    });
    assert_eq!(remap(&commands), Some((HUD_ROWS[2], 0)));
    let commands = with_cx(UiStyle::Custom, &texts, |cx| {
        logic.pick_row(cx, ROW_COUNT + 40, Pick::Press(Way::Stay, 0.8));
    });
    assert_eq!(remap(&commands), Some((HUD_ROWS[HUD_ROWS.len() - 1], 1)), "a row past the end is the last one");
}
