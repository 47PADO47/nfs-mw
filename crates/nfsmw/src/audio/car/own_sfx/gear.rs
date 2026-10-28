//! The gear-change trigger of a car whose manifest says `shift_trigger = "gear"` (docs/modding/car-sound.md, "Other
//! sounds"): the shift recordings play at every change of the gear number, up when the new gear is higher and down when it is
//! lower, with no minimum engine speed or road speed and no delay (spec `docs/specs/ride6-engine-sound.md` section 3,
//! "Gear events"). The default trigger is the engine mixer's gear clunk, which ignores up shifts under 3000 rpm and plays a
//! little after the shift begins (`blackbox_carsound`, the 2005 cars' rules).

/// The volume a gear-change recording plays at (before the host's one-shot level): the full level. A modelling choice
/// (spec section 7): the original posts the event with no level of its own and the gain is authored in a sound bank that is
/// not readable.
pub const GEAR_SHOT_VOLUME: f32 = 1.0;

/// Watches the gear number from frame to frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GearWatch {
    last: Option<i32>,
}

impl GearWatch {
    /// The gear now. `Some(true)` for a change to a higher gear, `Some(false)` to a lower one, `None` for no change. The
    /// first gear seen is no change: a car that starts in gear does not shift.
    pub fn observe(&mut self, gear: i32) -> Option<bool> {
        let last = self.last.replace(gear)?;
        if gear == last {
            return None;
        }
        Some(gear > last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_is_up_or_down_by_comparing_the_gear_numbers() {
        let mut watch = GearWatch::default();
        let seen: Vec<_> = [2, 2, 3, 3, 4, 3, 2, 2, 3].into_iter().map(|gear| watch.observe(gear)).collect();
        assert_eq!(seen, [None, None, Some(true), None, Some(true), Some(false), Some(false), None, Some(true)]);
    }

    #[test]
    fn the_first_gear_seen_is_not_a_shift() {
        assert_eq!(GearWatch::default().observe(5), None);
    }

    #[test]
    fn a_jump_over_several_gears_is_one_shift() {
        let mut watch = GearWatch::default();
        watch.observe(2);
        assert_eq!(watch.observe(5), Some(true));
        assert_eq!(watch.observe(2), Some(false));
    }
}
