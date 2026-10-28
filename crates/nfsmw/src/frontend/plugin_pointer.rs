//! Where a pointer press lands on the front end: the world map picture and the screens' buttons.

use super::*;

/// Whether the pointer is over the world map's picture, where the map draws its own cross cursor. Outside it (the
/// buttons, the list) the system pointer shows, so it can click. `app::cursor::update` owns the pointer's visibility
/// and asks this.
pub fn map_pointer_on_picture(fe: &Frontend, window: &Window) -> bool {
    let map_on_top =
        matches!(fe.stage, Stage::Map) && fe.screens.top() == Some(crate::frontend::ids::screen::WORLD_MAP);
    if !map_on_top {
        return false;
    }
    // The themed full-screen map draws its own cursor over the whole window.
    if fe.map_chrome == Chrome::Drawn {
        return true;
    }
    feng_point(window)
        .map(layout::view_of)
        .is_some_and(|view| (0.0..=1.0).contains(&view[0]) && (0.0..=1.0).contains(&view[1]))
}

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
        Some(Action::JukeboxGrab) => return Press::Button(pad::BUTTON0 << 5),
        Some(Action::MenuTriggerLeft) => return Press::Button(pad::LTRIGGER),
        Some(Action::MenuTriggerRight) => return Press::Button(pad::RTRIGGER),
        _ => {}
    }
    if let Some(commands) = fe.screens.click(point, env) {
        return Press::Screen(commands);
    }
    if fe.screens.focus_button_at(point) {
        return Press::Accept;
    }
    if fe.screens.release_at(point, crate::frontend::ids::MOUSE_LEFT_RELEASED) {
        return Press::Nothing;
    }
    Press::Nothing
}

#[cfg(test)]
#[path = "plugin_pointer_jukebox_tests.rs"]
mod jukebox_tests;
