//! What the full-screen map lists, as data: the blip the cursor is focused on and the legend's rows, under the frontend's
//! filter (the categories Locations, Races, Pursuit). `art.rs` draws the blips themselves as the game's icons.

use crate::frontend::world_map::control::style_name;
use crate::frontend::world_map::{PlacedBlip, WorldMapFrame};

/// The blip the cursor is focused on, if any.
pub fn focused(frame: &WorldMapFrame) -> Option<&PlacedBlip> {
    frame.session.attached.and_then(|index| frame.blips.get(index))
}

/// One row of the legend: a blip style of the current tab (its icon is `control::icon_name`'s), its name and how many
/// of it are on the map.
#[derive(Clone, Debug, PartialEq)]
pub struct LegendRow {
    pub style: &'static str,
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
            name: style_name(style),
            count: frame.blips.iter().filter(|blip| blip.style == style).count(),
        })
        .collect()
}
