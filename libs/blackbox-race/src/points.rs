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
