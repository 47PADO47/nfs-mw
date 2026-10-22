//! Fixed-step timing: the physics runs at 60 Hz whatever the frame rate is.

/// The physics step, seconds.
pub const STEP: f32 = 1.0 / 60.0;
const CLOCK_STEP: f64 = 1.0 / 60.0;
/// Compensate only for float conversion at a tick boundary, not meaningful elapsed time.
const CLOCK_EPSILON: f64 = 1e-8;

#[derive(Debug, Default)]
pub struct FixedClock {
    accumulator: f64,
    /// Every update with time to spend runs exactly one step, whatever its length: a server's world, whose own loop
    /// keeps the 60 Hz pace (`scenes/world/net`). Off for a playing scene, whose steps follow the frame time.
    one_per_update: bool,
}

impl FixedClock {
    /// Run exactly one step per update that has time to spend (`true`), or follow the frame time (`false`).
    pub fn set_one_per_update(&mut self, on: bool) {
        self.one_per_update = on;
        self.accumulator = 0.0;
    }

    /// Add a frame's time; returns how many steps to run now.
    pub fn advance(&mut self, dt: f32) -> u32 {
        if !dt.is_finite() || dt <= 0.0 {
            return 0;
        }
        if self.one_per_update {
            return 1;
        }
        self.accumulator += f64::from(dt);
        let steps = ((self.accumulator + CLOCK_EPSILON) / CLOCK_STEP) as u32;
        self.accumulator -= f64::from(steps) * CLOCK_STEP;
        steps
    }

    /// How far the display is between the last two steps, 0..1.
    pub fn alpha(&self) -> f32 {
        (self.accumulator / CLOCK_STEP).clamp(0.0, 1.0) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_follow_the_frame_time() {
        let mut clock = FixedClock::default();
        // 120 fps: a step every other frame.
        let steps: Vec<u32> = (0..6).map(|_| clock.advance(1.0 / 120.0)).collect();
        assert_eq!(steps.iter().sum::<u32>(), 3);
        // 30 fps: two steps per frame.
        assert_eq!(clock.advance(1.0 / 30.0), 2);
    }

    #[test]
    fn keeps_the_remainder_for_interpolation() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.advance(STEP * 1.5), 1);
        assert!((clock.alpha() - 0.5).abs() < 1e-4);
    }

    #[test]
    fn a_stall_keeps_all_elapsed_simulation_time() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.advance(5.0), 300);
        assert_eq!(clock.advance(0.0), 0, "all elapsed ticks ran");
    }

    #[test]
    fn one_per_update_runs_a_step_for_any_positive_time() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.advance(STEP * 0.5), 0);
        clock.set_one_per_update(true);
        assert_eq!([clock.advance(STEP), clock.advance(STEP * 0.25), clock.advance(5.0)], [1, 1, 1]);
        assert_eq!([clock.advance(0.0), clock.advance(f32::NAN)], [0, 0], "no time is no step");
        assert_eq!(clock.alpha(), 0.0, "the remainder of the earlier frame time is dropped");
        clock.set_one_per_update(false);
        assert_eq!(clock.advance(STEP * 2.0), 2);
    }

    #[test]
    fn zero_and_negative_time_do_nothing() {
        let mut clock = FixedClock::default();
        assert_eq!(clock.advance(0.0), 0);
        assert_eq!(clock.advance(-1.0), 0);
        assert_eq!(clock.advance(f32::NAN), 0);
        assert_eq!(clock.advance(f32::INFINITY), 0);
        assert_eq!(clock.advance(STEP), 1, "invalid frame time did not poison the clock");
    }
}
