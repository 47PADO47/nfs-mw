//! The blend tree of a rider, built as data for [`blackbox_skeleton`], for any of the bike types of the animation blueprint
//! ([`graph_sets`]: race, naked, motard, scooter, enduro).
//!
//! Every clip name is an asset of the rider animation set (`Animations/<type>/`), chosen by the animation blueprint's
//! `SET Animations` function for the bike type. Every number is a default of that blueprint's class (the variable name is
//! given); `docs/specs/ride6-rider-animation.md` lists what is data and what is a choice, `ride6-rider-styles.md` how the
//! type is picked.

use anyhow::{Context, Result};
use blackbox_skeleton::{Axis, BlendSpace, Graph, Node, Pack, SpaceSample, TimeMode, Value};

use super::graph_fingers;
use super::graph_reverse;
use super::graph_sets::{self, ClipSet, MOTARD};
use super::graph_stand;

/// Parameter order of the graph (the names are in [`Graph::param_names`]).
///
/// `lean` is `RiderLeaning`, `tuck` `RiderTuckedIn_Alpha`, `wheelie` the entry of the wheelie pose (`BikeWheelie` held to 8 per
/// second), `wheelie_alpha` `Wheelie_Alpha` (the minimum-to-maximum pose), `wheelie_direction` `Wheelie_Direction`, `stoppie`
/// `Stoppie_Alpha` and `stoppie_time` the stoppie clip's time over 1.27 s (`ride6-rider-round6.md`, section 4). A graph made
/// before the last three existed reads them as 0.
///
/// The last four feed the clips that were stored with ACL and could not be read before ([`crate::rider::acl_clips`]):
/// `standing` is `Wheelie_Standing_Alpha` (the rider stands on the pegs through a wheelie), `steer` the smoothed steering
/// input and `standing_entry` `Wheelie_Entry_Explicit` (the two axes of the standing wheelie's blend space), and `drift`
/// `Drift3BlendAlpha` (-1 to 1, positive when the bike's heading is to the right of its path: a drift in a right-hand turn).
///
/// The last four feed the reverse walk ([`graph_reverse`]): `reverse` is how much of the pose is the walk, `reverse_start` the
/// seconds into the start clip, `reverse_loop` 0 while the start clip plays and 1 once the loop has taken over, and
/// `reverse_cycle` the loop clip's phase (0 to 1).
///
/// The last two pick the lean pose (`docs/specs/ride6-rider-knee-foot.md`): `lean_out` is `RiderLean_LeftOrRight` (0 the left lean
/// clip, 1 the right one) and `straight` `LeanOrStraight_Alpha` (1 the upright pose over the lean sweep). A graph made before them
/// reads 0 for both.
///
/// `riding_style` picks the lean set of a graph built for every style ([`graph_selectable`]; `RidingStyle` of the blueprint: 0
/// balanced, 1 shoulders out, 2 old school). The five after it make the standstill ([`graph_stand`]): `stand` is how much of
/// the pose is the rider with his feet down, `stand_state` which of the stop, standing and start clips (0, 1, 2), `stand_leg`
/// which feet (0 both, 1 the left, 2 the right) and `stop_time` and `start_time` the seconds into the stop and start clips. The
/// last four put the fingers on the levers ([`graph_fingers`]): `clutch` and `brake` are the levers (0 let go, 1 pulled in) and
/// `clutch_alpha` and `brake_alpha` the weights of the finger layers.
pub const PARAMS: [&str; 28] = [
    "lean",
    "speed_kmh",
    "tuck",
    "wheelie",
    "stoppie",
    "wheelie_alpha",
    "stoppie_time",
    "wheelie_direction",
    "standing",
    "steer",
    "standing_entry",
    "drift",
    "reverse",
    "reverse_start",
    "reverse_loop",
    "reverse_cycle",
    "lean_out",
    "straight",
    "riding_style",
    "stand",
    "stand_state",
    "stand_leg",
    "stop_time",
    "start_time",
    "clutch",
    "clutch_alpha",
    "brake",
    "brake_alpha",
];
const LEAN: usize = 0;
const SPEED: usize = 1;
const TUCK: usize = 2;
const WHEELIE: usize = 3;
const STOPPIE: usize = 4;
const WHEELIE_ALPHA: usize = 5;
const STOPPIE_TIME: usize = 6;
const WHEELIE_DIRECTION: usize = 7;
const STANDING: usize = 8;
const STEER: usize = 9;
const STANDING_ENTRY: usize = 10;
const DRIFT: usize = 11;
const REVERSE: graph_reverse::Params = graph_reverse::Params { weight: 12, start: 13, looped: 14, cycle: 15 };
const LEAN_OUT: usize = 16;
const STRAIGHT: usize = 17;
const RIDING_STYLE: usize = 18;
const STAND: graph_stand::Params =
    graph_stand::Params { weight: 19, state: 20, leg: 21, stop_time: 22, start_time: 23 };
const FINGERS: graph_fingers::Params =
    graph_fingers::Params { clutch: 24, clutch_alpha: 25, brake: 26, brake_alpha: 27 };
/// `BrakeFingers` of the blueprint: how many fingers pull the brake lever. The blueprint's own default and the `BrakeFingers` of
/// the `RIDING_STYLE` presets are 2 (the `BRAKE_FINGERS` option of the riding style page offers 1, 2 and 4).
const BRAKE_FINGERS: u8 = 2;

/// The seconds the lean clips are swept over: the blueprint evaluates the left clip at `RiderLeanLeft_Explicit = 1 - RiderLeaning`
/// and the right one at `RiderLeanRight_Explicit = RiderLeaning + 1` seconds (`Rider_Lean_Polish`), so the clip's frame 0 is one
/// full lean, its middle (1 s) the upright body and its last frame (2 s) the other full lean.
const SWEEP_SECONDS: f32 = 2.0;

/// `FuzzifyRamp(BikeWheelie, 0.2, 0.4)` in `Wheelie/ Stoppie`: the wheelie pose takes over between these values. Which node
/// the blueprint binds `Wheelie_Entry_Alpha` to is in cooked binary data; this ramp on the entry is the reading of the names.
const WHEELIE_ENTRY_RAMP: (f32, f32) = (0.2, 0.4);
/// `Stoppie_Explicit` = stoppie value times 1.27 seconds (`MulFloat(StoppieValue, 1.27)`) into `motard_stoppie`.
const STOPPIE_CLIP_SECONDS: f32 = 1.27;
/// `AddNoise_Start` and `AddNoise_StartEnd` (km/h): the additive shake fades in over this speed range.
const NOISE_RAMP: (f32, f32) = (10.0, 220.0);
/// `Riding_AdditiveNoise_Alpha`: the weight of the shake at the end of the ramp.
const NOISE_ALPHA: f32 = 0.4;
/// `AddNoise_StartEnd` and `AddNoise_EndStart` (km/h): the low-speed shake gives way to the high-speed one.
const NOISE_SWAP: (f32, f32) = (220.0, 280.0);
/// The standing wheelie's blend space (`motard_Wheelie_Standing_BS`): the steering from -1 to 1 in two cells, the entry value
/// from 0 to 0.3 in one. Samples (steering, entry): the standing pose at (0, 0.3), the same pose leaning left at (-1, 0.3) and
/// right at (1, 0.30234), and `StandingInterpolation` at (0, 0), the pose the rider leaves the seat from.
const STANDING_STEER: (f32, f32, usize) = (-1.0, 1.0, 2);
const STANDING_ENTRY_RANGE: (f32, f32) = (0.0, 0.3);

fn node(pack: &Pack, name: &str, time: TimeMode) -> Result<Node> {
    let clip = pack.clip_index(name).with_context(|| format!("the animation {name} is not in the pack"))?;
    Ok(Node::Clip { clip, time })
}

fn looped(pack: &Pack, name: &str) -> Result<Node> {
    node(pack, name, TimeMode::Loop { rate: 1.0 })
}

fn blend(a: Node, b: Node, weight: Value) -> Node {
    Node::Blend { a: Box::new(a), b: Box::new(b), weight }
}

/// The clip `name` swept by the lean: at `lean` -1 to 1 it is at `0` to `SWEEP_SECONDS` seconds (`mirrored`: at `SWEEP_SECONDS` to 0).
fn sweep(pack: &Pack, name: &str, mirrored: bool) -> Result<Node> {
    let clip = pack.clip_index(name).with_context(|| format!("the animation {name} is not in the pack"))?;
    let end = SWEEP_SECONDS / pack.clips[clip].length;
    let (from, to) = match mirrored {
        true => (end, 0.0),
        false => (0.0, end),
    };
    // `Fraction` is the clip's length times the value: the explicit time over the length, held at the clip's last frame.
    let time = TimeMode::Fraction(Value::remap(LEAN, (-1.0, 1.0), (from, to)));
    Ok(Node::Clip { clip, time })
}

/// The lean pose of one set (`LeaningAnimations`, `Rider_Lean_Polish`): the left and the right lean clip each swept by
/// `RiderLeaning` (the left one backwards: `1 - lean` seconds), crossed by `RiderLean_LeftOrRight`, then crossed to the upright
/// pose by `LeanOrStraight_Alpha`.
fn lean_set(pack: &Pack, left: &str, centre: &str, right: &str) -> Result<Node> {
    let swept = blend(sweep(pack, left, true)?, sweep(pack, right, false)?, Value::param(LEAN_OUT));
    Ok(blend(swept, node(pack, centre, TimeMode::Hold)?, Value::param(STRAIGHT)))
}

/// The supermoto's tree. `riding_style` picks the lean clips: 0 the "knee" set, 1 the plain set, 2 the old-school set (the three
/// branches of `RidingStyle` in `SET Animations`).
pub fn motard_graph(pack: &Pack, riding_style: u8) -> Result<Graph> {
    graph_for(pack, &MOTARD, riding_style)
}

/// The tree of a bike type's clip set for one riding style. `riding_style` is the blueprint's `RidingStyle` (0 to 2; a larger
/// number is 1, the plain style of the sets that have one).
pub fn graph_for(pack: &Pack, set: &ClipSet, riding_style: u8) -> Result<Graph> {
    assemble(pack, set, riding(pack, set, riding_style)?)
}

/// The tree of a bike type's clip set for every riding style: the `riding_style` parameter picks the lean set (the blueprint's
/// `SET Animations` picks it by `RidingStyle`). A type whose styles all share the same clips (the scooter) has one lean set.
pub fn graph_selectable(pack: &Pack, set: &ClipSet) -> Result<Graph> {
    let mut styles: Vec<Node> =
        (0..graph_sets::STYLES as u8).map(|style| riding(pack, set, style)).collect::<Result<_>>()?;
    let alike = styles.iter().all(|node| *node == styles[0]);
    let node = match alike {
        true => styles.swap_remove(0),
        false => Node::Select { options: styles, index: Value::param(RIDING_STYLE) },
    };
    assemble(pack, set, node)
}

/// The rider's sitting pose of one riding style: the low-speed and the high-speed lean sets, crossed by the tuck.
fn riding(pack: &Pack, set: &ClipSet, riding_style: u8) -> Result<Node> {
    let style = usize::from(riding_style).min(graph_sets::STYLES - 1);
    let lean = &set.lean[style];
    // `Leaning_Standard_*` is the low-speed set, `Leaning_Aero_*` the high-speed one; the tuck blends them. The old-school style
    // sets its own centre poses (`Standard_Pose` and `AeroPose_*` of `SET Animations` for `RidingStyle == 2`): its lean clips sit
    // 13 cm further forward than the centre pose of the other two styles, the old-school centre pose with them. A pack that
    // lacks the style's poses (they are ACL clips) uses the first style's.
    let centre = match &set.centre[style] {
        own if has_clip(pack, own.low) && has_clip(pack, own.high) => own,
        _ => &set.centre[0],
    };
    let standard = lean_set(pack, lean.low[0], centre.low, lean.low[1])?;
    let aero = lean_set(pack, lean.high[0], centre.high, lean.high[1])?;
    Ok(blend(standard, aero, Value::remap(TUCK, (0.0, 1.0), (0.0, 1.0))))
}

/// Everything over the sitting pose `riding`: the wheelie, the stoppie, the standing wheelie and the drift, the reverse walk, the
/// standstill, the additive shake and the fingers on the levers.
fn assemble(pack: &Pack, set: &ClipSet, riding: Node) -> Result<Graph> {
    // Sitting wheelie: minimum to maximum pose, left to right by the lean.
    let wheelie_amount = Value::param(WHEELIE_ALPHA);
    let [left_min, left_max, right_min, right_max] = set.wheelie;
    let side =
        |min: &str, max: &str| -> Result<Node> { Ok(blend(looped(pack, min)?, looped(pack, max)?, wheelie_amount)) };
    let wheelie_pose = blend(
        side(left_min, left_max)?,
        side(right_min, right_max)?,
        Value::remap(WHEELIE_DIRECTION, (-1.0, 1.0), (0.0, 1.0)),
    );
    let with_wheelie = blend(riding, wheelie_pose, Value::remap(WHEELIE, WHEELIE_ENTRY_RAMP, (0.0, 1.0)));

    // Stoppie: the clip is stepped by the amount, not played.
    let stoppie_time =
        Value::remap(STOPPIE_TIME, (0.0, 1.0), (0.0, STOPPIE_CLIP_SECONDS / pack_length(pack, set.stoppie)?));
    let stoppie_pose = node(pack, set.stoppie, TimeMode::Fraction(stoppie_time))?;
    let with_stoppie = blend(with_wheelie, stoppie_pose, Value::param(STOPPIE));
    let with_drift = with_standing_and_drift(pack, set, with_stoppie)?;
    let walking = graph_reverse::with_walk(pack, with_drift, &set.walk, REVERSE)?;
    let standing = graph_stand::with_stand(pack, set, walking, STAND)?;

    // Additive shake of the body, low and high speed.
    let shake =
        blend(looped(pack, set.shake[0])?, looped(pack, set.shake[1])?, Value::remap(SPEED, NOISE_SWAP, (0.0, 1.0)));
    let shaken = Node::Additive {
        base: Box::new(standing),
        delta: Box::new(shake),
        weight: Value::remap(SPEED, NOISE_RAMP, (0.0, NOISE_ALPHA)),
    };
    let root = graph_fingers::with_fingers(pack, shaken, BRAKE_FINGERS, FINGERS)?;
    Ok(Graph { param_names: PARAMS.iter().map(|s| (*s).to_owned()).collect(), root })
}

fn has_clip(pack: &Pack, name: &str) -> bool {
    pack.clip_index(name).is_some()
}

/// The standing wheelie over the sitting one and the drift poses over everything, when the set has those clips and the pack
/// has them (they are the ACL ones: a pack made without them keeps the older tree).
///
/// `Wheelie_Standing_Alpha` weighs the blend space `Wheelie_Standing_BS` over the pose so far. The three-way drift blend takes the
/// left pose on the negative side and the right one on the positive: the blueprint's `Drift_L` and `Drift_R` lean to the rider's
/// left and right (the head 12 cm to either side of the centre). A set with only one of the two gets only that one.
fn with_standing_and_drift(pack: &Pack, set: &ClipSet, base: Node) -> Result<Node> {
    let with_standing = match set.standing.filter(|clips| clips.iter().all(|name| has_clip(pack, name))) {
        Some(clips) => standing_over(pack, &clips, base)?,
        None => base,
    };
    let Some(drift) = set.drift.filter(|clips| clips.iter().all(|name| has_clip(pack, name))) else {
        return Ok(with_standing);
    };
    Ok(Node::ThreeWay {
        negative: Box::new(node(pack, drift[0], TimeMode::Hold)?),
        zero: Box::new(with_standing),
        positive: Box::new(node(pack, drift[1], TimeMode::Hold)?),
        param: Value::param(DRIFT),
    })
}

/// The blend space of the standing wheelie weighed over `base` by `Wheelie_Standing_Alpha`.
fn standing_over(pack: &Pack, clips: &[&str; 4], base: Node) -> Result<Node> {
    let index = |name: &str| pack.clip_index(name).with_context(|| format!("the animation {name} is not in the pack"));
    let space = BlendSpace {
        x: Axis { min: STANDING_STEER.0, max: STANDING_STEER.1, divisions: STANDING_STEER.2 },
        y: Axis { min: STANDING_ENTRY_RANGE.0, max: STANDING_ENTRY_RANGE.1, divisions: 1 },
        samples: vec![
            SpaceSample { x: 1, y: 1, clip: index(clips[0])? },
            SpaceSample { x: 0, y: 1, clip: index(clips[1])? },
            SpaceSample { x: 2, y: 1, clip: index(clips[2])? },
            SpaceSample { x: 1, y: 0, clip: index(clips[3])? },
        ],
    };
    let standing_pose = Node::Space {
        space,
        x: Value::param(STEER),
        y: Value::param(STANDING_ENTRY),
        time: TimeMode::Loop { rate: 1.0 },
    };
    Ok(blend(base, standing_pose, Value::param(STANDING)))
}

fn pack_length(pack: &Pack, name: &str) -> Result<f32> {
    let clip = pack.clip_index(name).with_context(|| format!("the animation {name} is not in the pack"))?;
    Ok(pack.clips[clip].length)
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "graph_lean_tests.rs"]
mod lean_tests;
