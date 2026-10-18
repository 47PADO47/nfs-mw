//! The composer: the trees the presenters draw, built from the slots the active theme resolves.
//!
//! ADR 0002 says there is one presenter and one neutral tree. The composer is a second producer of that tree, not a
//! second draw path. Each presented **layer** is one `UiTree` and one presenter call, in the order the presenters have
//! always drawn them, so the layers stack as before (`docs/specs/ui-themes-compose.md`):
//!
//! 1. the radio layer: the `RadioCard` slot (the EA Trax chyron under Original, the card under Custom);
//! 2. the HUD layer: the nine HUD slots and the HUD nodes no slot takes ([`hud_layer`]);
//! 3. the busted overlay layer: the `BustedOverlay` slot, drawn over the HUD in the frame's full screen;
//! 4. one layer per visible menu screen: its menu slot, or its own tree when no slot takes it.
//!
//! Each layer is built the same way: resolve the slot, lay it out, build it into one `DrawList`, put the nodes in
//! object order (`DrawList::sort_by_index`, so ties in `z` draw as the package does), and make the draw order
//! (`into_tree` with the runtime's own rule, `blackbox_feng::tree::is_drawn`). Under the Original theme the result is
//! the tree the plugin stored, node for node and in the same draw order (`compose_tests.rs`).

use blackbox_feng::tree::is_drawn;
use blackbox_ui::{DrawList, UiTree};

use super::original::{HUD_SLOTS, emit_unclaimed};
use super::{ElementId, UiThemes};

/// The HUD layer: every HUD slot the theme resolves, then the nodes no HUD slot takes. The composed tree is in the HUD
/// package's object order. Reads the HUD tree from the frame cell, so the plugin must have stored it this frame.
pub fn hud_layer(themes: &UiThemes, time_s: f32) -> UiTree {
    let mut list = DrawList::new();
    for slot in HUD_SLOTS {
        themes.build_slot(slot, time_s, &mut list);
    }
    emit_unclaimed(&themes.frame(), &mut list);
    finish(list)
}

/// The layer of one slot: the radio card, the busted overlay, or a menu screen. The composed tree of a menu slot is the
/// screen's whole tree, in its object order.
pub fn slot_layer(themes: &UiThemes, slot: ElementId, time_s: f32) -> UiTree {
    let mut list = DrawList::new();
    themes.build_slot(slot, time_s, &mut list);
    finish(list)
}

/// Puts a layer's nodes in object order and makes its draw order, with the runtime's own visibility rule.
fn finish(mut list: DrawList) -> UiTree {
    list.sort_by_index();
    list.into_tree(is_drawn)
}
