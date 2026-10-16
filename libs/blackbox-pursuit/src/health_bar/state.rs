//! The bar state of each cop, kept between steps by key. Spec sections 12.4 and 13.2 (step 3): it steps once per
//! simulated step whether or not the bars are shown, and drops the state of a cop that is no longer sampled.

use std::collections::BTreeMap;

use super::lag::LagTracker;
use super::params::BarParams;
use super::sample::{Sample, displayed_health};

/// What the bar of one cop remembers between steps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CopBarState {
    lag: LagTracker,
    /// Seconds since the cop was first seen destroyed, or `None` while it is alive. Sticky: a destroyed cop stays so.
    dead_for: Option<f32>,
}

impl CopBarState {
    fn new(health: f32) -> Self {
        Self { lag: LagTracker::new(health), dead_for: None }
    }

    /// The lagging health, 0..=1.
    pub fn lag(&self) -> f32 {
        self.lag.lag()
    }

    /// Seconds since death, or `None` while the cop is alive.
    pub fn dead_for(&self) -> Option<f32> {
        self.dead_for
    }

    fn step(&mut self, sample: &Sample, dt: f32, params: &BarParams) {
        self.lag.step(displayed_health(sample), dt, params);
        self.dead_for = advance_death(self.dead_for, sample.destroyed, dt);
    }
}

/// The death clock: starts at 0 on the first step the cop is destroyed, then counts simulated seconds.
fn advance_death(dead_for: Option<f32>, destroyed: bool, dt: f32) -> Option<f32> {
    match (dead_for, destroyed) {
        (Some(elapsed), _) => Some(elapsed + dt),
        (None, true) => Some(0.0),
        (None, false) => None,
    }
}

/// The bar states of all sampled cops, keyed by the sample key. A `BTreeMap`, so the walk order is fixed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CopBarStates {
    cops: BTreeMap<u32, CopBarState>,
}

impl CopBarStates {
    /// Number of cops with a state.
    pub fn len(&self) -> usize {
        self.cops.len()
    }

    /// True when no cop has a state.
    pub fn is_empty(&self) -> bool {
        self.cops.is_empty()
    }

    /// The state of the cop with `key`, if it has one.
    pub fn get(&self, key: u32) -> Option<&CopBarState> {
        self.cops.get(&key)
    }

    /// Steps every sampled cop by `dt` and drops the state of every key that was not sampled. A key given twice is
    /// stepped once, by its first sample.
    pub fn step(&mut self, samples: &[Sample], dt: f32, params: &BarParams) {
        let mut next = BTreeMap::new();
        for sample in samples {
            if next.contains_key(&sample.key) {
                continue;
            }
            let mut state = match self.cops.remove(&sample.key) {
                Some(state) => state,
                None => CopBarState::new(displayed_health(sample)),
            };
            state.step(sample, dt, params);
            next.insert(sample.key, state);
        }
        self.cops = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health_bar::approx;
    use glam::Vec3;

    fn sample(key: u32, damage: f32, destroyed: bool) -> Sample {
        Sample { key, position: Vec3::ZERO, damage, destroyed }
    }

    #[test]
    fn a_cop_no_longer_sampled_loses_its_state_and_a_sampled_one_keeps_it() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        states.step(&[sample(1, 0.0, false), sample(2, 0.0, false)], 0.1, &params);
        assert_eq!(states.len(), 2);
        states.step(&[sample(2, 0.0, false)], 0.1, &params);
        assert!(states.get(1).is_none());
        assert!(states.get(2).is_some());
    }

    #[test]
    fn a_sampled_cop_keeps_its_lag_across_steps() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        states.step(&[sample(1, 0.0, false)], 0.25, &params);
        states.step(&[sample(1, 0.4, false)], 0.25, &params);
        let lag = states.get(1).map(CopBarState::lag);
        assert!(lag.is_some_and(|lag| approx(lag, 1.0)), "the hit holds the lag at 1.0");
    }

    #[test]
    fn flipped_cop_is_dead_at_positive_health_and_flashes() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        // Damage 0.4 would leave 0.6, but a flip destroys the car: the bar must read it as dead, at once.
        states.step(&[sample(1, 0.4, true)], 0.1, &params);
        let state = states.get(1).copied();
        assert_eq!(state.and_then(|s| s.dead_for()), Some(0.0));
        assert!(state.is_some_and(|s| approx(s.lag(), 0.0)));
    }

    #[test]
    fn death_clock_counts_simulated_seconds_from_the_first_destroyed_step() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        states.step(&[sample(1, 0.0, false)], 0.5, &params);
        assert_eq!(states.get(1).and_then(|s| s.dead_for()), None);
        states.step(&[sample(1, 1.0, true)], 0.5, &params);
        assert_eq!(states.get(1).and_then(|s| s.dead_for()), Some(0.0));
        states.step(&[sample(1, 1.0, true)], 0.5, &params);
        states.step(&[sample(1, 1.0, true)], 0.5, &params);
        assert_eq!(states.get(1).and_then(|s| s.dead_for()), Some(1.0));
    }

    #[test]
    fn death_is_sticky_even_if_the_flag_clears() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        states.step(&[sample(1, 1.0, true)], 0.5, &params);
        states.step(&[sample(1, 0.5, false)], 0.5, &params);
        assert_eq!(states.get(1).and_then(|s| s.dead_for()), Some(0.5));
    }

    #[test]
    fn a_key_given_twice_is_stepped_once() {
        let params = BarParams::default();
        let mut states = CopBarStates::default();
        states.step(&[sample(1, 0.0, false), sample(1, 0.4, false)], 0.25, &params);
        assert_eq!(states.len(), 1);
        assert!(states.get(1).is_some_and(|s| approx(s.lag(), 1.0)));
    }
}
