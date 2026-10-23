//! The sound a click makes. A pointer press plays the button effect only when it did something, so a click on a
//! greyed icon, a locked event or a shut screen stays silent; every screen decides that the same way.

use super::Cx;

impl Cx<'_> {
    /// Runs `act`, the press a click stands for, and plays the button-press effect if it sent a command.
    pub fn press_sound(&mut self, act: impl FnOnce(&mut Self)) {
        let before = self.commands.len();
        act(self);
        if self.commands.len() != before {
            crate::audio::menu::request(blackbox_feng::ids::BUTTON_PRESSED);
        }
    }
}
