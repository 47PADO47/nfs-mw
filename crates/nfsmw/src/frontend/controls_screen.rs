//! The keyboard controls list, in its two packages: `Options_PC_Controller.fng` (the main menu's) and
//! `Pause_PC_Controller.fng` (the in-game copy). Each draws eight rows, each with a primary and a secondary button; the
//! GUIDs of each package are in `controls_layout.rs`. A press on a button asks the frontend to take the next key for
//! that slot (`Command::Remap`). The packages' rows and buttons are in docs/specs/controls-remap.md.
//!
//! A list look (Remastered or Custom) publishes the list instead of drawing the package, and lists the HUD's actions
//! after the package's rows ([`HUD_ROWS`]). Those rows have no button in the package, so the list keeps its own focus
//! then, scrolled through the list's window of rows.

use blackbox_feng::ObjectRef;
use blackbox_feng::ids::{BUTTON_PRESSED, BUTTON_RELEASED, PAD_BACK, PAD_DOWN, PAD_LEFT, PAD_RIGHT, PAD_UP};

use bevy_input::keyboard::KeyCode;

use crate::input::{Action, Bindings, Control, hud_labels};

use super::controls_layout::{self, Layout, ROW_COUNT};
use super::ids::screen;
use super::logic::{Args, Command, Cx, ListScreen, Pick, ScreenLogic, wheel_as_rows};
use crate::i18n::{self, t};
use crate::ui::theme::remastered::menu::layout::window_start;
use crate::ui::theme::{DEFAULT_ASPECT, MenuEntry, MenuList, MenuStyle, MenuValue};
use crate::ui::{UiAssets, prompts};

/// The control each row changes, from the top. Steering is one axis, so its two rows are its two directions.
///
/// The package draws eight rows, so the original list has no row for the other actions: Map (`M`, the world map) and
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

/// The HUD's actions, the rows a list look adds after the package's, named by `input/hud_labels.rs`. The Original look
/// draws the package, which has no buttons for them.
pub const HUD_ROWS: [Control; 7] = [
    Control::button(Action::MapMode),
    Control::button(Action::MapExpand),
    Control::button(Action::MapZoomIn),
    Control::button(Action::MapZoomOut),
    Control::button(Action::MapOrientation),
    Control::button(Action::ToggleHud),
    Control::button(Action::DismissNotification),
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

/// The rows of the list: the package's eight, and in a list look the HUD's after them.
fn row_count(list_look: bool) -> usize {
    match list_look {
        true => ROW_COUNT + HUD_ROWS.len(),
        false => ROW_COUNT,
    }
}

/// The control of a row: the package's rows first, then the HUD's.
fn control(row: usize) -> Option<Control> {
    ROWS.iter().chain(&HUD_ROWS).nth(row).copied()
}

/// A row's name: the game's for the package's rows, the catalogue's for the HUD's.
fn row_label(row: usize) -> String {
    if let Some(label) = LABELS.get(row) {
        return i18n::game_or_error(*label);
    }
    control(row).and_then(|control| hud_labels::label(control.action)).unwrap_or_default()
}

/// A button of the list: the row, and the slot (0 primary, 1 secondary).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cell {
    row: usize,
    slot: usize,
}

impl Cell {
    /// The package's button for this cell; a HUD row has none.
    fn guid(self, layout: &Layout) -> Option<u32> {
        let column = match self.slot {
            0 => &layout.primary,
            _ => &layout.secondary,
        };
        column.get(self.row).copied()
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

/// Focuses the package's button of a cell; a HUD row's cell leaves the package's focus where it was.
fn focus(cx: &mut Cx, layout: &Layout, cell: Cell) {
    let Some(o) = cell.guid(layout).and_then(|guid| by_guid(cx, guid)) else { return };
    cx.rt.focus_button(o);
}

/// The cell one step away from `from` in a direction, clamped to a list of `rows` rows.
fn step(from: Cell, message: u32, rows: usize) -> Option<Cell> {
    let last = rows.saturating_sub(1);
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

/// The key names of each row, primary then secondary, from the bindings: the package's rows, then the HUD's. A slot
/// with no key shows the placeholder, and the slot `waiting` for its key says so.
pub fn key_texts(bindings: &Bindings, waiting: Option<(Control, usize)>) -> Vec<[String; 2]> {
    ROWS.iter()
        .chain(&HUD_ROWS)
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
    /// Whether a list look draws this list (`Cx::screen_list_look`), and the cell that has the focus. In a list look the
    /// cell is the list's own, since the HUD's rows have no button in the package.
    vertical: bool,
    cell: Cell,
    /// The first row of the list's window.
    top: usize,
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
            top: 0,
        }
    }

    fn back(&self) -> Command {
        if self.args.pause {
            return Command::Switch(screen::PAUSE_MENU.into(), Args { pause: true, options: true, ..self.args });
        }
        Command::Switch(screen::MAIN_MENU_SUB.into(), Args { options: true, ..self.args })
    }

    /// The first row of the window that shows the focused row.
    fn first(&self) -> usize {
        window_start(self.top, self.cell.row, row_count(self.vertical))
    }

    /// The cell a press changes: the list's own in a list look, else the package's focused button.
    fn pressed_cell(&self, cx: &Cx) -> Option<Cell> {
        match self.vertical {
            true => Some(self.cell),
            false => focused(cx, self.layout),
        }
    }
}

impl ScreenLogic for ControlsLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.screen_list_look(ListScreen::Controls);
        let layout = self.layout;
        for (row, label) in LABELS.iter().enumerate() {
            set_text(cx, layout.title[row], &i18n::game_or_error(*label));
        }
        self.cell = Cell { row: 0, slot: 0 };
        self.top = 0;
        focus(cx, layout, self.cell);
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn pick_row(&mut self, cx: &mut Cx, row: usize, pick: Pick) {
        let slot = match pick {
            Pick::Hover => self.cell.slot,
            Pick::Press(_, along) => usize::from(along >= 0.5),
        };
        self.cell = Cell { row: row.min(row_count(self.vertical) - 1), slot };
        focus(cx, self.layout, self.cell);
        if !matches!(pick, Pick::Press(..)) {
            return;
        }
        let Some(control) = control(self.cell.row) else { return };
        cx.send(Command::Remap(control, self.cell.slot));
        crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
    }

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let key = |row: usize, slot: usize| {
            self.shown.get(row).map_or_else(|| KEY_PLACEHOLDER.to_owned(), |texts| texts[slot].clone())
        };
        let entries = (0..row_count(true))
            .map(|row| MenuEntry {
                label: row_label(row),
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
            first: self.first(),
            hints: Vec::new(),
            style,
            nudge: None,
            scroll: 0.0,
            aspect: DEFAULT_ASPECT,
        })
    }

    fn tick(&mut self, cx: &mut Cx, _dt: f32) {
        self.vertical = cx.screen_list_look(ListScreen::Controls);
        if !self.vertical
            && let Some(cell) = focused(cx, self.layout)
        {
            self.cell = cell;
        }
        self.top = self.first();
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
            if let Some(cell) = self.pressed_cell(cx)
                && let Some(control) = control(cell.row)
            {
                cx.send(Command::Remap(control, cell.slot));
            }
            return;
        }
        if message == PAD_BACK {
            cx.send(self.back());
            return;
        }
        let from = self.pressed_cell(cx).unwrap_or(Cell { row: 0, slot: 0 });
        let Some(to) = step(from, message, row_count(self.vertical)) else { return };
        self.cell = to;
        focus(cx, layout, to);
    }
}

#[cfg(test)]
#[path = "controls_screen_tests.rs"]
mod tests;
