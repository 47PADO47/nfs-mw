//! The infraction log of one chase: which offences the player has committed since it began, each counted once,
//! and their fines. The fines are data (the caller passes the install's values in [`Fines`]).

/// The offences a chase can record, in the order the HUD lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Infraction {
    Speeding,
    Racing,
    RecklessDriving,
    /// A deliberate hit on a cop that damages it for the first time.
    Assault,
    /// A deliberate hit on traffic while the cops can see the player.
    HitAndRun,
    /// A deliberate, direct hit on a breakable object while the cops can see the player.
    DamageToProperty,
    /// The player got out of the cops' sight and the chase cooled down.
    ResistingArrest,
    /// Off the legal road while in the cops' sight.
    OffRoad,
}

impl Infraction {
    /// Every infraction, in HUD order.
    pub const ALL: [Infraction; 8] = [
        Infraction::Speeding,
        Infraction::Racing,
        Infraction::RecklessDriving,
        Infraction::Assault,
        Infraction::HitAndRun,
        Infraction::DamageToProperty,
        Infraction::ResistingArrest,
        Infraction::OffRoad,
    ];

    fn index(self) -> usize {
        match self {
            Infraction::Speeding => 0,
            Infraction::Racing => 1,
            Infraction::RecklessDriving => 2,
            Infraction::Assault => 3,
            Infraction::HitAndRun => 4,
            Infraction::DamageToProperty => 5,
            Infraction::ResistingArrest => 6,
            Infraction::OffRoad => 7,
        }
    }
}

/// The fine of each infraction, in [`Infraction::ALL`] order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fines([u32; 8]);

impl Fines {
    pub const fn new(values: [u32; 8]) -> Self {
        Self(values)
    }

    pub fn of(&self, infraction: Infraction) -> u32 {
        self.0[infraction.index()]
    }
}

/// The infractions of one chase. Recording the same offence twice does nothing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InfractionLog {
    bits: u8,
    fines: u32,
}

impl InfractionLog {
    /// Records `infraction`. Returns `true` if it was new (the HUD shows a new line only then).
    pub fn record(&mut self, infraction: Infraction, fines: &Fines) -> bool {
        let bit = 1u8 << infraction.index();
        if self.bits & bit != 0 {
            return false;
        }
        self.bits |= bit;
        self.fines = self.fines.saturating_add(fines.of(infraction));
        true
    }

    pub fn contains(&self, infraction: Infraction) -> bool {
        self.bits & (1u8 << infraction.index()) != 0
    }

    /// How many different infractions the chase has.
    pub fn count(&self) -> u32 {
        self.bits.count_ones()
    }

    /// The fines of every infraction recorded.
    pub fn total_fines(&self) -> u32 {
        self.fines
    }

    /// The recorded infractions, in HUD order.
    pub fn iter(&self) -> impl Iterator<Item = Infraction> + '_ {
        Infraction::ALL.into_iter().filter(|i| self.contains(*i))
    }

    /// Clears the log when a new chase starts.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FINES: Fines = Fines::new([150, 350, 1000, 350, 300, 100, 300, 75]);

    #[test]
    fn each_infraction_counts_once_and_adds_its_fine() {
        let mut log = InfractionLog::default();
        assert!(log.record(Infraction::Speeding, &FINES));
        assert!(!log.record(Infraction::Speeding, &FINES));
        assert_eq!(log.count(), 1);
        assert_eq!(log.total_fines(), 150);
    }

    #[test]
    fn totals_sum_the_fines_of_the_different_infractions() {
        let mut log = InfractionLog::default();
        log.record(Infraction::HitAndRun, &FINES);
        log.record(Infraction::OffRoad, &FINES);
        assert_eq!(log.count(), 2);
        assert_eq!(log.total_fines(), 375);
        assert!(log.contains(Infraction::OffRoad));
        assert!(!log.contains(Infraction::Assault));
    }

    #[test]
    fn iter_lists_in_hud_order_whatever_the_recording_order() {
        let mut log = InfractionLog::default();
        log.record(Infraction::OffRoad, &FINES);
        log.record(Infraction::Speeding, &FINES);
        let listed: Vec<_> = log.iter().collect();
        assert_eq!(listed, vec![Infraction::Speeding, Infraction::OffRoad]);
    }

    #[test]
    fn clear_empties_the_log() {
        let mut log = InfractionLog::default();
        log.record(Infraction::Racing, &FINES);
        log.clear();
        assert_eq!(log, InfractionLog::default());
    }

    #[test]
    fn fines_saturate_instead_of_overflowing() {
        let huge = Fines::new([u32::MAX; 8]);
        let mut log = InfractionLog::default();
        log.record(Infraction::Speeding, &huge);
        log.record(Infraction::Racing, &huge);
        assert_eq!(log.total_fines(), u32::MAX);
    }

    #[test]
    fn every_infraction_has_its_own_bit() {
        let mut log = InfractionLog::default();
        for infraction in Infraction::ALL {
            assert!(log.record(infraction, &FINES));
        }
        assert_eq!(log.count(), 8);
        assert_eq!(log.iter().count(), 8);
    }
}
