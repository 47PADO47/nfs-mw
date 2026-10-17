//! The sideways blend of a lane change: how much of the lateral gap is still to close, over the distance the car drives.
//! Rewrite extension (spec `docs/specs/ai-traffic-obstacles.md` §10.1: blend over 2 s × speed).

/// Seconds of driving the blend takes.
pub const BLEND_SECONDS: f32 = 2.0;
/// Below this speed (m/s) the blend is measured at this speed instead, so that a car that has almost stopped still
/// moves over in a few metres rather than in one step.
pub const MIN_BLEND_SPEED: f32 = 3.0;

/// The distance (metres) the blend takes at `speed` (m/s).
pub fn blend_length(speed: f32) -> f32 {
    BLEND_SECONDS * speed.abs().max(MIN_BLEND_SPEED)
}

/// The smooth step from 0 to 1 (zero slope at both ends), so the sideways speed starts and ends at zero.
pub fn smoothstep(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A blend over a distance. The default is finished, so a car that has not changed lane has nothing to close.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Blend {
    travelled: f32,
    length: f32,
}

impl Blend {
    /// A blend that starts now and runs for `length` metres.
    pub fn start(length: f32) -> Self {
        Self { travelled: 0.0, length: length.max(0.0) }
    }

    /// Adds the distance the car has driven since the last call (metres, never negative).
    pub fn advance(&mut self, metres: f32) {
        self.travelled += metres.max(0.0);
    }

    /// The share of the blend done, 0 to 1. A zero-length blend is done at once.
    pub fn progress(&self) -> f32 {
        match self.length > 0.0 {
            true => (self.travelled / self.length).clamp(0.0, 1.0),
            false => 1.0,
        }
    }

    pub fn done(&self) -> bool {
        self.progress() >= 1.0
    }

    /// The share of the lateral gap still to close: 1 at the start, 0 when the blend is done. The lateral gap is
    /// multiplied by this, so the steering target moves from the car's own position to the new lane without a step.
    pub fn remaining(&self) -> f32 {
        1.0 - smoothstep(self.progress())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blend_length_is_two_seconds_of_speed_with_a_floor() {
        assert_eq!(blend_length(10.0), 20.0);
        assert_eq!(blend_length(-4.0), 8.0, "reverse counts by its size");
        assert_eq!(blend_length(0.0), BLEND_SECONDS * MIN_BLEND_SPEED);
    }

    #[test]
    fn the_smooth_step_runs_from_zero_to_one_with_flat_ends() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert_eq!(smoothstep(0.5), 0.5);
        assert!(smoothstep(0.1) < 0.1, "slow start");
        assert_eq!(smoothstep(-1.0), 0.0);
        assert_eq!(smoothstep(2.0), 1.0);
    }

    #[test]
    fn a_default_blend_is_done_so_nothing_is_left_to_close() {
        let blend = Blend::default();
        assert!(blend.done());
        assert_eq!(blend.remaining(), 0.0);
    }

    #[test]
    fn the_gap_closes_smoothly_over_the_length() {
        let mut blend = Blend::start(10.0);
        assert_eq!(blend.remaining(), 1.0);
        let mut last = 1.0;
        for _ in 0..10 {
            blend.advance(1.0);
            let now = blend.remaining();
            assert!(now <= last, "never grows back");
            assert!(last - now <= 0.2, "no step: {last} to {now}");
            last = now;
        }
        assert!(blend.done());
        assert_eq!(blend.remaining(), 0.0);
    }

    #[test]
    fn advancing_backwards_or_past_the_end_changes_nothing_wrong() {
        let mut blend = Blend::start(4.0);
        blend.advance(-3.0);
        assert_eq!(blend.progress(), 0.0);
        blend.advance(100.0);
        assert_eq!(blend.progress(), 1.0);
    }
}
