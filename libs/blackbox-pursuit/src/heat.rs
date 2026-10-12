//! Heat: how hot a chase is, from 1 (a patrol cop's interest) to 10 (the whole force). It is a real number
//! between a base and a maximum; the integer part picks the cop wave.

/// The highest heat level there is.
pub const MAX_LEVEL: u32 = 10;

/// A chase's heat. The value is kept in `base..=max`; the level is its integer part, clamped to `1..=10`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Heat {
    value: f32,
    base: f32,
    max: f32,
}

impl Heat {
    /// Heat that starts at `base` and never leaves `base..=max`. A `max` below `base` is raised to it.
    pub fn new(base: f32, max: f32) -> Self {
        let base = base.max(0.0);
        Self { value: base, base, max: max.max(base) }
    }

    /// The heat value, `base..=max`.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// The level the cop wave is picked from: the integer part of the value, `1..=10`.
    pub fn level(&self) -> u32 {
        (self.value.floor().max(1.0) as u32).min(MAX_LEVEL)
    }

    /// Pursuit start: the heat is at least the base (`heat = max(heat, base)`).
    pub fn begin(&mut self) {
        self.value = self.value.max(self.base).min(self.max);
    }

    /// Adds `dt / time_per_level` heat, clamped to the range. A non-positive `time_per_level` (no growth set
    /// for the level) or a bad `dt` does nothing.
    pub fn grow(&mut self, dt: f32, time_per_level: f32) {
        if !(dt > 0.0 && dt.is_finite() && time_per_level > 0.0) {
            return;
        }
        self.value = (self.value + dt / time_per_level).clamp(self.base, self.max);
    }

    /// A forced chase at `level` (`PursueAtHeatLevel`): raises the heat to it if it is lower.
    pub fn raise_to(&mut self, level: f32) {
        if self.value < level {
            self.value = level.min(self.max);
        }
    }

    /// Multiplies the heat by `factor` (an escape or an event win), staying in the range.
    pub fn scale(&mut self, factor: f32) {
        if !factor.is_finite() {
            return;
        }
        self.value = (self.value * factor.max(0.0)).clamp(self.base, self.max);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_at_the_base_and_reads_its_integer_part() {
        let heat = Heat::new(1.4, 5.0);
        assert_eq!(heat.value(), 1.4);
        assert_eq!(heat.level(), 1);
    }

    #[test]
    fn a_max_below_the_base_is_raised_to_it() {
        let heat = Heat::new(3.0, 1.0);
        assert_eq!(heat.value(), 3.0);
        assert_eq!(heat.level(), 3);
    }

    #[test]
    fn level_is_clamped_to_one_to_ten() {
        assert_eq!(Heat::new(0.0, 0.0).level(), 1);
        assert_eq!(Heat::new(12.0, 12.0).level(), MAX_LEVEL);
    }

    #[test]
    fn grows_one_level_per_time_per_level() {
        let mut heat = Heat::new(1.0, 5.0);
        heat.grow(600.0, 600.0);
        assert!((heat.value() - 2.0).abs() < 1e-5);
        assert_eq!(heat.level(), 2);
    }

    #[test]
    fn growth_stops_at_the_maximum() {
        let mut heat = Heat::new(1.0, 2.0);
        heat.grow(10_000.0, 10.0);
        assert_eq!(heat.value(), 2.0);
    }

    #[test]
    fn no_growth_without_a_rate_or_a_time_step() {
        let mut heat = Heat::new(1.0, 5.0);
        heat.grow(60.0, 0.0);
        heat.grow(f32::NAN, 60.0);
        heat.grow(-5.0, 60.0);
        assert_eq!(heat.value(), 1.0);
    }

    #[test]
    fn begin_keeps_a_higher_heat_and_lifts_a_lower_one_to_the_base() {
        let mut heat = Heat::new(2.0, 5.0);
        heat.raise_to(1.0);
        heat.begin();
        assert_eq!(heat.value(), 2.0);
    }

    #[test]
    fn forced_chase_raises_the_heat_but_never_lowers_it() {
        let mut heat = Heat::new(1.0, 6.0);
        heat.raise_to(4.0);
        assert_eq!(heat.level(), 4);
        heat.raise_to(2.0);
        assert_eq!(heat.level(), 4);
    }

    #[test]
    fn a_forced_chase_stops_at_the_maximum() {
        let mut heat = Heat::new(1.0, 3.0);
        heat.raise_to(9.0);
        assert_eq!(heat.value(), 3.0);
    }

    #[test]
    fn escape_scales_the_heat_within_its_range() {
        let mut heat = Heat::new(1.0, 5.0);
        heat.grow(4.0 * 600.0, 600.0);
        heat.scale(0.5);
        assert!((heat.value() - 2.5).abs() < 1e-5);
        heat.scale(0.0);
        assert_eq!(heat.value(), 1.0, "never below the base");
        heat.scale(f32::INFINITY);
        assert_eq!(heat.value(), 1.0, "a bad factor changes nothing");
    }
}
