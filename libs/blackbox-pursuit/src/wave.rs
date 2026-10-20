//! The cop wave as a rule: which car type a chase spawns next. A wave is a list of car types with how many of each it
//! wants and a spawn weight; a chase picks, at random by weight, among the types it still lacks. The wave is read again
//! at every pick, so a rise in heat (a different row) brings the types of that row.
//! Spec: `docs/specs/ai-pursuit-cops.md` (section 4).

/// One car type of a wave: its name, how many the wave wants, and its spawn weight in percent (0 counts as 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot<'a> {
    pub name: &'a str,
    pub count: u32,
    pub chance: u32,
}

/// The index of the slot to spawn next, or `None` when the wave has every car it wants.
///
/// `have` counts the cars of a type already in the chase. `roll` is a uniform number in `0..1`; it is clamped.
/// A type is eligible while `have(name) < count`, and eligible types are drawn by weight.
pub fn pick_needed(slots: &[Slot<'_>], have: impl Fn(&str) -> u32, roll: f32) -> Option<usize> {
    let weights: Vec<(usize, u32)> =
        slots.iter().enumerate().filter(|(_, s)| have(s.name) < s.count).map(|(i, s)| (i, s.chance.max(1))).collect();
    let total: u32 = weights.iter().map(|w| w.1).sum();
    if total == 0 {
        return None;
    }
    let mut left = ((roll.clamp(0.0, 1.0) * total as f32) as u32).min(total - 1);
    for (index, weight) in weights {
        if left < weight {
            return Some(index);
        }
        left -= weight;
    }
    None
}

/// The counts of a wave scaled to the cops the chase may have now (`AdjustedCopCounts`). `nominal` is the count of each
/// ground record in order; `want` is `clamp(sum(nominal), min, max)` for the chase's limits. Each record gets
/// `round(count * want / nominal_left)`, taken off `want` and `nominal_left` as it goes, so the counts add up to `want`.
pub fn adjusted_counts(nominal: &[u32], want: u32) -> Vec<u32> {
    let mut nominal_left: u32 = nominal.iter().sum();
    let mut want_left = want;
    let mut out = Vec::with_capacity(nominal.len());
    for &count in nominal {
        let scaled = match nominal_left {
            0 => 0,
            _ => ((count as f32 * want_left as f32) / nominal_left as f32 + 0.5) as u32,
        };
        let scaled = scaled.min(want_left);
        out.push(scaled);
        want_left -= scaled;
        nominal_left -= count;
    }
    out
}

/// The cops the chase may have now: the nominal total held to `[min, max]` (`want_cops = clamp(nominal, min, max)`).
pub fn want_cops(nominal_total: u32, min: u32, max: u32) -> u32 {
    nominal_total.max(min).min(max.max(min))
}

/// The car type that sends cops away this run (`RemoveUnwantedVehicles`). `surplus` is, per type in the contingent's
/// order, how many cops of it the chase has over what it wants. The pick is random weighted by surplus: type `i` is taken
/// unless `roll < earlier / (earlier + surplus_i)`, where `earlier` is the surplus of the types passed over.
/// Returns the index and the number to send, at most 2.
pub fn pick_surplus(surplus: &[u32], roll: f32) -> Option<(usize, u32)> {
    let mut passed = 0u32;
    for (i, &over) in surplus.iter().enumerate() {
        if over < 1 {
            continue;
        }
        if roll >= passed as f32 / (passed + over) as f32 {
            return Some((i, over.min(2)));
        }
        passed += over;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wave_is_scaled_to_what_the_chase_may_have() {
        assert_eq!(adjusted_counts(&[4], 4), [4]);
        assert_eq!(adjusted_counts(&[4], 2), [2]);
        assert_eq!(adjusted_counts(&[4], 0), [0]);
        assert_eq!(adjusted_counts(&[7, 2], 9), [7, 2], "the whole wave");
        assert_eq!(adjusted_counts(&[7, 2], 3), [2, 1], "7*3/9 rounds to 2, then 1 of the last 2");
        assert_eq!(adjusted_counts(&[3, 3, 3], 5).iter().sum::<u32>(), 5, "the counts add up to the want");
        assert_eq!(adjusted_counts(&[], 5), Vec::<u32>::new());
    }

    #[test]
    fn a_want_beyond_the_wave_scales_it_up() {
        assert_eq!(
            adjusted_counts(&[2, 2], 10),
            [5, 5],
            "a cool-down that wants more cops than the row has scales it up"
        );
    }

    #[test]
    fn the_cops_wanted_are_held_between_the_limits() {
        assert_eq!(want_cops(4, 0, 3), 3);
        assert_eq!(want_cops(4, 0, 9), 4);
        assert_eq!(want_cops(4, 6, 9), 6, "cool-down never wants fewer than its minimum");
        assert_eq!(want_cops(4, 2, 1), 2, "the minimum wins over a smaller maximum");
        assert_eq!(want_cops(0, 0, 0), 0);
    }

    #[test]
    fn the_surplus_type_is_picked_by_weight() {
        assert_eq!(pick_surplus(&[0, 0], 0.5), None, "no surplus, nobody leaves");
        assert_eq!(pick_surplus(&[3], 0.0), Some((0, 2)), "at most two leave at a time");
        assert_eq!(pick_surplus(&[1], 0.9), Some((0, 1)));
        // Two types over by 1 and 3: the first is taken unless the roll is under 0 (never), the second needs roll >= 1/4.
        assert_eq!(pick_surplus(&[1, 3], 0.0), Some((0, 1)));
        assert_eq!(pick_surplus(&[1, 3], 0.9), Some((0, 1)), "the first type is always taken when nothing precedes it");
        assert_eq!(pick_surplus(&[0, 3], 0.5), Some((1, 2)));
    }

    fn heat_four() -> [Slot<'static>; 1] {
        [Slot { name: "copgtoghost", count: 8, chance: 100 }]
    }

    #[test]
    fn a_wave_that_is_met_picks_nothing() {
        assert_eq!(pick_needed(&heat_four(), |_| 8, 0.5), None);
    }

    #[test]
    fn a_risen_heat_brings_its_own_type_once_the_old_wave_is_met() {
        // Heat 3 wants copgto x7; once they are all in the chase, heat 4's copgtoghost x8 is what gets picked.
        let heat_three = [Slot { name: "copgto", count: 7, chance: 100 }];
        let have = |name: &str| if name == "copgto" { 7 } else { 0 };
        assert_eq!(pick_needed(&heat_three, have, 0.5), None);
        assert_eq!(pick_needed(&heat_four(), |_| 0, 0.5), Some(0));
    }

    #[test]
    fn met_types_are_skipped_and_the_rest_share_the_weight() {
        let slots = [
            Slot { name: "copgto", count: 7, chance: 100 },
            Slot { name: "copsuvpatrol", count: 2, chance: 80 },
            Slot { name: "copsportghost", count: 8, chance: 20 },
        ];
        let have = |name: &str| if name == "copgto" { 7 } else { 0 };
        // copgto is met; the remaining weights are 80 and 20 out of 100.
        assert_eq!(pick_needed(&slots, have, 0.0), Some(1));
        assert_eq!(pick_needed(&slots, have, 0.79), Some(1));
        assert_eq!(pick_needed(&slots, have, 0.81), Some(2));
        assert_eq!(pick_needed(&slots, have, 1.0), Some(2));
    }

    #[test]
    fn a_zero_weight_still_counts_as_one() {
        let slots = [Slot { name: "copsport", count: 1, chance: 0 }];
        assert_eq!(pick_needed(&slots, |_| 0, 0.9), Some(0));
    }
}
