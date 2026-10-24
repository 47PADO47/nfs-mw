//! The Custom map legend: a token-styled panel of the current filter's blips (icon, name, distance), drawn over the
//! recoloured FEng map in place of the screen's own list (which the frontend hides while this is up). It reads the open
//! map from the cell the plugin fills ([`WorldMapLegendCell`]) and draws nothing when the map is closed.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use blackbox_ui::{Component, DrawList, Image, LayoutCx, Rect, Rounded, Text, justify, token_keys};

use super::skin::keys as skin_keys;
use crate::frontend::world_map::WorldMapFrame;
use crate::frontend::world_map::legend::{LegendRow, VISIBLE_ROWS, is_empty, rows, total};


// Placement in FEng units (640 x 480, origin centre, y down): the panel sits on the right, inside the map container and
// just below the screen's own filter tab (which stays, recoloured, with its switch).
const PANEL_X: f32 = 170.0;
const PANEL_Y: f32 = 18.0;
const PANEL_W: f32 = 220.0;
const PANEL_H: f32 = 345.0;
const ROW_STEP: f32 = 22.0;
const ROW_H: f32 = 10.0;
const ICON: f32 = 15.0;
const ROW_SCALE: f32 = 1.0;
const DISTANCE_W: f32 = 64.0;
/// Extra top/bottom padding for the row list, beyond the panel's own token padding.
const ROW_LIST_PAD_Y: f32 = 22.0;
/// A little left/right inset for the row items inside the panel.
const ITEM_INSET: f32 = 6.0;
/// The scrollbar's width, drawn in the right padding gutter (so it does not overlap the rows).
const SCROLLBAR_W: f32 = 4.0;
/// The shortest the scrollbar thumb gets, so it stays grabbable with many rows.
const THUMB_MIN: f32 = 14.0;

/// One frame's copy of the open map's legend.
#[derive(Clone, Debug, Default, PartialEq)]
pub(in crate::ui::theme) struct Legend {
    pub open: bool,
    pub rows: Vec<LegendRow>,
    pub empty: bool,
    /// How many blips the filter has in all, and the list's scroll offset: the scrollbar's size and place.
    pub total: usize,
    pub scroll: usize,
}

/// The shared cell. Clones share one legend.
#[derive(Clone, Debug, Default)]
pub struct WorldMapLegendCell {
    feed: Arc<Mutex<Legend>>,
}

impl WorldMapLegendCell {
    pub fn new() -> Self {
        Self::default()
    }

    /// Copies this frame's open map into the cell.
    pub(crate) fn set(&self, frame: &WorldMapFrame) {
        let scroll = frame.session.list_scroll;
        *self.lock() = Legend {
            open: true,
            rows: rows(frame, scroll, VISIBLE_ROWS),
            empty: is_empty(frame),
            total: total(frame),
            scroll,
        };
    }

    /// Marks the map closed, so the legend draws nothing.
    pub(crate) fn clear(&self) {
        *self.lock() = Legend::default();
    }

    pub(in crate::ui::theme) fn snapshot(&self) -> Legend {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, Legend> {
        self.feed.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The Custom map legend component. Built per resolve from the shared cell; it holds no state of its own.
pub struct MapLegend {
    cell: WorldMapLegendCell,
    body: Option<Body>,
}

impl MapLegend {
    pub fn new(cell: WorldMapLegendCell) -> Self {
        Self { cell, body: None }
    }
}

impl Component for MapLegend {
    fn layout(&mut self, cx: &LayoutCx<'_>, _bounds: Rect) {
        let legend = self.cell.snapshot();
        self.body = legend.open.then(|| Body::of(&legend, cx));
    }

    fn build(&self, out: &mut DrawList) {
        let Some(body) = &self.body else { return };
        for part in &body.parts {
            part.build(out);
        }
    }
}

struct Body {
    parts: Vec<Box<dyn Component>>,
}

impl Body {
    fn of(legend: &Legend, cx: &LayoutCx<'_>) -> Self {
        let radius = cx.skin.metric(token_keys::RADIUS).unwrap_or(6.0);
        let pad = cx.skin.metric(token_keys::PADDING).unwrap_or(10.0);
        let font = skin_keys::LEGEND_FONT;
        let panel = Rect::new(PANEL_X, PANEL_Y, PANEL_W, PANEL_H);
        let mut parts: Vec<Box<dyn Component>> = Vec::new();

        parts.push(Box::new(Rounded::new(token_keys::BACKGROUND, panel, radius, cx)));
        let left = panel.min_x() + pad + ITEM_INSET;
        let right = panel.max_x() - pad - ITEM_INSET;
        let width = right - left;

        // The screen's own filter switch (`< LOCATIONS >`) stays at the top, so the legend has no title of its own. The
        // row list has extra top/bottom padding; the scrollbar sits in the right padding gutter.
        let rows_top = panel.min_y() + ROW_LIST_PAD_Y;
        if legend.empty {
            let line = lt(left, rows_top, width, ROW_H);
            let empty = crate::i18n::t!("map-legend-empty");
            parts.push(text(cx, font, &empty, token_keys::MUTED, ROW_SCALE, justify::V_CENTRE, line));
        } else {
            for (k, row) in legend.rows.iter().enumerate() {
                row_item(&mut parts, row, left, width, rows_top + k as f32 * ROW_STEP, font, cx);
            }
            scrollbar(&mut parts, legend, panel, pad, rows_top, cx);
        }
        Self { parts }
    }
}

/// A slim scrollbar in the right padding gutter when the filter has more blips than fit: a faint track with a thumb
/// sized by the fraction shown and placed by the scroll offset. Nothing is drawn when all the rows fit.
fn scrollbar(
    parts: &mut Vec<Box<dyn Component>>,
    legend: &Legend,
    panel: Rect,
    pad: f32,
    rows_top: f32,
    cx: &LayoutCx<'_>,
) {
    if legend.total <= VISIBLE_ROWS {
        return;
    }
    let track_h = VISIBLE_ROWS as f32 * ROW_STEP;
    let x = panel.max_x() - pad / 2.0 - SCROLLBAR_W / 2.0;
    parts.push(Box::new(Rounded::new(
        token_keys::BORDER,
        lt(x, rows_top, SCROLLBAR_W, track_h),
        SCROLLBAR_W / 2.0,
        cx,
    )));
    let thumb_h = (track_h * VISIBLE_ROWS as f32 / legend.total as f32).max(THUMB_MIN);
    let max_scroll = legend.total - VISIBLE_ROWS;
    let t = (legend.scroll as f32 / max_scroll as f32).clamp(0.0, 1.0);
    let thumb_top = rows_top + (track_h - thumb_h) * t;
    parts.push(Box::new(Rounded::new(
        token_keys::MUTED,
        lt(x, thumb_top, SCROLLBAR_W, thumb_h),
        SCROLLBAR_W / 2.0,
        cx,
    )));
}

fn row_item(
    parts: &mut Vec<Box<dyn Component>>,
    row: &LegendRow,
    left: f32,
    width: f32,
    top: f32,
    font: &str,
    cx: &LayoutCx<'_>,
) {
    let mid = top + ROW_H / 2.0;
    let icon = lt(left, mid - ICON / 2.0, ICON, ICON);
    parts.push(image(cx, row.icon, row.tint, icon));
    let name_left = left + ICON + 8.0;
    let name = lt(name_left, mid - ROW_H / 2.0, width - ICON - 8.0 - DISTANCE_W, ROW_H);
    let colour = if row.attached { token_keys::SECONDARY } else { token_keys::PRIMARY };
    parts.push(text(cx, font, &row.name, colour, ROW_SCALE, justify::V_CENTRE, name));
    let distance = lt(width + left - DISTANCE_W, mid - ROW_H / 2.0, DISTANCE_W, ROW_H);
    parts.push(text(
        cx,
        font,
        &row.distance,
        token_keys::MUTED,
        ROW_SCALE,
        justify::H_RIGHT | justify::V_CENTRE,
        distance,
    ));
}

fn lt(left: f32, top: f32, w: f32, h: f32) -> Rect {
    Rect::new(left + w / 2.0, top + h / 2.0, w, h)
}

fn text(
    cx: &LayoutCx<'_>,
    font: &str,
    s: &str,
    colour: &str,
    scale: f32,
    flags: u32,
    rect: Rect,
) -> Box<dyn Component> {
    let mut t = Text::new(font, s).with_colour_key(colour).with_scale(scale).with_justification(flags);
    t.layout(cx, rect);
    Box::new(t)
}

fn image(cx: &LayoutCx<'_>, key: &str, tint: [u8; 4], rect: Rect) -> Box<dyn Component> {
    let mut i = Image::new(key).with_tint(tint);
    i.layout(cx, rect);
    Box::new(i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::world_map::control::blip_rgba;
    use crate::frontend::world_map::{Chrome, Filter, PlacedBlip, WorldMap, WorldMapFrame};
    use blackbox_ui::{DrawList, NodeKind, Skin, UiNode};

    fn test_skin() -> Skin {
        use crate::ui::theme::custom::{card_skin, palette};
        palette::remastered().skin().merged(card_skin())
    }

    fn frame() -> WorldMapFrame {
        let mut session = WorldMap::new();
        session.filter = Filter::Locations;
        session.attached = Some(1);
        let blips = vec![
            PlacedBlip { style: crate::hud::SAFE_HOUSE, world: [0.0, 0.0], map: [0.0, 0.0] },
            PlacedBlip { style: crate::hud::AUTO_SHOP, world: [1000.0, 0.0], map: [0.0, 0.0] },
        ];
        WorldMapFrame { session, blips, chrome: Chrome::Recoloured, ..Default::default() }
    }

    fn built(cell: &WorldMapLegendCell) -> Vec<UiNode> {
        let skin = test_skin();
        let mut legend = MapLegend::new(cell.clone());
        legend.layout(&LayoutCx::new(&skin, 0.0), Rect::SCREEN);
        let mut out = DrawList::new();
        legend.build(&mut out);
        out.iter().cloned().collect()
    }

    #[test]
    fn a_closed_map_draws_nothing() {
        assert!(built(&WorldMapLegendCell::new()).is_empty());
    }

    #[test]
    fn the_legend_shows_the_filter_rows_with_coloured_icons() {
        let cell = WorldMapLegendCell::new();
        cell.set(&frame());
        let nodes = built(&cell);
        let texts: Vec<&str> = nodes.iter().filter_map(|n| n.text.as_deref()).collect();
        assert!(texts.contains(&"Safe house") && texts.contains(&"Auto shop"), "the rows");
        let tints: Vec<[u8; 4]> =
            nodes.iter().filter(|n| matches!(n.kind, NodeKind::Image { .. })).map(|n| n.colour).collect();
        assert!(tints.contains(&blip_rgba(crate::hud::SAFE_HOUSE)), "the safe house icon is green");
    }
}
