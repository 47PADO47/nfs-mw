//! The marker inventory: the Get Out of Jail Free, Extra Impound Strike and Impound Release markers the player holds
//! (`docs/specs/career-economy-flows.md` sections 6.1, 6.2 and 6.5).
//!
//! The inventory is the 63-entry table of 12-byte entries `{type, aux, flag}`. The
//! flows spec gives it that layout and calls it the marker inventory. The save page (`docs/formats/career-save.md`,
//! section 3.1, row `0x5739`) documents the same 756 bytes as the pending popup queue, with the same entry shape. The
//! two readings are not reconciled (flows spec open question 8 and this crate's notes). This module follows the flows
//! spec: the table is read and written as entries, a held marker is an entry whose `flag` is not 0, and a free slot is
//! one whose `flag` is 0.
//!
//! Every entry is kept as read: the three words are stored whole, so [`MarkerInventory::to_table`] gives back the bytes
//! it was built from, unknown kinds and flag values included. The rule of the infractions screen
//! ([`super::InfractionsScreen::open`]) says how many markers the first visit grants; this module only stores them.

use crate::LayoutError;
use crate::bytes::{put_u32, u32_at};
use crate::settings::POPUP_QUEUE_LEN;

/// The size of one table entry: three u32 words, `{type, aux, flag}`.
pub const ENTRY_LEN: usize = 12;
/// The number of entries in the table (`756 / 12`).
pub const ENTRY_COUNT: usize = POPUP_QUEUE_LEN / ENTRY_LEN;

/// The marker kinds the career uses, with their entry types (`career-economy-flows.md` 6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerKind {
    /// Get Out of Jail Free (type `0x11`): pays a bust without cash.
    JailFree,
    /// Extra Impound Strike (type `0x14`): raises an impounded car's strike limit by one, up to 5.
    ExtraStrike,
    /// Impound Release (type `0x15`): releases an impounded car without its cash fee.
    ImpoundRelease,
}

impl MarkerKind {
    /// The entry type of this kind.
    pub const fn code(self) -> u32 {
        match self {
            MarkerKind::JailFree => 0x11,
            MarkerKind::ExtraStrike => 0x14,
            MarkerKind::ImpoundRelease => 0x15,
        }
    }
}

/// One table entry. `kind` is the entry type, `aux` its second word (its meaning is not known), `flag` 0 for a free
/// slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Entry {
    pub kind: u32,
    pub aux: u32,
    pub flag: u32,
}

impl Entry {
    /// A held marker of `kind`. The flag is 1, as the flows spec writes it (`{type, aux, 1}`); `aux` is 0 because the
    /// spec does not say what a new marker carries there.
    fn held(kind: MarkerKind) -> Entry {
        Entry { kind: kind.code(), aux: 0, flag: 1 }
    }

    fn is_held(&self, kind: MarkerKind) -> bool {
        self.flag != 0 && self.kind == kind.code()
    }
}

/// The markers held by the player: the 63 entries of the table, in table order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerInventory {
    entries: Vec<Entry>,
}

impl MarkerInventory {
    /// An inventory with no markers (every entry free). A new career starts with this.
    pub fn empty() -> MarkerInventory {
        MarkerInventory { entries: vec![Entry::default(); ENTRY_COUNT] }
    }

    /// Reads the inventory from the table bytes (at least [`POPUP_QUEUE_LEN`] bytes; bytes past it are not read).
    pub fn from_table(table: &[u8]) -> Result<MarkerInventory, LayoutError> {
        LayoutError::check("marker table", table, POPUP_QUEUE_LEN)?;
        let entries = (0..ENTRY_COUNT)
            .map(|i| {
                let at = i * ENTRY_LEN;
                Entry { kind: u32_at(table, at), aux: u32_at(table, at + 4), flag: u32_at(table, at + 8) }
            })
            .collect();
        Ok(MarkerInventory { entries })
    }

    /// The table bytes, [`POPUP_QUEUE_LEN`] of them. [`MarkerInventory::from_table`] of the result gives this
    /// inventory back.
    pub fn to_table(&self) -> Vec<u8> {
        let mut table = vec![0u8; POPUP_QUEUE_LEN];
        for (i, entry) in self.entries.iter().enumerate() {
            let at = i * ENTRY_LEN;
            put_u32(&mut table, at, entry.kind);
            put_u32(&mut table, at + 4, entry.aux);
            put_u32(&mut table, at + 8, entry.flag);
        }
        table
    }

    /// The number of held markers of `kind`.
    pub fn count(&self, kind: MarkerKind) -> u32 {
        self.entries.iter().filter(|e| e.is_held(kind)).count() as u32
    }

    /// Adds `n` markers of `kind` to the first free slots and returns how many were added. The table has
    /// [`ENTRY_COUNT`] slots, so a full table adds fewer than `n`; the caller decides what that means.
    pub fn add(&mut self, kind: MarkerKind, n: u32) -> u32 {
        let mut added = 0;
        for entry in self.entries.iter_mut().filter(|e| e.flag == 0) {
            if added == n {
                break;
            }
            *entry = Entry::held(kind);
            added += 1;
        }
        added
    }

    /// Removes one held marker of `kind` (the first one in table order). Returns false when none is held.
    pub fn take(&mut self, kind: MarkerKind) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|e| e.is_held(kind)) else {
            return false;
        };
        *entry = Entry::default();
        true
    }
}

#[cfg(test)]
#[path = "markers_tests.rs"]
mod tests;
