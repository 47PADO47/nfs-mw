//! The strings rasterised in the Windows font, and the UI textures they live in.
//!
//! Every distinct string is rasterised once and uploaded as a texture of its own. A string that changes every second (the
//! Remastered pursuit timer) would fill the GPU with textures nobody draws again, so the cache keeps a bounded number and
//! frees the ones that have gone unused for a while.
//!
//! Lookups hash the string and the font key without allocating; the cache owns a copy of the text only for the strings it
//! stores, and checks it against the lookup so two strings with one hash never share a picture.
//!
//! An entry is only freed once it is old enough that no mesh built for the frame being drawn can still use its texture:
//! the clock is the number of `present` calls, and a frame makes a handful of them (the HUD layers, one per menu screen).

use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};

use blackbox_gfx::UiTextureId;

/// How many strings the cache holds before it frees the least recently used ones.
pub(super) const CAPACITY: usize = 512;
/// An entry is never freed younger than this many ticks: more than the `present` calls of any frame, so a texture a
/// frame's meshes still point at is never freed under them.
const SAFE_AGE: u64 = 256;
/// A string not drawn for this many ticks is freed even when the cache is not full.
const IDLE_AGE: u64 = 4096;
/// How often, in ticks, the cache looks for idle entries.
const SWEEP_EVERY: u64 = 256;

struct Entry {
    text: String,
    font: u32,
    id: UiTextureId,
    aspect: f32,
    last_used: u64,
}

pub(super) struct RasterCache {
    entries: HashMap<u64, Entry, foldhash::fast::FixedState>,
    hasher: foldhash::fast::FixedState,
    /// The ids already handed out; each string gets a new one, so a freed id never comes back for another picture.
    issued: u64,
    /// Counts `present` calls.
    tick: u64,
    /// The first id; the strings take the ones after it.
    base: u64,
}

impl Default for RasterCache {
    fn default() -> Self {
        Self::new(super::RASTER_TEXTURE_BIT)
    }
}

impl RasterCache {
    pub fn new(base: u64) -> Self {
        Self {
            entries: HashMap::with_hasher(foldhash::fast::FixedState::default()),
            hasher: foldhash::fast::FixedState::default(),
            issued: 0,
            tick: 0,
            base,
        }
    }

    fn key(&self, text: &str, font: u32) -> u64 {
        let mut hasher = self.hasher.build_hasher();
        text.hash(&mut hasher);
        font.hash(&mut hasher);
        hasher.finish()
    }

    /// Starts a `present` call. Every `SWEEP_EVERY` ticks the idle entries are freed (their ids go to `freed`).
    pub fn tick(&mut self, freed: &mut Vec<UiTextureId>) {
        self.tick += 1;
        if !self.tick.is_multiple_of(SWEEP_EVERY) {
            return;
        }
        let now = self.tick;
        self.entries.retain(|_, entry| {
            let keep = now - entry.last_used <= IDLE_AGE;
            if !keep {
                freed.push(entry.id);
            }
            keep
        });
    }

    /// The texture and the width over height of a string already rasterised, marking it as used now.
    pub fn get(&mut self, text: &str, font: u32) -> Option<(UiTextureId, f32)> {
        let key = self.key(text, font);
        let entry = self.entries.get_mut(&key)?;
        if entry.font != font || entry.text != text {
            return None;
        }
        entry.last_used = self.tick;
        Some((entry.id, entry.aspect))
    }

    /// Stores a string just rasterised and returns the id its texture must be uploaded under. Past `CAPACITY` strings the
    /// least recently used ones that are old enough to be safe are freed (their ids go to `freed`).
    pub fn insert(&mut self, text: &str, font: u32, aspect: f32, freed: &mut Vec<UiTextureId>) -> UiTextureId {
        self.evict(freed);
        let id = UiTextureId(self.base | self.issued);
        self.issued += 1;
        let entry = Entry { text: text.to_owned(), font, id, aspect, last_used: self.tick };
        // Two strings with one hash: the newer replaces the older, whose texture is freed.
        if let Some(old) = self.entries.insert(self.key(text, font), entry) {
            freed.push(old.id);
        }
        id
    }

    fn evict(&mut self, freed: &mut Vec<UiTextureId>) {
        while self.entries.len() >= CAPACITY {
            let oldest = self.entries.iter().min_by_key(|(_, e)| e.last_used).map(|(key, e)| (*key, e.last_used));
            // Everything left was used lately: a frame can use more strings than the cache holds, so it grows for now.
            let Some((key, _)) = oldest.filter(|(_, last_used)| self.tick - last_used > SAFE_AGE) else {
                return;
            };
            if let Some(entry) = self.entries.remove(&key) {
                freed.push(entry.id);
            }
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: u64 = 1 << 40;

    fn cache() -> RasterCache {
        RasterCache::new(BASE)
    }

    fn ticks(cache: &mut RasterCache, count: u64, freed: &mut Vec<UiTextureId>) {
        for _ in 0..count {
            cache.tick(freed);
        }
    }

    #[test]
    fn a_stored_string_is_found_by_text_and_font() {
        let (mut cache, mut freed) = (cache(), Vec::new());
        let id = cache.insert("12:03", 7, 2.5, &mut freed);
        assert_eq!(cache.get("12:03", 7), Some((id, 2.5)));
        assert_eq!(cache.get("12:03", 8), None, "another font is another picture");
        assert_eq!(cache.get("12:04", 7), None);
        assert!(freed.is_empty());
    }

    #[test]
    fn every_string_gets_a_new_id() {
        let (mut cache, mut freed) = (cache(), Vec::new());
        let ids: Vec<_> = (0..5).map(|k| cache.insert(&k.to_string(), 1, 1.0, &mut freed)).collect();
        for (k, id) in ids.iter().enumerate() {
            assert_eq!(id.0, BASE + k as u64);
        }
    }

    #[test]
    fn past_the_capacity_the_oldest_old_enough_strings_are_freed() {
        let (mut cache, mut freed) = (cache(), Vec::new());
        let first = cache.insert("first", 1, 1.0, &mut freed);
        ticks(&mut cache, SAFE_AGE + 1, &mut freed);
        for k in 0..CAPACITY - 1 {
            cache.insert(&format!("fill {k}"), 1, 1.0, &mut freed);
        }
        assert_eq!(cache.len(), CAPACITY);
        assert!(freed.is_empty(), "at capacity, nothing is freed until another string comes");
        cache.insert("one more", 1, 1.0, &mut freed);
        assert_eq!(freed, vec![first], "the string nobody drew since is the one that goes");
        assert_eq!(cache.len(), CAPACITY);
        assert_eq!(cache.get("first", 1), None);
    }

    #[test]
    fn a_string_used_this_frame_is_never_freed_for_room() {
        let (mut cache, mut freed) = (cache(), Vec::new());
        for k in 0..CAPACITY + 20 {
            cache.insert(&format!("busy {k}"), 1, 1.0, &mut freed);
        }
        assert!(freed.is_empty(), "every string is younger than the safe age, so the cache grows instead");
        assert_eq!(cache.len(), CAPACITY + 20);
    }

    #[test]
    fn using_a_string_keeps_it_from_being_the_oldest() {
        let (mut cache, mut freed) = (cache(), Vec::new());
        let kept = cache.insert("kept", 1, 1.0, &mut freed);
        let stale = cache.insert("stale", 1, 1.0, &mut freed);
        for k in 0..CAPACITY - 2 {
            cache.insert(&format!("fill {k}"), 1, 1.0, &mut freed);
        }
        ticks(&mut cache, SAFE_AGE + 1, &mut freed);
        // Everything but the stale string is drawn again.
        for k in 0..CAPACITY - 2 {
            assert!(cache.get(&format!("fill {k}"), 1).is_some());
        }
        assert_eq!(cache.get("kept", 1).map(|e| e.0), Some(kept));
        cache.insert("new", 1, 1.0, &mut freed);
        assert!(freed.contains(&stale));
        assert!(!freed.contains(&kept));
        assert_eq!(cache.get("kept", 1).map(|e| e.0), Some(kept));
    }

    #[test]
    fn strings_idle_for_long_are_freed_without_the_cache_being_full() {
        let (mut cache, mut freed) = (cache(), Vec::new());
        let timer = cache.insert("00:01", 1, 1.0, &mut freed);
        let shown = cache.insert("always", 1, 1.0, &mut freed);
        for _ in 0..IDLE_AGE / SWEEP_EVERY + 2 {
            ticks(&mut cache, SWEEP_EVERY, &mut freed);
            cache.get("always", 1);
        }
        assert_eq!(freed, vec![timer]);
        assert_eq!(cache.get("always", 1).map(|e| e.0), Some(shown));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn a_changing_timer_never_holds_more_than_a_bounded_number_of_textures() {
        let (mut cache, mut freed) = (cache(), Vec::new());
        // A string a second for an hour, 60 frames of 3 present calls each per second.
        for second in 0..3600 {
            cache.insert(&format!("{second}"), 1, 1.0, &mut freed);
            ticks(&mut cache, 180, &mut freed);
        }
        assert!(cache.len() <= (IDLE_AGE + SWEEP_EVERY) as usize / 180 + 3, "{} strings are still held", cache.len());
        assert_eq!(freed.len() + cache.len(), 3600, "every texture is either held or freed");
    }
}
