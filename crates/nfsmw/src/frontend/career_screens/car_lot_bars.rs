//! The stat bars of the car lot's car card (`Car_Select.fng`): the LED strips of TOP SPEED, HANDLING and ACCELERATION, and
//! how a bar's fraction lights them. The car lot and the custom race's car select draw their bars with [`set_bar`].

use crate::frontend::logic::Cx;

/// One LED strip of a stat bar: its image GUID, its centre x inside the bar's group, its size in pixels and its uv width.
/// The strips are `LED_main.tga` at 32 pixels per uv unit (`[exe][verified]`, `Car_Select.fng`). Both strips of a bar
/// start at the same left edge (x = -60), so the shorter first strip overlays the start of the full one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Strip {
    pub guid: u32,
    pub centre_x: f32,
    pub width: f32,
    pub uv_width: f32,
}

/// The strips of the TOP SPEED bar (group `0xF1737`), the HANDLING bar (`0xF173F`) and the ACCELERATION bar (`0xF172F`).
pub(crate) const TOP_SPEED_STRIPS: [Strip; 2] = [
    Strip { guid: 0x000F_1738, centre_x: -20.0, width: 80.0, uv_width: 2.5 },
    Strip { guid: 0x000F_1739, centre_x: 20.0, width: 160.0, uv_width: 5.0 },
];
pub(crate) const HANDLING_STRIPS: [Strip; 2] = [
    Strip { guid: 0x000F_1740, centre_x: 0.0, width: 120.0, uv_width: 3.75 },
    Strip { guid: 0x000F_1741, centre_x: 20.0, width: 160.0, uv_width: 5.0 },
];
pub(crate) const ACCELERATION_STRIPS: [Strip; 2] = [
    Strip { guid: 0x000F_1730, centre_x: -40.0, width: 40.0, uv_width: 1.25 },
    Strip { guid: 0x000F_1731, centre_x: 20.0, width: 160.0, uv_width: 5.0 },
];

/// What one strip shows for a fraction: its lit uv (the `u1` of its texture rectangle), its new width and centre x, and
/// whether it is shown at all.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripFill {
    pub guid: u32,
    pub lit_uv: f32,
    pub width: f32,
    pub centre_x: f32,
    pub shown: bool,
}

/// Splits a bar's fraction over its strips. The bar is as wide as its widest strip (`total` uv); the lit part is
/// `fraction * total` uv from the left edge, and every strip lights that part up to its own uv width (the strips share
/// the left edge). A lit strip keeps its pixels per uv and its left edge, so its width and centre move with the lit part.
/// A strip with nothing lit is hidden.
pub fn strip_fills(fraction: f32, strips: &[Strip]) -> Vec<StripFill> {
    let total = strips.iter().map(|strip| strip.uv_width).fold(0.0, f32::max);
    let lit_total = fraction.clamp(0.0, 1.0) * total;
    strips.iter().map(|strip| strip_fill(lit_total, strip)).collect()
}

fn strip_fill(lit_total: f32, strip: &Strip) -> StripFill {
    let lit_uv = lit_total.clamp(0.0, strip.uv_width);
    let width = lit_uv * strip.width / strip.uv_width;
    let left = strip.centre_x - strip.width / 2.0;
    StripFill { guid: strip.guid, lit_uv, width, centre_x: left + width / 2.0, shown: lit_uv > 0.0 }
}

/// Draws one bar: each strip's uv rectangle, width and position for `fraction`.
pub(crate) fn set_bar(cx: &mut Cx, strips: &[Strip; 2], fraction: f32) {
    for fill in strip_fills(fraction, strips) {
        let Some(object) = cx.guid(fill.guid) else { continue };
        cx.rt.set_hidden(object, !fill.shown);
        if !fill.shown {
            continue;
        }
        cx.set_uv(fill.guid, [0.0, 0.0, fill.lit_uv, 1.0]);
        let Some(size) = cx.rt.size(object) else { continue };
        cx.rt.set_size_xy(object, fill.width, size.y);
        let Some(position) = cx.rt.position(object) else { continue };
        cx.rt.set_position_xy(object, fill.centre_x, position.y);
    }
}
