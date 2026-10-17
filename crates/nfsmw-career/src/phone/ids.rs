//! Message ids: the immediate set, the story split, the label names and the sender table
//! (`docs/specs/career-phone.md` sections 1.2, 1.3 and 3.1, `docs/specs/career-text-2.md` section 1.2).
//!
//! The sender table is numeric metadata only. The sender text, the subject and the body are game text and are
//! loaded at run time from the install language table by label ([`label_for`]); none of it is copied here.

/// The script event the win tick fires when an impounded car becomes releasable (`0x78`) is the SMS with this
/// id: "your car can be released".
pub const IMPOUND_RELEASE_SMS: u32 = 120;

/// The id of the message a new player gets on the first drive of the prologue (`drive_to_target` start).
pub const FIRST_DRIVE_SMS: u32 = 15;

/// Who a message is from. The names are the in-game contacts; the display text comes from the language table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sender {
    /// The contact who briefs the player (45 messages).
    Rog,
    /// 26 messages.
    Mia,
    /// 11 messages.
    Razor,
    /// 4 messages.
    Cross,
    /// 3 messages.
    Vic,
    /// 2 messages.
    Ronnie,
    /// 2 messages.
    Earl,
    /// The police department notice (id 120).
    RockportPolice,
    /// The empty sender of id 122.
    NoSender,
}

/// Every valid message id with its sender: the 95 messages of the language table, ids 10 to 137 with gaps.
#[rustfmt::skip]
pub const SENDERS: [(u8, Sender); 95] = [
    (10, Sender::Mia), (11, Sender::Razor), (12, Sender::Rog), (13, Sender::Rog), (15, Sender::Mia),
    (16, Sender::Rog), (17, Sender::Razor), (18, Sender::Rog), (19, Sender::Mia), (20, Sender::Rog),
    (21, Sender::Rog), (22, Sender::Rog), (23, Sender::Rog), (24, Sender::Vic), (25, Sender::Mia),
    (26, Sender::Mia), (27, Sender::Rog), (29, Sender::Rog), (30, Sender::Rog), (31, Sender::Mia),
    (32, Sender::Vic), (33, Sender::Rog), (35, Sender::Rog), (36, Sender::Rog), (37, Sender::Mia),
    (38, Sender::Mia), (40, Sender::Mia), (41, Sender::Rog), (42, Sender::Rog), (43, Sender::Mia),
    (46, Sender::Rog), (47, Sender::Mia), (48, Sender::Razor), (49, Sender::Rog), (50, Sender::Mia),
    (52, Sender::Rog), (53, Sender::Mia), (54, Sender::Mia), (55, Sender::Earl), (57, Sender::Rog),
    (58, Sender::Mia), (59, Sender::Razor), (60, Sender::Rog), (61, Sender::Mia), (63, Sender::Rog),
    (64, Sender::Ronnie), (65, Sender::Mia), (66, Sender::Razor), (69, Sender::Rog), (70, Sender::Mia),
    (72, Sender::Rog), (73, Sender::Rog), (74, Sender::Vic), (76, Sender::Rog), (77, Sender::Razor),
    (79, Sender::Rog), (80, Sender::Rog), (81, Sender::Razor), (82, Sender::Mia), (84, Sender::Ronnie),
    (85, Sender::Mia), (86, Sender::Razor), (88, Sender::Rog), (89, Sender::Rog), (90, Sender::Razor),
    (91, Sender::Mia), (93, Sender::Rog), (94, Sender::Razor), (95, Sender::Cross), (96, Sender::Mia),
    (97, Sender::Rog), (98, Sender::Rog), (100, Sender::Cross), (101, Sender::Cross), (102, Sender::Cross),
    (105, Sender::Rog), (110, Sender::Earl), (111, Sender::Razor), (120, Sender::RockportPolice), (121, Sender::Mia),
    (122, Sender::NoSender), (123, Sender::Rog), (124, Sender::Mia), (125, Sender::Rog), (126, Sender::Rog),
    (127, Sender::Rog), (128, Sender::Rog), (129, Sender::Mia), (130, Sender::Rog), (132, Sender::Mia),
    (133, Sender::Rog), (134, Sender::Rog), (135, Sender::Rog), (136, Sender::Rog), (137, Sender::Rog),
];

/// The sender of message `id`, `None` when the language table has no such message.
pub fn sender_of(id: u32) -> Option<Sender> {
    let id = u8::try_from(id).ok()?;
    SENDERS.iter().find(|(known, _)| *known == id).map(|&(_, sender)| sender)
}

/// True when the language table has the message (the slot id byte is the index, not `0xFF`).
pub fn is_known(id: u32) -> bool {
    sender_of(id).is_some()
}

/// The `EAddSMS` ids that are delivered at once instead of queued: 10 to 14, 95 and 96.
pub const fn is_immediate(id: u32) -> bool {
    matches!(id, 10..=14 | 95 | 96)
}

/// The story / non-story split: ids 110, 111, 120 to 130 and 132 to 137 are non-story (tutorial tips, impound
/// notices and extra texts); everything else is a story message. Non-story messages have their own inbox tab and
/// no voice line.
pub const fn is_story(id: u32) -> bool {
    !matches!(id, 110 | 111 | 120..=130 | 132..=137)
}

/// The language-table label of the message body: `SMS_MESSAGE_<n>`.
pub fn label_for(id: u32) -> String {
    format!("SMS_MESSAGE_{id}")
}

/// The label of the sender text: `SMS_MESSAGE_<n>_FROM`.
pub fn from_label_for(id: u32) -> String {
    format!("SMS_MESSAGE_{id}_FROM")
}

/// The label of the subject text: `SMS_MESSAGE_<n>_SUBJECT`.
pub fn subject_label_for(id: u32) -> String {
    format!("SMS_MESSAGE_{id}_SUBJECT")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(sender: Sender) -> usize {
        SENDERS.iter().filter(|(_, s)| *s == sender).count()
    }

    #[test]
    fn the_table_has_95_ascending_ids_in_range() {
        assert!(SENDERS.windows(2).all(|w| w[0].0 < w[1].0));
        assert_eq!(SENDERS[0].0, 10);
        assert_eq!(SENDERS[94].0, 137);
    }

    #[test]
    fn sender_totals_match_the_spec() {
        assert_eq!(count(Sender::Rog), 45);
        assert_eq!(count(Sender::Mia), 26);
        assert_eq!(count(Sender::Razor), 11);
        assert_eq!(count(Sender::Cross), 4);
        assert_eq!(count(Sender::Vic), 3);
        assert_eq!(count(Sender::Ronnie), 2);
        assert_eq!(count(Sender::Earl), 2);
        assert_eq!(count(Sender::RockportPolice), 1);
        assert_eq!(count(Sender::NoSender), 1);
    }

    #[test]
    fn lookups_follow_the_table() {
        assert_eq!(sender_of(10), Some(Sender::Mia));
        assert_eq!(sender_of(95), Some(Sender::Cross));
        assert_eq!(sender_of(120), Some(Sender::RockportPolice));
        assert_eq!(sender_of(122), Some(Sender::NoSender));
        assert_eq!(sender_of(14), None, "14 is immediate but has no message");
        assert_eq!(sender_of(138), None);
        assert_eq!(sender_of(266), None, "ids do not wrap into the u8 range");
        assert!(is_known(137) && !is_known(0));
    }

    #[test]
    fn the_immediate_set_is_10_to_14_95_and_96() {
        let immediate: Vec<u32> = (0..150).filter(|&id| is_immediate(id)).collect();
        assert_eq!(immediate, [10, 11, 12, 13, 14, 95, 96]);
    }

    #[test]
    fn the_non_story_ids_are_the_documented_list() {
        let non_story: Vec<u32> = (0..150).filter(|&id| !is_story(id)).collect();
        let expected: Vec<u32> = [110, 111].into_iter().chain(120..=130).chain(132..=137).collect();
        assert_eq!(non_story, expected);
        assert!(is_story(15) && is_story(105) && is_story(131), "131 is not in the list");
    }

    #[test]
    fn the_tip_ids_in_the_table_are_exactly_the_non_story_ones() {
        let non_story_known: Vec<u8> =
            SENDERS.iter().map(|&(id, _)| id).filter(|&id| !is_story(u32::from(id))).collect();
        assert_eq!(
            non_story_known,
            [110, 111, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 132, 133, 134, 135, 136, 137]
        );
    }

    #[test]
    fn labels_have_the_documented_format() {
        assert_eq!(label_for(15), "SMS_MESSAGE_15");
        assert_eq!(from_label_for(15), "SMS_MESSAGE_15_FROM");
        assert_eq!(subject_label_for(137), "SMS_MESSAGE_137_SUBJECT");
    }
}
