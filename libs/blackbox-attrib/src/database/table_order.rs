//! The order a class walks its collections in (`docs/formats/attributes.md`, "Collection table order").
//!
//! A class keeps its collections in an open-addressed table of collection keys: a key goes to slot `key % size` and on to
//! the next free slot. Walking the class (first collection, next collection) reads the table slot by slot, so the order
//! depends on the keys, the table size and the order the collections were added, not on the file order alone.

/// The longest probe the table allows: past it the table grows.
const MAX_PROBE: usize = 40;

/// A class's table of collection keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyTable {
    slots: Vec<Option<u32>>,
    count: usize,
    longest_probe: usize,
}

impl KeyTable {
    /// An empty table of `size` slots (at least one).
    pub fn with_size(size: usize) -> Self {
        Self { slots: vec![None; size.max(1)], count: 0, longest_probe: 0 }
    }

    /// A table of `size` slots holding `keys`, added in this order.
    pub fn of_keys(size: usize, keys: impl IntoIterator<Item = u32>) -> Self {
        let mut table = Self::with_size(size);
        for key in keys {
            table.insert(key);
        }
        table
    }

    /// Adds `key`; a key already present is left where it is. A full table grows first, and a table whose longest probe
    /// passes [`MAX_PROBE`] grows after.
    pub fn insert(&mut self, key: u32) {
        if self.count == self.slots.len() {
            self.grow(grown(self.slots.len()));
        }
        if !self.place(key) {
            return;
        }
        if self.longest_probe > MAX_PROBE {
            self.grow(grown(self.slots.len()));
        }
    }

    /// The keys in slot order: the order a walk of the class gives.
    pub fn keys(&self) -> impl Iterator<Item = u32> + '_ {
        self.slots.iter().filter_map(|slot| *slot)
    }

    /// The number of slots.
    pub fn size(&self) -> usize {
        self.slots.len()
    }

    /// Puts `key` in its slot or the next free one. False when the key is already there.
    fn place(&mut self, key: u32) -> bool {
        let size = self.slots.len();
        let mut slot = key as usize % size;
        let mut probe = 0;
        while let Some(held) = self.slots[slot] {
            if held == key {
                return false;
            }
            slot = (slot + 1) % size;
            probe += 1;
        }
        self.slots[slot] = Some(key);
        self.count += 1;
        self.longest_probe = self.longest_probe.max(probe);
        true
    }

    /// Moves every key, in slot order, into a table of `size` slots, one slot larger each time a probe still passes
    /// [`MAX_PROBE`].
    fn grow(&mut self, mut size: usize) {
        let old: Vec<u32> = self.keys().collect();
        loop {
            *self = Self::with_size(size);
            for &key in &old {
                self.place(key);
            }
            if self.longest_probe <= MAX_PROBE {
                return;
            }
            size += 1;
        }
    }
}

/// The size a table grows to: five quarters of the old size, rounded up to a multiple of four.
fn grown(size: usize) -> usize {
    ((size * 5 / 4) + 3) & !3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_come_in_slot_order_with_collisions_after_their_slot() {
        // 12 % 10 = 2, 22 % 10 = 2 (probes to 3), 3 % 10 = 3 (probes to 4), 9 stays at 9.
        let table = KeyTable::of_keys(10, [9, 12, 22, 3]);
        assert_eq!(table.keys().collect::<Vec<_>>(), [12, 22, 3, 9]);
    }

    #[test]
    fn a_probe_wraps_at_the_end_of_the_table() {
        let table = KeyTable::of_keys(4, [3, 7]);
        assert_eq!(table.keys().collect::<Vec<_>>(), [7, 3]);
    }

    #[test]
    fn a_full_table_grows_by_five_quarters_rounded_to_four_and_keeps_its_keys() {
        let table = KeyTable::of_keys(4, [0, 1, 2, 3, 4]);
        assert_eq!(table.size(), 8);
        assert_eq!(table.keys().collect::<Vec<_>>(), [0, 1, 2, 3, 4]);
    }

    #[test]
    fn a_key_added_twice_is_kept_once() {
        assert_eq!(KeyTable::of_keys(8, [5, 5]).keys().count(), 1);
    }
}
