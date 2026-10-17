//! A cache of two values keyed by what they were made for (a size and format), for the GPU images a pass needs
//! at each of the two view sizes drawn every frame (the screen and a rear-view mirror capture).

/// Two entries, the most recently used one kept. A third key replaces the other entry.
pub(super) struct SizedPair<K, T> {
    entries: [Option<(K, T)>; 2],
    /// The index of the entry used last.
    newest: usize,
}

impl<K: PartialEq + Copy, T> SizedPair<K, T> {
    pub fn new() -> Self {
        Self { entries: [None, None], newest: 0 }
    }

    /// The value made for `key`, making it with `make` (and dropping the older entry) when neither entry is for `key`.
    pub fn get_or_make(&mut self, key: K, make: impl FnOnce() -> T) -> &mut T {
        if let Some(index) = self.index_of(key) {
            self.newest = index;
            return self.entries[index].as_mut().map(|(_, value)| value).expect("index_of found the entry");
        }
        let index = 1 - self.newest;
        self.newest = index;
        &mut self.entries[index].insert((key, make())).1
    }

    /// The value made for `key`, if one is cached.
    pub fn get(&self, key: K) -> Option<&T> {
        let index = self.index_of(key)?;
        self.entries[index].as_ref().map(|(_, value)| value)
    }

    /// Drop both entries.
    pub fn clear(&mut self) {
        self.entries = [None, None];
        self.newest = 0;
    }

    /// How many entries are cached.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.iter().flatten().count()
    }

    fn index_of(&self, key: K) -> Option<usize> {
        self.entries.iter().position(|entry| entry.as_ref().is_some_and(|(k, _)| *k == key))
    }
}

impl<K: PartialEq + Copy, T> Default for SizedPair<K, T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::SizedPair;

    #[test]
    fn alternating_keys_make_each_value_once() {
        let mut pair = SizedPair::<u32, u32>::new();
        let mut made = 0;
        for i in 0..10 {
            let key = if i % 2 == 0 { 1 } else { 2 };
            let value = pair.get_or_make(key, || {
                made += 1;
                key * 10
            });
            assert_eq!(*value, key * 10);
        }
        assert_eq!(made, 2, "the screen and the mirror size each make their image once");
        assert_eq!(pair.len(), 2);
    }

    #[test]
    fn third_key_replaces_least_recently_used() {
        let mut pair = SizedPair::<u32, u32>::new();
        pair.get_or_make(1, || 10);
        pair.get_or_make(2, || 20);
        // Using key 1 again makes key 2 the least recently used.
        pair.get_or_make(1, || unreachable!("key 1 is cached"));
        pair.get_or_make(3, || 30);
        assert_eq!(pair.get(1), Some(&10));
        assert_eq!(pair.get(2), None);
        assert_eq!(pair.get(3), Some(&30));
    }

    #[test]
    fn get_does_not_make() {
        let pair = SizedPair::<u32, u32>::new();
        assert_eq!(pair.get(1), None);
    }

    #[test]
    fn clear_empties_both_entries() {
        let mut pair = SizedPair::<u32, u32>::new();
        pair.get_or_make(1, || 10);
        pair.get_or_make(2, || 20);
        pair.clear();
        assert_eq!(pair.get(1), None);
        assert_eq!(pair.get(2), None);
        assert_eq!(pair.len(), 0);
    }
}
