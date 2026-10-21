//! Speed-trap points (`docs/specs/race-mode.md` section 9).

/// Conversion from km/h to m/s that the notification applies.
pub const KMH_TO_MS: f32 = 0.277_78;

/// The speeds a racer set at each speed trap of the race, by trap slot. A new pass of the same trap replaces the
/// slot, as the original stores one number per trap.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpeedTraps {
    slots: Vec<Option<f32>>,
}

impl SpeedTraps {
    /// A racer's slots for a race with `traps` speed traps.
    pub fn new(traps: usize) -> Self {
        Self { slots: vec![None; traps] }
    }

    /// Records the speed of a pass through trap `slot`, in km/h. Returns the stored value in m/s, or `None` for a slot
    /// the race does not have.
    pub fn record(&mut self, slot: usize, kmh: f32) -> Option<f32> {
        let entry = self.slots.get_mut(slot)?;
        let speed = kmh * KMH_TO_MS;
        *entry = Some(speed);
        Some(speed)
    }

    /// A pass through trap `slot` at `kmh`: it pays only when the speed beats the trap's threshold and the racer's
    /// best at that trap (`docs/specs/career-scripts-events.md` section 1.3). Returns the stored speed in m/s when it
    /// paid.
    pub fn record_pass(&mut self, slot: usize, kmh: f32, threshold_kmh: f32) -> Option<f32> {
        let speed = kmh * KMH_TO_MS;
        let previous = (*self.slots.get(slot)?).unwrap_or(0.0);
        if kmh <= threshold_kmh || speed <= previous {
            return None;
        }
        self.record(slot, kmh)
    }

    /// The stored speed of each trap in m/s, by slot; `None` for a trap not passed.
    pub fn slots(&self) -> &[Option<f32>] {
        &self.slots
    }

    /// How many traps the racer has passed.
    pub fn passed(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    /// The sum of the stored speeds, in m/s.
    pub fn total(&self) -> f32 {
        self.slots.iter().flatten().sum()
    }
}

/// Points lost for time over the limit: `penalty_per_second * seconds_over`, never negative.
pub fn overtime_loss(penalty_per_second: f32, seconds_over: f32) -> f32 {
    (penalty_per_second * seconds_over).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pass_stores_the_speed_in_metres_per_second() {
        let mut traps = SpeedTraps::new(3);
        let stored = traps.record(1, 180.0).unwrap();
        assert!((stored - 50.0).abs() < 0.01, "{stored}");
        assert_eq!(traps.passed(), 1);
    }

    #[test]
    fn passing_a_trap_again_replaces_its_slot() {
        let mut traps = SpeedTraps::new(2);
        traps.record(0, 100.0);
        traps.record(0, 200.0);
        assert!((traps.total() - 200.0 * KMH_TO_MS).abs() < 1e-3);
        assert_eq!(traps.passed(), 1);
    }

    #[test]
    fn a_pass_pays_only_above_the_threshold_and_the_previous_best() {
        let mut traps = SpeedTraps::new(1);
        assert_eq!(traps.record_pass(0, 80.0, 88.0), None, "under the threshold");
        assert!(traps.record_pass(0, 100.0, 88.0).is_some());
        assert_eq!(traps.record_pass(0, 95.0, 88.0), None, "not better than the best");
        assert!(traps.record_pass(0, 120.0, 88.0).is_some());
        assert!((traps.total() - 120.0 * KMH_TO_MS).abs() < 1e-3);
        assert_eq!(traps.record_pass(3, 200.0, 88.0), None, "no such trap");
    }

    #[test]
    fn an_unknown_trap_is_ignored() {
        let mut traps = SpeedTraps::new(1);
        assert_eq!(traps.record(4, 100.0), None);
        assert_eq!(traps.total(), 0.0);
    }

    #[test]
    fn overtime_costs_points_by_the_second() {
        assert_eq!(overtime_loss(10.0, 3.0), 30.0);
        assert_eq!(overtime_loss(10.0, -2.0), 0.0);
    }
}
