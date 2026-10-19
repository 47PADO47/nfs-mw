//! The pure content of the Remastered map legend: the current filter's blips as rows (icon, colour, name, distance),
//! windowed to keep the attached blip in view, and the empty state. No FEng and no theme here, so the themed legend
//! (`ui/theme/remastered/map_legend`) draws the same data the FEng list shows, but in the token look.

use super::control::{blip_rgba, format_metres, icon_name, style_name};
use super::{PlacedBlip, WorldMapFrame, distance_metres};

/// How many rows the themed legend shows at once. The panel is sized to fit this many; the rest are reached by
/// scrolling (the session's `list_scroll`), the scrollbar showing the position. The map's own FEng list keeps its
/// seven rows (`screen::ROWS`).
pub const VISIBLE_ROWS: usize = 13;

/// One legend row: the blip's icon file, its colour, its name, the distance from the player and whether it is attached.
#[derive(Clone, Debug, PartialEq)]
pub struct LegendRow {
    pub icon: &'static str,
    pub tint: [u8; 4],
    pub name: &'static str,
    pub distance: String,
    pub attached: bool,
}

/// `count` of the current filter's blips as legend rows, from `scroll` (the list's scroll offset).
pub fn rows(frame: &WorldMapFrame, scroll: usize, count: usize) -> Vec<LegendRow> {
    let session = &frame.session;
    let listed = session.listed(&frame.blips);
    listed
        .iter()
        .skip(scroll)
        .take(count)
        .map(|&index| {
            let blip = &frame.blips[index];
            row(blip, frame.player, session.attached == Some(index))
        })
        .collect()
}

/// How many blips the current filter has (the legend's row total, for the scrollbar).
pub fn total(frame: &WorldMapFrame) -> usize {
    frame.session.listed(&frame.blips).len()
}

/// Whether the current filter shows no blip (the legend's empty state).
pub fn is_empty(frame: &WorldMapFrame) -> bool {
    total(frame) == 0
}

fn row(blip: &PlacedBlip, player: [f32; 2], attached: bool) -> LegendRow {
    LegendRow {
        icon: icon_name(blip.style),
        tint: blip_rgba(blip.style),
        name: style_name(blip.style),
        distance: format_metres(distance_metres(player, blip)),
        attached,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::world_map::{Filter, WorldMap};
    use crate::hud::{AUTO_SHOP, RACE_SPRINT, SAFE_HOUSE};

    fn frame(attached: Option<usize>) -> WorldMapFrame {
        let mut session = WorldMap::new();
        session.filter = Filter::Locations;
        session.attached = attached;
        let blips = vec![
            PlacedBlip { style: SAFE_HOUSE, world: [0.0, 0.0], map: [0.0, 0.0] },
            PlacedBlip { style: AUTO_SHOP, world: [1000.0, 0.0], map: [0.0, 0.0] },
            PlacedBlip { style: RACE_SPRINT, world: [0.0, 0.0], map: [0.0, 0.0] },
        ];
        WorldMapFrame { session, blips, player: [0.0, 0.0], chrome_themed: true }
    }

    #[test]
    fn the_rows_are_the_filters_blips_coloured_with_name_and_distance() {
        let rows = rows(&frame(Some(1)), 0, VISIBLE_ROWS);
        assert_eq!(rows.len(), 2, "the sprint is a race, not a location");
        assert_eq!(rows[0].name, "Safe house");
        assert_eq!((rows[1].name, rows[1].distance.as_str(), rows[1].attached), ("Auto shop", "1.0 km", true));
        assert_eq!(rows[0].tint, blip_rgba(SAFE_HOUSE));
        assert_eq!(total(&frame(Some(1))), 2);
        assert!(!is_empty(&frame(None)));
    }

    #[test]
    fn scrolling_skips_the_rows_above_the_offset() {
        let rows = rows(&frame(None), 1, VISIBLE_ROWS);
        assert_eq!(rows.len(), 1, "two locations, scrolled past the first");
        assert_eq!(rows[0].name, "Auto shop");
    }
}
