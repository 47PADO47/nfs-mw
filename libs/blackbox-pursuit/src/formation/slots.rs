//! The slots of each formation: where a cop waits relative to the target (`x`, `z` in the target's frame, metres), its
//! priority (smaller is filled first) and the tightening of the box-in and rolling block as the finisher approaches.
//! Spec: `docs/specs/ai-pursuit-formations.md` sections 1 and 3. The in-position offsets of the finishers are not here.
//! The herd's `x` is rewritten each update by its own rule (section 4) and is not modelled.

use super::Formation;
use glam::Vec2;

/// One slot: a pursuit offset in the target's frame and its priority.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    pub offset: Vec2,
    pub priority: u32,
}

const fn slot(x: f32, z: f32, priority: u32) -> Slot {
    Slot { offset: Vec2::new(x, z), priority }
}

const STAGGER: [Slot; 6] = [
    slot(0.0, -13.0, 1),
    slot(0.0, 13.0, 1),
    slot(3.5, -13.0, 2),
    slot(-3.5, 13.0, 2),
    slot(-3.5, -13.0, 3),
    slot(3.5, 13.0, 3),
];
const FOLLOW: [Slot; 6] = [
    slot(0.0, -13.0, 1),
    slot(3.5, -13.0, 2),
    slot(-3.5, -13.0, 2),
    slot(0.0, -17.0, 3),
    slot(3.5, -17.0, 4),
    slot(-3.5, -17.0, 4),
];
const HERD: [Slot; 3] = [slot(-3.0, 0.0, 1), slot(-3.0, 5.0, 2), slot(-3.0, -5.0, 3)];
const BOX_IN: [Slot; 4] = [slot(0.0, 14.0, 1), slot(-3.5, 0.0, 2), slot(3.5, 0.0, 2), slot(0.0, -7.5, 4)];
const ROLLING_BLOCK: [Slot; 5] =
    [slot(0.0, 14.0, 1), slot(-2.5, 14.0, 2), slot(2.5, 10.0, 2), slot(-5.0, 14.0, 3), slot(5.0, 14.0, 3)];
const PIT: [Slot; 2] = [slot(4.0, -2.7, 1), slot(-4.0, -2.7, 1)];

/// The slots of a formation, as the spec gives them (before tightening).
pub fn slots(formation: Formation) -> &'static [Slot] {
    match formation {
        Formation::StaggerFollow => &STAGGER,
        Formation::Follow => &FOLLOW,
        Formation::Herd => &HERD,
        Formation::BoxIn => &BOX_IN,
        Formation::RollingBlock => &ROLLING_BLOCK,
        Formation::Pit => &PIT,
    }
}

/// The tightening scale of the box-in and rolling block (section 1): `f` is the finisher's progress (1 when the
/// in-position timer is empty, 0 when full), `t` the formation's tightness (0.5 without a level). Other formations do not
/// tighten and always scale by 1.
pub fn tightening(formation: Formation, f: f32, t: f32) -> f32 {
    let k = match formation {
        Formation::BoxIn => 0.2 * t + 0.2,
        Formation::RollingBlock => 0.4 * t,
        _ => return 1.0,
    };
    f.clamp(0.0, 1.0) * k + (1.0 - k)
}

/// The pursuit offsets of a formation this update: the spec's slots, each scaled by the tightening.
pub fn offsets(formation: Formation, f: f32, t: f32) -> Vec<Vec2> {
    let scale = tightening(formation, f, t);
    slots(formation).iter().map(|s| s.offset * scale).collect()
}

/// The slot indices in the order cops fill them: priority first, then the table's order.
pub fn fill_order(formation: Formation) -> Vec<usize> {
    let table = slots(formation);
    let mut order: Vec<usize> = (0..table.len()).collect();
    order.sort_by_key(|&i| table[i].priority);
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_in_has_one_cop_in_front_and_two_beside() {
        let box_in = slots(Formation::BoxIn);
        assert_eq!(box_in.len(), 4);
        assert_eq!(box_in[0].offset, Vec2::new(0.0, 14.0), "in front");
        assert_eq!(box_in[1].offset.x.abs(), 3.5, "beside");
        assert_eq!(box_in[2].offset.x.abs(), 3.5, "beside");
    }

    #[test]
    fn the_tightening_is_one_when_the_timer_is_empty_and_smaller_when_full() {
        // f = 1 (nothing done yet): no tightening. f = 0 (the finisher's timer is full): the scale is k.
        assert_eq!(tightening(Formation::BoxIn, 1.0, 0.5), 1.0);
        let full = tightening(Formation::BoxIn, 0.0, 0.5);
        assert!((full - 0.7).abs() < 1e-5, "k = 0.2*0.5 + 0.2 = 0.3, scale = 1 - k = 0.7 when full: {full}");
    }

    #[test]
    fn the_rolling_block_tightens_by_its_own_factor() {
        let full = tightening(Formation::RollingBlock, 0.0, 0.5);
        assert!((full - 0.8).abs() < 1e-5, "k = 0.4*0.5 = 0.2, scale = 1 - k = 0.8 when full: {full}");
    }

    #[test]
    fn other_formations_do_not_tighten() {
        assert_eq!(tightening(Formation::Follow, 0.0, 0.5), 1.0);
        assert_eq!(tightening(Formation::Pit, 0.0, 0.0), 1.0);
    }

    #[test]
    fn offsets_scale_with_the_tightening() {
        let loose = offsets(Formation::BoxIn, 1.0, 0.5);
        let tight = offsets(Formation::BoxIn, 0.0, 0.5);
        assert_eq!(loose[0], Vec2::new(0.0, 14.0));
        assert!((tight[0].y - 14.0 * 0.7).abs() < 1e-4);
    }

    #[test]
    fn cops_fill_the_priority_slots_first() {
        let order = fill_order(Formation::Follow);
        assert_eq!(order[0], 0, "priority 1 is the first slot");
        assert_eq!(slots(Formation::Follow)[order[5]].priority, 4);
    }
}

/// The aim of each cop in a formation this update: the cops nearest the target take the highest-priority slots, and
/// each slot is placed in the target's frame (`forward` is the target's facing, metres in the map plane). Cops beyond
/// the slots get `None` and keep chasing the target. Input and output are in the same order as `cops`.
pub fn assign(formation: Formation, cops: &[Vec2], target: Vec2, forward: Vec2) -> Vec<Option<Vec2>> {
    let forward = forward.normalize_or_zero();
    let side = Vec2::new(forward.y, -forward.x);
    let offsets = offsets(formation, 1.0, 0.5);
    let mut by_distance: Vec<usize> = (0..cops.len()).collect();
    by_distance.sort_by(|&a, &b| (cops[a] - target).length().total_cmp(&(cops[b] - target).length()));
    let mut aims = vec![None; cops.len()];
    for (&cop, slot) in by_distance.iter().zip(fill_order(formation)) {
        let offset = offsets[slot];
        aims[cop] = Some(target + side * offset.x + forward * offset.y);
    }
    aims
}

#[cfg(test)]
mod assign_tests {
    use super::*;

    #[test]
    fn the_nearest_cop_takes_the_first_priority_slot() {
        // Box-in: the first slot (priority 1) is one car length in front of the target, along its facing (+X here).
        let cops = [Vec2::new(-30.0, 0.0), Vec2::new(20.0, 0.0)];
        let aims = assign(Formation::BoxIn, &cops, Vec2::ZERO, Vec2::X);
        let nearest = aims[1].expect("the nearer cop has a slot");
        assert!((nearest.x - 14.0).abs() < 1e-4, "in front: {nearest:?}");
    }

    #[test]
    fn cops_beyond_the_slots_keep_the_chase() {
        let cops: Vec<Vec2> = (0..6).map(|i| Vec2::new(-(i as f32) * 10.0, 0.0)).collect();
        let aims = assign(Formation::Pit, &cops, Vec2::ZERO, Vec2::X);
        assert_eq!(aims.iter().filter(|a| a.is_some()).count(), 2, "pit has two slots");
    }

    #[test]
    fn slots_are_placed_across_the_targets_facing() {
        // A target facing +Z: the side is then across X, and the box-in's beside slots are at x = +-3.5 from it.
        let cops = [Vec2::new(0.0, -20.0), Vec2::new(0.0, -10.0), Vec2::new(0.0, -5.0)];
        let aims = assign(Formation::BoxIn, &cops, Vec2::ZERO, Vec2::Y);
        let placed: Vec<Vec2> = aims.into_iter().flatten().collect();
        assert_eq!(placed.len(), 3);
        assert!(placed.iter().any(|p| (p.y - 14.0).abs() < 1e-4), "one in front along the facing");
    }

    #[test]
    fn no_cops_no_aims() {
        assert!(assign(Formation::Follow, &[], Vec2::ZERO, Vec2::X).is_empty());
    }
}

/// How close the herd presses the target: `crowd = max(1, min(3, ro - rl + 2))`, where `ro` is the target's signed
/// lateral distance from the road centre and `rl` the greatest offset of a traffic lane on its side (spec section 4).
pub fn herd_crowd(ro: f32, rl: f32) -> f32 {
    (ro - rl + 2.0).clamp(1.0, 3.0)
}

/// The herd's offsets for a crowd: every slot's `x` is `-crowd`, its `z` as the table gives (spec section 4).
pub fn herd_offsets(crowd: f32) -> Vec<Vec2> {
    slots(Formation::Herd).iter().map(|s| Vec2::new(-crowd, s.offset.y)).collect()
}

#[cfg(test)]
mod herd_tests {
    use super::*;

    #[test]
    fn the_crowd_is_clamped_between_one_and_three_metres() {
        assert_eq!(herd_crowd(0.0, 0.0), 2.0);
        assert_eq!(herd_crowd(-10.0, 0.0), 1.0, "at least one metre");
        assert_eq!(herd_crowd(10.0, 0.0), 3.0, "at most three metres");
    }

    #[test]
    fn the_outer_traffic_lane_pulls_the_herd_in() {
        // A target 4 m right of centre with traffic lanes out to 3 m: 4 - 3 + 2 = 3 m.
        assert_eq!(herd_crowd(4.0, 3.0), 3.0);
        // The same target with traffic lanes out to 5 m: 4 - 5 + 2 = 1 m.
        assert_eq!(herd_crowd(4.0, 5.0), 1.0);
    }

    #[test]
    fn every_herd_slot_sits_at_minus_crowd_in_x() {
        let offsets = herd_offsets(2.5);
        assert_eq!(offsets.len(), 3);
        assert!(offsets.iter().all(|o| (o.x + 2.5).abs() < 1e-6));
        assert_eq!(offsets[1].y, 5.0, "the z values are the table's");
    }
}

/// The in-position offsets of a formation during its finisher (spec section 1, the in-position column), with the
/// tightness `t` (0.5 without a level). `None` for formations without a finisher. Box-in and rolling block use `foff = 2
/// - 5t` for the depth, box-in `s = 0.7 - 0.5t`, rolling block `s = 1 - 0.8t`. Herd and follow have none.
pub fn in_position_offsets(formation: Formation, t: f32) -> Option<Vec<Vec2>> {
    match formation {
        Formation::Pit => Some(vec![Vec2::new(-10.0, -2.7), Vec2::new(10.0, -2.7)]),
        Formation::BoxIn => {
            let (foff, s) = (2.0 - 5.0 * t, 0.7 - 0.5 * t);
            Some(vec![Vec2::new(0.0, foff), Vec2::new(-3.5 * s, 0.0), Vec2::new(3.5 * s, 0.0), Vec2::new(0.0, -7.5 * s)])
        }
        Formation::RollingBlock => {
            let (foff, s) = (2.0 - 5.0 * t, 1.0 - 0.8 * t);
            Some(slots(formation).iter().map(|slot| Vec2::new(slot.offset.x * s, foff)).collect())
        }
        _ => None,
    }
}

/// The slot each cop is given (the same order as [`assign`]): the cop index and its slot index, nearest cops first.
pub fn assign_slots(formation: Formation, cops: &[Vec2], target: Vec2) -> Vec<Option<usize>> {
    let mut by_distance: Vec<usize> = (0..cops.len()).collect();
    by_distance.sort_by(|&a, &b| (cops[a] - target).length().total_cmp(&(cops[b] - target).length()));
    let mut slot_of = vec![None; cops.len()];
    for (&cop, slot) in by_distance.iter().zip(fill_order(formation)) {
        slot_of[cop] = Some(slot);
    }
    slot_of
}

#[cfg(test)]
mod in_position_tests {
    use super::*;

    #[test]
    fn the_pit_sweeps_across_the_rear() {
        let offsets = in_position_offsets(Formation::Pit, 0.5).expect("pit has a finisher");
        assert_eq!(offsets[0], Vec2::new(-10.0, -2.7), "the first cop sweeps to the far rear");
        assert_eq!(offsets[1], Vec2::new(10.0, -2.7));
    }

    #[test]
    fn the_box_in_closes_in_by_its_tightness() {
        let offsets = in_position_offsets(Formation::BoxIn, 0.5).expect("box-in has a finisher");
        assert!((offsets[0].y - (2.0 - 2.5)).abs() < 1e-5, "foff = 2 - 5*0.5");
        assert!((offsets[1].x + 3.5 * 0.45).abs() < 1e-5, "s = 0.7 - 0.5*0.5 = 0.45");
    }

    #[test]
    fn the_rolling_block_scales_its_row() {
        let offsets = in_position_offsets(Formation::RollingBlock, 0.5).expect("has a finisher");
        assert!((offsets[0].y + 0.5).abs() < 1e-5, "foff = -0.5");
        assert!((offsets[0].x - 0.0).abs() < 1e-5);
        assert!((offsets[1].x + 2.5 * 0.6).abs() < 1e-5, "s = 1 - 0.8*0.5 = 0.6");
    }

    #[test]
    fn formations_without_a_finisher_have_no_in_position_offsets() {
        assert!(in_position_offsets(Formation::Follow, 0.5).is_none());
        assert!(in_position_offsets(Formation::Herd, 0.5).is_none());
    }

    #[test]
    fn the_slot_assignment_matches_the_offsets_order() {
        let cops = [Vec2::new(0.0, -20.0), Vec2::new(0.0, -2.0)];
        let slots_of = assign_slots(Formation::Pit, &cops, Vec2::ZERO);
        assert_eq!(slots_of[1], Some(0), "the nearest cop takes the first slot");
        assert_eq!(slots_of[0], Some(1));
    }
}
