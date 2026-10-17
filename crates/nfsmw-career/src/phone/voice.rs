//! The voice lines behind the calls (`docs/specs/career-phone.md` section 1.4): which `(group, line)` selector a
//! call id maps to. Playing the take is the audio layer's job; this is only the selector.

use super::ids::is_known;

/// A voice take selector: the speech system gets the masks `1 << group` and `1 << line`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceTake {
    /// The group: follows the Blacklist bins (16 = prologue, 0 = the post-game set).
    pub group: u8,
    /// The line inside the group.
    pub line: u8,
}

impl VoiceTake {
    /// The two one-bit masks of the `CellCall` speech request: `(1 << group, 1 << line)`.
    pub const fn masks(self) -> (u32, u32) {
        (1 << self.group, 1 << self.line)
    }
}

/// `(first id, last id, group)`; the line is `id - first id`.
const GROUPS: [(u32, u32, u8); 17] = [
    (10, 16, 16),
    (17, 22, 15),
    (23, 29, 14),
    (30, 36, 13),
    (37, 41, 12),
    (42, 46, 11),
    (47, 52, 10),
    (53, 57, 9),
    (58, 63, 8),
    (64, 69, 7),
    (70, 72, 6),
    (73, 76, 5),
    (77, 79, 4),
    (80, 84, 3),
    (85, 88, 2),
    (89, 96, 1),
    (97, 105, 0),
];

/// The selector of a call id; ids outside 10 to 105 have no voice. (Whether a take was recorded for the id is
/// [`has_recorded_take`].)
pub fn voice_take(id: u32) -> Option<VoiceTake> {
    GROUPS
        .iter()
        .find(|&&(first, last, _)| (first..=last).contains(&id))
        .map(|&(first, _, group)| VoiceTake { group, line: (id - first) as u8 })
}

/// True when the install has a recorded take for the id: every valid message from 10 to 105 except 16 (75 of the
/// 76 valid ids). A request for id 16 is still made and plays nothing.
pub fn has_recorded_take(id: u32) -> bool {
    id != 16 && voice_take(id).is_some() && is_known(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phone::ids::SENDERS;

    #[test]
    fn selectors_follow_the_group_table() {
        assert_eq!(voice_take(10), Some(VoiceTake { group: 16, line: 0 }));
        assert_eq!(voice_take(16), Some(VoiceTake { group: 16, line: 6 }));
        assert_eq!(voice_take(17), Some(VoiceTake { group: 15, line: 0 }));
        assert_eq!(voice_take(52), Some(VoiceTake { group: 10, line: 5 }));
        assert_eq!(voice_take(89), Some(VoiceTake { group: 1, line: 0 }));
        assert_eq!(voice_take(96), Some(VoiceTake { group: 1, line: 7 }));
        assert_eq!(voice_take(97), Some(VoiceTake { group: 0, line: 0 }));
        assert_eq!(voice_take(105), Some(VoiceTake { group: 0, line: 8 }));
    }

    #[test]
    fn ids_outside_10_to_105_have_no_voice() {
        assert_eq!(voice_take(9), None);
        assert_eq!(voice_take(106), None);
        assert_eq!(voice_take(120), None);
        assert_eq!(voice_take(u32::MAX), None);
    }

    #[test]
    fn the_groups_tile_10_to_105_without_gaps_or_overlaps() {
        let mut next = 10;
        for &(first, last, _) in &GROUPS {
            assert_eq!(first, next);
            assert!(last >= first);
            next = last + 1;
        }
        assert_eq!(next, 106);
    }

    #[test]
    fn the_request_masks_are_one_bit_each() {
        assert_eq!(VoiceTake { group: 16, line: 6 }.masks(), (1 << 16, 1 << 6));
        assert_eq!(VoiceTake { group: 0, line: 0 }.masks(), (1, 1));
    }

    #[test]
    fn seventy_five_of_the_seventy_six_valid_calls_have_a_take() {
        let valid_in_range = SENDERS.iter().filter(|&&(id, _)| voice_take(u32::from(id)).is_some()).count();
        assert_eq!(valid_in_range, 76);
        let with_take = SENDERS.iter().filter(|&&(id, _)| has_recorded_take(u32::from(id))).count();
        assert_eq!(with_take, 75);
        assert!(!has_recorded_take(16));
        assert!(!has_recorded_take(14), "no such message");
    }
}
