//! The keyboard controls list, in its two packages: `Options_PC_Controller.fng` (the main menu's) and
//! `Pause_PC_Controller.fng` (the in-game copy). Each draws eight rows, each with a primary and a secondary button; the
//! GUIDs of each package are in `controls_layout.rs`. A press on a button asks the frontend to take the next key for
//! that slot (`Command::Remap`). The packages' rows and buttons are in docs/specs/controls-remap.md.

use blackbox_feng::ObjectRef;
use blackbox_feng::ids::{BUTTON_PRESSED, BUTTON_RELEASED, PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};

use bevy_input::keyboard::KeyCode;

use crate::input::{Action, Bindings, Control};

use super::controls_layout::{self, Layout, ROW_COUNT};
use super::ids::screen;
use super::logic::{Args, Command, Cx, Pick, ScreenLogic, wheel_as_rows};
use crate::i18n::{self, t};
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle, MenuValue};
use crate::ui::{UiAssets, prompts};

/// The control each row changes, from the top. Steering is one axis, so its two rows are its two directions.
///
/// The package draws eight rows, so the list has no row for the other actions: Map (`M`, the world map) and
/// ShiftUp are not on it. They stay rebindable in the bindings file (`[bindings]`, `docs/specs/controller-settings.md`).
pub const ROWS: [Control; ROW_COUNT] = [
    Control::button(Action::Throttle),
    Control::button(Action::Brake),
    Control::axis(Action::Steer, -1.0),
    Control::axis(Action::Steer, 1.0),
    Control::button(Action::Handbrake),
    Control::button(Action::Speedbreaker),
    Control::button(Action::Nos),
    Control::button(Action::ShiftDown),
];

/// The game's name for each row's control (`GAME_ACTION_GAS`, `GAME_ACTION_BRAKE`, `GAME_ACTION_STEERLEFT`,
/// `GAME_ACTION_STEERRIGHT`, `GAME_ACTION_HANDBRAKE`, `GAME_ACTION_GAMEBREAKER`, `N2O`, `GAME_ACTION_SHIFTDOWN`).
/// `GAME_ACTION_NOS` is not used: its English text reads `N20`.
const LABELS: [u32; ROW_COUNT] =
    [0x90B4_1FD0, 0x8DE9_E91A, 0x9054_2A63, 0x9B48_3E56, 0xB771_2E95, 0x4B22_DF6B, 0x0000_C62E, 0xB39C_462B];
/// The list's title, "Controls" (`OPT_CONTROLLER`).
const TITLE: u32 = 0x04DC_6DB5;
/// Shown in every key field until the screen gets the bindings (the original shows the key names there).
const KEY_PLACEHOLDER: &str = "---";

/// A button of the list: the row, and the slot (0 primary, 1 secondary).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cell {
    row: usize,
    slot: usize,
}

impl Cell {
    fn guid(self, layout: &Layout) -> u32 {
        match self.slot {
            0 => layout.primary[self.row],
            _ => layout.secondary[self.row],
        }
    }
}

/// The cell a button with this GUID is in this package.
fn cell_of_guid(layout: &Layout, guid: u32) -> Option<Cell> {
    (0..ROW_COUNT).find_map(|row| {
        if layout.primary[row] == guid {
            return Some(Cell { row, slot: 0 });
        }
        (layout.secondary[row] == guid).then_some(Cell { row, slot: 1 })
    })
}

/// The object with this GUID in the screen's package.
fn by_guid(cx: &Cx, guid: u32) -> Option<ObjectRef> {
    let def = cx.rt.package(cx.package)?;
    let index = def.objects.iter().position(|o| o.guid == guid)?;
    Some(ObjectRef { package: cx.package, index })
}

fn focused(cx: &Cx, layout: &Layout) -> Option<Cell> {
    let button = cx.rt.current_button(cx.package)?;
    let guid = cx.rt.package(cx.package)?.objects.get(button.index)?.guid;
    cell_of_guid(layout, guid)
}

/// Sets the text of the object with this GUID; a GUID the package lacks is left alone.
fn set_text(cx: &mut Cx, guid: u32, text: &str) {
    if let Some(o) = by_guid(cx, guid) {
        cx.rt.set_text(o, text.to_owned());
    }
}

fn focus(cx: &mut Cx, layout: &Layout, cell: Cell) {
    if let Some(o) = by_guid(cx, cell.guid(layout)) {
        cx.rt.focus_button(o);
    }
}

/// The cell one step away from `from` in a direction (clamped to the list).
fn step(from: Cell, message: u32) -> Option<Cell> {
    let last = ROW_COUNT - 1;
    Some(match message {
        PAD_UP => Cell { row: from.row.saturating_sub(1), ..from },
        PAD_DOWN => Cell { row: (from.row + 1).min(last), ..from },
        PAD_LEFT => Cell { slot: 0, ..from },
        PAD_RIGHT => Cell { slot: 1, ..from },
        _ => return None,
    })
}

/// The name a key shows as on the list, in capitals: the key's name (`KeyW` is `W`, `Space` is `SPACE`), with the long
/// ones shortened (`ArrowDown` is `DOWN`, `ControlLeft` is `L CTRL`) so they fit a key cap.
fn key_name(key: KeyCode) -> String {
    let id = match key {
        KeyCode::ArrowUp => "keys-up",
        KeyCode::ArrowDown => "keys-down",
        KeyCode::ArrowLeft => "keys-left",
        KeyCode::ArrowRight => "keys-right",
        KeyCode::ControlLeft => "keys-short-control-left",
        KeyCode::ControlRight => "keys-short-control-right",
        KeyCode::ShiftLeft => "keys-short-shift-left",
        KeyCode::ShiftRight => "keys-short-shift-right",
        KeyCode::AltLeft => "keys-short-alt-left",
        KeyCode::AltRight => "keys-short-alt-right",
        _ => return prompts::key_word(key).to_uppercase(),
    };
    i18n::text(id, &i18n::Args::new()).to_uppercase()
}

/// What a slot says while the next key pressed will be bound to it.
fn waiting_text() -> String {
    t!("keys-waiting").to_uppercase()
}

/// The key names of each row, primary then secondary, from the bindings. A slot with no key shows the placeholder, and the
/// slot `waiting` for its key says so.
pub fn key_texts(bindings: &Bindings, waiting: Option<(Control, usize)>) -> Vec<[String; 2]> {
    ROWS.iter()
        .map(|control| {
            let keys = bindings.keyboard_keys(*control);
            let text = |slot: usize| match waiting {
                Some((wanted, at)) if wanted == *control && at == slot => waiting_text(),
                _ => keys.get(slot).map_or_else(|| KEY_PLACEHOLDER.to_owned(), |k| key_name(*k)),
            };
            [text(0), text(1)]
        })
        .collect()
}

/// The controls list. It keeps the key names it last showed, so it only writes the ones that changed. Back returns
/// to the option categories it was opened from (`icon_menu.rs`), as the option lists do.
pub struct ControlsLogic {
    args: Args,
    layout: &'static Layout,
    shown: Vec<[String; 2]>,
    /// Whether the Remastered look draws this list (`Cx::vertical_look`), and the cell that has the focus.
    vertical: bool,
    cell: Cell,
}

impl ControlsLogic {
    /// The list for the package called `name` (`Options_PC_Controller.fng` or `Pause_PC_Controller.fng`).
    pub fn new(name: &str, args: Args) -> Self {
        Self {
            args,
            layout: controls_layout::for_package(name),
            shown: Vec::new(),
            vertical: false,
            cell: Cell { row: 0, slot: 0 },
        }
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
        self.vertical = cx.vertical_look();
        let layout = self.layout;
        for (row, label) in LABELS.iter().enumerate() {
            set_text(cx, layout.title[row], &i18n::game_or_error(*label));
        }
        focus(cx, layout, Cell { row: 0, slot: 0 });
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        let slot = match pick {
            Pick::Hover => self.cell.slot,
            Pick::Press(_, along) => usize::from(along >= 0.5),
        };
        self.cell = Cell { row: row.min(ROW_COUNT - 1), slot };
        focus(cx, self.layout, self.cell);
        if matches!(pick, Pick::Press(..)) {
            cx.send(Command::Remap(ROWS[self.cell.row], self.cell.slot));
            crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
        }
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let key = |row: usize, slot: usize| {
            self.shown.get(row).map_or_else(|| KEY_PLACEHOLDER.to_owned(), |texts| texts[slot].clone())
        };
        let entries = LABELS
            .iter()
            .enumerate()
            .map(|(row, label)| MenuEntry {
                label: i18n::game_or_error(*label),
                enabled: true,
                value: MenuValue::Keys { primary: key(row, 0), secondary: key(row, 1), focus: Some(self.cell.slot) },
                icon: None,
            })
            .collect();
        let style = if self.args.pause { MenuStyle::Pause } else { MenuStyle::Sub };
        Some(MenuList {
            title: i18n::game_or_error(TITLE),
            message: String::new(),
            entries,
            selected: self.cell.row,
            first: 0,
            hints: Vec::new(),
            style,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.vertical_look();
        if let Some(cell) = focused(cx, self.layout) {
            self.cell = cell;
        }
        if self.shown.as_slice() == cx.key_texts {
            return;
        }
        let layout = self.layout;
        for (row, texts) in cx.key_texts.iter().enumerate().take(ROW_COUNT) {
            set_text(cx, layout.primary_text[row], &texts[0]);
            set_text(cx, layout.secondary_text[row], &texts[1]);
        }
        self.shown = cx.key_texts.to_vec();
    }

    fn scroll(&mut self, cx: &mut Cx, steps: i32) {
        wheel_as_rows(self, cx, steps);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        let layout = self.layout;
        // The hint says "press ENTER to re-map". The pad accept reaches the field as its release (`BUTTON_RELEASED`),
        // and a mouse click as its press.
        if message == BUTTON_PRESSED || message == BUTTON_RELEASED {
            if let Some(cell) = focused(cx, layout) {
                cx.send(Command::Remap(ROWS[cell.row], cell.slot));
            }
            return;
        }
        if message == PAD_BACK {
            cx.send(self.back());
            return;
        }
        let from = focused(cx, layout).unwrap_or(Cell { row: 0, slot: 0 });
        if let Some(to) = step(from, message) {
            focus(cx, layout, to);
        }
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(step(top, PAD_UP), Some(top));
        assert_eq!(step(top, PAD_RIGHT), Some(Cell { row: 0, slot: 1 }));
        let bottom = Cell { row: ROW_COUNT - 1, slot: 1 };
        assert_eq!(step(bottom, PAD_DOWN), Some(bottom));
        assert_eq!(step(bottom, PAD_LEFT), Some(Cell { row: ROW_COUNT - 1, slot: 0 }));
        assert_eq!(step(top, BUTTON_PRESSED), None);
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
        assert_eq!(texts.len(), ROW_COUNT);
        assert!(texts.iter().all(|t| t[0] == KEY_PLACEHOLDER && t[1] == KEY_PLACEHOLDER));
    }
}
