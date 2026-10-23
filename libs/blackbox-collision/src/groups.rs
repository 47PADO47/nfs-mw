//! The set of enabled scenery groups as a bitset: a group test is a shift and a mask, with no tree walk per instance.

/// Scenery group ids (`u16`) in or out of the world. Group 0 is not a group, but is stored like any other id.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct GroupSet {
    words: Vec<u64>,
}

impl GroupSet {
    pub fn contains(&self, group: u16) -> bool {
        let Some(word) = self.words.get(usize::from(group >> 6)) else { return false };
        *word >> (group & 63) & 1 != 0
    }

    /// Adds `group`; true when it was not in the set.
    pub fn insert(&mut self, group: u16) -> bool {
        let index = usize::from(group >> 6);
        if self.words.len() <= index {
            self.words.resize(index + 1, 0);
        }
        let bit = 1u64 << (group & 63);
        let added = self.words[index] & bit == 0;
        self.words[index] |= bit;
        added
    }

    /// Removes `group`; true when it was in the set.
    pub fn remove(&mut self, group: u16) -> bool {
        let Some(word) = self.words.get_mut(usize::from(group >> 6)) else { return false };
        let bit = 1u64 << (group & 63);
        let present = *word & bit != 0;
        *word &= !bit;
        present
    }
}
