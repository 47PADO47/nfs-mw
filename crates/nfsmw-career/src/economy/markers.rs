//! The marker and part-unlock inventory the career saves at file `0x5739`
//! (`docs/specs/career-economy-flows.md` section 6.5, `docs/formats/career-save.md` row `0x5739`).
//!
//! The inventory is a 63-slot table of 12-byte entries `{u32 type, u32 aux, u32 flag}`. It holds the three marker
//! kinds the career uses (Get Out of Jail Free `0x11`, Extra Impound Strike `0x14`, Impound Release `0x15`) and the
//! part unlocks (types `1..=0x10`). It is not the popup queue: that is a separate 6-slot table that is not saved.
//!
//! The rules, from the executable (`career-economy-flows.md` 6.5):
//! - a free slot has `type == 0`;
//! - an active entry has `flag == 1`;
//! - add writes `{type, aux, 1}` into the first free slot, with no duplicate check;
//! - count is the number of active entries whose type and aux match; an entry of type `0xE` with aux `0` matches
//!   any aux;
//! - remove takes the first active match: `type = 0`, `flag = 0`, and `aux` is kept.
//!
//! Every entry is kept as read: the three words are stored whole, so [`MarkerInventory::to_table`] gives back the
//! bytes it was built from, unknown kinds and flag values included. The rule of the infractions screen
//! ([`super::InfractionsScreen::open`]) says how many markers the first visit grants; this module only stores them.

use crate::LayoutError;
use crate::bytes::{put_u32, u32_at};
use crate::settings::INVENTORY_LEN;

/// The size of one table entry: three u32 words, `{type, aux, flag}`.
pub const ENTRY_LEN: usize = 12;
/// The number of entries in the table (`756 / 12`).
pub const ENTRY_COUNT: usize = INVENTORY_LEN / ENTRY_LEN;
/// The entry type whose count ignores `aux` when the requested aux is 0.
const WILDCARD_TYPE: u32 = 0xE;

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

/// One table entry. `kind` is the entry type, `aux` its second word (its meaning is not known), `flag` 1 when the
/// entry is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Entry {
    pub kind: u32,
    pub aux: u32,
    pub flag: u32,
}

impl Entry {
    fn is_free(&self) -> bool {
        self.kind == 0
    }

    /// True when this active entry counts for `kind` and `aux` (see the module rules).
    fn matches(&self, kind: u32, aux: u32) -> bool {
        if self.flag != 1 || self.kind != kind {
            return false;
        }
        (kind == WILDCARD_TYPE && aux == 0) || self.aux == aux
    }
}

/// The markers and part unlocks held by the player: the 63 entries of the table, in table order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerInventory {
    entries: Vec<Entry>,
}

impl MarkerInventory {
    /// An inventory with no markers (every entry free). A new career starts with this.
    pub fn empty() -> MarkerInventory {
        MarkerInventory { entries: vec![Entry::default(); ENTRY_COUNT] }
    }

    /// Reads the inventory from the table bytes (at least [`INVENTORY_LEN`] bytes; bytes past it are not read).
    pub fn from_table(table: &[u8]) -> Result<MarkerInventory, LayoutError> {
        LayoutError::check("marker table", table, INVENTORY_LEN)?;
        let entries = (0..ENTRY_COUNT)
            .map(|i| {
                let at = i * ENTRY_LEN;
                Entry { kind: u32_at(table, at), aux: u32_at(table, at + 4), flag: u32_at(table, at + 8) }
            })
            .collect();
        Ok(MarkerInventory { entries })
    }

    /// The table bytes, [`INVENTORY_LEN`] of them. [`MarkerInventory::from_table`] of the result gives this
    /// inventory back.
    pub fn to_table(&self) -> Vec<u8> {
        let mut table = vec![0u8; INVENTORY_LEN];
        for (i, entry) in self.entries.iter().enumerate() {
            let at = i * ENTRY_LEN;
            put_u32(&mut table, at, entry.kind);
            put_u32(&mut table, at + 4, entry.aux);
            put_u32(&mut table, at + 8, entry.flag);
        }
        table
    }

    /// The number of active entries of `kind` with `aux` (`0xE` with aux 0 counts every aux).
    pub fn count_entries(&self, kind: u32, aux: u32) -> u32 {
        self.entries.iter().filter(|e| e.matches(kind, aux)).count() as u32
    }

    /// The number of held markers of `kind`.
    pub fn count(&self, kind: MarkerKind) -> u32 {
        self.count_entries(kind.code(), 0)
    }

    /// Adds `n` entries of `kind` with `aux` to the first free slots and returns how many were added. The table has
    /// [`ENTRY_COUNT`] slots, so a full table adds fewer than `n`; the caller decides what that means.
    pub fn add_entries(&mut self, kind: u32, aux: u32, n: u32) -> u32 {
        let mut added = 0;
        for entry in self.entries.iter_mut().filter(|e| e.is_free()) {
            if added == n {
                break;
            }
            *entry = Entry { kind, aux, flag: 1 };
            added += 1;
        }
        added
    }

    /// Adds `n` markers of `kind` (aux 0) and returns how many were added.
    pub fn add(&mut self, kind: MarkerKind, n: u32) -> u32 {
        self.add_entries(kind.code(), 0, n)
    }

    /// Removes the first active entry of `kind` with `aux`: its type and flag are cleared and its aux kept. Returns
    /// false when none matches.
    pub fn take_entry(&mut self, kind: u32, aux: u32) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|e| e.matches(kind, aux)) else {
            return false;
        };
        entry.kind = 0;
        entry.flag = 0;
        true
    }

    /// Backroom redemption uses an exact auxiliary key and clears kind/state (garage-shop.md).
    pub fn redeem_part(&mut self, kind: u32, aux: u32) -> bool {
        if !(1..=16).contains(&kind) {
            return false;
        }
        let Some(entry) =
            self.entries.iter_mut().find(|entry| entry.kind == kind && entry.aux == aux && entry.flag == 1)
        else {
            return false;
        };
        entry.kind = 0;
        entry.flag = 0;
        true
    }

    /// Removes one held marker of `kind` (the first one in table order). Returns false when none is held.
    pub fn take(&mut self, kind: MarkerKind) -> bool {
        self.take_entry(kind.code(), 0)
    }
}

#[cfg(test)]
#[path = "markers_tests.rs"]
mod tests;
