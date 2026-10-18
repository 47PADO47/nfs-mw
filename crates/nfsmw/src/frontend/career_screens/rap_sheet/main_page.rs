//! The rap sheet main page (`RapSheetMain.fng`): the case file header, the figures of the career and the six page
//! buttons. A button plays the package's leave animation and opens its page with the safe house arguments; Back
//! returns to the safe house menu. The remembered button is kept for the session (`docs/specs/career-rapsheet.md`
//! sections 1.3 and 3).

use blackbox_feng::ids::{BUTTON_PRESSED, PAD_BACK};
use nfsmw_career::rapsheet::{Page, RapTotals, highlight_button, main_button_page};

use super::open;
use crate::career::CareerHandle;
use crate::frontend::ids::{EXIT_COMPLETE, INIT_COMPLETE};
use crate::frontend::infractions::money;
use crate::frontend::logic::{Cx, ScreenLogic};

/// The six page buttons, `BUTTON_1` to `BUTTON_6` (`fe_hash_upper`), in number order.
const BUTTONS: [u32; 6] = [0xCDA0_A66B, 0xCDA0_A66C, 0xCDA0_A66D, 0xCDA0_A66E, 0xCDA0_A66F, 0xCDA0_A670];

/// The header and figure objects (`fe_hash_upper` of their names).
const CASE_FILE: u32 = 0x1232_703A;
const PLAYER_NAME: u32 = 0xE3DA_78E7;
const TOTAL_BOUNTY: u32 = 0xE3DA_78E8;
const TOTAL_COST_STATE: u32 = 0xE3DA_78E9;
const TOTAL_CARS_IMPOUNDED: u32 = 0xE3DA_78EA;
const TOTAL_FINES_DUE: u32 = 0xE3DA_78EB;
const TOTAL_EVADED: u32 = 0xE3DA_78EC;
const TOTAL_BUSTED: u32 = 0xE3DA_78ED;
const CARS_MONITORED: u32 = 0xE3DA_78EE;

/// The key of the remembered button in the screen memory (`Remembered::main`, default 1).
const MEMORY_KEY: &str = "RapSheetMain";

/// The button object of a button number 1 to 6.
pub fn button_hash(number: u8) -> Option<u32> {
    let index = usize::from(number.checked_sub(1)?);
    BUTTONS.get(index).copied()
}

/// The printed serial: the 16 bytes up to the first zero.
pub fn serial_text(serial: &[u8; 16]) -> String {
    let end = serial.iter().position(|&b| b == 0).unwrap_or(serial.len());
    String::from_utf8_lossy(&serial[..end]).into_owned()
}

/// The text of each header and figure object. The wording of the labels other than the case file, the name and the
/// bounty is taken from the label keys (`RAP_TOTAL_*`, `RAP_CARS_MONITORED`) and is not checked against the install.
pub fn figures(totals: &RapTotals, serial: &str, name: &str) -> Vec<(u32, String)> {
    let bounty = u32::try_from(totals.bounty).unwrap_or(u32::MAX);
    let fines_due = u32::try_from(totals.fines_due).unwrap_or(u32::MAX);
    vec![
        (CASE_FILE, format!("CASE FILE: {serial}")),
        (PLAYER_NAME, format!("NAME: {name}")),
        (TOTAL_BOUNTY, format!("BOUNTY: {}", money(bounty))),
        (TOTAL_COST_STATE, format!("COST TO STATE: {}", money(totals.cost_to_state))),
        (TOTAL_CARS_IMPOUNDED, format!("CARS IMPOUNDED: {}", totals.cars_impounded)),
        (TOTAL_FINES_DUE, format!("FINES DUE: {}", money(fines_due))),
        (TOTAL_EVADED, format!("TIMES EVADED: {}", totals.times_evaded)),
        (TOTAL_BUSTED, format!("TIMES BUSTED: {}", totals.times_busted)),
        (CARS_MONITORED, format!("CARS MONITORED: {}", totals.cars_monitored)),
    ]
}

/// Where a leave goes and which button it remembers: a pressed button opens its page, anything else (Back, or a
/// leave without a press) goes to the safe house menu and remembers button 1.
pub fn destination(pressed: Option<u8>) -> (Page, u8) {
    let Some(button) = pressed else { return (Page::SafeHouseMenu, 1) };
    (main_button_page(button), button)
}

/// `RapSheetMain.fng`.
#[derive(Default)]
pub struct MainPage {
    /// The button pressed, waiting for the leave animation to end. `None` after Back.
    pressed: Option<u8>,
}

impl MainPage {
    fn fill(&self, cx: &mut Cx) {
        let texts = CareerHandle::shared()
            .with(|career| {
                let totals = RapTotals::compute(&career.stable, &career.history);
                figures(&totals, &serial_text(&career.settings.serial), &career.alias)
            })
            .unwrap_or_default();
        for (hash, text) in texts {
            let Some(o) = cx.object(hash) else { continue };
            cx.rt.set_text(o, text);
        }
    }

    /// Highlights the remembered button (1 when none was remembered).
    fn highlight(&self, cx: &mut Cx) {
        let remembered = cx.memory.get(MEMORY_KEY).copied().unwrap_or(0);
        let button = highlight_button(u8::try_from(remembered).unwrap_or(0));
        let Some(hash) = button_hash(button) else { return };
        let Some(o) = cx.object(hash) else { return };
        cx.rt.focus_button(o);
    }

    /// The button number of the focused button, when it is one of the six.
    fn pressed_button(cx: &Cx) -> Option<u8> {
        let focus = cx.rt.current_button(cx.package)?;
        (1..=6u8).find(|&number| button_hash(number).and_then(|hash| cx.object(hash)) == Some(focus))
    }

    fn exit(&self, cx: &mut Cx, pressed: Option<u8>) {
        let (page, button) = destination(pressed);
        cx.memory.insert(MEMORY_KEY.to_owned(), usize::from(button));
        cx.send(open(page));
    }
}

impl ScreenLogic for MainPage {
    fn start(&mut self, cx: &mut Cx) {
        self.fill(cx);
        self.highlight(cx);
    }

    fn message(&mut self, cx: &mut Cx, message: u32) {
        match message {
            INIT_COMPLETE => self.highlight(cx),
            BUTTON_PRESSED => {
                let Some(number) = Self::pressed_button(cx) else { return };
                self.pressed = Some(number);
            }
            PAD_BACK => self.pressed = None,
            EXIT_COMPLETE => {
                let pressed = self.pressed.take();
                self.exit(cx, pressed);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "main_page_tests.rs"]
mod tests;
