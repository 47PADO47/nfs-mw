//! The native HUD palette has a separate animated frame, rather than per-swatch border artwork.

use crate::frontend::logic::Cx;
use blackbox_feng::ObjectRef;

pub(super) fn follow(cx: &mut Cx, swatches: &[ObjectRef], selected: usize) {
    let Some(frame) = cx.object(0xb893_252a) else { return };
    let Some(chosen) = swatches.get(selected) else {
        cx.rt.set_hidden(frame, true);
        return;
    };
    let tree = cx.rt.tree(cx.package);
    let (Some(chosen), Some(border)) = (tree.nodes.get(chosen.index), tree.nodes.get(frame.index)) else {
        return;
    };
    let position = border
        .parent
        .and_then(|parent| tree.nodes.get(parent))
        .map_or(chosen.position, |parent| parent.world.inverse().transform_point3(chosen.position));
    cx.rt.set_position_xy(frame, position.x, position.y);
    cx.rt.set_hidden(frame, false);
    super::super::chrome::native_focus(cx, frame, true);
}
