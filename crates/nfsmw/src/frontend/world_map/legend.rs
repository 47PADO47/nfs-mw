//! What a themed legend needs to know of the current filter, with no FEng and no theme here: how many blips the tab
//! has, whether it has none (the legend's empty state), and how many rows a scrolled legend shows at once.

use super::WorldMapFrame;

/// How many rows a themed legend shows at once. The rest are reached by scrolling (the session's `list_scroll`). The
/// map's own FEng list keeps its seven rows (`screen::ROWS`).
pub const VISIBLE_ROWS: usize = 13;

/// How many blips the current filter has.
pub fn total(frame: &WorldMapFrame) -> usize {
    frame.session.listed(&frame.blips).len()
}

/// Whether the current filter shows no blip (the legend's empty state).
pub fn is_empty(frame: &WorldMapFrame) -> bool {
    total(frame) == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::world_map::{Chrome, Filter, PlacedBlip, WorldMap};
    use crate::hud::{AUTO_SHOP, RACE_SPRINT, SAFE_HOUSE};

    fn frame(filter: Filter) -> WorldMapFrame {
        let mut session = WorldMap::new();
        session.filter = filter;
        let blips = vec![
            PlacedBlip { style: SAFE_HOUSE, world: [0.0, 0.0], map: [0.0, 0.0] },
            PlacedBlip { style: AUTO_SHOP, world: [1000.0, 0.0], map: [0.0, 0.0] },
            PlacedBlip { style: RACE_SPRINT, world: [0.0, 0.0], map: [0.0, 0.0] },
        ];
        WorldMapFrame { session, blips, chrome: Chrome::Drawn, ..Default::default() }
    }

    #[test]
    fn the_total_counts_the_filters_blips_and_an_empty_tab_says_so() {
        assert_eq!(total(&frame(Filter::Locations)), 2, "the sprint is a race, not a location");
        assert_eq!(total(&frame(Filter::Races)), 1);
        assert!(!is_empty(&frame(Filter::Locations)));
        assert!(is_empty(&frame(Filter::Pursuit)), "no roadblock on the map");
    }
}
