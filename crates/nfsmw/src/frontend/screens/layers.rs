//! The composed layers of the menu slots, kept from one frame to the next.
//!
//! Under the Original theme a menu slot's layer is a copy of the screen's tree (`ui/theme/original/menu.rs`), so it
//! changes only when that tree does. A tree has a serial that is never used twice (`Presented::serial`); a layer composed
//! from the tree with serial N is the layer for any later frame that shows serial N in the same slot. A theme that draws
//! its menus from lists and the clock composes every frame (`UiThemes::menus_are_static`).

use std::sync::Arc;

use blackbox_ui::UiTree;

use crate::ui::theme::ElementId;

#[derive(Default)]
pub struct MenuLayers {
    /// The layer last composed for each slot, with the serial of the tree it was composed from.
    layers: Vec<(ElementId, u64, Arc<UiTree>)>,
}

impl MenuLayers {
    /// The layer of `slot` for the tree with `serial`. `compose` runs unless the layer kept from an earlier frame was
    /// composed from the same tree and the theme is `reusable` (the layer reads nothing but the tree).
    pub fn layer(
        &mut self,
        slot: ElementId,
        serial: u64,
        reusable: bool,
        compose: impl FnOnce() -> UiTree,
    ) -> Arc<UiTree> {
        if !reusable {
            self.layers.clear();
            return Arc::new(compose());
        }
        if let Some((_, _, layer)) = self.layers.iter().find(|(s, n, _)| *s == slot && *n == serial) {
            return layer.clone();
        }
        let layer = Arc::new(compose());
        self.layers.retain(|(s, _, _)| *s != slot);
        self.layers.push((slot, serial, layer.clone()));
        layer
    }

    /// Forgets the slots that no screen took this frame, so a closed screen's layer is not kept.
    pub fn keep_only(&mut self, taken: &[ElementId]) {
        self.layers.retain(|(slot, _, _)| taken.contains(slot));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_of(count: usize) -> UiTree {
        UiTree { nodes: vec![Default::default(); count], draw_order: Vec::new() }
    }

    #[test]
    fn the_same_serial_composes_once() {
        let mut layers = MenuLayers::default();
        let mut composed = 0;
        let mut ask = |layers: &mut MenuLayers, serial| {
            layers.layer(ElementId::MainMenu, serial, true, || {
                composed += 1;
                tree_of(2)
            })
        };
        let first = ask(&mut layers, 1);
        let again = ask(&mut layers, 1);
        assert!(Arc::ptr_eq(&first, &again), "the layer is handed on");
        ask(&mut layers, 2);
        assert_eq!(composed, 2, "a new tree composes again");
    }

    #[test]
    fn a_live_theme_composes_every_frame() {
        let mut layers = MenuLayers::default();
        let mut composed = 0;
        for _ in 0..3 {
            layers.layer(ElementId::MainMenu, 1, false, || {
                composed += 1;
                tree_of(1)
            });
        }
        assert_eq!(composed, 3);
    }

    #[test]
    fn a_slot_nobody_took_is_forgotten() {
        let mut layers = MenuLayers::default();
        layers.layer(ElementId::MainMenu, 1, true, || tree_of(1));
        layers.layer(ElementId::Dialog, 2, true, || tree_of(1));
        layers.keep_only(&[ElementId::Dialog]);
        let mut composed = false;
        layers.layer(ElementId::MainMenu, 1, true, || {
            composed = true;
            tree_of(1)
        });
        assert!(composed, "the main menu's layer was dropped");
        let mut kept = true;
        layers.layer(ElementId::Dialog, 2, true, || {
            kept = false;
            tree_of(1)
        });
        assert!(kept, "the dialog's layer stayed");
    }
}
