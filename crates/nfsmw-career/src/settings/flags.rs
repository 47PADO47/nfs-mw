//! The career flags word (settings `+4`, save file `0x403F`).
//!
//! Names follow `docs/formats/career-state.md` section 3 where it names a bit and `docs/formats/career-save.md`
//! section 3.3 otherwise. The two pages disagree on what bits `0x20000` and `0x8000..0x10000` are for (see the
//! per-constant notes); both readings are kept as aliases of the same bit and the values are never interpreted
//! by this crate beyond the rules the economy spec states.

/// The flags word of the career settings. Unknown bits are kept as they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Flags(pub u32);

impl Flags {
    /// A career has been started (the profile is not new); decides New Career or Resume.
    pub const CAREER_STARTED: Flags = Flags(0x1);
    /// Set when the Underground 2 cash prompt is shown (career-save 3.3, bit 1).
    pub const UG2_PROMPT_SHOWN: Flags = Flags(0x2);
    /// The Rap Sheet is unlocked (script hook `SetHasRapSheet`).
    pub const HAS_RAP_SHEET: Flags = Flags(0x10);
    /// The opening sequence / reputation overview has been seen (read by `SkipCareerIntro`).
    pub const OPENING_SEEN: Flags = Flags(0x20);
    /// Bits 6 to 10 (`0x40..=0x400`): set together by two routines, probably one-time dialogs. Unconfirmed.
    pub const DIALOGS_SEEN: Flags = Flags(0x7C0);
    /// Game over (every car impounded). Cleared on every load: the loader merges flags with
    /// `(old | stored) & !0x800`.
    pub const GAME_OVER: Flags = Flags(0x800);
    /// The infractions screen has been shown before (the first time grants the starting markers).
    pub const INFRACTIONS_SEEN: Flags = Flags(0x1000);
    /// Bit 13, set by an availability routine. Unknown.
    pub const UNKNOWN_0X2000: Flags = Flags(0x2000);
    /// Career complete (winning the last race): unlocks every rival challenge and the first bonus cars.
    pub const CAREER_COMPLETE: Flags = Flags(0x4000);
    /// Event `19.8.69` done (career-state) / unlocks the `BONUS_SL65` car (career-save).
    pub const EVENT_19_8_69_DONE: Flags = Flags(0x8000);
    /// Every challenge-series race done (career-state) / unlocks the `BONUS_GT2` car (career-save).
    pub const CHALLENGE_SERIES_DONE: Flags = Flags(0x10000);
    /// First safe-house visit done (career-state) / the bonus-content popup was already queued (career-save
    /// 3.3 and 2.3). The two pages disagree; the bit is the same.
    pub const FIRST_SAFEHOUSE_VISIT: Flags = Flags(0x20000);
    /// Alias of [`Flags::FIRST_SAFEHOUSE_VISIT`] under the name `career-save.md` gives it.
    pub const BONUS_POPUP_QUEUED: Flags = Flags(0x20000);
    /// Unlocks the car `0x34498EB2`; set from a bonus-content probe.
    pub const BONUS_CAR_0X34498EB2: Flags = Flags(0x40000);
    /// The scripted E3 demo car has been given.
    pub const E3_CAR_GIVEN: Flags = Flags(0x100000);

    /// No bit set.
    pub const EMPTY: Flags = Flags(0);

    /// True when every bit of `other` is set.
    pub const fn contains(self, other: Flags) -> bool {
        self.0 & other.0 == other.0
    }

    /// True when any bit of `other` is set.
    pub const fn intersects(self, other: Flags) -> bool {
        self.0 & other.0 != 0
    }

    /// The flags with `other` set.
    pub const fn with(self, other: Flags) -> Flags {
        Flags(self.0 | other.0)
    }

    /// The flags with `other` cleared.
    pub const fn without(self, other: Flags) -> Flags {
        Flags(self.0 & !other.0)
    }

    /// Sets the bits of `other`.
    pub fn insert(&mut self, other: Flags) {
        *self = self.with(other);
    }

    /// Clears the bits of `other`.
    pub fn remove(&mut self, other: Flags) {
        *self = self.without(other);
    }

    /// The flags after a load: stored flags are merged into the running ones and game over is cleared
    /// (`career-save.md` 2.3).
    pub const fn merged_on_load(self, stored: Flags) -> Flags {
        Flags((self.0 | stored.0) & !Flags::GAME_OVER.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_needs_every_bit_and_intersects_any() {
        let f = Flags(0x11);
        assert!(f.contains(Flags::CAREER_STARTED));
        assert!(f.contains(Flags(0x11)));
        assert!(!f.contains(Flags(0x13)));
        assert!(f.intersects(Flags(0x13)));
        assert!(!f.intersects(Flags::OPENING_SEEN));
    }

    #[test]
    fn insert_and_remove_touch_only_their_bits() {
        let mut f = Flags::CAREER_STARTED;
        f.insert(Flags::HAS_RAP_SHEET);
        assert_eq!(f, Flags(0x11));
        f.remove(Flags::CAREER_STARTED);
        assert_eq!(f, Flags::HAS_RAP_SHEET);
    }

    #[test]
    fn a_load_merges_the_flags_and_drops_game_over() {
        let running = Flags(0x1).with(Flags::GAME_OVER);
        let stored = Flags(0x10).with(Flags::GAME_OVER);
        assert_eq!(running.merged_on_load(stored), Flags(0x11));
    }

    #[test]
    fn the_flags_of_the_three_known_saves_have_the_expected_bits() {
        let f = Flags(0x001D_F7F3);
        assert!(f.contains(Flags::CAREER_STARTED.with(Flags::HAS_RAP_SHEET).with(Flags::OPENING_SEEN)));
        assert!(f.contains(Flags::CAREER_COMPLETE.with(Flags::E3_CAR_GIVEN)));
        assert!(!f.intersects(Flags::GAME_OVER.with(Flags::FIRST_SAFEHOUSE_VISIT)));
    }
}
