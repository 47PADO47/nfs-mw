//! Who draws the world's transient cards (the entry card, a mod's line, the end of a chase, the speed-trap points): the
//! world's egui cards, or the active theme's own HUD components. The Custom HUD (and a mod theme with tokens, which
//! shares its components) draws them from the HUD state (`ui/theme/custom`), so the world leaves its egui cards out;
//! Original and Remastered keep them.
//!
//! The HUD plugin decides it each frame from the theme that fills the `Banner` slot (`plugin.rs`, `present`); the world
//! reads it when it draws its egui panels (`scenes/world/scene_hud.rs`). The two run on different threads, so the flag is
//! an atomic. A frame late at a style change is harmless: both looks read the same state.

use std::sync::atomic::{AtomicBool, Ordering};

static ROUTED: AtomicBool = AtomicBool::new(false);

/// Records whether the active theme draws the cards itself.
pub fn set_cards_routed(routed: bool) {
    ROUTED.store(routed, Ordering::Relaxed);
}

/// Whether the active theme draws the cards itself, so the world's egui cards stay out.
pub fn cards_routed() -> bool {
    ROUTED.load(Ordering::Relaxed)
}
