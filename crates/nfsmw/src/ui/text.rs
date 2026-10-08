//! Measuring strings the way the screens' layout code needs.

use blackbox_feng::font::TextStyle;
use blackbox_feng::{ObjectRef, Runtime};
use glam::Vec2;

use super::UiAssets;

/// The width and height a string object's text takes (its scale applied, no maximum width); zero when the
/// object is not a string or its font is missing.
pub fn text_size(rt: &Runtime, assets: &UiAssets, o: ObjectRef) -> Vec2 {
    let Some(info) = rt.string_info(o) else { return Vec2::ZERO };
    let Some(font) = assets.font(info.font) else { return Vec2::ZERO };
    let texture = assets.texture_size(assets.resolve(font.texture_hash)).unwrap_or((256, 256));
    let style = TextStyle { justification: 0, leading: info.leading, max_width: 0 };
    let layout = font.layout(&info.text, style, texture);
    Vec2::new(layout.width * info.scale.0, layout.height * info.scale.1)
}
