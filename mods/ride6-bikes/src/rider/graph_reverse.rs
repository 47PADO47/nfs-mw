//! The reverse walk in the rider's blend tree: the rider puts his feet down and walks the bike backwards (the blueprint's
//! `Retro` state, `docs/specs/ride6-rider-styles.md`, section 4).
//!
//! The blueprint's state has two parts. `RetroStart` plays the start clip (`Retro_Start_*`) from `RetroStart_ExplicitStart` to
//! `RetroStart_ExplicitEnd`, stepped by an explicit time; `RetroLoop` then plays the loop clip (`Retro_Loop_*`) at a play rate that
//! grows with the speed backwards. The graph reads four parameters for it, all made by the game's rider runtime: `reverse` (how
//! much of the pose is the walk), `reverse_start` (seconds into the start clip), `reverse_loop` (0 while the start clip plays,
//! 1 once the loop has taken over) and `reverse_cycle` (the loop clip's phase, 0 to 1).

use anyhow::{Context, Result};
use blackbox_skeleton::{Node, Pack, TimeMode, Value};

use super::graph_sets::Walk;

/// `Retro(0.8, 1.567, -1, -1, -20, 1, 4, 4)`: the start clip is played from `0.8` to `1.567` seconds (`RetroStart_ExplicitStart`,
/// `RetroStart_ExplicitEnd`); the walk begins when the speed is below -1 (`RetroStart_Speed_Treshold`, the units of the blueprint's
/// `VehicleSpeed`: km/h); the loop's play rate goes from 1 at -1 to 4 at -20 (`RetroLoop_PlayRate_SpeedMin/Max`, `_PlayRateMin/Max`).
pub const START_SECONDS: (f32, f32) = (0.8, 1.567);
pub const START_SPEED_KMH: f32 = -1.0;
pub const RATE_SPEED_KMH: (f32, f32) = (-1.0, -20.0);
pub const RATE: (f32, f32) = (1.0, 4.0);

/// Parameter indices of the walk, as `graph.rs` orders them.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub weight: usize,
    pub start: usize,
    pub looped: usize,
    pub cycle: usize,
}

/// `base` with the walk blended over it by the `reverse` parameter, when the pack has both clips of `walk`; else `base`.
pub fn with_walk(pack: &Pack, base: Node, walk: &Walk, params: Params) -> Result<Node> {
    let (Some(start), Some(looped)) = (pack.clip_index(walk.start), pack.clip_index(walk.looped)) else {
        return Ok(base);
    };
    let length = pack.clips[start].length;
    ensure_length(walk.start, length)?;
    let seconds = Value::remap(params.start, (0.0, length), (0.0, 1.0));
    let started = Node::Clip { clip: start, time: TimeMode::Fraction(seconds) };
    let cycling = Node::Clip { clip: looped, time: TimeMode::Fraction(Value::param(params.cycle)) };
    let walking = Node::Blend { a: Box::new(started), b: Box::new(cycling), weight: Value::param(params.looped) };
    Ok(Node::Blend { a: Box::new(base), b: Box::new(walking), weight: Value::param(params.weight) })
}

fn ensure_length(name: &str, length: f32) -> Result<()> {
    (length > 0.0 && length.is_finite()).then_some(()).with_context(|| format!("the animation {name} has no length"))
}

/// The length of the loop clip in seconds: the game wraps the cycle with it. `None` when the pack lacks the walk.
pub fn loop_seconds(pack: &Pack, walk: &Walk) -> Option<f32> {
    let looped = pack.clip_index(walk.looped)?;
    pack.clip_index(walk.start)?;
    Some(pack.clips[looped].length)
}
