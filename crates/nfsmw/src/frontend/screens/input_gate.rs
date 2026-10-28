//! Host-owned transitions also gate semantic input and its automatic sound feedback.

use blackbox_feng::{PackageId, ids};

use super::Screens;
use crate::frontend::logic::Footer;

impl Screens {
    pub(super) fn top_accepts_input(&self) -> bool {
        !self.await_release && self.stack.last().is_some_and(|top| top.logic.accepts_input())
    }

    pub(super) fn sound_allowed(&self, package: PackageId, message: u32) -> bool {
        // Showcase directions operate its camera. Its authored responses do not request navigation sounds.
        let orbit = self.stack.iter().any(|screen| screen.id == package && screen.logic.footer() == Footer::Orbit);
        !orbit || !direction(message)
    }
}

fn direction(message: u32) -> bool {
    matches!(
        message,
        ids::PAD_UP
            | ids::PAD_DOWN
            | ids::PAD_LEFT
            | ids::PAD_RIGHT
            | ids::PAD_UPLEFT
            | ids::PAD_UPRIGHT
            | ids::PAD_DOWNLEFT
            | ids::PAD_DOWNRIGHT
    )
}
