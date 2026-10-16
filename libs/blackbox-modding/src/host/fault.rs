//! The fault counter of one mod (ADR 0005 section 3, "Panic containment"). Five faults quarantine the mod: it gets no
//! more events and is not called until it is reloaded or the game restarts.

/// Faults after which a mod is quarantined.
pub const QUARANTINE_AFTER: u32 = 5;

/// Counts the faults of one mod.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FaultCounter {
    faults: u32,
}

impl FaultCounter {
    /// Count one fault. Returns true only for the fault that quarantines the mod. Faults after that are not counted.
    pub fn record(&mut self) -> bool {
        if self.is_quarantined() {
            return false;
        }
        self.faults += 1;
        self.faults == QUARANTINE_AFTER
    }

    /// Whether the mod has reached the quarantine limit.
    pub fn is_quarantined(&self) -> bool {
        self.faults >= QUARANTINE_AFTER
    }

    /// The faults counted so far.
    pub fn faults(&self) -> u32 {
        self.faults
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fifth_fault_quarantines_and_only_that_one_says_so() {
        let mut counter = FaultCounter::default();
        for _ in 1..QUARANTINE_AFTER {
            assert!(!counter.record());
            assert!(!counter.is_quarantined());
        }
        assert!(counter.record());
        assert!(counter.is_quarantined());
        assert_eq!(counter.faults(), QUARANTINE_AFTER);
    }

    #[test]
    fn faults_after_quarantine_are_not_counted() {
        let mut counter = FaultCounter::default();
        for _ in 0..QUARANTINE_AFTER {
            counter.record();
        }
        assert!(!counter.record());
        assert_eq!(counter.faults(), QUARANTINE_AFTER);
    }
}
