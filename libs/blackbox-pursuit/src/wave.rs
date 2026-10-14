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

#[cfg(test)]
mod tests {
    use super::*;

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
