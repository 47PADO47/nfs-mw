//! The reverse walk's signals: when the bike rolls backwards the rider puts his feet down and walks it
//! (`docs/specs/ride6-rider-styles.md`, section 4).
//!
//! The animation blueprint's `Retro` function and state machine: while the bike's speed is below `RetroStart_Speed_Treshold`
//! (-1 km/h) the rider is in the `RetroStart` state, whose clip is played by an explicit time that goes from
//! `RetroStart_ExplicitStart` to `RetroStart_ExplicitEnd` at one clip second a second (`SmoothFloat_Constant(.., 1)`); when it
//! reaches the end the `RetroLoop` state takes over and plays its clip at a play rate that goes from `RetroLoop_PlayRateMin` at
//! `RetroLoop_PlayRate_SpeedMin` to `_PlayRateMax` at `_SpeedMax` (`FuzzifyRamp` of the speed, then `DefuzzifyFloatEx`). The
//! numbers come with the rig (`styles.json`, [`Walk`]). The signals this makes drive the graph's parameters `reverse`,
//! `reverse_start`, `reverse_loop` and `reverse_cycle`.

use super::script::{Constant, ramp};
use super::style::Walk;

/// How fast the walk fades in and out (per second): the pose crosses to the walk and back in a quarter of a second. A
/// modelling choice: the blueprint's state machine blends its states over a time that is stored in cooked data not read.
const FADE_RATE: f32 = 4.0;

/// What the graph reads.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Signals {
    /// How much of the pose is the walk, 0 to 1.
    pub weight: f32,
    /// Seconds into the start clip.
    pub start: f32,
    /// 0 while the start clip plays, 1 once the loop has taken over.
    pub looped: f32,
    /// The loop clip's phase, 0 to 1.
    pub cycle: f32,
}

/// The walk's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reverse {
    walk: Option<Walk>,
    fade: Constant,
    start: f32,
    cycle: f32,
}

impl Reverse {
    /// A walk with the numbers `walk`; `None`: the rider never walks the bike back.
    pub fn new(walk: Option<Walk>) -> Self {
        Self { start: walk.map_or(0.0, |w| w.start_seconds[0]), walk, ..Self::default() }
    }

    /// Advance by `dt` seconds with the bike moving at `speed_kmh` along itself (negative backwards).
    pub fn step(&mut self, speed_kmh: f32, dt: f32) -> Signals {
        let Some(walk) = self.walk else { return Signals::default() };
        let walking = speed_kmh.is_finite() && speed_kmh < walk.start_speed_kmh;
        let weight = self.fade.step(f32::from(walking), FADE_RATE, dt);
        let [from, to] = walk.start_seconds;
        // `RetroStart_Explicit` moves towards the end of the window while the walk is on and back to its start while it is off.
        let goal = if walking { to } else { from };
        self.start += (goal - self.start).clamp(-dt, dt);
        let looped = walking && self.start >= to;
        match looped {
            true => {
                let rate = walk.rate[0]
                    + (walk.rate[1] - walk.rate[0]) * ramp(speed_kmh, walk.rate_speed_kmh[0], walk.rate_speed_kmh[1]);
                self.cycle = (self.cycle + dt * rate / walk.loop_seconds).rem_euclid(1.0);
            }
            false => self.cycle = 0.0,
        }
        Signals { weight, start: self.start, looped: f32::from(looped), cycle: self.cycle }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walk() -> Walk {
        Walk {
            start_seconds: [0.8, 1.567],
            start_speed_kmh: -1.0,
            rate_speed_kmh: [-1.0, -20.0],
            rate: [1.0, 4.0],
            loop_seconds: 2.0,
        }
    }

    fn run(reverse: &mut Reverse, speed: f32, seconds: f32) -> Signals {
        let mut out = Signals::default();
        for _ in 0..(seconds * 100.0) as usize {
            out = reverse.step(speed, 0.01);
        }
        out
    }

    #[test]
    fn a_rider_without_a_walk_never_walks() {
        let mut reverse = Reverse::new(None);
        assert_eq!(run(&mut reverse, -10.0, 1.0), Signals::default());
    }

    #[test]
    fn rolling_forwards_or_standing_still_is_not_the_walk() {
        let mut reverse = Reverse::new(Some(walk()));
        let out = run(&mut reverse, 30.0, 1.0);
        assert_eq!((out.weight, out.looped, out.cycle), (0.0, 0.0, 0.0));
        assert_eq!(out.start, 0.8, "the start clip waits at the start of its window");
        assert_eq!(run(&mut reverse, -0.5, 1.0).weight, 0.0, "slower than the threshold backwards is still standing");
    }

    #[test]
    fn rolling_back_plays_the_start_clip_then_loops_it() {
        let mut reverse = Reverse::new(Some(walk()));
        let early = run(&mut reverse, -5.0, 0.3);
        assert!(early.weight > 0.9 && early.looped == 0.0, "{early:?}");
        assert!((early.start - 1.1).abs() < 0.02, "the window is walked at a clip second a second: {early:?}");
        let later = run(&mut reverse, -5.0, 0.6);
        assert_eq!(later.looped, 1.0);
        assert_eq!(later.start, 1.567);
        assert_eq!(later.weight, 1.0);
        assert!(later.cycle > 0.0 && later.cycle < 1.0);
    }

    #[test]
    fn the_loop_plays_faster_the_faster_the_bike_rolls_back() {
        let cycles = |speed: f32| {
            let mut reverse = Reverse::new(Some(walk()));
            run(&mut reverse, speed, 1.0); // through the start clip
            let before = reverse.cycle;
            let after = run(&mut reverse, speed, 0.1).cycle;
            (after - before).rem_euclid(1.0)
        };
        let (slow, fast) = (cycles(-1.5), cycles(-20.0));
        // 0.1 s at about rate 1 (just past the threshold, which is not walking yet) over a 2 s loop is 0.05 of a cycle; at rate 4 it is 0.2.
        assert!((slow - 0.05).abs() < 0.01, "{slow}");
        assert!((fast - 0.2).abs() < 0.01, "{fast}");
        assert!(cycles(-10.0) > slow && cycles(-10.0) < fast);
    }

    #[test]
    fn stopping_walks_the_pose_back_to_riding_and_rewinds_the_start_clip() {
        let mut reverse = Reverse::new(Some(walk()));
        run(&mut reverse, -5.0, 2.0);
        let out = run(&mut reverse, 0.0, 1.5);
        assert_eq!((out.weight, out.looped, out.cycle), (0.0, 0.0, 0.0));
        assert_eq!(out.start, 0.8);
    }
}
