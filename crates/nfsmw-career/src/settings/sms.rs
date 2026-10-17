//! The SMS inbox as the save stores it: 150 slots of `{id, state, sequence}` and a sequence counter
//! (`docs/formats/career-save.md` 3.2, `docs/specs/career-phone.md` 1.1). The rules that move slots between states
//! are in [`crate::phone`].

use crate::bytes::{put_u16, u16_at};

/// Number of slots in the inbox.
pub const SMS_SLOTS: usize = 150;
/// Bytes of the slot table in the save.
pub const SMS_TABLE_LEN: usize = SMS_SLOTS * 4;
/// The `id` byte of a slot whose message has no label in the language table.
pub const SMS_INVALID_ID: u8 = 0xFF;

/// The state byte of a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmsState {
    /// Not delivered, or deleted (0).
    Empty,
    /// Delivered and unread (2).
    Unread,
    /// Read (4).
    Read,
    /// Any other byte (never produced by the game; state 6 would be both bits).
    Other(u8),
}

impl SmsState {
    /// The byte stored in the save.
    pub const fn byte(self) -> u8 {
        match self {
            SmsState::Empty => 0,
            SmsState::Unread => 2,
            SmsState::Read => 4,
            SmsState::Other(b) => b,
        }
    }

    /// Decodes a state byte.
    pub const fn from_byte(b: u8) -> SmsState {
        match b {
            0 => SmsState::Empty,
            2 => SmsState::Unread,
            4 => SmsState::Read,
            other => SmsState::Other(other),
        }
    }
}

/// One inbox slot, four bytes in the save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmsSlot {
    /// The slot's own index, or [`SMS_INVALID_ID`].
    pub id: u8,
    /// The state byte; see [`SmsSlot::state`].
    pub state: u8,
    /// The delivery order, 0 when the slot never received one.
    pub sequence: u16,
}

impl SmsSlot {
    /// The decoded state.
    pub const fn state(&self) -> SmsState {
        SmsState::from_byte(self.state)
    }

    /// The list screen shows a slot when its state has bit 1 or bit 2 set (states 2 and 4).
    pub const fn is_listed(&self) -> bool {
        self.id != SMS_INVALID_ID && self.state & 6 != 0
    }

    /// True while the slot is delivered and not yet opened (state bit 1).
    pub const fn is_unread(&self) -> bool {
        self.state & 2 != 0
    }
}

/// The 150 slots and the delivery counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmsInbox {
    /// The slots, indexed by message id.
    pub slots: [SmsSlot; SMS_SLOTS],
    /// The last sequence number given (the first message gets 1).
    pub counter: u16,
}

impl SmsInbox {
    /// A fresh inbox: every slot has `id = index`, state 0, sequence 0, counter 0. The game then marks slots
    /// whose label is missing from the language table with [`SMS_INVALID_ID`]; use
    /// [`SmsInbox::invalidate_missing`] for that.
    pub fn new() -> SmsInbox {
        let mut slots = [SmsSlot { id: 0, state: 0, sequence: 0 }; SMS_SLOTS];
        for (i, slot) in slots.iter_mut().enumerate() {
            slot.id = i as u8;
        }
        SmsInbox { slots, counter: 0 }
    }

    /// Sets the id byte of every slot to [`SMS_INVALID_ID`] for which `has_label(index)` is false (the language
    /// table has no `SMS_MESSAGE_<index>`).
    pub fn invalidate_missing(&mut self, has_label: impl Fn(u8) -> bool) {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if !has_label(i as u8) {
                slot.id = SMS_INVALID_ID;
            }
        }
    }

    /// The slot of message `id`, or `None` for `id >= 150`.
    pub fn slot(&self, id: u32) -> Option<&SmsSlot> {
        self.slots.get(usize::try_from(id).ok()?)
    }

    /// The slot of message `id`, mutable.
    pub fn slot_mut(&mut self, id: u32) -> Option<&mut SmsSlot> {
        self.slots.get_mut(usize::try_from(id).ok()?)
    }

    /// Reads the 600-byte slot table and the counter.
    pub(crate) fn from_parts(table: &[u8], counter: &[u8]) -> SmsInbox {
        let mut inbox = SmsInbox::new();
        for (i, slot) in inbox.slots.iter_mut().enumerate() {
            *slot = SmsSlot { id: table[4 * i], state: table[4 * i + 1], sequence: u16_at(table, 4 * i + 2) };
        }
        inbox.counter = u16_at(counter, 0);
        inbox
    }

    /// Writes the slot table into 600 bytes and the counter into 2.
    pub(crate) fn write_parts(&self, table: &mut [u8], counter: &mut [u8]) {
        for (i, slot) in self.slots.iter().enumerate() {
            table[4 * i] = slot.id;
            table[4 * i + 1] = slot.state;
            put_u16(table, 4 * i + 2, slot.sequence);
        }
        put_u16(counter, 0, self.counter);
    }
}

impl Default for SmsInbox {
    fn default() -> Self {
        SmsInbox::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_inbox_numbers_its_slots() {
        let inbox = SmsInbox::new();
        assert_eq!(inbox.slots[0].id, 0);
        assert_eq!(inbox.slots[149].id, 149);
        assert!(inbox.slots.iter().all(|s| s.state == 0 && s.sequence == 0));
        assert_eq!(inbox.counter, 0);
    }

    #[test]
    fn missing_labels_become_0xff() {
        let mut inbox = SmsInbox::new();
        inbox.invalidate_missing(|i| i == 15 || i == 16);
        assert_eq!(inbox.slots[15].id, 15);
        assert_eq!(inbox.slots[14].id, SMS_INVALID_ID);
        assert_eq!(inbox.slots.iter().filter(|s| s.id != SMS_INVALID_ID).count(), 2);
    }

    #[test]
    fn slot_lookup_stops_at_150() {
        let inbox = SmsInbox::new();
        assert!(inbox.slot(149).is_some());
        assert!(inbox.slot(150).is_none());
        assert!(inbox.slot(u32::MAX).is_none());
    }

    #[test]
    fn states_decode_and_list_rules_follow_the_bits() {
        assert_eq!(SmsState::from_byte(2), SmsState::Unread);
        assert_eq!(SmsState::from_byte(6), SmsState::Other(6));
        assert_eq!(SmsState::Other(6).byte(), 6);
        let unread = SmsSlot { id: 1, state: 2, sequence: 1 };
        let read = SmsSlot { id: 1, state: 4, sequence: 1 };
        let empty = SmsSlot { id: 1, state: 0, sequence: 0 };
        let invalid = SmsSlot { id: SMS_INVALID_ID, state: 2, sequence: 1 };
        assert!(unread.is_listed() && unread.is_unread());
        assert!(read.is_listed() && !read.is_unread());
        assert!(!empty.is_listed());
        assert!(!invalid.is_listed());
    }

    #[test]
    fn the_table_round_trips() {
        let mut inbox = SmsInbox::new();
        inbox.slots[15] = SmsSlot { id: 15, state: 2, sequence: 1 };
        inbox.slots[90] = SmsSlot { id: 0xFF, state: 4, sequence: 0xBEEF };
        inbox.counter = 74;
        let mut table = vec![0u8; SMS_TABLE_LEN];
        let mut counter = [0u8; 2];
        inbox.write_parts(&mut table, &mut counter);
        assert_eq!(&table[60..64], &[15, 2, 1, 0]);
        assert_eq!(&table[360..364], &[0xFF, 4, 0xEF, 0xBE]);
        assert_eq!(counter, [74, 0]);
        assert_eq!(SmsInbox::from_parts(&table, &counter), inbox);
    }
}
