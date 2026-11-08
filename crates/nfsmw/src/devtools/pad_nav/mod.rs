//! Navigating and closing every developer panel with a controller (`docs/devtools-vehicle.md`).
//!
//! The panels are egui windows. egui already moves a keyboard focus between widgets with the arrow keys and Tab and presses the
//! focused one with Enter, and a slider or a stepper changes with Left and Right; the controller is those keys. The D-pad and
//! the left stick are the arrows (they repeat while held), A is Enter, the bumpers are Tab and Shift+Tab, and B closes a popup or
//! leaves a text field, else the panel (back to the devtools menu when the menu opened it). Both stick clicks together, L3 and R3,
//! open and close the devtools menu. While a panel is up the game does not read the controller: the panels set `UiFocus`
//! (`ui_gate.rs`, `panels.rs`).
//!
//! [`PadNav`] is the logic, as plain data (`state.rs`); [`collect`] reads the controllers and hands its result to the panels.

mod state;
mod system;
#[cfg(test)]
mod tests;

pub(crate) use state::Button;
pub(super) use system::{collect, follow_focus};

/// A scroll area that follows the keyboard focus: `area.show(ui, add)`, and the focused widget kept in view ([`follow_focus`]). The
/// panels the controller walks use it in place of `show`.
pub(crate) fn scrolled<R>(
    area: egui::ScrollArea,
    ui: &mut egui::Ui,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    area.show(ui, |ui| {
        let out = add(ui);
        follow_focus(ui);
        out
    })
}
