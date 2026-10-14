//! The keyboard controls list (`Options_PC_Controller.fng`): eight rows, each with a primary and a secondary button.
//! A press on a button asks the frontend to take the next key for that slot (`Command::Remap`). The package's rows
//! and buttons are in docs/specs/controls-remap.md.

use blackbox_feng::ObjectRef;
use blackbox_feng::ids::{BUTTON_PRESSED, BUTTON_RELEASED, PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};

use bevy_input::keyboard::KeyCode;

use crate::input::{Action, Bindings, Control};

use super::ids::screen;
use super::logic::{Args, Command, Cx, ScreenLogic};

/// The control each row changes, from the top. Steering is one axis, so its two rows are its two directions.
pub const ROWS: [Control; 8] = [
    Control::button(Action::Throttle),
    Control::button(Action::Brake),
    Control::axis(Action::Steer, -1.0),
    Control::axis(Action::Steer, 1.0),
    Control::button(Action::Handbrake),
    Control::button(Action::Speedbreaker),
    Control::button(Action::Nos),
    Control::button(Action::ShiftDown),
];

/// The primary and secondary buttons of each row, by GUID.
const PRIMARY: [u32; ROWS.len()] =
    [0x000f_a349, 0x0010_4689, 0x0010_468e, 0x0010_4693, 0x0010_469a, 0x0010_469f, 0x0010_46a4, 0x0010_46a9];
const SECONDARY: [u32; ROWS.len()] =
    [0x000f_a366, 0x0010_46dc, 0x0010_46e4, 0x0010_46ed, 0x0010_46f5, 0x0010_46fe, 0x0010_4707, 0x0010_4710];

/// The text object of each row's title (the package's `option_data` strings at x = -298), from the top, by GUID.
const TITLE: [u32; ROWS.len()] =
    [0x000e_1917, 0x000f_a36b, 0x000f_a37c, 0x000f_a388, 0x0010_4073, 0x0010_4077, 0x0010_407b, 0x0010_407e];
/// The title of each row, in the game's words for the control (the English originals were not read from the table).
const LABELS: [&str; ROWS.len()] =
    ["Accelerate", "Brake / Reverse", "Steer Left", "Steer Right", "Handbrake", "Speedbreaker", "N2O", "Shift Down"];
/// Shown in every key field until the screen gets the bindings (the original shows the key names there).
const KEY_PLACEHOLDER: &str = "---";
/// The text objects inside the primary and secondary buttons (the `option_data` strings at x = -14 and x = 146).
const PRIMARY_TEXT: [u32; ROWS.len()] =
    [0x000f_a34a, 0x000f_a36f, 0x000f_a380, 0x000f_a38c, 0x0010_409b, 0x0010_409f, 0x0010_40a3, 0x0010_40a7];
const SECONDARY_TEXT: [u32; ROWS.len()] =
    [0x000f_a367, 0x000f_a373, 0x000f_a384, 0x000f_a390, 0x0010_40c3, 0x0010_40c7, 0x0010_40cb, 0x0010_40cf];

/// A button of the list: the row, and the slot (0 primary, 1 secondary).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cell {
    row: usize,
    slot: usize,
}

impl Cell {
    fn guid(self) -> u32 {
        match self.slot {
            0 => PRIMARY[self.row],
            _ => SECONDARY[self.row],
        }
    }
}

/// The cell a button with this GUID is.
fn cell_of_guid(guid: u32) -> Option<Cell> {
    (0..ROWS.len()).find_map(|row| {
        if PRIMARY[row] == guid {
            return Some(Cell { row, slot: 0 });
        }
        (SECONDARY[row] == guid).then_some(Cell { row, slot: 1 })
    })
}

/// The object with this GUID in the screen's package.
fn by_guid(cx: &Cx, guid: u32) -> Option<ObjectRef> {
    let def = cx.rt.package(cx.package)?;
    let index = def.objects.iter().position(|o| o.guid == guid)?;
    Some(ObjectRef { package: cx.package, index })
}

fn focused(cx: &Cx) -> Option<Cell> {
    let button = cx.rt.current_button(cx.package)?;
    let guid = cx.rt.package(cx.package)?.objects.get(button.index)?.guid;
    cell_of_guid(guid)
}

/// Sets the text of the object with this GUID; a GUID the package lacks is left alone.
fn set_text(cx: &mut Cx, guid: u32, text: &str) {
    if let Some(o) = by_guid(cx, guid) {
        cx.rt.set_text(o, text.to_owned());
    }
}

fn focus(cx: &mut Cx, cell: Cell) {
    if let Some(o) = by_guid(cx, cell.guid()) {
        cx.rt.focus_button(o);
    }
}

/// The cell one step away from `from` in a direction (clamped to the list).
fn step(from: Cell, message: u32) -> Option<Cell> {
    let last = ROWS.len() - 1;
    Some(match message {
        PAD_UP => Cell { row: from.row.saturating_sub(1), ..from },
        PAD_DOWN => Cell { row: (from.row + 1).min(last), ..from },
        PAD_LEFT => Cell { slot: 0, ..from },
        PAD_RIGHT => Cell { slot: 1, ..from },
        _ => return None,
    })
}

/// The name a key shows as on the list: the code's name without its `Key` prefix (`KeyW` is `W`, `Space` is `SPACE`).
fn key_name(key: KeyCode) -> String {
    let code = format!("{key:?}");
    code.strip_prefix("Key").unwrap_or(&code).to_uppercase()
}

/// The key names of each row, primary then secondary, from the bindings. A slot with no key shows the placeholder.
pub fn key_texts(bindings: &Bindings) -> Vec<[String; 2]> {
    ROWS.iter()
        .map(|control| {
            let keys = bindings.keyboard_keys(*control);
            let text = |slot: usize| keys.get(slot).map_or_else(|| KEY_PLACEHOLDER.to_owned(), |k| key_name(*k));
            [text(0), text(1)]
        })
        .collect()
}

/// The controls list. It keeps the key names it last showed, so it only writes the ones that changed. Back returns
/// to the option categories it was opened from (`icon_menu.rs`), as the option lists do.
pub struct ControlsLogic {
    args: Args,
    shown: Vec<[String; 2]>,
}

impl ControlsLogic {
    pub fn new(args: Args) -> Self {
        Self { args, shown: Vec::new() }
    }

    fn back(&self) -> Command {
        if self.args.pause {
            return Command::Switch(screen::PAUSE_MENU.into(), Args { pause: true, options: true, ..self.args });
        }
        Command::Switch(screen::MAIN_MENU_SUB.into(), Args { options: true, ..self.args })
    }
}

impl ScreenLogic for ControlsLogic {
    fn start(&mut self, cx: &mut Cx) {
        for row in 0..ROWS.len() {
            set_text(cx, TITLE[row], LABELS[row]);
        }
        focus(cx, Cell { row: 0, slot: 0 });
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        if self.shown.as_slice() == cx.key_texts {
            return;
        }
        for (row, texts) in cx.key_texts.iter().enumerate().take(ROWS.len()) {
            set_text(cx, PRIMARY_TEXT[row], &texts[0]);
            set_text(cx, SECONDARY_TEXT[row], &texts[1]);
        }
        self.shown = cx.key_texts.to_vec();
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        // The hint says "press ENTER to re-map". The pad accept reaches the field as its release (`BUTTON_RELEASED`),
        // and a mouse click as its press.
        if message == BUTTON_PRESSED || message == BUTTON_RELEASED {
            if let Some(cell) = focused(cx) {
                cx.send(Command::Remap(ROWS[cell.row], cell.slot));
            }
            return;
        }
        if message == PAD_BACK {
            cx.send(self.back());
            return;
        }
        let from = focused(cx).unwrap_or(Cell { row: 0, slot: 0 });
        if let Some(to) = step(from, message) {
            focus(cx, to);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_button_belongs_to_one_row_and_slot() {
        for row in 0..ROWS.len() {
            assert_eq!(cell_of_guid(PRIMARY[row]), Some(Cell { row, slot: 0 }));
            assert_eq!(cell_of_guid(SECONDARY[row]), Some(Cell { row, slot: 1 }));
        }
    }

    #[test]
    fn the_two_steering_rows_are_the_two_directions_of_steer() {
        assert_eq!(ROWS[2], Control::axis(Action::Steer, -1.0));
        assert_eq!(ROWS[3], Control::axis(Action::Steer, 1.0));
    }

    #[test]
    fn the_arrows_move_the_focus_and_stop_at_the_ends() {
        let top = Cell { row: 0, slot: 0 };
        assert_eq!(step(top, PAD_UP), Some(top));
        assert_eq!(step(top, PAD_RIGHT), Some(Cell { row: 0, slot: 1 }));
        let bottom = Cell { row: ROWS.len() - 1, slot: 1 };
        assert_eq!(step(bottom, PAD_DOWN), Some(bottom));
        assert_eq!(step(bottom, PAD_LEFT), Some(Cell { row: ROWS.len() - 1, slot: 0 }));
        assert_eq!(step(top, BUTTON_PRESSED), None);
    }

    #[test]
    fn a_key_shows_its_name_without_the_key_prefix() {
        assert_eq!(key_name(KeyCode::KeyW), "W");
        assert_eq!(key_name(KeyCode::Space), "SPACE");
    }

    #[test]
    fn an_unbound_control_shows_the_placeholder() {
        let texts = key_texts(&Bindings(Vec::new()));
        assert_eq!(texts.len(), ROWS.len());
        assert!(texts.iter().all(|t| t[0] == KEY_PLACEHOLDER && t[1] == KEY_PLACEHOLDER));
    }
}
