//! Pointer selection must use the icon currently bound to a package slot, including a shifted window.

use super::{Cx, IconMenu, Pick, Vec2};
use crate::frontend::icon_strip::Hit;
use crate::frontend::logic::Way;

impl IconMenu {
    pub(super) fn click_icon(&mut self, cx: &mut Cx, point: Vec2) -> bool {
        if self.vertical {
            return false;
        }
        let Some(hit) = self.strip.hit(cx, point) else { return false };
        // Consume hits during exit too: generic button focus must not change the pending selection.
        if !self.reacts {
            return true;
        }
        if let Hit::Icon(row) = hit {
            self.pick(cx, row, Pick::Press(Way::Stay, 0.0));
        }
        true
    }
}
