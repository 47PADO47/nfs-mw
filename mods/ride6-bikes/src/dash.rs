//! The screen of a bike's instrument cluster, at rest.
//!
//! RIDE 6 draws a cluster with two Unreal widgets (the base colour and the emissive one) into the textures of `mat_gauge`; the
//! exporter composes both at the state of a bike at rest into `tex/<picture>_dash.png` (`tools/ride6-exporter/dash.py`), and
//! [`composed`] takes that picture in place of the cluster's static one. A bike with no widget keeps the older fallback below: it
//! looks for the black window of the static picture and draws a small panel into it.
//!
//! RIDE 6 shows the cluster's picture (`gaugeNNN_d`, the base colour of `mat_gauge`) with a black window where the instrument
//! panel is drawn at run time: the picture holds only the bezel, the warning lamps and, on a bike with a colour display, a black
//! rectangle that the material's emissive input (a black texture in every one of the 287 gauge materials) lights up. Left as it
//! is the display is a hole of black in the bezel, which is what the bikes with a screen showed ("the screens do not work").
//! The importer cannot draw a live instrument panel (the car's textures do not change while it is drawn), so it fills the black
//! window with the panel of an engine at rest: the rev bar with its first segments lit, the gear `N` and the speed `0 KM/H`.
//! See [the material model](../../../docs/formats/ride6-materials.md#instrument-clusters).
//!
//! The window is found, not read: no material parameter names it (a search of the 4,393 material instances of the 293 exported bikes for `screen`,
//! `display`, `tft`, `dash`, `clock` and `lcd` finds none), so it is the largest rectangle of near black texels that is
//! well inside the picture and has the proportions of a display. A picture without such a rectangle (the dials of an analogue
//! cluster, a bezel on a black ground) is left as it is.

use anyhow::Result;
use blackbox_tpk::Level;

use crate::textures::{Picture, TexStore};

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

/// The longest side of the cluster picture of a bike with a live screen: it is stored as 32-bit colour (the host writes into it), so it
/// is kept small; a display of a cluster is a quarter of it.
pub const LIVE_SIZE: u32 = 512;

/// The live screen of a bike's cluster: the texture it is on, the display in it and the colours to paint with (`[bike.dash]`,
/// `docs/modding/ride6-bike-dash.md`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Screen {
    pub texture: String,
    /// `[x, y, width, height]` in texels of the stored texture.
    pub window: [u32; 4],
    /// Background, lit segment, unlit segment, text, label, gear: texel colours, the lift of the picture applied.
    pub colors: [[u8; 3]; 6],
    /// The cluster's own places for the speed, the gear and the revs, from the exporter's layout; empty: the generic panel.
    #[serde(default)]
    pub fields: Vec<Field>,
}

/// One place of the cluster that shows a value (`[[bike.dash.field]]`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Field {
    /// `speed`, `gear` or `rpm_bar`.
    pub channel: String,
    /// `[x, y, width, height]` in texels of the stored texture.
    pub rect: [u32; 4],
    /// The digits' colour (the lit part of a bar), lifted like the picture, and the colour of the picture in the rectangle.
    pub color: [u8; 3],
    pub background: [u8; 3],
    pub align: String,
    pub segments: u32,
}

#[derive(serde::Deserialize)]
struct RawField {
    channel: String,
    rect: [f32; 4],
    #[serde(default)]
    colour: Option<[u8; 3]>,
    #[serde(default)]
    align: Option<String>,
    #[serde(default)]
    segments: Option<u32>,
}

#[derive(serde::Deserialize)]
struct Layout {
    canvas: f32,
    fields: Vec<RawField>,
}

/// The median of each channel of the texels of `level` in `rect`: the picture's colour with the few glyph texels voted out.
fn median_colour(level: &Level, [x, y, w, h]: [u32; 4]) -> [u8; 3] {
    let mut channels: [Vec<u8>; 3] = Default::default();
    for row in y..(y + h).min(level.height) {
        for column in x..(x + w).min(level.width) {
            let at = ((row * level.width + column) * 4) as usize;
            for (channel, list) in channels.iter_mut().enumerate() {
                list.push(level.rgba[at + channel]);
            }
        }
    }
    channels.map(|mut list| {
        list.sort_unstable();
        list.get(list.len() / 2).copied().unwrap_or(0)
    })
}

/// The fields of the exporter's layout `json` (rectangles in its canvas) for the stored picture `level`: scaled to it, each with its
/// colour lifted by `gain` and the median colour of the picture under it. A rectangle too small or outside the picture is left out.
pub fn fields_from(json: &str, level: &Level, gain: f32) -> Vec<Field> {
    let Ok(layout) = serde_json::from_str::<Layout>(json) else { return Vec::new() };
    let scale = level.width as f32 / layout.canvas.max(1.0);
    let mut out = Vec::new();
    for raw in layout.fields {
        let [x, y, w, h] = raw.rect.map(|v| v * scale);
        let (x0, y0) = (x.max(0.0).floor() as u32, y.max(0.0).floor() as u32);
        let (x1, y1) =
            (((x + w).ceil().max(0.0) as u32).min(level.width), ((y + h).ceil().max(0.0) as u32).min(level.height));
        if x1 < x0 + 4 || y1 < y0 + 4 {
            continue;
        }
        let rect = [x0, y0, x1 - x0, y1 - y0];
        let colour = raw.colour.unwrap_or([110, 200, 255]);
        out.push(Field {
            channel: raw.channel,
            rect,
            color: crate::grade::lift_colour(colour, gain),
            background: median_colour(level, rect),
            align: raw.align.unwrap_or_else(|| "center".to_owned()),
            segments: raw.segments.unwrap_or(1),
        });
    }
    out
}

impl Screen {
    /// The screen of `texture` (`picture_size` its stored size, `from` the size of the picture the `window` was found on), with the
    /// colours of the idle panel lifted by `gain` as the picture is.
    pub fn new(texture: &str, window: Window, from: (u32, u32), picture_size: (u32, u32), gain: f32) -> Self {
        let scale = |value: u32, to: u32, of: u32| (u64::from(value) * u64::from(to) / u64::from(of.max(1))) as u32;
        let (x, y) = (scale(window.x, picture_size.0, from.0), scale(window.y, picture_size.1, from.1));
        let (width, height) =
            (scale(window.width, picture_size.0, from.0), scale(window.height, picture_size.1, from.1));
        let lifted = [BACKGROUND, LIT, DIM, WHITE, GREY, GREEN].map(|colour| crate::grade::lift_colour(colour, gain));
        Self { texture: texture.to_owned(), window: [x, y, width, height], colors: lifted, fields: Vec::new() }
    }

    /// The `[bike.dash]` table.
    pub fn table(&self) -> String {
        let mut text = self.window_table();
        let colour = |[r, g, b]: [u8; 3]| format!("[{r}, {g}, {b}]");
        for field in &self.fields {
            let [x, y, w, h] = field.rect;
            text.push_str(&format!(
                "
[[bike.dash.field]]
channel = \"{}\"
rect = [{x}, {y}, {w}, {h}]
color = {}
background = {}
align = \"{}\"
segments = {}
",
                field.channel,
                colour(field.color),
                colour(field.background),
                field.align,
                field.segments
            ));
        }
        text
    }

    fn window_table(&self) -> String {
        let [x, y, w, h] = self.window;
        let [background, lit, dim, text, label, gear] = self.colors;
        let colour = |[r, g, b]: [u8; 3]| format!("[{r}, {g}, {b}]");
        format!(
            "[bike.dash]
texture = \"{}\"
window = [{x}, {y}, {w}, {h}]

[bike.dash.colors]
background = {}
lit = {}
dim = {}
text = {}
label = {}
gear = {}
",
            self.texture,
            colour(background),
            colour(lit),
            colour(dim),
            colour(text),
            colour(label),
            colour(gear)
        )
    }
}

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

/// The cluster picture the exporter composed from the bike's widgets (`<stem>_dash.png`), if it wrote one.
pub fn composed(store: &mut TexStore, stem: &str) -> Result<Option<Picture>> {
    let Some(picture) = store.picture(&format!("{}_dash", stem.to_ascii_lowercase()))? else { return Ok(None) };
    Ok(Some(Picture { width: picture.width, height: picture.height, rgba: picture.rgba.clone() }))
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
    fn a_screen_scales_its_window_to_the_stored_picture_and_lifts_its_colours() {
        let window = Window { x: 100, y: 200, width: 400, height: 200 };
        let screen = Screen::new("CAR_GAUGE1_D", window, (1024, 1024), (512, 512), 2.0);
        assert_eq!(screen.window, [50, 100, 200, 100]);
        assert!(
            screen.colors[0].iter().zip(BACKGROUND).all(|(lifted, plain)| *lifted >= plain),
            "the lift only brightens"
        );
        assert!(screen.colors[1][2] > LIT[2] || screen.colors[1][2] == 255, "{:?}", screen.colors[1]);
        let table = screen.table();
        assert!(
            table.starts_with(
                "[bike.dash]
texture = \"CAR_GAUGE1_D\"
window = [50, 100, 200, 100]
"
            ),
            "{table}"
        );
        assert!(table.contains(
            "[bike.dash.colors]
background = ["
        ));
    }

    #[test]
    fn a_layout_gives_fields_scaled_to_the_picture_with_the_median_background_and_lifted_colour() {
        // A 100 x 100 picture of grey with a white glyph of a few texels in the middle of the field.
        let mut level = Level { width: 100, height: 100, rgba: [40u8, 50, 60, 255].repeat(100 * 100) };
        for row in 40..44 {
            for column in 40..44 {
                let at = (row * 100 + column) * 4;
                level.rgba[at..at + 3].copy_from_slice(&[255, 255, 255]);
            }
        }
        let json = r#"{"canvas": 200, "fields": [
            {"channel": "speed", "rect": [60.0, 60.0, 60.0, 40.0], "colour": [100, 200, 50], "align": "right"},
            {"channel": "rpm_bar", "rect": [0.0, 0.0, 100.0, 20.0], "segments": 3},
            {"channel": "gear", "rect": [10.0, 10.0, 2.0, 2.0]}]}"#;
        let fields = fields_from(json, &level, 1.0);
        assert_eq!(fields.len(), 2, "the two texel rectangle is left out");
        assert_eq!((fields[0].channel.as_str(), fields[0].rect), ("speed", [30, 30, 30, 20]));
        assert_eq!((fields[0].color, fields[0].align.as_str()), ([100, 200, 50], "right"));
        assert_eq!(fields[0].background, [40, 50, 60], "the glyph is voted out");
        assert_eq!((fields[1].channel.as_str(), fields[1].segments, fields[1].rect), ("rpm_bar", 3, [0, 0, 50, 10]));
        assert!(fields_from("not json", &level, 1.0).is_empty());
        let mut screen =
            Screen::new("CAR_G_D", Window { x: 0, y: 0, width: 50, height: 50 }, (100, 100), (100, 100), 1.0);
        screen.fields = fields;
        let table = screen.table();
        assert!(
            table.contains(
                "[[bike.dash.field]]
channel = \"speed\"
rect = [30, 30, 30, 20]
"
            ),
            "{table}"
        );
        assert!(table.parse::<toml::Table>().is_ok(), "{table}");
    }

    #[test]
    fn text_is_as_wide_as_its_glyphs() {
        assert_eq!(text_width(2, "N"), 10);
        assert_eq!(text_width(1, "KM/H"), 23);
        assert_eq!(text_width(1, "?"), 0);
    }
}
