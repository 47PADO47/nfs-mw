//! The eight infraction kinds and the fixed fine table (`docs/specs/career-economy.md` section 1.4).
//!
//! A kind is one bit of the pursuit's infraction mask; the counter index in a career record is the bit position.

/// One kind of infraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InfractionKind {
    /// Bit `0x01`.
    Speeding,
    /// Bit `0x02`.
    Racing,
    /// Bit `0x04`.
    RecklessDriving,
    /// Bit `0x08`: assault on a police car.
    Assault,
    /// Bit `0x10`.
    HitAndRun,
    /// Bit `0x20`: damage to property.
    DamageToProperty,
    /// Bit `0x40`.
    ResistingArrest,
    /// Bit `0x80`.
    OffRoad,
}

/// The fines, indexed by counter index. They are the one `u32` field of the eight `infractions` collections of
/// the install ([verified] in the spec); they match `ai-pursuit-heat.md` section 4.
pub const FINES: [u32; 8] = [150, 350, 1000, 350, 300, 100, 300, 75];

impl InfractionKind {
    /// All kinds in counter-index order.
    pub const ALL: [InfractionKind; 8] = [
        InfractionKind::Speeding,
        InfractionKind::Racing,
        InfractionKind::RecklessDriving,
        InfractionKind::Assault,
        InfractionKind::HitAndRun,
        InfractionKind::DamageToProperty,
        InfractionKind::ResistingArrest,
        InfractionKind::OffRoad,
    ];

    /// The counter index (the bit position).
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The bit in an infraction mask.
    pub const fn bit(self) -> u32 {
        1 << self as u32
    }

    /// The fine for one infraction of this kind.
    pub const fn fine(self) -> u32 {
        FINES[self as usize]
    }

    /// The kind of a counter index, `None` past 7.
    pub fn from_index(index: usize) -> Option<InfractionKind> {
        InfractionKind::ALL.get(index).copied()
    }

    /// The kind of a single mask bit, `None` for zero, several bits or a bit above `0x80`.
    pub fn from_bit(bit: u32) -> Option<InfractionKind> {
        if !bit.is_power_of_two() {
            return None;
        }
        InfractionKind::from_index(bit.trailing_zeros() as usize)
    }

    /// The name of the `infractions` collection that holds the fine (an identifier, not display text).
    pub const fn collection_name(self) -> &'static str {
        match self {
            InfractionKind::Speeding => "speeding",
            InfractionKind::Racing => "racing",
            InfractionKind::RecklessDriving => "reckless_driving",
            InfractionKind::Assault => "assault",
            InfractionKind::HitAndRun => "hit_and_run",
            InfractionKind::DamageToProperty => "damage",
            InfractionKind::ResistingArrest => "resisting_arrest",
            InfractionKind::OffRoad => "off_road",
        }
    }
}

/// The set of kinds committed in one pursuit (the infraction manager's mask). Bits above `0x80` have no counter
/// and are ignored by every method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InfractionMask(pub u32);

impl InfractionMask {
    /// No infraction.
    pub const NONE: InfractionMask = InfractionMask(0);

    /// The mask holding exactly these kinds.
    pub fn of(kinds: &[InfractionKind]) -> InfractionMask {
        InfractionMask(kinds.iter().fold(0, |m, k| m | k.bit()))
    }

    /// True when the kind is set.
    pub const fn contains(self, kind: InfractionKind) -> bool {
        self.0 & kind.bit() != 0
    }

    /// Sets a kind.
    pub fn insert(&mut self, kind: InfractionKind) {
        self.0 |= kind.bit();
    }

    /// The kinds that are set, in counter-index order.
    pub fn kinds(self) -> impl Iterator<Item = InfractionKind> {
        InfractionKind::ALL.into_iter().filter(move |k| self.contains(*k))
    }

    /// The number of distinct kinds (the script variable `total_infractions`).
    pub fn count(self) -> u32 {
        self.kinds().count() as u32
    }

    /// The fine of the pursuit: each kind of the mask counts once.
    pub fn fine(self) -> u32 {
        self.kinds().map(InfractionKind::fine).sum()
    }
}

/// `sum(counters[i] * FINES[i])`, the fine of a set of infraction counters. Computed in `u64` so eight full
/// `u16` counters cannot overflow.
pub fn fine_of_counters(counters: &[u16; 8]) -> u64 {
    counters.iter().zip(FINES).map(|(&c, fine)| u64::from(c) * u64::from(fine)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fine_table_matches_the_spec() {
        let by_name: Vec<(&str, u32)> = InfractionKind::ALL.iter().map(|k| (k.collection_name(), k.fine())).collect();
        assert_eq!(
            by_name,
            [
                ("speeding", 150),
                ("racing", 350),
                ("reckless_driving", 1000),
                ("assault", 350),
                ("hit_and_run", 300),
                ("damage", 100),
                ("resisting_arrest", 300),
                ("off_road", 75),
            ]
        );
    }

    #[test]
    fn bits_and_indices_are_log2_of_each_other() {
        for (i, kind) in InfractionKind::ALL.into_iter().enumerate() {
            assert_eq!(kind.index(), i);
            assert_eq!(kind.bit(), 1 << i);
            assert_eq!(InfractionKind::from_bit(kind.bit()), Some(kind));
        }
        assert_eq!(InfractionKind::from_bit(0), None);
        assert_eq!(InfractionKind::from_bit(0x03), None);
        assert_eq!(InfractionKind::from_bit(0x100), None);
        assert_eq!(InfractionKind::from_bit(0x80), Some(InfractionKind::OffRoad));
    }

    #[test]
    fn a_mask_counts_distinct_kinds_and_sums_one_fine_each() {
        let mask =
            InfractionMask::of(&[InfractionKind::Speeding, InfractionKind::RecklessDriving, InfractionKind::OffRoad]);
        assert_eq!(mask.0, 0x85);
        assert_eq!(mask.count(), 3);
        assert_eq!(mask.fine(), 150 + 1000 + 75);
        assert_eq!(InfractionMask::NONE.fine(), 0);
        assert_eq!(InfractionMask(0xFF).fine(), FINES.iter().sum::<u32>());
        assert_eq!(InfractionMask(0xFF00).count(), 0);
    }

    #[test]
    fn counter_fines_multiply_count_by_amount() {
        assert_eq!(fine_of_counters(&[0; 8]), 0);
        assert_eq!(fine_of_counters(&[2, 0, 1, 0, 0, 0, 0, 3]), 2 * 150 + 1000 + 3 * 75);
        assert_eq!(fine_of_counters(&[u16::MAX; 8]), 65_535 * FINES.iter().map(|&f| u64::from(f)).sum::<u64>());
    }
}
