//! The small signal language the rider's animation blueprint is written in (`docs/specs/ride6-rider-round6.md`, section 4):
//! per-frame smoothers with a memory, a delay line, and the fuzzy-logic operators. Each is the operation the reference's script
//! library performs on its value stack, written out; the numbers they are used with are the blueprint's own and live in
//! `params.rs`.

use std::collections::VecDeque;

/// A difference this small (squared) counts as none: the reference snaps to the input below it (1e-8).
const SNAP: f32 = 1e-8;

/// `FuzzifyRamp(x, a, b)`: 0 at `a`, 1 at `b`, linear between and held beyond, in either direction; a range narrower than 1e-8
/// is a step at `b`.
pub fn ramp(x: f32, a: f32, b: f32) -> f32 {
    let width = b - a;
    if width.abs() <= 1e-8 {
        return f32::from(x >= b);
    }
    ((x - a) / width).clamp(0.0, 1.0)
}

/// `DefuzzifyFloatEx(rules, default)` for rules of `(weight, value)`: with the weights adding up to 1 or more, their weighted
/// mean; below 1, the weighted values plus the rest of the unit weight on `default`.
pub fn defuzzify(rules: &[(f32, f32)], default: f32) -> f32 {
    let (weight, sum) = rules.iter().fold((0.0, 0.0), |(w, s), &(rw, rv)| (w + rw, s + rw * rv));
    if rules.is_empty() || weight < 1.0 {
        return sum + (1.0 - weight) * default;
    }
    sum / weight
}

/// `LogicalSum`: the probabilistic sum, `a + b - a b`.
pub fn fuzzy_sum(a: f32, b: f32) -> f32 {
    a + b - a * b
}

/// `LogicalNot`.
pub fn fuzzy_not(a: f32) -> f32 {
    1.0 - a
}

/// `SmoothFloat_Interpolate(x, rate)`: each frame the value covers the share `rate dt` (at most 1) of what is left to the input;
/// it starts at 0.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Interpolate(f32);

impl Interpolate {
    pub fn step(&mut self, x: f32, rate: f32, dt: f32) -> f32 {
        let left = x - self.0;
        self.0 = match rate > 0.0 && left * left >= SNAP {
            true => self.0 + (rate * dt).clamp(0.0, 1.0) * left,
            false => x,
        };
        self.0
    }
}

/// `SmoothFloat_Constant(x, rate)`: the value moves towards the input by at most `rate dt` a frame; it starts at 0.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Constant(f32);

impl Constant {
    pub fn step(&mut self, x: f32, rate: f32, dt: f32) -> f32 {
        let left = x - self.0;
        if left * left >= SNAP {
            let reach = rate * dt;
            self.0 += left.clamp(-reach, reach);
        }
        self.0
    }
}

/// `FlashbackFloat(x, time)`: the input as it was `time` seconds ago; 0 until that much history exists.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flashback {
    /// `(age in seconds, value)`, newest last.
    history: VecDeque<(f32, f32)>,
}

impl Flashback {
    pub fn step(&mut self, x: f32, time: f32, dt: f32) -> f32 {
        for sample in &mut self.history {
            sample.0 += dt;
        }
        self.history.push_back((0.0, x));
        // Keep the newest sample that is old enough and everything newer.
        while self.history.len() > 1 && self.history[1].0 >= time {
            self.history.pop_front();
        }
        match self.history.front() {
            Some(&(age, value)) if age >= time => value,
            _ => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ramp_runs_either_way_and_a_flat_one_is_a_step_at_its_end() {
        assert_eq!(ramp(0.25, 0.0, 1.0), 0.25);
        assert_eq!(ramp(-0.5, 0.0, -0.85), 0.5 / 0.85);
        assert_eq!(ramp(2.0, 0.0, -0.85), 0.0, "past the start on the other side");
        assert_eq!(ramp(5.0, 3.0, 3.0), 1.0);
        assert_eq!(ramp(2.0, 3.0, 3.0), 0.0);
    }

    #[test]
    fn defuzzification_is_the_weighted_mean_with_the_rest_on_the_default() {
        // Weights of 1 or more: the mean; the value of a single full rule.
        assert_eq!(defuzzify(&[(1.0, 0.7)], 5.0), 0.7);
        assert_eq!(defuzzify(&[(1.0, 2.0), (3.0, 4.0)], 9.0), 3.5);
        // Weights under 1: the default fills up to 1.
        assert!((defuzzify(&[(0.25, 1.0)], 0.0) - 0.25).abs() < 1e-7);
        assert!((defuzzify(&[(0.25, 1.0)], 1.0) - 1.0).abs() < 1e-7);
        assert_eq!(defuzzify(&[], 3.0), 3.0);
        assert_eq!(defuzzify(&[(0.0, 8.0)], 3.0), 3.0);
    }

    #[test]
    fn the_logical_operators_are_the_product_family() {
        assert!((fuzzy_sum(0.5, 0.5) - 0.75).abs() < 1e-7);
        assert_eq!(fuzzy_sum(0.0, 0.3), 0.3);
        assert_eq!(fuzzy_not(0.25), 0.75);
    }

    #[test]
    fn the_interpolation_covers_a_share_of_the_rest_each_frame_and_never_overshoots() {
        let mut y = Interpolate::default();
        assert!((y.step(1.0, 4.0, 0.25) - 1.0).abs() < 1e-7, "rate times dt of 1 reaches it");
        let mut y = Interpolate::default();
        assert!((y.step(1.0, 4.0, 0.1) - 0.4).abs() < 1e-7);
        assert!((y.step(1.0, 4.0, 0.1) - 0.64).abs() < 1e-6);
        assert_eq!(Interpolate::default().step(1.0, 100.0, 1.0), 1.0, "the share is held at 1");
        assert_eq!(Interpolate::default().step(0.7, 0.0, 0.1), 0.7, "no rate: the input");
    }

    #[test]
    fn the_constant_smoother_moves_by_at_most_its_rate() {
        let mut y = Constant::default();
        assert!((y.step(1.0, 3.0, 0.1) - 0.3).abs() < 1e-7);
        assert!((y.step(1.0, 3.0, 0.1) - 0.6).abs() < 1e-6);
        let mut y = Constant::default();
        y.step(1.0, 100.0, 1.0);
        assert!((y.step(-1.0, 3.0, 0.1) - 0.7).abs() < 1e-6, "down as well");
        let held = y.step(0.7, 3.0, 0.1);
        assert_eq!(y.step(0.7, 3.0, 0.1), held);
    }

    #[test]
    fn the_flashback_gives_the_input_of_a_while_ago() {
        let mut f = Flashback::default();
        // One step is a tenth of a second; the delay is a quarter of one.
        let out: Vec<f32> = (0..8).map(|i| f.step(i as f32 + 1.0, 0.25, 0.1)).collect();
        assert_eq!(&out[..3], &[0.0, 0.0, 0.0], "no history yet");
        assert_eq!(out[3], 1.0);
        assert_eq!(out[7], 5.0);
    }
}
