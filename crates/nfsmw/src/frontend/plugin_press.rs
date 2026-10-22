//! What a mouse click on the menus presses, for the front end's frame system (`plugin.rs`).

use blackbox_feng::runtime::pad;
use glam::Vec2;

use super::flow::Frontend;
use super::logic::{Command, Env};
use crate::input::{Action, Bindings, InputDevice};

/// What a click on the menus presses.
pub(super) enum Press {
    /// Accept, held while the button is held.
    Accept,
    /// Back, once.
    Back,
    /// Quit, once, as the quit action does on the main menu.
    Quit,
    /// A face button, once, as the pad mask it presses (Game Stats on the main menu, Randomize on the Quick Play brief, Defaults,
    /// View Tutorial, Showcase).
    Button(u32),
    /// A screen's own part (an option row or its arrow), with what it queued.
    Screen(Vec<Command>),
    Nothing,
}

/// What a click at a point of the menus presses: a footer hint first, then the screen's own parts, then a button.
pub(super) fn press_at(
    fe: &mut Frontend,
    point: Vec2,
    bindings: &Bindings,
    device: InputDevice,
    env: &mut Env,
) -> Press {
    match fe.screens.hint_at(point, bindings, device) {
        Some(Action::MenuAccept) => return Press::Accept,
        Some(Action::MenuBack) => return Press::Back,
        Some(Action::MenuQuit) => return Press::Quit,
        Some(Action::MenuButton1) => return Press::Button((pad::BUTTON0 << 1) | (pad::BUTTON0 << 5)),
        Some(Action::MenuButton2) => return Press::Button(pad::BUTTON0 << 4),
        Some(Action::MenuButton3) => return Press::Button(pad::BUTTON0),
        _ => {}
    }
    if let Some(commands) = fe.screens.click(point, env) {
        return Press::Screen(commands);
    }
    if fe.screens.focus_button_at(point) {
        return Press::Accept;
    }
    if fe.screens.release_at(point, super::ids::MOUSE_LEFT_RELEASED) {
        return Press::Nothing;
    }
    Press::Nothing
}
