//! The minimap's interior icons in a chase: which kinds may show, and the hiding spots of the cool-down.
//!
//! The original's minimap lists only map elements that are switched on (both low bits of the element's flag byte) and
//! whose type the icon mask allows, nearest first, one pool object per element ([decomp] the minimap's icon pass and its
//! element list). The switch for a hiding spot was not traced to its writer. [observed] in the original: the hiding
//! spots appear on the minimap while the chase cools down. [ours] The places that cannot be entered in a chase (race
//! starts, auto shops, car lots) leave the minimap while a chase runs, and the safe houses show only in its cool-down,
//! the rule the world's entry markers follow (`effects/entry_marks.rs`). [observed] The pursuit breakers' icons leave
//! the minimap in the cool-down, and the hiding spots outside the view stand at its rim.

use glam::{Vec2, Vec3};

use crate::hud::{
    AUTO_SHOP, CAR_LOT, HIDING_SPOT, MapBlip, PURSUIT_BREAKER, RACE_CIRCUIT, RACE_DRAG, RACE_KNOCKOUT, RACE_SPEEDTRAP,
    RACE_SPRINT, RACE_TOLLBOOTH, SAFE_HOUSE,
};

/// Where the chase stands for the minimap's interior icons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Chase {
    None,
    /// The cops are on the player.
    Active,
    /// The cool-down meter runs.
    CoolDown,
}

/// Whether a blip of `style` may show while the chase is `chase`. Styles that are not interiors are not this rule's.
pub(super) fn shown(style: &str, chase: Chase) -> bool {
    const ENTRIES: [&str; 8] =
        [RACE_SPRINT, RACE_DRAG, RACE_CIRCUIT, RACE_KNOCKOUT, RACE_TOLLBOOTH, RACE_SPEEDTRAP, AUTO_SHOP, CAR_LOT];
    match chase {
        Chase::None => style != HIDING_SPOT,
        Chase::Active => !ENTRIES.contains(&style) && style != SAFE_HOUSE && style != HIDING_SPOT,
        Chase::CoolDown => !ENTRIES.contains(&style) && style != PURSUIT_BREAKER,
    }
}

/// The hiding spots' blips (render space), nearest to `player` first so the nearest take the pool's objects.
pub(super) fn hiding_blips(spots: impl Iterator<Item = Vec3>, player: Vec3, out: &mut Vec<MapBlip>) {
    let here = Vec2::new(player.x, player.y);
    let mut spots: Vec<Vec2> = spots.map(|p| Vec2::new(p.x, p.y)).collect();
    spots.sort_by(|a, b| a.distance_squared(here).total_cmp(&b.distance_squared(here)));
    out.extend(spots.into_iter().map(|p| MapBlip {
        style: HIDING_SPOT,
        position: p.to_array(),
        active: false,
        heading: None,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::COP;

    #[test]
    fn out_of_a_chase_every_interior_shows_but_the_hiding_spots() {
        for style in [RACE_SPRINT, AUTO_SHOP, CAR_LOT, SAFE_HOUSE, COP] {
            assert!(shown(style, Chase::None), "{style}");
        }
        assert!(!shown(HIDING_SPOT, Chase::None));
    }

    #[test]
    fn a_running_chase_hides_the_places_that_cannot_be_entered() {
        for style in [RACE_DRAG, RACE_TOLLBOOTH, AUTO_SHOP, CAR_LOT, SAFE_HOUSE, HIDING_SPOT] {
            assert!(!shown(style, Chase::Active), "{style}");
        }
        assert!(shown(COP, Chase::Active) && shown(PURSUIT_BREAKER, Chase::Active));
    }

    #[test]
    fn the_cool_down_brings_the_safe_houses_and_the_hiding_spots() {
        assert!(shown(SAFE_HOUSE, Chase::CoolDown) && shown(HIDING_SPOT, Chase::CoolDown));
        assert!(!shown(AUTO_SHOP, Chase::CoolDown) && !shown(RACE_SPRINT, Chase::CoolDown));
        assert!(!shown(PURSUIT_BREAKER, Chase::CoolDown), "the breakers' icons go in the cool-down");
    }

    #[test]
    fn the_nearest_hiding_spots_come_first() {
        let spots = [Vec3::new(500.0, 0.0, 9.0), Vec3::new(10.0, 10.0, 0.0), Vec3::new(-100.0, 0.0, 0.0)];
        let mut out = Vec::new();
        hiding_blips(spots.into_iter(), Vec3::ZERO, &mut out);
        let xs: Vec<f32> = out.iter().map(|b| b.position[0]).collect();
        assert_eq!(xs, vec![10.0, -100.0, 500.0]);
        assert!(out.iter().all(|b| b.style == HIDING_SPOT && b.heading.is_none()));
    }
}
