//! Splits a text into the lines of a column of string objects, as a screen that shows a long text on one string per
//! line does (the tuning help, the SMS message): each line is as wide as the first string's maximum width allows, measured
//! with its installed font.

use blackbox_feng::font::TextStyle;
use blackbox_feng::runtime::ObjectRef;

use super::logic::Cx;

/// The lines of `text` for the string `object`: words are kept whole, `^` and new lines break a line. Without the
/// object or its font the text is split at its line breaks only.
pub fn wrap(cx: &Cx, object: Option<ObjectRef>, text: &str) -> Vec<String> {
    let unwrapped = || text.split(['^', '\n']).map(str::to_owned).collect();
    let Some(info) = object.and_then(|object| cx.rt.string_info(object)) else { return unwrapped() };
    let Some(font) = cx.assets.font(info.font) else { return unwrapped() };
    let size = cx.assets.texture_size(cx.assets.resolve(font.texture_hash)).unwrap_or((256, 256));
    let mut lines = Vec::new();
    for paragraph in text.split(['^', '\n']) {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let next = match line.is_empty() {
                true => word.to_owned(),
                false => format!("{line} {word}"),
            };
            // Native wrapping happens before the object's animated scale is applied.
            let width = font.layout_scaled(&next, TextStyle::default(), size, cx.assets.font_scale(info.font)).width;
            if line.is_empty() || width <= info.max_width.max(1) as f32 {
                line = next;
                continue;
            }
            lines.push(std::mem::replace(&mut line, word.to_owned()));
        }
        lines.push(line);
    }
    lines
}
