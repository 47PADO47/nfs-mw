//! The screen of a bike's instrument cluster, drawn once at the engine's idle.
//!
//! RIDE 6 shows the cluster's picture (`gaugeNNN_d`, the base colour of `mat_gauge`) with a black window where the instrument
//! panel is drawn at run time: the picture holds only the bezel, the warning lamps and, on a bike with a colour display, a black
//! rectangle that the material's emissive input (a black texture in every one of the 287 gauge materials) lights up. Left as it
//! is the display is a hole of black in the bezel, which is what the bikes with a screen showed ("the screens do not work").
//! The importer cannot draw a live instrument panel (the car's textures do not change while it is drawn), so it fills the black
//! window with the panel of an engine at rest: the rev bar with its first segments lit, the gear `N` and the speed `0 KM/H`.
//! See [the material model](../../../docs/formats/ride6-materials.md#instrument-clusters).
//!
//! The window is found, not read: no material parameter names it (a search of the 3,100 material instances for `screen`,
//! `display`, `tft`, `dash`, `clock` and `lcd` finds none), so it is the largest rectangle of near black texels that is
//! well inside the picture and has the proportions of a display. A picture without such a rectangle (the dials of an analogue
//! cluster, a bezel on a black ground) is left as it is.

use crate::textures::Picture;

/// The brightest channel of a texel that still counts as black.
const BLACK: u8 = 12;
/// The side of the grid the picture is searched on, in cells at most.
const GRID: u32 = 128;
/// The window's share of the picture below which it is not a display.
const MIN_AREA: f32 = 0.04;
/// The proportions (width over height) of a display.
const ASPECT: std::ops::RangeInclusive<f32> = 1.0..=3.5;
/// The share of the window left undrawn at each edge: the corners of a display are rounded.
const INSET: f32 = 0.03;

const BACKGROUND: [u8; 3] = [10, 16, 28];
const LIT: [u8; 3] = [110, 200, 255];
const DIM: [u8; 3] = [38, 48, 62];
const WHITE: [u8; 3] = [235, 240, 245];
const GREY: [u8; 3] = [140, 150, 160];
const GREEN: [u8; 3] = [70, 225, 100];
/// The segments of the rev bar and how many of them are lit at idle.
const SEGMENTS: u32 = 24;
const LIT_SEGMENTS: u32 = 3;

/// A rectangle of the picture in texels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// The glyphs of the panel: five columns by seven rows, a bit per column from the left (bit 4).
fn glyph(c: char) -> Option<[u8; 7]> {
    let rows = match c {
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        _ => return None,
    };
    Some(rows)
}

/// The largest rectangle of near black texels of `picture` that is a display, if there is one.
pub fn find_window(picture: &Picture) -> Option<Window> {
    let block = (picture.width.max(picture.height) / GRID).max(1);
    let (columns, rows) = (picture.width.div_ceil(block), picture.height.div_ceil(block));
    let mut black = vec![true; (columns * rows) as usize];
    for y in 0..picture.height {
        for x in 0..picture.width {
            let at = ((y * picture.width + x) * 4) as usize;
            if picture.rgba[at..at + 3].iter().any(|&c| c > BLACK) {
                black[((y / block) * columns + x / block) as usize] = false;
            }
        }
    }
    let (x0, y0, x1, y1) = largest_rectangle(&black, columns, rows)?;
    let cells = ((x1 - x0) * (y1 - y0)) as f32;
    let (w, h) = ((x1 - x0) as f32, (y1 - y0) as f32);
    let inside = x0 >= 1 && y0 >= 1 && x1 < columns && y1 < rows;
    if cells < MIN_AREA * (columns * rows) as f32 || !ASPECT.contains(&(w / h)) || !inside {
        return None;
    }
    let (inset_x, inset_y) = ((w * INSET).ceil() as u32, (h * INSET).ceil() as u32);
    let (left, top) = ((x0 + inset_x) * block, (y0 + inset_y) * block);
    let (right, bottom) = (((x1 - inset_x) * block).min(picture.width), ((y1 - inset_y) * block).min(picture.height));
    Some(Window { x: left, y: top, width: right.checked_sub(left)?, height: bottom.checked_sub(top)? })
}

/// The cell rectangle `(x0, y0, x1, y1)` (end exclusive) of the most cells in a grid of booleans, by the histogram method.
fn largest_rectangle(grid: &[bool], columns: u32, rows: u32) -> Option<(u32, u32, u32, u32)> {
    let mut heights = vec![0u32; columns as usize];
    let mut best: Option<(u32, (u32, u32, u32, u32))> = None;
    for y in 0..rows {
        for x in 0..columns {
            let h = &mut heights[x as usize];
            *h = if grid[(y * columns + x) as usize] { *h + 1 } else { 0 };
        }
        for start in 0..columns {
            let mut low = u32::MAX;
            for end in start..columns {
                low = low.min(heights[end as usize]);
                if low == 0 {
                    break;
                }
                let area = low * (end - start + 1);
                if best.is_none_or(|(most, _)| area > most) {
                    best = Some((area, (start, y + 1 - low, end + 1, y + 1)));
                }
            }
        }
    }
    best.map(|(_, rectangle)| rectangle)
}

fn fill(picture: &mut Picture, (x, y, width, height): (u32, u32, u32, u32), colour: [u8; 3]) {
    for row in y..(y + height).min(picture.height) {
        for column in x..(x + width).min(picture.width) {
            let at = ((row * picture.width + column) * 4) as usize;
            picture.rgba[at..at + 3].copy_from_slice(&colour);
        }
    }
}

/// `text` with its top left corner at `(x, y)`, the glyphs `unit` texels per dot.
fn text(picture: &mut Picture, (x, y): (u32, u32), unit: u32, colour: [u8; 3], text: &str) {
    let mut at = x;
    for c in text.chars() {
        let Some(rows) = glyph(c) else { continue };
        for (row, bits) in rows.iter().enumerate() {
            for column in 0..5u32 {
                if (*bits >> (4 - column)) & 1 == 1 {
                    fill(picture, (at + column * unit, y + row as u32 * unit, unit, unit), colour);
                }
            }
        }
        at += 6 * unit;
    }
}

/// The width of `text` in dots of `unit` texels, without the gap after the last glyph.
fn text_width(unit: u32, text: &str) -> u32 {
    let glyphs = text.chars().filter(|&c| glyph(c).is_some()).count() as u32;
    (glyphs * 6).saturating_sub(1) * unit
}

/// Draw the panel of an engine at rest into `window`.
pub fn draw_idle(picture: &mut Picture, window: Window) {
    let Window { x, y, width, height } = window;
    fill(picture, (x, y, width, height), BACKGROUND);
    // The rev bar along the top: segments with a gap, the first few lit.
    let (bar_y, bar_h) = (y + height / 12, height / 9);
    let pitch = width * 9 / 10 / SEGMENTS;
    let bar_x = x + width / 20;
    for segment in 0..SEGMENTS {
        let colour = if segment < LIT_SEGMENTS { LIT } else { DIM };
        fill(picture, (bar_x + segment * pitch, bar_y, (pitch * 3 / 4).max(1), bar_h), colour);
    }
    // The gear on the left and the speed on the right, the unit under the speed.
    let unit = (height / 3 / 7).max(1);
    let top = y + height * 3 / 10;
    text(picture, (x + width / 12, top), unit, GREEN, "N");
    let speed_x = x + width - width / 12 - text_width(unit, "0");
    text(picture, (speed_x, top), unit, WHITE, "0");
    let small = (unit / 3).max(1);
    let label_x = x + width - width / 12 - text_width(small, "KM/H");
    text(picture, (label_x, top + 7 * unit + 2 * small), small, GREY, "KM/H");
}

/// Draw the idle panel into the display of `picture` when it has one; returns whether it did.
pub fn fill_screen(picture: &mut Picture) -> bool {
    let Some(window) = find_window(picture) else { return false };
    draw_idle(picture, window);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A picture of 64 x 64 dark grey with a black rectangle at `(x, y)` of `width` x `height`.
    fn cluster(x: u32, y: u32, width: u32, height: u32) -> Picture {
        let mut picture = Picture { width: 64, height: 64, rgba: [40u8, 40, 40, 255].repeat(64 * 64) };
        fill(&mut picture, (x, y, width, height), [0, 0, 0]);
        picture
    }

    #[test]
    fn the_black_display_of_a_cluster_is_found() {
        let picture = cluster(12, 20, 40, 24);
        let window = find_window(&picture).expect("a display");
        assert!(window.x >= 12 && window.x + window.width <= 52, "{window:?}");
        assert!(window.y >= 20 && window.y + window.height <= 44, "{window:?}");
        assert!(window.width >= 32 && window.height >= 18, "the window is most of the black: {window:?}");
    }

    #[test]
    fn a_picture_with_no_display_is_left_alone() {
        let flat = cluster(0, 0, 0, 0);
        assert_eq!(find_window(&flat), None);
        // Black that runs to the edge is a background, not a window.
        let background = cluster(0, 0, 64, 40);
        assert_eq!(find_window(&background), None);
        // A black strip is no display.
        let strip = cluster(8, 30, 48, 4);
        assert_eq!(find_window(&strip), None);
    }

    #[test]
    fn the_idle_panel_lights_the_first_segments_and_writes_the_gear() {
        let mut picture = cluster(12, 20, 40, 24);
        assert!(fill_screen(&mut picture));
        let lit = picture.rgba.as_chunks::<4>().0.iter().filter(|t| t[..3] == LIT).count();
        let green = picture.rgba.as_chunks::<4>().0.iter().filter(|t| t[..3] == GREEN).count();
        let black = picture.rgba.as_chunks::<4>().0.iter().filter(|t| t[..3] == [0, 0, 0]).count();
        assert!(lit > 0 && green > 0, "lit {lit}, green {green}");
        assert!(black < 40 * 24 / 4, "the black is mostly painted over: {black}");
        // The bezel around the display is untouched.
        assert_eq!(&picture.rgba[..4], &[40, 40, 40, 255]);
    }

    #[test]
    fn text_is_as_wide_as_its_glyphs() {
        assert_eq!(text_width(2, "N"), 10);
        assert_eq!(text_width(1, "KM/H"), 23);
        assert_eq!(text_width(1, "?"), 0);
    }
}
