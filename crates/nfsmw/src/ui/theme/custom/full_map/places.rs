//! What the full-screen map marks, as data: the blips of the current tab (the frontend's filter categories: Locations,
//! Races, Pursuit) as the centred map's mark kinds, the GPS route's end, and the legend's rows. `draw.rs` of the
//! centred map turns the marks into shapes.

use crate::frontend::world_map::control::style_name;
use crate::frontend::world_map::{PlacedBlip, WorldMapFrame};
use crate::hud::{AUTO_SHOP, CAR_LOT, ROADBLOCK, SAFE_HOUSE};
use crate::ui::theme::custom::map::marks::{Mark, MarkKind};

/// The mark a blip style is drawn as: shops and car lots as a shop's dot, a safe house's square, a roadblock's bar, and
/// a race of every type as the race diamond.
pub fn kind_of(style: &str) -> MarkKind {
    match style {
        AUTO_SHOP | CAR_LOT => MarkKind::Shop,
        SAFE_HOUSE => MarkKind::SafeHouse,
        ROADBLOCK => MarkKind::Roadblock,
        _ => MarkKind::Race,
    }
}

/// The marks of `frame`, back to front: the blips the current tab shows, at their world places, then the end of the
/// GPS route.
pub fn marks(frame: &WorldMapFrame) -> Vec<Mark> {
    let listed = frame.session.listed(&frame.blips);
    let mut out: Vec<Mark> = listed
        .iter()
        .filter_map(|&index| frame.blips.get(index))
        .map(|blip| Mark::new(kind_of(blip.style), blip.world))
        .collect();
    out.extend(frame.route.as_ref().map(|route| Mark::new(MarkKind::Target, route.target)));
    out
}

/// The blip the cursor is focused on, if any.
pub fn focused(frame: &WorldMapFrame) -> Option<&PlacedBlip> {
    frame.session.attached.and_then(|index| frame.blips.get(index))
}

/// One row of the legend: a blip style of the current tab, its mark, its name and how many of it are on the map.
#[derive(Clone, Debug, PartialEq)]
pub struct LegendRow {
    pub style: &'static str,
    pub kind: MarkKind,
    pub name: String,
    pub count: usize,
}

/// The legend of the current tab: one row per blip style the tab shows, in the tab's order.
pub fn legend_rows(frame: &WorldMapFrame) -> Vec<LegendRow> {
    let styles = frame.session.filter.styles();
    styles
        .iter()
        .map(|&style| LegendRow {
            style,
            kind: kind_of(style),
            name: style_name(style),
            count: frame.blips.iter().filter(|blip| blip.style == style).count(),
        })
        .collect()
}
