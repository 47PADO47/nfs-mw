//! The shared cell the Custom radio card reads its state from.
//!
//! The same producer rule as the Original cells (`original/frame.rs`): the owning plugin copies the state in once per
//! frame, and the component only reads. The state is the card on the air (`hud::Card`), how long ago it was announced
//! (`RadioHud::age`) and where the card sits on the window (the anchor, computed from the screen and the HUD layout).
//! The copy is plain data, so the cell is `Send + Sync`, as `Theme` requires.
//!
//! The cell is shared by clone: `build_themes` gives the Custom theme one clone, and `UiThemes::radio` gives the plugin
//! another. Each card built reads whatever the plugin stored last.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::hud::{Card, RadioHud};
use crate::settings::HudLayout;
use crate::ui::present::Screen;

use super::radio_placement::{Anchor, anchor};

/// One frame's copy of the radio state. Empty (no card) draws nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RadioFeed {
    /// The song on the air, or `None` when the radio is idle.
    pub card: Option<Card>,
    /// Seconds since the card was announced (`RadioHud::age`).
    pub age: f32,
    /// Where the card sits (see `radio_placement`).
    pub anchor: Anchor,
}

/// The shared cell. Clones share one feed.
#[derive(Clone, Debug, Default)]
pub struct RadioFeedCell {
    feed: Arc<Mutex<RadioFeed>>,
}

impl RadioFeedCell {
    /// An empty cell: the card draws nothing until the plugin sets it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Copies this frame's radio state into the cell: the card on the air, its age, and the anchor for `screen` under
    /// `layout`. Call it once per frame after `radio::update` (the same `RadioHud`), with the window's `Screen` and
    /// the settings' `hud_layout`. Replaces the previous frame's copy.
    // wired into the HUD plugin in T12: until then nothing fills the cell and the Custom card draws nothing.
    #[allow(dead_code)]
    pub(crate) fn set(&self, hud: &RadioHud, screen: Screen, layout: HudLayout) {
        let feed = RadioFeed { card: hud.song().cloned(), age: hud.age(), anchor: anchor(screen, layout) };
        *self.lock() = feed;
    }

    /// This frame's copy, as the component reads it.
    pub(super) fn snapshot(&self) -> RadioFeed {
        self.lock().clone()
    }

    /// The feed lock. A poisoned lock still holds a whole feed (each write replaces all of it), so the poison is ignored.
    fn lock(&self) -> MutexGuard<'_, RadioFeed> {
        self.feed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
