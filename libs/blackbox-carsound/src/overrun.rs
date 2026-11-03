//! The overrun backfire: bangs and flames of an engine whose throttle is shut, driven by a charge that builds while
//! the engine is fed and drains while it is not.
//!
//! This is a controller of the *decision* only. It takes the engine's throttle and speed once per frame and says when
//! a bang or a flame happens and how strong it is (0 to 1); which recording plays and how the flames are drawn is
//! the host's business. The rules are those of the "accumulator" backfire system of a recent motorcycle game, written
//! from its spec (the content mod's engine-sound spec in `docs/specs/`, section "Backfire"), and no file or tuning
//! comes with the library: every number is a field of [`OverrunTuning`].
//!
//! The rules, in one place:
//!
//! 1. **Charge.** Every frame the charge moves by `rate * rpm_fraction * dt * s`, where `s` is the throttle when it is
//!    open and `-1` when it is shut (0 or less); it stays between 0 and 1. A bike held at full throttle at high revs
//!    charges at `rate * rpm_fraction` per second, and drains at the same speed once the throttle is shut.
//! 2. **Arming.** The throttle falling through `throttle_arm` (below it now, at or above it the frame before) arms a
//!    bang and starts its timer.
//! 3. **Bang.** While armed the timer runs as long as the throttle is still above `throttle_fire` or was already at
//!    or below it the frame before. The frame the throttle falls through `throttle_fire` ends the arming: if the
//!    timer is within `window_seconds`, the strength is `charge * (1 - timer / window_seconds)`; the bang happens if
//!    that is above `bang_audio_threshold` and a random draw is at most the probability, which goes from
//!    `probability_closed` (the driver's own throttle shut) to `probability_open` (fully open) with `raw_throttle`.
//! 4. **Flame.** With the throttle shut now and the frame before and `rpm_fraction` not below `flame_min_rpm`, a
//!    timer runs; after `flame_delay` seconds the flame is *active* and every frame emits a flame of strength
//!    `charge * rpm_fraction` until the timer is reset (throttle opened or revs too low), at which point the active
//!    flag is cleared with one last flame of strength 0.

use crate::rng::Rng;

/// Every number of the controller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverrunTuning {
    /// Charge per second at full revs and full throttle.
    pub accumulator_rate: f32,
    /// The throttle falling through this arms a bang.
    pub throttle_arm: f32,
    /// The throttle falling through this fires the armed bang.
    pub throttle_fire: f32,
    /// The longest time between arming and firing for the bang to happen, seconds.
    pub window_seconds: f32,
    /// Seconds of a shut throttle before the flames start.
    pub flame_delay: f32,
    /// Flames need revs (as a fraction of the maximum) at or above this.
    pub flame_min_rpm: f32,
    /// A bang needs a strength above this to be heard.
    pub bang_audio_threshold: f32,
    /// A bang above this strength also lights the pipe; at or below it is audio only.
    pub bang_effect_threshold: f32,
    /// A flame above this strength also lights the pipe; at or below it is audio only.
    pub flame_effect_threshold: f32,
    /// Chance of a bang whose strength is high enough, when the driver's throttle is shut.
    pub probability_closed: f32,
    /// The same chance when the driver's throttle is fully open.
    pub probability_open: f32,
}

/// What the controller needs of the engine each frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverrunInput {
    /// Frame time, seconds.
    pub dt: f32,
    /// The throttle the engine gets (after any electronic intervention), 0 to 1.
    pub throttle: f32,
    /// The throttle the driver asks for, 0 to 1.
    pub raw_throttle: f32,
    /// Engine speed over its maximum, 0 to 1.
    pub rpm_fraction: f32,
}

/// A bang: one short explosion in the exhaust.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bang {
    /// The strength, 0 to 1.
    pub intensity: f32,
    /// The strength above the audio threshold, rescaled to 0 to 1: the level the recording plays at.
    pub audio_level: f32,
    /// The strength above the effect threshold, rescaled to 0 to 1, when the pipe lights too.
    pub effect_level: Option<f32>,
}

/// A flame frame while the engine overruns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flame {
    /// The strength, 0 to 1.
    pub intensity: f32,
    /// The strength above the effect threshold, rescaled to 0 to 1, when the pipe lights too.
    pub effect_level: Option<f32>,
}

/// What one frame produced.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OverrunOutput {
    pub bang: Option<Bang>,
    pub flame: Option<Flame>,
}

/// The overrun controller of one engine.
#[derive(Debug, Clone)]
pub struct OverrunBackfire {
    tuning: OverrunTuning,
    rng: Rng,
    charge: f32,
    bang_armed: bool,
    bang_timer: f32,
    flame_timer: f32,
    flame_active: bool,
    last_throttle: f32,
}

impl OverrunInput {
    /// The input with a non-finite number read as zero, the time not negative and the levels held to 0 to 1.
    fn sanitized(self) -> Self {
        let unit = |value: f32| if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 };
        Self {
            dt: if self.dt.is_finite() { self.dt.max(0.0) } else { 0.0 },
            throttle: unit(self.throttle),
            raw_throttle: unit(self.raw_throttle),
            rpm_fraction: unit(self.rpm_fraction),
        }
    }
}

/// `(value - low) / (1 - low)` between 0 and 1; 0 when `low` is 1 or more.
fn above(value: f32, low: f32) -> f32 {
    if low >= 1.0 {
        return 0.0;
    }
    ((value - low) / (1.0 - low)).clamp(0.0, 1.0)
}

impl OverrunBackfire {
    pub fn new(tuning: OverrunTuning, seed: u32) -> Self {
        Self {
            tuning,
            rng: Rng::new(seed ^ 0x0B4C_F1AE),
            charge: 0.0,
            bang_armed: false,
            bang_timer: 0.0,
            flame_timer: 0.0,
            flame_active: false,
            last_throttle: 0.0,
        }
    }

    /// The charge now, 0 to 1.
    pub fn charge(&self) -> f32 {
        self.charge
    }

    /// Forget everything (a restart).
    pub fn reset(&mut self) {
        self.charge = 0.0;
        self.bang_armed = false;
        self.bang_timer = 0.0;
        self.flame_timer = 0.0;
        self.flame_active = false;
        self.last_throttle = 0.0;
    }

    /// A uniform draw in `[0, 1)` with 15 bits of resolution.
    fn draw(&mut self) -> f32 {
        ((self.rng.next_u32() >> 8) & 0x7FFF) as f32 / 32768.0
    }

    /// One frame.
    pub fn update(&mut self, input: OverrunInput) -> OverrunOutput {
        let input = input.sanitized();
        let t = self.tuning;
        let last = self.last_throttle;
        self.last_throttle = input.throttle;
        let feed = if input.throttle <= 0.0 { -1.0 } else { input.throttle };
        self.charge = (self.charge + t.accumulator_rate * input.rpm_fraction * input.dt * feed).clamp(0.0, 1.0);

        let mut out = OverrunOutput::default();
        if input.throttle < t.throttle_arm && t.throttle_arm <= last {
            self.bang_armed = true;
            self.bang_timer = 0.0;
        }
        if self.bang_armed {
            out.bang = self.bang_step(input, last);
        }
        out.flame = self.flame_step(input, last);
        out
    }

    fn bang_step(&mut self, input: OverrunInput, last: f32) -> Option<Bang> {
        let t = self.tuning;
        let waiting = t.throttle_fire < input.throttle || last <= t.throttle_fire;
        if waiting {
            self.bang_timer += input.dt;
            return None;
        }
        self.bang_armed = false;
        let timer = self.bang_timer;
        self.bang_timer = 0.0;
        if timer > t.window_seconds {
            return None;
        }
        let fraction = if t.window_seconds > 0.0 { (timer / t.window_seconds).clamp(0.0, 1.0) } else { 0.0 };
        let intensity = self.charge * (1.0 - fraction);
        if intensity <= t.bang_audio_threshold {
            return None;
        }
        let chance = t.probability_closed + (t.probability_open - t.probability_closed) * input.raw_throttle;
        if self.draw() > chance {
            return None;
        }
        let effect_level = (intensity > t.bang_effect_threshold).then(|| above(intensity, t.bang_effect_threshold));
        Some(Bang { intensity, audio_level: above(intensity, t.bang_audio_threshold), effect_level })
    }

    fn flame_step(&mut self, input: OverrunInput, last: f32) -> Option<Flame> {
        let t = self.tuning;
        let shut = input.throttle == 0.0 && last == 0.0 && input.rpm_fraction >= t.flame_min_rpm;
        let mut timer = 0.0;
        if shut {
            self.flame_timer += input.dt;
            timer = self.flame_timer;
            if timer >= t.flame_delay {
                self.flame_active = true;
            }
        }
        if !shut {
            self.flame_timer = 0.0;
        }
        if !self.flame_active {
            return None;
        }
        let mut intensity = self.charge * input.rpm_fraction;
        if timer == 0.0 {
            intensity = 0.0;
            self.flame_active = false;
        }
        let effect_level = (intensity > t.flame_effect_threshold).then(|| above(intensity, t.flame_effect_threshold));
        Some(Flame { intensity, effect_level })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tuning() -> OverrunTuning {
        OverrunTuning {
            accumulator_rate: 0.2,
            throttle_arm: 0.9,
            throttle_fire: 0.1,
            window_seconds: 2.0,
            flame_delay: 0.2,
            flame_min_rpm: 0.1,
            bang_audio_threshold: 0.0,
            bang_effect_threshold: 0.5,
            flame_effect_threshold: 0.0,
            probability_closed: 1.0,
            probability_open: 1.0,
        }
    }

    fn frame(throttle: f32, rpm: f32) -> OverrunInput {
        OverrunInput { dt: 0.1, throttle, raw_throttle: throttle, rpm_fraction: rpm }
    }

    /// Hold full throttle for `seconds` at `rpm`.
    fn charge_up(fx: &mut OverrunBackfire, seconds: f32, rpm: f32) {
        for _ in 0..(seconds * 10.0) as usize {
            fx.update(frame(1.0, rpm));
        }
    }

    #[test]
    fn charge_builds_with_throttle_and_drains_without() {
        let mut fx = OverrunBackfire::new(tuning(), 1);
        charge_up(&mut fx, 2.0, 1.0);
        assert!((fx.charge() - 0.4).abs() < 1e-4);
        fx.update(OverrunInput { dt: 1.0, throttle: 0.0, raw_throttle: 0.0, rpm_fraction: 1.0 });
        assert!((fx.charge() - 0.2).abs() < 1e-4);
        fx.update(OverrunInput { dt: 10.0, throttle: 0.0, raw_throttle: 0.0, rpm_fraction: 1.0 });
        assert_eq!(fx.charge(), 0.0);
    }

    #[test]
    fn a_lift_after_a_long_pull_bangs() {
        let mut fx = OverrunBackfire::new(tuning(), 1);
        charge_up(&mut fx, 5.0, 1.0);
        let mut bang = None;
        for throttle in [0.5, 0.05, 0.0] {
            bang = bang.or(fx.update(frame(throttle, 1.0)).bang);
        }
        let bang = bang.expect("a bang");
        assert!(bang.intensity > 0.9);
        assert!(bang.effect_level.is_some());
    }

    #[test]
    fn a_slow_lift_is_too_late() {
        let mut fx = OverrunBackfire::new(tuning(), 1);
        charge_up(&mut fx, 5.0, 1.0);
        // Armed at 0.5, but the throttle only reaches the firing level after 3 seconds.
        let mut bang = None;
        fx.update(frame(0.5, 1.0));
        for _ in 0..30 {
            bang = bang.or(fx.update(frame(0.4, 1.0)).bang);
        }
        bang = bang.or(fx.update(frame(0.0, 1.0)).bang);
        assert!(bang.is_none());
    }

    #[test]
    fn an_empty_charge_stays_quiet_when_the_audio_threshold_is_set() {
        let mut t = tuning();
        t.bang_audio_threshold = 0.3;
        let mut fx = OverrunBackfire::new(t, 1);
        charge_up(&mut fx, 0.5, 1.0);
        let mut bang = None;
        for throttle in [0.5, 0.0] {
            bang = bang.or(fx.update(frame(throttle, 1.0)).bang);
        }
        assert!(bang.is_none());
    }

    #[test]
    fn the_chance_follows_the_driver_throttle() {
        let mut t = tuning();
        t.probability_closed = 0.0;
        t.probability_open = 0.0;
        let mut fx = OverrunBackfire::new(t, 7);
        charge_up(&mut fx, 5.0, 1.0);
        let mut any = false;
        for throttle in [0.5, 0.0] {
            any |= fx.update(frame(throttle, 1.0)).bang.is_some();
        }
        assert!(!any);
    }

    #[test]
    fn flames_start_after_the_delay_and_stop_with_a_zero_frame() {
        let mut fx = OverrunBackfire::new(tuning(), 3);
        charge_up(&mut fx, 5.0, 1.0);
        fx.update(frame(0.0, 1.0));
        let mut flames = 0;
        for _ in 0..5 {
            flames += usize::from(fx.update(frame(0.0, 1.0)).flame.is_some());
        }
        assert!(flames >= 3);
        let end = fx.update(frame(0.8, 1.0)).flame.expect("closing frame");
        assert_eq!(end.intensity, 0.0);
        assert!(fx.update(frame(0.8, 1.0)).flame.is_none());
    }

    #[test]
    fn no_flames_below_the_minimum_revs() {
        let mut fx = OverrunBackfire::new(tuning(), 3);
        charge_up(&mut fx, 5.0, 1.0);
        for _ in 0..20 {
            assert!(fx.update(frame(0.0, 0.05)).flame.is_none());
        }
    }

    #[test]
    fn nonsense_input_is_read_as_zero_and_keeps_the_charge_in_range() {
        let mut fx = OverrunBackfire::new(tuning(), 1);
        charge_up(&mut fx, 1.0, 1.0);
        let before = fx.charge();
        fx.update(OverrunInput { dt: f32::NAN, throttle: f32::NAN, raw_throttle: 9.0, rpm_fraction: f32::INFINITY });
        assert!(fx.charge() <= before && fx.charge() >= 0.0);
        fx.update(OverrunInput { dt: -5.0, throttle: 7.0, raw_throttle: -1.0, rpm_fraction: 4.0 });
        assert!((0.0..=1.0).contains(&fx.charge()));
    }

    #[test]
    fn an_instant_lift_bangs_at_the_full_charge() {
        let mut fx = OverrunBackfire::new(tuning(), 1);
        charge_up(&mut fx, 5.0, 1.0);
        let bang = fx.update(frame(0.0, 1.0)).bang.expect("a bang");
        assert!(bang.intensity > 0.95, "no time passed between arming and firing: {}", bang.intensity);
    }

    #[test]
    fn the_stock_numbers_bang_only_after_a_near_instant_lift_from_a_full_charge() {
        let stock = OverrunTuning {
            accumulator_rate: 0.2,
            throttle_arm: 0.9,
            throttle_fire: 0.1,
            window_seconds: 2.0,
            flame_delay: 0.2,
            flame_min_rpm: 0.1,
            bang_audio_threshold: 0.95,
            bang_effect_threshold: 0.8,
            flame_effect_threshold: 1.0,
            probability_closed: 1.0,
            probability_open: 1.0,
        };
        let step = |fx: &mut OverrunBackfire, throttle: f32| {
            fx.update(OverrunInput { dt: 0.01, throttle, raw_throttle: throttle, rpm_fraction: 1.0 })
        };
        // Charge for six seconds at the limit, then lift within one frame: the strength is the full charge.
        let mut fx = OverrunBackfire::new(stock, 1);
        (0..600).for_each(|_| {
            step(&mut fx, 1.0);
        });
        let bang = step(&mut fx, 0.0).bang.expect("a bang after a full pull");
        assert!(bang.intensity > 0.99 && bang.audio_level > 0.9);
        assert_eq!(bang.effect_level.map(|level| level > 0.9), Some(true));
        // The same lift after a pull that charged to 0.5 is too weak for the audio threshold of 0.95.
        let mut half = OverrunBackfire::new(stock, 1);
        (0..250).for_each(|_| {
            step(&mut half, 1.0);
        });
        assert!(half.charge() < 0.6);
        assert!(step(&mut half, 0.0).bang.is_none());
        // And a lift that takes a second to reach the firing level scales the strength by half the window: no bang.
        let mut slow = OverrunBackfire::new(stock, 1);
        (0..600).for_each(|_| {
            step(&mut slow, 1.0);
        });
        let mut bang = None;
        for _ in 0..100 {
            bang = bang.or(step(&mut slow, 0.5).bang);
        }
        bang = bang.or(step(&mut slow, 0.0).bang);
        assert!(bang.is_none());
    }

    #[test]
    fn same_seed_same_result() {
        let run = |seed| {
            let mut t = tuning();
            t.probability_closed = 0.5;
            t.probability_open = 0.5;
            let mut fx = OverrunBackfire::new(t, seed);
            let mut bangs = Vec::new();
            for round in 0..20 {
                charge_up(&mut fx, 3.0, 1.0);
                for throttle in [0.5, 0.0, 0.0] {
                    bangs.push((round, fx.update(frame(throttle, 1.0)).bang.is_some()));
                }
            }
            bangs
        };
        assert_eq!(run(5), run(5));
    }
}
