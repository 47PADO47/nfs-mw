//! Formation selection and weighted matching in the target frame. The caller filters candidates.
//! Spec: `docs/specs/ai-pursuit-formations.md` sections 3 and 8.

use glam::Vec2;

use super::{Formation, slots};

/// The maximum simultaneous members, independent of the number of authored slots.
pub fn capacity(formation: Formation) -> usize {
    match formation {
        Formation::Pit => 1,
        Formation::BoxIn | Formation::RollingBlock => 4,
        Formation::Herd => 3,
        Formation::Follow | Formation::StaggerFollow => 6,
    }
}

/// Select priority slots collectively near the candidates, then match them with longitudinal distance weighted 0.25.
/// Inputs and output offsets use the target's right/forward frame. Unmatched candidates have no formation slot.
pub fn assign(formation: Formation, candidates: &[Vec2], offsets: &[Vec2]) -> Vec<Option<usize>> {
    let table = slots::slots(formation);
    assert_eq!(offsets.len(), table.len());
    let count = candidates.len().min(capacity(formation)).min(offsets.len());
    let mut selected = Vec::with_capacity(count);
    while selected.len() < count {
        let next = (0..offsets.len()).filter(|i| !selected.contains(i)).min_by(|&a, &b| {
            table[a].priority.cmp(&table[b].priority).then_with(|| {
                let distance = |slot: usize| candidates.iter().map(|p| p.distance(offsets[slot])).sum::<f32>();
                distance(a).total_cmp(&distance(b))
            })
        });
        let Some(next) = next else { break };
        selected.push(next);
    }
    let points: Vec<Vec2> = selected.iter().map(|&i| offsets[i]).collect();
    match_offsets(candidates, &points).into_iter().map(|slot| slot.map(|s| selected[s])).collect()
}

fn distance(a: Vec2, b: Vec2) -> f32 {
    let delta = a - b;
    Vec2::new(delta.x, delta.y * 0.25).length_squared()
}

/// Assign the most isolated remaining slot first, to its closest remaining car.
pub fn match_offsets(candidates: &[Vec2], offsets: &[Vec2]) -> Vec<Option<usize>> {
    let mut assigned = vec![None; candidates.len()];
    let mut free = vec![true; offsets.len()];
    for _ in 0..candidates.len().min(offsets.len()) {
        let mut farthest: Option<(usize, f32)> = None;
        for (cop, &position) in candidates.iter().enumerate() {
            if assigned[cop].is_some() {
                continue;
            }
            for (slot, &point) in offsets.iter().enumerate() {
                if !free[slot] {
                    continue;
                }
                let gap = distance(position, point);
                if farthest.is_none_or(|(_, previous)| gap > previous) {
                    farthest = Some((slot, gap));
                }
            }
        }
        let Some((slot, _)) = farthest else { break };
        let nearest = (0..candidates.len())
            .filter(|&i| assigned[i].is_none())
            .min_by(|&a, &b| distance(candidates[a], offsets[slot]).total_cmp(&distance(candidates[b], offsets[slot])));
        let Some(cop) = nearest else { break };
        assigned[cop] = Some(slot);
        free[slot] = false;
    }
    assigned
}

/// Extra cars alternate ahead and behind, three columns wide, with further rows five metres apart.
pub fn grid(count: usize) -> Vec<Vec2> {
    (0..count)
        .map(|i| {
            let row = i / 6;
            let side = match i % 2 {
                0 => 1.0,
                _ => -1.0,
            };
            let column = ((i / 2 + 1) % 3) as f32 - 1.0;
            Vec2::new(3.5 * column, side * (5.0 * row as f32 + 25.0))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_single_pit_attacker_uses_the_rear_quarter_on_its_own_side() {
        let offsets = slots::offsets(Formation::Pit, 1.0, 0.5);
        let assignments = assign(Formation::Pit, &[Vec2::new(-4.0, -2.7)], &offsets);
        assert_eq!(assignments, [Some(1)]);
    }

    #[test]
    fn attack_limits_do_not_use_every_authored_slot() {
        for (formation, expected) in [(Formation::Pit, 1), (Formation::RollingBlock, 4)] {
            let offsets = slots::offsets(formation, 1.0, 0.5);
            let assignments = assign(formation, &grid(8), &offsets);
            assert_eq!(assignments.iter().flatten().count(), expected);
        }
    }

    #[test]
    fn slot_matching_weights_lateral_position_more_than_distance_to_player() {
        let cars = [Vec2::new(-3.5, 20.0), Vec2::new(3.5, -20.0)];
        let offsets = [Vec2::new(-3.5, 0.0), Vec2::new(3.5, 0.0)];
        assert_eq!(match_offsets(&cars, &offsets), [Some(0), Some(1)]);
    }

    #[test]
    fn a_second_equal_priority_slot_is_chosen_from_the_cars_positions() {
        let offsets = slots::offsets(Formation::BoxIn, 1.0, 0.5);
        let assignments = assign(Formation::BoxIn, &[offsets[0], offsets[2]], &offsets);
        assert_eq!(assignments, [Some(0), Some(2)]);
    }

    #[test]
    fn all_extra_cars_get_distinct_grid_slots() {
        let offsets = grid(14);
        assert_eq!(offsets[0], Vec2::new(0.0, 25.0));
        assert_eq!(offsets[1], Vec2::new(0.0, -25.0));
        assert_eq!(offsets[6], Vec2::new(0.0, 30.0));
        for (i, point) in offsets.iter().enumerate() {
            assert!(!offsets[..i].contains(point));
        }
    }
}
