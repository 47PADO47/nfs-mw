//! The cop cars on the road against their budget (`docs/specs/ai-pursuit-cops.md` sections 1 and 5): how many may be
//! active, which one goes when a new one would break the budget, and which one is taken from where it stands when
//! none is free. Pure rules; the caller counts the cars and measures the view.

use glam::Vec3;

/// The most cop cars active at once (`mMaxActiveCopCars`, which is also the platform budget of the original): wave cops are
/// asked for only below it. A rewrite setting may raise it.
pub const MAX_ACTIVE: usize = 8;

/// Whether one more active car would break `budget`: the active count after adding it exceeds the budget.
pub fn over_budget(active_after: usize, budget: usize) -> bool {
    active_after > budget
}

/// The priority of an active cop for staying on the road (`UpdateCopPriorities`): the dot product of the unit direction
/// from the player to the cop with the player's forward, plus one when the cop is in view. Cops ahead of the player and
/// in view stay; cops behind it and out of view go first. A cop at the player's position has no direction (priority 0).
pub fn priority(player: Vec3, player_forward: Vec3, cop: Vec3, in_view: bool) -> f32 {
    let direction = (cop - player).try_normalize().unwrap_or(Vec3::ZERO);
    direction.dot(player_forward) + f32::from(in_view)
}

/// The index of the lowest priority; the first of equal ones (a later one must be strictly lower to win).
pub fn lowest(priorities: impl IntoIterator<Item = f32>) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, value) in priorities.into_iter().enumerate() {
        if best.is_none_or(|(_, lowest)| value < lowest) {
            best = Some((index, value));
        }
    }
    best.map(|(index, _)| index)
}

/// Seconds a cop must have been out of view, and metres from the view, to be taken for a new car.
pub const STEAL_OFFSCREEN: f32 = 5.0;
pub const STEAL_DISTANCE: f32 = 100.0;

/// The cop taken from where it stands when no car is free (`GetActiveCopVehicleFromOutOfView`): of the candidates, given as
/// `(seconds out of view, distance to the view)`, the farthest among those out of view for more than [`STEAL_OFFSCREEN`]
/// and farther than [`STEAL_DISTANCE`]. `None` when none qualifies.
pub fn steal(candidates: impl IntoIterator<Item = (f32, f32)>) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (index, (offscreen, distance)) in candidates.into_iter().enumerate() {
        if offscreen > STEAL_OFFSCREEN && distance > STEAL_DISTANCE && best.is_none_or(|(_, far)| distance > far) {
            best = Some((index, distance));
        }
    }
    best.map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_budget_is_broken_by_the_car_that_would_exceed_it() {
        assert!(!over_budget(8, 8));
        assert!(over_budget(9, 8));
        assert!(!over_budget(0, 0));
    }

    #[test]
    fn a_cop_ahead_and_in_view_outranks_one_behind_and_out_of_view() {
        let player = Vec3::ZERO;
        let forward = Vec3::Z;
        let ahead = priority(player, forward, Vec3::new(0.0, 0.0, 200.0), true);
        let behind = priority(player, forward, Vec3::new(0.0, 0.0, -200.0), false);
        let beside = priority(player, forward, Vec3::new(200.0, 0.0, 0.0), false);
        assert!((ahead - 2.0).abs() < 1e-5);
        assert!((behind + 1.0).abs() < 1e-5);
        assert!(beside.abs() < 1e-5);
        assert!(ahead > beside && beside > behind);
    }

    #[test]
    fn being_in_view_adds_one_and_a_cop_on_the_player_has_no_direction() {
        let at = Vec3::new(5.0, 0.0, 5.0);
        assert_eq!(priority(at, Vec3::Z, at, false), 0.0);
        assert_eq!(priority(at, Vec3::Z, at, true), 1.0);
    }

    #[test]
    fn the_lowest_priority_goes_and_a_tie_goes_to_the_first() {
        assert_eq!(lowest([2.0, -1.0, 0.5]), Some(1));
        assert_eq!(lowest([1.0, 1.0, 1.0]), Some(0));
        assert_eq!(lowest(std::iter::empty()), None);
    }

    #[test]
    fn only_a_cop_unseen_for_long_and_far_away_may_be_taken_and_the_farthest_is() {
        assert_eq!(steal([(6.0, 150.0), (9.0, 300.0), (2.0, 900.0)]), Some(1), "the farthest of those unseen for long");
        assert_eq!(steal([(5.0, 500.0)]), None, "five seconds is not more than five");
        assert_eq!(steal([(30.0, 100.0)]), None, "inside the distance");
        assert_eq!(steal(std::iter::empty()), None);
    }
}
