//! The inbox screens and their actions: the list with its two tabs, opening, replaying and deleting a message
//! (`docs/specs/career-phone.md` sections 3.2 and 4).

use super::ids::is_story;
use super::voice::{VoiceTake, voice_take};
use crate::settings::{SmsInbox, SmsSlot, SmsState};

/// The two tabs of the inbox: story messages and the "system / tip" messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxTab {
    /// Story messages (the default tab).
    Story,
    /// Tips and notices: the non-story ids.
    Tips,
}

/// The tab a message is listed in.
pub const fn tab_of(id: u32) -> InboxTab {
    match is_story(id) {
        true => InboxTab::Story,
        false => InboxTab::Tips,
    }
}

/// One row of the inbox list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListEntry {
    /// The message id.
    pub id: u32,
    /// The delivery sequence the list is sorted by.
    pub sequence: u16,
    /// The row shows the "new message" icon (state bit 1).
    pub unread: bool,
}

/// What opening a message did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenEffect {
    /// The voice line to play: a story message that was still unread. `None` for a re-read, a tip, or an id
    /// outside the voiced range.
    pub play_voice: Option<VoiceTake>,
}

impl SmsInbox {
    /// Gives the slot the next sequence number and sets it to state 2 (delivered, unread). The counter is
    /// incremented first and the new value stored, so the first message gets 1; it wraps like the original u16.
    /// Does nothing for an id with no slot.
    pub(crate) fn mark_delivered(&mut self, id: u32) {
        let counter = self.counter.wrapping_add(1);
        let Some(slot) = self.slot_mut(id) else { return };
        slot.state = SmsState::Unread.byte();
        slot.sequence = counter;
        self.counter = counter;
    }

    /// `deliver(id)`: a slot in state 0 becomes delivered with the next sequence number. Returns whether it was
    /// delivered (the caller then posts the `ECellCall(id)` request).
    pub fn deliver(&mut self, id: u32) -> bool {
        match self.slot(id) {
            Some(slot) if slot.state == 0 => {}
            _ => return false,
        }
        self.mark_delivered(id);
        true
    }

    /// The inbox list of one tab: the delivered or read slots with a valid id, found from id 149 down to 0, sorted
    /// by sequence with the larger first (equal sequences keep that order, so calls shown as text with sequence
    /// 0 come last), then filtered by [`tab_of`].
    pub fn list(&self, tab: InboxTab) -> Vec<ListEntry> {
        let mut items: Vec<&SmsSlot> = self.slots.iter().rev().filter(|slot| slot.is_listed()).collect();
        items.sort_by_key(|slot| std::cmp::Reverse(slot.sequence));
        items
            .into_iter()
            .filter(|slot| tab_of(u32::from(slot.id)) == tab)
            .map(|slot| ListEntry { id: u32::from(slot.id), sequence: slot.sequence, unread: slot.is_unread() })
            .collect()
    }

    /// The message selected by default in a tab: the first listed id.
    pub fn default_selection(&self, tab: InboxTab) -> Option<u32> {
        self.list(tab).first().map(|entry| entry.id)
    }

    /// True when the tab holds an unread message (the widgets that mark "unread in the other tab").
    pub fn has_unread(&self, tab: InboxTab) -> bool {
        self.list(tab).iter().any(|entry| entry.unread)
    }

    /// Opens a message in the inbox: a story message that is still unread plays its voice line, then the slot
    /// becomes read (state 4). Returns `None` and changes nothing for a slot that is not in the list (not
    /// delivered or an invalid id); the spec does not say what opening such a slot would do, and the screen
    /// cannot reach one.
    pub fn open_message(&mut self, id: u32) -> Option<OpenEffect> {
        let slot = self.slot_mut(id).filter(|slot| slot.is_listed())?;
        let play_voice = match slot.is_unread() && is_story(id) {
            true => voice_take(id),
            false => None,
        };
        slot.state = SmsState::Read.byte();
        Some(OpenEffect { play_voice })
    }

    /// The replay action: story messages only, listed ones only. Returns the voice line to play after stopping
    /// other speech.
    pub fn replay(&self, id: u32) -> Option<VoiceTake> {
        let slot = self.slot(id).filter(|slot| slot.is_listed())?;
        match is_story(u32::from(slot.id)) {
            true => voice_take(id),
            false => None,
        }
    }

    /// The delete action (after the confirmation dialog): the slot goes back to state 0. Its sequence number is
    /// left as it was. A message sent again later is delivered again. Returns false for an id with no slot.
    pub fn delete(&mut self, id: u32) -> bool {
        let Some(slot) = self.slot_mut(id) else { return false };
        slot.state = SmsState::Empty.byte();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SMS_INVALID_ID;

    fn inbox_with_labels() -> SmsInbox {
        let mut inbox = SmsInbox::new();
        inbox.invalidate_missing(|i| crate::phone::ids::is_known(u32::from(i)));
        inbox
    }

    #[test]
    fn delivery_gives_increasing_sequence_numbers_from_one() {
        let mut inbox = inbox_with_labels();
        assert!(inbox.deliver(15));
        assert!(inbox.deliver(17));
        assert_eq!(inbox.slots[15], SmsSlot { id: 15, state: 2, sequence: 1 });
        assert_eq!(inbox.slots[17], SmsSlot { id: 17, state: 2, sequence: 2 });
        assert_eq!(inbox.counter, 2);
    }

    #[test]
    fn a_delivered_or_read_slot_is_not_delivered_again() {
        let mut inbox = inbox_with_labels();
        assert!(inbox.deliver(15));
        assert!(!inbox.deliver(15));
        inbox.open_message(15);
        assert!(!inbox.deliver(15));
        assert_eq!(inbox.counter, 1);
        assert!(!inbox.deliver(150), "no such slot");
    }

    #[test]
    fn the_counter_wraps_like_a_u16() {
        let mut inbox = inbox_with_labels();
        inbox.counter = u16::MAX;
        assert!(inbox.deliver(15));
        assert_eq!(inbox.slots[15].sequence, 0);
        assert_eq!(inbox.counter, 0);
    }

    #[test]
    fn the_list_is_newest_first_and_split_by_tab() {
        let mut inbox = inbox_with_labels();
        for id in [17, 124, 18, 120, 19] {
            inbox.deliver(id);
        }
        let story: Vec<u32> = inbox.list(InboxTab::Story).iter().map(|e| e.id).collect();
        let tips: Vec<u32> = inbox.list(InboxTab::Tips).iter().map(|e| e.id).collect();
        assert_eq!(story, [19, 18, 17]);
        assert_eq!(tips, [120, 124]);
        assert_eq!(inbox.default_selection(InboxTab::Story), Some(19));
        assert_eq!(inbox.default_selection(InboxTab::Tips), Some(120));
    }

    #[test]
    fn entries_without_a_sequence_sort_last_and_equal_sequences_keep_high_ids_first() {
        let mut inbox = inbox_with_labels();
        inbox.slots[10] = SmsSlot { id: 10, state: 4, sequence: 0 };
        inbox.slots[12] = SmsSlot { id: 12, state: 4, sequence: 0 };
        inbox.slots[17] = SmsSlot { id: 17, state: 4, sequence: 3 };
        inbox.slots[18] = SmsSlot { id: 18, state: 2, sequence: 3 };
        let ids: Vec<u32> = inbox.list(InboxTab::Story).iter().map(|e| e.id).collect();
        assert_eq!(ids, [18, 17, 12, 10]);
    }

    #[test]
    fn undelivered_deleted_and_invalid_slots_are_not_listed() {
        let mut inbox = inbox_with_labels();
        inbox.deliver(15);
        inbox.deliver(17);
        inbox.delete(15);
        inbox.slots[30] = SmsSlot { id: SMS_INVALID_ID, state: 2, sequence: 9 };
        let ids: Vec<u32> = inbox.list(InboxTab::Story).iter().map(|e| e.id).collect();
        assert_eq!(ids, [17]);
        assert_eq!(inbox.slots[15].sequence, 1, "delete keeps the sequence");
    }

    #[test]
    fn unread_marks_follow_the_state_and_the_tabs_report_it_separately() {
        let mut inbox = inbox_with_labels();
        inbox.deliver(17);
        inbox.deliver(123);
        assert!(inbox.has_unread(InboxTab::Story) && inbox.has_unread(InboxTab::Tips));
        inbox.open_message(17);
        assert!(!inbox.has_unread(InboxTab::Story));
        assert!(inbox.has_unread(InboxTab::Tips));
        assert!(!inbox.list(InboxTab::Story)[0].unread);
    }

    #[test]
    fn opening_an_unread_story_message_plays_its_voice_once_and_marks_it_read() {
        let mut inbox = inbox_with_labels();
        inbox.deliver(17);
        let first = inbox.open_message(17).unwrap();
        assert_eq!(first.play_voice, Some(VoiceTake { group: 15, line: 0 }));
        assert_eq!(inbox.slots[17].state, 4);
        let second = inbox.open_message(17).unwrap();
        assert_eq!(second.play_voice, None, "a re-read plays nothing");
        assert_eq!(inbox.slots[17].state, 4);
    }

    #[test]
    fn tips_and_unlisted_slots_have_no_voice_on_open() {
        let mut inbox = inbox_with_labels();
        inbox.deliver(123);
        assert_eq!(inbox.open_message(123), Some(OpenEffect { play_voice: None }));
        assert_eq!(inbox.slots[123].state, 4);
        assert_eq!(inbox.open_message(17), None, "not delivered");
        assert_eq!(inbox.slots[17].state, 0);
        assert_eq!(inbox.open_message(500), None);
    }

    #[test]
    fn replay_is_for_listed_story_messages() {
        let mut inbox = inbox_with_labels();
        inbox.deliver(17);
        inbox.deliver(123);
        assert_eq!(inbox.replay(17), Some(VoiceTake { group: 15, line: 0 }));
        assert_eq!(inbox.replay(123), None);
        assert_eq!(inbox.replay(18), None);
    }

    #[test]
    fn deleting_resets_the_state_so_a_resend_delivers_again() {
        let mut inbox = inbox_with_labels();
        inbox.deliver(17);
        inbox.open_message(17);
        assert!(inbox.delete(17));
        assert_eq!(inbox.slots[17].state, 0);
        assert!(inbox.deliver(17));
        assert_eq!(inbox.slots[17].sequence, 2);
        assert!(!inbox.delete(150));
    }

    #[test]
    fn tabs_are_chosen_by_the_story_predicate() {
        assert_eq!(tab_of(105), InboxTab::Story);
        assert_eq!(tab_of(110), InboxTab::Tips);
        assert_eq!(tab_of(131), InboxTab::Story);
        assert_eq!(tab_of(137), InboxTab::Tips);
    }
}
