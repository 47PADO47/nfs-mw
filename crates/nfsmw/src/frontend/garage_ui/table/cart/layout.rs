//! Cart icons align with the actual font glyphs, not an authored sample string's origin.
use crate::frontend::logic::Cx;
use blackbox_feng::{ObjectRef, font::TextStyle};

pub(super) fn label_center(cx: &Cx, label: ObjectRef) -> Option<f32> {
    let info = cx.rt.string_info(label)?;
    let font = cx.assets.font(info.font)?;
    let layout = font.layout_scaled(
        &info.text,
        TextStyle { justification: info.justification, leading: info.leading, max_width: info.max_width },
        cx.assets.texture_size(cx.assets.resolve(font.texture_hash))?,
        cx.assets.font_scale(font.hash),
    );
    let mut top = f32::INFINITY;
    let mut bottom = f32::NEG_INFINITY;
    for glyph in layout.quads {
        top = top.min(glyph.y0);
        bottom = bottom.max(glyph.y1);
    }
    if !top.is_finite() {
        return None;
    }
    Some(cx.rt.position(label)?.y + (top + bottom) * 0.5 * info.scale.1)
}
