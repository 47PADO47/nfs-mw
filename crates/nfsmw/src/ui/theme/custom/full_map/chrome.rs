//! The full-screen map's chrome, in the Custom tokens and the OS fonts: the backdrop over the frozen world, the title
//! with the filter's tabs and the zoom at the top of the safe area, and the legend plate at the right. Places are in
//! canvas units (480 high, origin at the centre, y down) from the menus' frame (`custom/menu/layout.rs`), so the map's
//! footer and the menus' sit on the same line.

use blackbox_ui::{Component, LayoutCx, Panel, Polyline, Rect, Skin, justify, token_keys};
use glam::Vec2;

use super::art::{NO_TINT, PLAYER_ICON, icon};
use super::places::{self, LegendRow};
use crate::frontend::world_map::control::{blip_rgba, icon_name};
use crate::frontend::world_map::layout::Picture;
use crate::frontend::world_map::legend::is_empty;
use crate::frontend::world_map::{Filter, WorldMapFrame};
use crate::i18n::t;
use crate::ui::raster_text::{CUSTOM_LABEL_FONT_KEY, width_units};
use crate::ui::theme::custom::map::draw::colour;
use crate::ui::theme::custom::menu::layout::MenuFrame;
use crate::ui::theme::custom::plate::{Plate, PlateStyle};
use crate::ui::theme::custom::text::{Label, Sizing, size};
use crate::ui::theme::custom::tokens::{fade, keys};

/// The depths of the backdrop: the scrim over the whole window, and the plate under the map's picture, both behind the
/// roads (`map::ROAD_Z`).
pub const GROUND_Z: f32 = 70.0;
pub const PLATE_Z: f32 = 69.5;
/// The depth of the marks and arrows on a plate: nearer than the plate (`DEFAULT_Z`), so a shape's dark edge, drawn
/// half a unit behind it, still lies on the plate.
pub const CONTENT_Z: f32 = 0.5;
/// The tabs' text size, the gap between two tabs and the accent rule under the current one (thickness, drop below the
/// text's middle), at scale 1.
const TAB_SIZE: f32 = 13.0;
const TAB_GAP: f32 = 18.0;
const TAB_RULE: (f32, f32) = (2.0, 11.0);
/// How far down the safe area the title's middle is, and the tabs' below it, at scale 1.
const TITLE_DROP: f32 = 14.0;
const TABS_DROP: f32 = 30.0;
/// The zoom readout's size.
const ZOOM_SIZE: f32 = 12.0;
/// The legend plate: its width, its top below the tabs, the row pitch, the inner padding and the text size, at scale 1.
const LEGEND_WIDTH: f32 = 190.0;
const LEGEND_DROP: f32 = 24.0;
const ROW_PITCH: f32 = 20.0;
const LEGEND_PAD: f32 = 10.0;
const ROW_SIZE: f32 = 12.0;
/// Where a row's mark sits from the plate's left, and where its name starts, at scale 1.
const MARK_X: f32 = 14.0;
const NAME_X: f32 = 30.0;
/// The side of the icons in the legend, and the route's sample stroke's half length, at scale 1.
const ROW_ICON: f32 = 15.0;
const ROUTE_SAMPLE: f32 = 7.0;
/// The width of the box a string is laid out in. Wide enough for any string; the justification places it.
const TEXT_BOX: f32 = 640.0;

/// How a string sits on the point it is drawn at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// Its left edge on the point.
    Left,
    /// Its right edge on the point.
    Right,
}

/// `label` with its `align` side at `(x, y)`, centred on `y`, at `sizing`.
pub fn text(label: Label, (x, y): (f32, f32), align: Align, sizing: Sizing, cx: &LayoutCx<'_>) -> Box<dyn Component> {
    let height = sizing.units(40.0);
    let (centre, flags) = match align {
        Align::Left => (x + TEXT_BOX / 2.0, justify::V_CENTRE),
        Align::Right => (x - TEXT_BOX / 2.0, justify::H_RIGHT | justify::V_CENTRE),
    };
    Box::new(label.justify(flags).laid_out(Rect::new(centre, y, TEXT_BOX, height), sizing, cx))
}

/// The width of `text` in the label face at `size`. An estimate stands in when the font did not load.
pub fn text_width(text: &str, size: f32, sizing: Sizing) -> f32 {
    let estimate = text.chars().count() as f32 * 0.5 * sizing.units(size);
    width_units(CUSTOM_LABEL_FONT_KEY, text, sizing.glyph_scale(size)).unwrap_or(estimate)
}

/// A rectangle in the skin colour `key` (the neutral default's when the skin lacks it), its alpha multiplied by
/// `opacity`, at depth `z`.
pub fn panel(key: &'static str, rect: Rect, (opacity, z): (f32, f32), cx: &LayoutCx<'_>) -> Box<dyn Component> {
    let skin = Skin::new().with_colour(key, fade(colour(cx, key), opacity));
    let mut panel = Panel::new(key).with_z(z);
    panel.layout(&LayoutCx::new(&skin, cx.time_s), rect);
    Box::new(panel)
}

/// The backdrop that sets the open map apart from the HUD's own light map, at `open` (0 to 1, the soft transition):
///
/// - the theme's **scrim** over the whole window, which dims the frozen world behind the map everywhere, and
/// - the theme's **overlay** plate under the map's picture (the screen-high square the map is fitted to), which holds the
///   map with a nearly opaque ground.
///
/// Over the picture the two add up, so the map is on a darker ground than the dimmed scene beside it. The HUD map's plate
/// (`map/backdrop.rs`) is only a third of the scrim's alpha.
pub fn backdrop(window: Rect, open: f32, cx: &LayoutCx<'_>) -> Vec<Box<dyn Component>> {
    let side = Picture::FULL.side;
    let picture = Rect::new(0.0, 0.0, side, side);
    vec![
        panel(token_keys::SCRIM, window, (open, GROUND_Z), cx),
        panel(token_keys::OVERLAY, picture, (open, PLATE_Z), cx),
    ]
}

/// "MAP" in the spaced face at the safe area's top left, the filter's tabs under it (the current one bright, with an
/// accent rule), and the zoom at the top right.
pub fn title(map: &WorldMapFrame, frame: &MenuFrame, cx: &LayoutCx<'_>, out: &mut Vec<Box<dyn Component>>) {
    let (safe, s, sizing) = (frame.safe(), frame.scale(), frame.sizing());
    let title_y = safe.min_y() + TITLE_DROP * s;
    let heading = Label::heading(t!("map-title").to_uppercase(), size::MENU_TITLE).colour(keys::TEXT);
    out.push(text(heading, (safe.min_x(), title_y), Align::Left, sizing, cx));
    let tabs_y = safe.min_y() + TABS_DROP * s;
    let mut x = safe.min_x();
    for filter in Filter::ALL {
        let name = filter.label().to_uppercase();
        let width = text_width(&name, TAB_SIZE, sizing);
        let current = filter == map.session.filter;
        let tone = match current {
            true => keys::TEXT,
            false => keys::TEXT_SECONDARY,
        };
        out.push(text(Label::label(name, TAB_SIZE).colour(tone), (x, tabs_y), Align::Left, sizing, cx));
        if current {
            let (thickness, drop) = TAB_RULE;
            let rule = Rect::new(x + width / 2.0, tabs_y + drop * s, width, thickness * s);
            out.push(panel(keys::ACCENT, rule, (1.0, blackbox_ui::DEFAULT_Z), cx));
        }
        x += width + TAB_GAP * s;
    }
    let zoom = format!("{} x{:.1}", t!("map-zoom").to_uppercase(), map.session.camera.zoom());
    let zoom = Label::label(zoom, ZOOM_SIZE).colour(keys::TEXT_SECONDARY);
    out.push(text(zoom, (safe.max_x(), title_y), Align::Right, sizing, cx));
}

/// The legend plate at the safe area's right: the player's arrow, the GPS route when there is one, then each blip style
/// of the current tab with its mark, its name and how many are on the map. The focused blip's style is lit. A tab with
/// nothing on the map says so.
pub fn legend(map: &WorldMapFrame, frame: &MenuFrame, cx: &LayoutCx<'_>, out: &mut Vec<Box<dyn Component>>) {
    let (safe, s, sizing) = (frame.safe(), frame.scale(), frame.sizing());
    let rows = places::legend_rows(map);
    let route = map.route.is_some();
    let empty = is_empty(map);
    let lines = 1 + usize::from(route) + rows.len() + usize::from(empty);
    let width = LEGEND_WIDTH * s;
    let height = lines as f32 * ROW_PITCH * s + 2.0 * LEGEND_PAD * s;
    let top = safe.min_y() + (TABS_DROP + LEGEND_DROP) * s;
    let left = safe.max_x() - width;
    let plate = Rect::new(left + width / 2.0, top + height / 2.0, width, height);
    out.push(Box::new(Plate::new(plate, PlateStyle::SURFACE.with_accent(keys::ACCENT).scaled(s), cx)));
    let row_y = |row: usize| top + LEGEND_PAD * s + (row as f32 + 0.5) * ROW_PITCH * s;
    let mark_at = |row: usize| Vec2::new(left + MARK_X * s, row_y(row));
    let name_x = left + NAME_X * s;
    let count_x = plate.max_x() - LEGEND_PAD * s;
    let mut row = 0;
    let side = ROW_ICON * s;
    out.push(icon(PLAYER_ICON, mark_at(row), side, (NO_TINT, 0.0), CONTENT_Z, cx));
    let player = Label::label(t!("map-player"), ROW_SIZE);
    out.push(text(player, (name_x, row_y(row)), Align::Left, sizing, cx));
    row += 1;
    if route {
        let at = mark_at(row);
        let half = Vec2::new(ROUTE_SAMPLE * s, 0.0);
        let stroke = Polyline::new(vec![at - half, at + half], 3.0 * s, colour(cx, token_keys::GUIDE));
        out.push(Box::new(stroke.with_z(CONTENT_Z)));
        out.push(text(Label::label(t!("map-route"), ROW_SIZE), (name_x, row_y(row)), Align::Left, sizing, cx));
        row += 1;
    }
    let lit = places::focused(map).map(|blip| blip.style);
    for LegendRow { style, name, count } in rows {
        let tone = match lit == Some(style) {
            true => keys::ACCENT,
            false => keys::TEXT,
        };
        out.push(icon(icon_name(style), mark_at(row), side, (blip_rgba(style), 0.0), CONTENT_Z, cx));
        out.push(text(Label::label(name, ROW_SIZE).colour(tone), (name_x, row_y(row)), Align::Left, sizing, cx));
        let number = Label::numeric(count.to_string(), ROW_SIZE).colour(keys::TEXT_SECONDARY);
        out.push(text(number, (count_x, row_y(row)), Align::Right, sizing, cx));
        row += 1;
    }
    if empty {
        let line = Label::label(t!("map-legend-empty"), ROW_SIZE).colour(keys::TEXT_SECONDARY);
        out.push(text(line, (left + MARK_X * s, row_y(row)), Align::Left, sizing, cx));
    }
}
