//! Pointer selection must use the icon currently bound to a package slot, including a shifted window.

use super::{Cx, IconMenu, Pick, Vec2};
use crate::frontend::logic::Way;

impl IconMenu {
    pub(super) fn click_icon(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.vertical {
            return false;
        }
        let objects: Vec<_> = self.slots.iter().map(|slot| slot.obj).collect();
        let Some(hit) = cx.rt.object_at(&objects, point) else { return false };
        // Consume hits during exit too: generic button focus must not change the pending selection.
        if !self.reacts {
            return true;
        }
        let Some(slot) = self.slots.iter().position(|slot| slot.obj == hit) else { return true };
        let Some(row) = self.icon_at(self.base + slot) else { return true };
        self.pick(cx, row, Pick::Press(Way::Stay, 0.0));
        true
    }
}
