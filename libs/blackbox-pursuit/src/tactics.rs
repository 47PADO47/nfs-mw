//! Cop tactics as geometry: which tactic a cop uses against the player and where a blocking car or a pit point
//! goes. Map plane coordinates; `right` is `forward` turned clockwise.

use glam::Vec2;

/// Within this distance (m) a cop that is ahead of the player blocks the road.
pub const BLOCK_RANGE: f32 = 60.0;
/// Within this distance (m) a cop rams the player.
pub const RAM_RANGE: f32 = 6.0;
/// Lateral band (m) beside the player where a cop can pit it.
pub const PIT_LATERAL: (f32, f32) = (1.0, 4.0);
/// Longitudinal band (m) around the player's middle where a pit is possible.
pub const PIT_LONGITUDINAL: f32 = 6.0;

/// What a cop does next to the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tactic {
    Follow,
    /// Get in front and turn across the road.
    Block,
    /// Come alongside and push the rear quarter.
    Pit,
    /// Drive straight into the player.
    Ram,
}

/// The cop's place relative to the player, and which tactics the chase allows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TacticInput {
    /// Distance from the cop to the player, metres.
    pub distance: f32,
    /// Offset of the cop along the player's forward direction, metres (positive is ahead).
    pub along: f32,
    /// Offset of the cop to the player's right, metres (negative is left).
    pub across: f32,
    pub can_block: bool,
    pub can_pit: bool,
}

/// The tactic for this cop. A pit comes first (it needs the cop alongside), then a block, then a ram close in.
pub fn choose(input: &TacticInput) -> Tactic {
    let across = input.across.abs();
    let beside = (PIT_LATERAL.0..=PIT_LATERAL.1).contains(&across);
    if input.can_pit && beside && input.along.abs() <= PIT_LONGITUDINAL {
        return Tactic::Pit;
    }
    if input.can_block && input.along > 0.0 && input.distance <= BLOCK_RANGE {
        return Tactic::Block;
    }
    if input.distance <= RAM_RANGE {
        return Tactic::Ram;
    }
    Tactic::Follow
}

/// The `count` points of a road block: across the road, `ahead` metres in front of the player, spread evenly from
/// one edge of the road to the other (`road_half_width` each side). One car blocks the middle.
pub fn block_slots(player: Vec2, forward: Vec2, ahead: f32, road_half_width: f32, count: usize) -> Vec<Vec2> {
    let centre = player + forward.normalize_or_zero() * ahead;
    let right = right_of(forward);
    match count {
        0 => Vec::new(),
        1 => vec![centre],
        _ => (0..count)
            .map(|i| {
                let t = i as f32 / (count - 1) as f32;
                centre + right * road_half_width * (2.0 * t - 1.0)
            })
            .collect(),
    }
}

/// The point a pitting cop drives at: the player's rear quarter on the given side (`side` is +1 right, -1 left).
pub fn pit_point(player: Vec2, forward: Vec2, side: f32) -> Vec2 {
    let forward = forward.normalize_or_zero();
    player - forward * 1.5 + right_of(forward) * side.signum() * 1.0
}

/// `forward` turned clockwise by a quarter turn.
fn right_of(forward: Vec2) -> Vec2 {
    Vec2::new(forward.y, -forward.x).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(distance: f32, along: f32, across: f32) -> TacticInput {
        TacticInput { distance, along, across, can_block: true, can_pit: true }
    }

    #[test]
    fn a_cop_alongside_pits() {
        assert_eq!(choose(&input(3.0, 0.0, 2.0)), Tactic::Pit);
        assert_eq!(choose(&input(3.0, 0.0, -2.0)), Tactic::Pit, "either side");
    }

    #[test]
    fn a_cop_ahead_blocks_within_range() {
        assert_eq!(choose(&input(40.0, 35.0, 0.0)), Tactic::Block);
        assert_eq!(choose(&input(80.0, 75.0, 0.0)), Tactic::Follow, "too far to block");
    }

    #[test]
    fn a_cop_behind_close_in_rams_and_far_back_follows() {
        assert_eq!(choose(&input(5.0, -5.0, 0.0)), Tactic::Ram);
        assert_eq!(choose(&input(30.0, -30.0, 0.0)), Tactic::Follow);
    }

    #[test]
    fn disallowed_tactics_fall_through() {
        let no_pit = TacticInput { can_pit: false, ..input(3.0, 0.0, 2.0) };
        assert_eq!(choose(&no_pit), Tactic::Ram);
        let no_block = TacticInput { can_block: false, ..input(40.0, 35.0, 0.0) };
        assert_eq!(choose(&no_block), Tactic::Follow);
    }

    #[test]
    fn block_spreads_across_the_road_in_front() {
        let slots = block_slots(Vec2::ZERO, Vec2::X, 30.0, 6.0, 3);
        assert_eq!(slots.len(), 3);
        for slot in &slots {
            assert!((slot.x - 30.0).abs() < 1e-4, "ahead of the player");
        }
        assert!((slots[0].y - slots[2].y).abs() > 11.9, "edge to edge: 12 m apart at a half width of 6 m");
        assert!(slots[1].y.abs() < 1e-4, "middle car centred");
    }

    #[test]
    fn a_single_block_car_stands_in_the_middle() {
        assert_eq!(block_slots(Vec2::ZERO, Vec2::X, 20.0, 5.0, 1), vec![Vec2::new(20.0, 0.0)]);
        assert!(block_slots(Vec2::ZERO, Vec2::X, 20.0, 5.0, 0).is_empty());
    }

    #[test]
    fn pit_point_is_the_rear_quarter_on_the_chosen_side() {
        let right = pit_point(Vec2::ZERO, Vec2::X, 1.0);
        let left = pit_point(Vec2::ZERO, Vec2::X, -1.0);
        assert!(right.x < 0.0 && left.x < 0.0, "behind the player");
        assert!((right.y - left.y).abs() > 1.9, "opposite sides");
    }
}

/// Half the width of the road a block spreads across, metres (two lanes of the install's roads).
pub const ROAD_HALF_WIDTH: f32 = 7.0;
/// How far ahead of the player a road block stands, metres.
pub const BLOCK_AHEAD: f32 = 30.0;
/// The most cars a road block uses.
pub const MAX_BLOCK_CARS: usize = 3;

/// The point each cop drives at this think, given the player and which tactics the chase allows. `None` means the
/// cop chases the player itself (follow or ram). Cops blocking take the block slots nearest first.
pub fn plan(cops: &[Vec2], player: Vec2, forward: Vec2, can_block: bool, can_pit: bool) -> Vec<Option<Vec2>> {
    let forward = forward.normalize_or_zero();
    let right = right_of(forward);
    let tactics: Vec<(Tactic, f32, f32)> = cops
        .iter()
        .map(|cop| {
            let rel = *cop - player;
            let along = rel.dot(forward);
            let across = rel.dot(right);
            let input = TacticInput { distance: rel.length(), along, across, can_block, can_pit };
            (choose(&input), along, across)
        })
        .collect();
    let mut blockers: Vec<usize> = (0..cops.len()).filter(|&i| tactics[i].0 == Tactic::Block).collect();
    blockers.sort_by(|&a, &b| (cops[a] - player).length().total_cmp(&(cops[b] - player).length()));
    let slots = block_slots(player, forward, BLOCK_AHEAD, ROAD_HALF_WIDTH, blockers.len().min(MAX_BLOCK_CARS));
    let mut aims = vec![None; cops.len()];
    for (k, &i) in blockers.iter().enumerate() {
        aims[i] = slots.get(k % slots.len().max(1)).copied();
    }
    for (i, &(tactic, _, across)) in tactics.iter().enumerate() {
        if tactic == Tactic::Pit {
            aims[i] = Some(pit_point(player, forward, across.signum()));
        }
    }
    aims
}

#[cfg(test)]
mod plan_tests {
    use super::*;

    #[test]
    fn a_cop_ahead_and_far_gets_a_block_slot_ahead_of_the_player() {
        let aims = plan(&[Vec2::new(40.0, 0.0)], Vec2::ZERO, Vec2::X, true, true);
        let aim = aims[0].expect("blocks");
        assert!((aim.x - BLOCK_AHEAD).abs() < 1e-4);
    }

    #[test]
    fn a_cop_alongside_is_given_the_rear_quarter_to_pit() {
        let aims = plan(&[Vec2::new(0.0, 2.5)], Vec2::ZERO, Vec2::X, false, true);
        let aim = aims[0].expect("pits");
        assert!(aim.x < 0.0, "behind the player: {aim:?}");
    }

    #[test]
    fn a_cop_that_neither_blocks_nor_pits_chases_the_player() {
        let aims = plan(&[Vec2::new(-30.0, 0.0)], Vec2::ZERO, Vec2::X, true, true);
        assert_eq!(aims[0], None);
    }

    #[test]
    fn blockers_share_the_slots_across_the_road() {
        let cops = [Vec2::new(40.0, 0.0), Vec2::new(45.0, 1.0), Vec2::new(50.0, -1.0)];
        let aims: Vec<Vec2> = plan(&cops, Vec2::ZERO, Vec2::X, true, false).into_iter().flatten().collect();
        assert_eq!(aims.len(), 3);
        let spread = aims.iter().map(|a| a.y).fold(f32::MIN, f32::max) - aims.iter().map(|a| a.y).fold(f32::MAX, f32::min);
        assert!(spread > ROAD_HALF_WIDTH, "spread across the road: {spread}");
    }

    #[test]
    fn no_cops_no_aims() {
        assert!(plan(&[], Vec2::ZERO, Vec2::X, true, true).is_empty());
    }
}
