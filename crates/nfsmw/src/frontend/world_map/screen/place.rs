//! The pure part of drawing the map: which pool object each blip takes, which rows the list shows and what a row says.
//! No FEng here, so these are tested without the game.

use super::super::control::{format_metres, style_name};
use super::super::{Filter, PlacedBlip, distance_metres};
use super::pools;

/// The first listed row shown. The window of `rows` rows keeps the attached blip in it, and starts at the top
/// otherwise.
pub fn window_top(listed: &[usize], attached: Option<usize>, rows: usize) -> usize {
    let Some(at) = attached.and_then(|a| listed.iter().position(|&i| i == a)) else { return 0 };
    (at + 1).saturating_sub(rows)
}

/// What a row of the list says: the style's name and the distance from the player.
pub fn row_text(blip: &PlacedBlip, player: [f32; 2]) -> String {
    format!("{}   {}", style_name(blip.style), format_metres(distance_metres(player, blip)))
}

/// A blip drawn on a pool object: the blip's index in the frame's blips, and the pool object's GUID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Drawn {
    pub blip: usize,
    pub guid: u32,
}

/// The pool objects the blips of the filter take, in blip order. A style with no pool object left is not drawn; its
/// blips stay in the list and can still be attached.
pub fn assign(blips: &[PlacedBlip], filter: Filter) -> Vec<Drawn> {
    let mut taken: Vec<(&'static str, usize)> = Vec::new();
    let mut drawn = Vec::new();
    for (index, blip) in blips.iter().enumerate() {
        if !filter.contains(blip.style) {
            continue;
        }
        let position = match taken.iter().position(|(style, _)| *style == blip.style) {
            Some(position) => position,
            None => {
                taken.push((blip.style, 0));
                taken.len() - 1
            }
        };
        let used = &mut taken[position].1;
        let Some(&guid) = pools::slots(blip.style).get(*used) else { continue };
        *used += 1;
        drawn.push(Drawn { blip: index, guid });
    }
    drawn
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{AUTO_SHOP, RACE_SPRINT, SAFE_HOUSE};

    fn blip(style: &'static str, map: [f32; 2]) -> PlacedBlip {
        PlacedBlip { style, world: [0.0, 0.0], map }
    }

    #[test]
    fn each_blip_takes_the_next_pool_object_of_its_style() {
        let blips = [blip(SAFE_HOUSE, [0.1, 0.1]), blip(AUTO_SHOP, [0.2, 0.2]), blip(SAFE_HOUSE, [0.3, 0.3])];
        let drawn = assign(&blips, Filter::Locations);
        let safe = pools::slots(SAFE_HOUSE);
        assert_eq!(
            drawn,
            vec![
                Drawn { blip: 0, guid: safe[0] },
                Drawn { blip: 1, guid: pools::slots(AUTO_SHOP)[0] },
                Drawn { blip: 2, guid: safe[1] },
            ]
        );
    }

    #[test]
    fn only_the_filters_styles_are_drawn() {
        let blips = [blip(SAFE_HOUSE, [0.1, 0.1]), blip(RACE_SPRINT, [0.2, 0.2])];
        assert_eq!(assign(&blips, Filter::Locations), vec![Drawn { blip: 0, guid: pools::slots(SAFE_HOUSE)[0] }]);
        assert_eq!(assign(&blips, Filter::Races), vec![Drawn { blip: 1, guid: pools::slots(RACE_SPRINT)[0] }]);
    }

    #[test]
    fn a_blip_beyond_its_pool_is_not_drawn() {
        let blips: Vec<PlacedBlip> = (0..5).map(|i| blip(SAFE_HOUSE, [i as f32 * 0.1, 0.0])).collect();
        let drawn = assign(&blips, Filter::Locations);
        assert_eq!(drawn.len(), pools::slots(SAFE_HOUSE).len(), "four safe houses have four pool objects");
        assert_eq!(drawn.last().map(|d| d.blip), Some(3));
    }

    #[test]
    fn the_list_window_keeps_the_attached_row_in_view() {
        let listed: Vec<usize> = (0..10).collect();
        assert_eq!(window_top(&listed, None, 7), 0, "nothing attached: the top");
        assert_eq!(window_top(&listed, Some(3), 7), 0, "already in the first window");
        assert_eq!(window_top(&listed, Some(7), 7), 1, "one row past it scrolls by one");
        assert_eq!(window_top(&listed, Some(9), 7), 3, "the last blip is the last row");
    }

    #[test]
    fn a_row_names_the_style_and_the_distance() {
        let blip = PlacedBlip { style: AUTO_SHOP, world: [1500.0, 0.0], map: [0.0, 0.0] };
        assert_eq!(row_text(&blip, [0.0, 0.0]), "Auto shop   1.5 km");
        assert_eq!(row_text(&blip, [1500.0, 0.0]), "Auto shop   0 m");
    }
}
