//! The second-order filter the rider's controller uses everywhere: stiffness `K` (a squared natural frequency) and a damping
//! ratio `z`. One step is the exact solution of `x'' = -K x - 2 z sqrt(K) x'` over the step for the under-, critically and
//! over-damped case, so a large `K dt^2` cannot make it unstable (`docs/specs/ride6-lean-controller.md`).

/// The state of the filter: its output and the output's speed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SecondOrder {
    output: f32,
    speed: f32,
}

impl SecondOrder {
    pub fn new(value: f32) -> Self {
        Self { output: value, speed: 0.0 }
    }

    pub fn output(&self) -> f32 {
        self.output
    }

    /// Hold the output at `value` and stop it.
    pub fn reset(&mut self, value: f32) {
        *self = Self::new(value);
    }

    /// Move the output towards `input` for `dt` seconds with stiffness `k` and damping ratio `ratio`; returns the output.
    pub fn step(&mut self, input: f32, k: f32, ratio: f32, dt: f32) -> f32 {
        if dt.is_nan() || dt <= 0.0 || k.is_nan() || k <= 0.0 {
            self.output = input;
            self.speed = 0.0;
            return input;
        }
        let (x, v) = advance(self.output - input, self.speed, k, ratio.max(0.0), dt);
        self.output = input + x;
        self.speed = v;
        self.output
    }
}

/// The offset `x` and speed `v` of the free motion `x'' = -K x - 2 z sqrt(K) x'` after `dt`.
fn advance(x: f32, v: f32, k: f32, ratio: f32, dt: f32) -> (f32, f32) {
    let w = k.sqrt();
    if (ratio - 1.0).abs() < 1e-4 {
        let decay = (-w * dt).exp();
        let c = v + w * x;
        return (decay * (x + c * dt), decay * (v - w * c * dt));
    }
    if ratio < 1.0 {
        let wd = w * (1.0 - ratio * ratio).sqrt();
        let decay = (-ratio * w * dt).exp();
        let (sin, cos) = (wd * dt).sin_cos();
        let b = (v + ratio * w * x) / wd;
        let x_new = decay * (x * cos + b * sin);
        let v_new = decay * (v * cos - (w * w * x + ratio * w * v) / wd * sin);
        return (x_new, v_new);
    }
    let root = (ratio * ratio - 1.0).sqrt();
    let (s1, s2) = (w * (-ratio + root), w * (-ratio - root));
    let a = (v - s2 * x) / (s1 - s2);
    let b = (s1 * x - v) / (s1 - s2);
    let (e1, e2) = ((s1 * dt).exp(), (s2 * dt).exp());
    (a * e1 + b * e2, a * s1 * e1 + b * s2 * e2)
}

/// The acceleration `-K x - 2 z sqrt(K) v` of a spring-damper at offset `x` and speed `v`.
pub fn acceleration(x: f32, v: f32, k: f32, ratio: f32) -> f32 {
    -k * x - 2.0 * ratio * k.max(0.0).sqrt() * v
}

/// The time constant (s) of the slowest decaying mode of the filter: the inverse of its slow pole when over-damped, of the real
/// part of its poles otherwise. 0 for a filter that has no stiffness (it passes its input at once).
pub fn slow_time_constant(k: f32, ratio: f32) -> f32 {
    if k.is_nan() || k <= 0.0 || ratio.is_nan() || ratio <= 0.0 {
        return 0.0;
    }
    let w = k.sqrt();
    let rate = match ratio > 1.0 {
        true => w * (ratio - (ratio * ratio - 1.0).sqrt()),
        false => w * ratio,
    };
    1.0 / rate.max(f32::EPSILON)
}

/// The constant acceleration that carries a spring-damper at offset `x` and speed `v` to the speed the exact solution of
/// `x'' = -K x - 2 z sqrt(K) x'` has after `dt`. Evaluating [`acceleration`] once per step and holding it over the step is
/// unstable as soon as `2 z sqrt(K) dt` exceeds 2 (the lean controller's `K = 1000`, `z = 3.6` at 60 Hz gives 3.8); this one
/// is the same law, realised over the step the way the filters are.
pub fn step_acceleration(x: f32, v: f32, k: f32, ratio: f32, dt: f32) -> f32 {
    if dt.is_nan() || dt <= 0.0 || k.is_nan() || k <= 0.0 {
        return acceleration(x, v, k, ratio);
    }
    let (_, v_new) = advance(x, v, k, ratio.max(0.0), dt);
    (v_new - v) / dt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_damping_regime_settles_on_the_input_without_blowing_up() {
        for ratio in [0.3, 1.0, 2.25, 3.6] {
            let mut f = SecondOrder::new(0.0);
            for _ in 0..600 {
                f.step(1.0, 1000.0, ratio, 1.0 / 60.0);
            }
            assert!((f.output() - 1.0).abs() < 1e-3, "{ratio} {}", f.output());
        }
    }

    #[test]
    fn an_overdamped_filter_does_not_overshoot() {
        let mut f = SecondOrder::new(0.0);
        let mut highest: f32 = 0.0;
        for _ in 0..300 {
            highest = highest.max(f.step(1.0, 1000.0, 3.0, 1.0 / 60.0));
        }
        assert!(highest <= 1.0 + 1e-4, "{highest}");
    }

    #[test]
    fn the_step_is_the_same_in_two_halves() {
        let (mut whole, mut halves) = (SecondOrder::new(0.2), SecondOrder::new(0.2));
        whole.step(1.0, 400.0, 0.5, 0.02);
        halves.step(1.0, 400.0, 0.5, 0.01);
        halves.step(1.0, 400.0, 0.5, 0.01);
        assert!((whole.output() - halves.output()).abs() < 1e-5);
    }

    #[test]
    fn a_stiff_filter_with_a_long_step_stays_finite() {
        let mut f = SecondOrder::new(0.0);
        assert!(f.step(1.0, 1.0e8, 2.0, 0.1).is_finite());
        assert!(f.step(-1.0, 1.0e8, 0.2, 0.1).is_finite());
    }

    #[test]
    fn the_step_acceleration_is_stable_where_the_plain_one_is_not() {
        // 2 z sqrt(K) dt = 3.8: the plain acceleration held over a step flips the speed's sign and grows it.
        let (k, ratio, dt) = (1000.0_f32, 3.6_f32, 1.0 / 60.0);
        let (mut x, mut v) = (0.1_f32, 0.0_f32);
        for _ in 0..300 {
            let a = step_acceleration(x, v, k, ratio, dt);
            x += v * dt + 0.5 * a * dt * dt;
            v += a * dt;
        }
        assert!(x.abs() < 1e-3 && v.abs() < 1e-2, "{x} {v}");
    }

    #[test]
    fn the_slow_time_constant_is_that_of_the_slow_pole() {
        // K = 1000, z = 3: poles at -5.4 and -185 per second.
        assert!((slow_time_constant(1000.0, 3.0) - 0.1847).abs() < 1e-3);
        assert!((slow_time_constant(400.0, 0.5) - 0.1).abs() < 1e-6);
        assert_eq!(slow_time_constant(0.0, 3.0), 0.0);
    }

    #[test]
    fn the_acceleration_pulls_back_and_damps() {
        assert!(acceleration(1.0, 0.0, 1000.0, 2.0) < 0.0);
        assert!(acceleration(0.0, 1.0, 1000.0, 2.0) < 0.0);
    }
}
