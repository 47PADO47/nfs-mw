//! Bind groups kept for recent inputs. A full-screen pass binds the same layout, sampler and parameter buffer
//! on every draw, so the input views are the whole key: a steady frame finds its bind groups here and creates
//! none.

/// How many bind groups one pass keeps. Bounded: the oldest is dropped first, so views of images a resize
/// replaced are released within a few frames.
const BIND_GROUP_CACHE: usize = 32;

/// Bind groups made for recent inputs, keyed by the two input views `K` they were built from. A pass with
/// one input passes the same view twice.
pub struct BindCache<K, V> {
    entries: Vec<(K, K, V)>,
    /// The slot the next insert overwrites once the cache is full.
    next: usize,
}

impl<K, V> Default for BindCache<K, V> {
    fn default() -> Self {
        Self { entries: Vec::new(), next: 0 }
    }
}

impl<K: PartialEq + Clone, V> BindCache<K, V> {
    /// The value made for the inputs `a` and `b`. `make` runs only when the pair is not cached.
    pub fn get_or_insert_with(&mut self, a: &K, b: &K, make: impl FnOnce() -> V) -> &V {
        let index = match self.entries.iter().position(|(ka, kb, _)| ka == a && kb == b) {
            Some(index) => index,
            None => self.insert(a, b, make()),
        };
        &self.entries[index].2
    }

    /// Store an entry and return its slot: a new one while the cache is filling, then the oldest one's.
    fn insert(&mut self, a: &K, b: &K, value: V) -> usize {
        let entry = (a.clone(), b.clone(), value);
        if self.entries.len() < BIND_GROUP_CACHE {
            self.entries.push(entry);
            return self.entries.len() - 1;
        }
        let slot = self.next;
        self.entries[slot] = entry;
        self.next = (slot + 1) % BIND_GROUP_CACHE;
        slot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Look up `key` in a cache of integers whose values are `key * 10`, counting the `make` calls in `made`.
    fn lookup(cache: &mut BindCache<u32, u32>, key: u32, made: &mut u32) -> u32 {
        *cache.get_or_insert_with(&key, &key, || {
            *made += 1;
            key * 10
        })
    }

    #[test]
    fn repeated_inputs_are_made_once() {
        let mut cache = BindCache::default();
        let mut made = 0;
        assert_eq!(lookup(&mut cache, 1, &mut made), 10);
        assert_eq!(lookup(&mut cache, 1, &mut made), 10);
        assert_eq!(made, 1);
    }

    #[test]
    fn both_keys_must_match() {
        let mut cache: BindCache<u32, u32> = BindCache::default();
        let first = *cache.get_or_insert_with(&1, &2, || 12);
        let other = *cache.get_or_insert_with(&1, &3, || 13);
        let again = *cache.get_or_insert_with(&1, &2, || 99);
        assert_eq!((first, other, again), (12, 13, 12));
    }

    #[test]
    fn full_cache_evicts_the_oldest_first() {
        let window = BIND_GROUP_CACHE as u32;
        let mut cache = BindCache::default();
        let mut made = 0;
        for key in 0..window {
            lookup(&mut cache, key, &mut made);
        }
        assert_eq!(made, window);
        // One more key takes the slot of key 0, the oldest.
        lookup(&mut cache, window, &mut made);
        assert_eq!(cache.entries.len(), BIND_GROUP_CACHE);
        assert_eq!(made, window + 1);
        lookup(&mut cache, 1, &mut made);
        assert_eq!(made, window + 1);
        lookup(&mut cache, 0, &mut made);
        assert_eq!(made, window + 2);
    }

    #[test]
    fn round_robin_keeps_the_newest_window() {
        let window = BIND_GROUP_CACHE as u32;
        let total = window * 3 + 5;
        let mut cache = BindCache::default();
        let mut made = 0;
        for key in 0..total {
            lookup(&mut cache, key, &mut made);
        }
        assert_eq!(made, total);
        assert_eq!(cache.entries.len(), BIND_GROUP_CACHE);
        // The newest window is still cached: hits make nothing.
        for key in total - window..total {
            lookup(&mut cache, key, &mut made);
        }
        assert_eq!(made, total);
        // The key just before that window was evicted.
        lookup(&mut cache, total - window - 1, &mut made);
        assert_eq!(made, total + 1);
    }
}
