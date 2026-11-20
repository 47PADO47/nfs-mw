//! The dim panel under the screens opened over the game (`docs/specs/fe-package-footer.md` section 2).
//!
//! The original pushes `InGameBackground.fng` (a near-black panel over the whole screen) directly beneath the first
//! screen it opens while the game runs, and takes it away with that stack. Only the win photo is pushed without it; the
//! screens opened over the photo (its results) get it. The pause menus get it too when the stack is opened outside the
//! game (the tests open them alone).

use super::{Screen, Screens, shown};
use crate::frontend::ids::screen;
use crate::frontend::logic::Plain;

pub(super) const NAME: &str = "InGameBackground.fng";

impl Screens {
    /// Marks the stack as opened over the running game: the flow calls it before it opens a screen over the frozen game.
    /// It holds until the stack empties (`clear`, or the last screen closing).
    pub fn set_in_game(&mut self) {
        self.in_game = true;
    }

    /// Puts the dim panel on top of the stack, under the screen `name` that is about to open, when the rule asks for it.
    pub(super) fn open_in_game_background(&mut self, name: &str) {
        if !wants_background(name, self.in_game) || self.stack.iter().any(is_background) {
            return;
        }
        let Some(package) = self.catalog.find(NAME).cloned() else { return };
        let id = self.runtime.load(package);
        // Over the game the panel goes directly under the screen that opens (the photo can be under it); a pause menu
        // opened over other screens (the tests do) keeps it at the bottom of the stack, as before.
        let at = match self.in_game {
            true => self.stack.len(),
            false => 0,
        };
        self.stack.insert(
            at,
            Screen {
                id,
                name: NAME.into(),
                logic: Box::new(Plain),
                slot: None,
                look: None,
                pose: None,
                shown: shown::Shown::default(),
            },
        );
    }

    /// Takes the dim panel away once no screen is left above it; an empty stack is no longer over the game.
    pub(super) fn close_bare_background(&mut self) {
        while self.stack.last().is_some_and(is_background) {
            let Some(old) = self.stack.pop() else { return };
            self.runtime.unload(old.id);
        }
        if self.stack.is_empty() {
            self.in_game = false;
        }
    }

    pub(super) fn visible_screens(&self) -> impl Iterator<Item = &Screen> {
        let start = self.visible_start();
        self.stack
            .iter()
            .enumerate()
            .filter(move |(index, screen)| *index >= start || is_background(screen))
            .map(|(_, screen)| screen)
    }
}

/// Whether opening `name` puts the dim panel under it: every screen over the running game but the win photo, and the
/// pause menus always.
pub(super) fn wants_background(name: &str, in_game: bool) -> bool {
    if name.eq_ignore_ascii_case(NAME) || name.eq_ignore_ascii_case(screen::PHOTO) {
        return false;
    }
    in_game || name.to_ascii_lowercase().starts_with("pause_")
}

/// Stretches the dim panel across a window wider than 4:3, as the original draws its whole 640 x 480 front end stretched
/// to the window (install behaviour, not read in code): its side strips fade out towards x = +/-509, so at the window's
/// edge (x = +/-320 in the original's frame) the panel is still mostly dark. [ours] Only this package is stretched; the
/// other screens keep the remake's aspect handling.
pub(super) fn stretch_to_window(tree: &mut blackbox_feng::UiTree, aspect: f32) {
    let stretch = aspect / (4.0 / 3.0);
    if !stretch.is_finite() || stretch <= 1.0 {
        return;
    }
    let scale = glam::Mat4::from_scale(glam::Vec3::new(stretch, 1.0, 1.0));
    for node in &mut tree.nodes {
        node.world = scale * node.world;
    }
}

fn is_background(screen: &Screen) -> bool {
    screen.name.eq_ignore_ascii_case(NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_goes_under_screens_over_the_game_but_not_the_photo() {
        assert!(wants_background("SMS_Message.fng", true));
        assert!(wants_background("PostRace_Results.fng", true));
        assert!(!wants_background(screen::PHOTO, true), "the win photo is pushed without the panel");
        assert!(!wants_background("SMS_Message.fng", false), "menus outside the game have none");
        assert!(wants_background("Pause_Main.fng", false));
        assert!(!wants_background(NAME, true));
    }

    #[test]
    fn the_panel_is_stretched_to_a_wide_window_only() {
        let mut tree = blackbox_feng::UiTree::default();
        tree.nodes.push(blackbox_feng::UiNode {
            world: glam::Mat4::from_translation(glam::Vec3::new(382.0, 0.0, 1.0)),
            ..Default::default()
        });
        stretch_to_window(&mut tree, 4.0 / 3.0);
        assert_eq!(tree.nodes[0].world.w_axis.x, 382.0, "4:3 is the original's frame");
        stretch_to_window(&mut tree, 16.0 / 9.0);
        assert!((tree.nodes[0].world.w_axis.x - 382.0 * 4.0 / 3.0).abs() < 1e-3);
    }
}
