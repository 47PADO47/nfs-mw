//! The credits screen (`Credits.fng`), opened from the option categories' Credits icon. The roll is the install's
//! credits file (`CREDITS/NA_ENGLISH.TXT`) in the package's text object `CreditsArea`. The package's own roll scripts
//! do not start in this port and the renderer does not clip text, so the logic scrolls a window of lines through the
//! box itself (docs/specs/frontend-menus.md, "Credits"). Back returns to the option categories.

use std::sync::OnceLock;

use blackbox_feng::ids::PAD_BACK;
use game_install::GameDir;
use glam::Vec2;

use super::ids::{self, screen};
use super::logic::{Args, Category, Command, Cx, Pick, ScreenLogic};
use crate::ui::UiAssets;
use crate::ui::theme::{DEFAULT_ASPECT, MenuList, MenuStyle};

/// The roll's text object in the package: `CreditsArea` (`fe_hash_upper`).
const ROLL: u32 = 0xE609_3486;
/// Lines shown in the box at once.
const WINDOW: usize = 12;
/// Lines of the roll the Remastered look shows at once, one more than fits so the roll can slide in and out.
const LIST_WINDOW: usize = 16;
/// How fast the roll moves, in lines per second.
const LINES_PER_SECOND: f32 = 2.0;
/// The visible Back button: its centre and half size on the screen (FEng units). Its picture is the package's footer
/// label (`0xF8107`), which has no mouse answer of its own.
const BACK_CENTRE: Vec2 = Vec2::new(-245.0, 208.0);
const BACK_HALF: Vec2 = Vec2::new(50.0, 20.0);
/// The box's centre on the screen (FEng units, y points down), where the window's text is placed.
const BOX_CENTRE_Y: f32 = -11.0;

/// The credits file's lines, read once when the front end starts (`prime`). Empty when the install has none.
static LINES: OnceLock<Vec<String>> = OnceLock::new();

/// Reads the credits file for the roll. A failure leaves the roll empty.
pub fn prime(dir: &GameDir) {
    let lines = match read_text(dir) {
        Ok(text) => text.lines().map(str::to_owned).collect(),
        Err(e) => {
            log::warn!("the credits have no text: {e}");
            Vec::new()
        }
    };
    let _ = LINES.set(lines);
}

fn read_text(dir: &GameDir) -> Result<String, String> {
    let bytes = dir.read("CREDITS/NA_ENGLISH.TXT").map_err(|e| format!("CREDITS/NA_ENGLISH.TXT: {e:#}"))?;
    Ok(decode_utf16(&bytes).replace("\r\n", "\n"))
}

/// UTF-16LE with a byte order mark, as the credits file is written.
fn decode_utf16(bytes: &[u8]) -> String {
    let body = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
    let units: Vec<u16> = body.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
    String::from_utf16_lossy(&units)
}

/// The window of the roll that is shown after `first` lines have scrolled past.
fn window(lines: &[String], first: usize) -> String {
    let end = (first + WINDOW).min(lines.len());
    lines.get(first..end).map_or_else(String::new, |w| w.join("\n"))
}

/// Shows the credits roll and goes back to the option categories when Back is pressed.
#[derive(Default)]
pub struct CreditsLogic {
    lines: Vec<String>,
    elapsed: f32,
    /// The first line the window shows, once the text was set.
    shown: Option<usize>,
    /// Whether the Remastered look draws the roll (`Cx::vertical_look`).
    vertical: bool,
}

impl CreditsLogic {
    fn set_window(&mut self, cx: &mut Cx, first: usize) {
        self.shown = Some(first);
        if let Some(roll) = cx.object(ROLL) {
            cx.rt.set_text(roll, window(&self.lines, first));
        }
    }
}

impl ScreenLogic for CreditsLogic {
    fn start(&mut self, cx: &mut Cx) {
        self.vertical = cx.vertical_look();
        self.lines = LINES.get().cloned().unwrap_or_default();
        self.set_window(cx, 0);
        // The package starts with its buttons off, so Back does nothing until it is told to switch them on.
        cx.rt.post_to_package(cx.package, ids::INPUT_ENABLE);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        if message != PAD_BACK {
            return;
        }
        let args = Args { options: true, category: Category::Audio, ..Args::default() };
        cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), args));
    }

    fn vertical(&self) -> bool {
        self.vertical
    }

    fn pick_row(&mut self, _cx: &mut Cx, _row: usize, _pick: Pick) {}

    fn list(&self, _assets: &UiAssets) -> Option<MenuList> {
        if !self.vertical {
            return None;
        }
        let position = self.elapsed * LINES_PER_SECOND;
        let first = position as usize;
        let end = (first + LIST_WINDOW).min(self.lines.len());
        let message = self.lines.get(first..end).map_or_else(String::new, |lines| lines.join("\n"));
        Some(MenuList {
            title: "Credits".into(),
            message,
            entries: Vec::new(),
            selected: 0,
            first: 0,
            hints: Vec::new(),
            style: MenuStyle::Credits,
            nudge: None,
            scroll: position.fract(),
            aspect: DEFAULT_ASPECT,
        })
    }

    fn click(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.vertical || !(point - BACK_CENTRE).abs().cmple(BACK_HALF).all() {
            return false;
        }
        let args = Args { options: true, category: Category::Audio, ..Args::default() };
        cx.send(Command::Switch(screen::MAIN_MENU_SUB.into(), args));
        true
    }

    fn tick(&mut self, cx: &mut Cx, dt: f32) {
        self.vertical = cx.vertical_look();
        self.elapsed += dt;
        let first = (self.elapsed * LINES_PER_SECOND) as usize;
        if self.shown != Some(first) {
            self.set_window(cx, first);
        }
        if let Some(roll) = cx.object(ROLL) {
            cx.rt.set_position_xy(roll, 0.0, BOX_CENTRE_Y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decode_utf16, window};

    #[test]
    fn the_credits_file_is_utf16_with_a_bom() {
        let bytes = [0xFF, 0xFE, b'N', 0, b'e', 0, b'\r', 0, b'\n', 0];
        assert_eq!(decode_utf16(&bytes), "Ne\r\n");
    }

    #[test]
    fn the_window_scrolls_through_the_lines_and_ends_with_them() {
        let lines: Vec<String> = (0..20).map(|i| i.to_string()).collect();
        assert_eq!(window(&lines, 0).lines().count(), 12);
        assert_eq!(window(&lines, 15), "15\n16\n17\n18\n19");
        assert_eq!(window(&lines, 25), "");
    }
}
