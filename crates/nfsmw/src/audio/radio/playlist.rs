//! Which song plays next: the original's play list rules (spec `docs/specs/music-graph.md` §6).
//!
//! A [`Playlist`] holds one bit per song: `mask` for the songs that may play and `unplayed` for those not yet
//! played in this round. Ordered mode takes the next song after the last one; shuffle mode picks one of the
//! unplayed at random. When a round is over the mask is copied back, so every song plays once before any plays
//! again, and a song that ends a round does not start the next one.

/// The songs the bit masks can hold (the original uses 28 bits).
pub const MAX_SONGS: usize = 28;

/// How the next song is picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// In the order of the song list.
    #[default]
    Ordered,
    /// At random, without replacement.
    Shuffle,
}

/// The state of one play list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Playlist {
    mask: u32,
    unplayed: u32,
    last: Option<usize>,
}

impl Playlist {
    /// A list over the songs whose number is in `enabled` (numbers from `MAX_SONGS` up are ignored).
    pub fn new(enabled: impl IntoIterator<Item = usize>) -> Self {
        let mask = enabled.into_iter().filter(|&n| n < MAX_SONGS).fold(0u32, |m, n| m | 1 << n);
        Self { mask, unplayed: mask, last: None }
    }

    /// How many songs may play.
    pub fn len(&self) -> usize {
        self.mask.count_ones() as usize
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.mask == 0
    }

    /// Whether song `n` may play.
    pub fn contains(&self, n: usize) -> bool {
        n < MAX_SONGS && self.mask & (1 << n) != 0
    }

    /// Start over: every song unplayed and no last song (what the original does when the lists are rebuilt).
    pub fn reset(&mut self) {
        self.unplayed = self.mask;
        self.last = None;
    }

    /// The song that played last in this round, if any.
    #[cfg(test)]
    pub fn last(&self) -> Option<usize> {
        self.last
    }

    /// The next song, or `None` when no song may play. `random(count)` returns a number below `count`.
    pub fn next(&mut self, mode: Mode, random: &mut dyn FnMut(usize) -> usize) -> Option<usize> {
        if self.mask == 0 {
            return None;
        }
        if self.unplayed == 0 {
            self.reset();
        }
        let pick = match mode {
            Mode::Ordered => self.ordered(),
            Mode::Shuffle => self.shuffled(random),
        };
        self.take(pick);
        Some(pick)
    }

    /// The first unplayed song after the last one; after the end of the list, the first unplayed song.
    fn ordered(&mut self) -> usize {
        let from = self.last.map_or(0, |n| n + 1);
        let after = (from..MAX_SONGS).find(|&n| self.unplayed & (1 << n) != 0);
        if let Some(n) = after {
            return n;
        }
        self.reset();
        (0..MAX_SONGS).find(|&n| self.unplayed & (1 << n) != 0).unwrap_or(0)
    }

    /// The k-th unplayed song for a random k.
    fn shuffled(&self, random: &mut dyn FnMut(usize) -> usize) -> usize {
        let count = self.unplayed.count_ones() as usize;
        let k = random(count).min(count - 1);
        (0..MAX_SONGS).filter(|&n| self.unplayed & (1 << n) != 0).nth(k).unwrap_or(0)
    }

    /// Mark `n` played. When that empties the round, the mask is copied back with `n` left out.
    fn take(&mut self, n: usize) {
        self.last = Some(n);
        self.unplayed &= !(1 << n);
        if self.unplayed == 0 {
            self.unplayed = self.mask & !(1 << n);
        }
    }
}

/// A small xorshift generator, seeded from the clock: the radio needs a shuffle, not good randomness.
#[derive(Debug, Clone)]
pub struct Random(u64);

impl Random {
    pub fn new(seed: u64) -> Self {
        Self(seed | 1)
    }

    /// Seeded from the system clock.
    pub fn from_clock() -> Self {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        Self::new(nanos as u64 ^ 0x9E37_79B9_7F4A_7C15)
    }

    /// A number below `count` (0 when `count` is 0).
    pub fn below(&mut self, count: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        if count == 0 {
            return 0;
        }
        (self.0.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as usize % count
    }
}
