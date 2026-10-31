//! The blend tree of a rider, built as data for [`blackbox_skeleton`], for any of the bike types of the animation blueprint
//! ([`graph_sets`]: race, naked, motard, scooter, enduro).
//!
//! Every clip name is an asset of the rider animation set (`Animations/<type>/`), chosen by the animation blueprint's
//! `SET Animations` function for the bike type. Every number is a default of that blueprint's class (the variable name is
//! given); `docs/specs/ride6-rider-animation.md` lists what is data and what is a choice, `ride6-rider-styles.md` how the
//! type is picked.

use anyhow::{Context, Result};
use blackbox_skeleton::{Axis, BlendSpace, Graph, Node, Pack, SpaceSample, TimeMode, Value};

use super::graph_reverse;
use super::graph_sets::{self, ClipSet, MOTARD};

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
pub const PARAMS: [&str; 16] = [
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

/// Left, centre, right by the signed lean.
fn lean_set(pack: &Pack, left: &str, centre: &str, right: &str) -> Result<Node> {
    Ok(Node::ThreeWay {
        negative: Box::new(looped(pack, left)?),
        zero: Box::new(node(pack, centre, TimeMode::Hold)?),
        positive: Box::new(looped(pack, right)?),
        param: Value::param(LEAN),
    })
}

/// The supermoto's tree. `riding_style` picks the lean clips: 0 the "knee" set, 1 the plain set, 2 the old-school set (the three
/// branches of `RidingStyle` in `SET Animations`).
pub fn motard_graph(pack: &Pack, riding_style: u8) -> Result<Graph> {
    graph_for(pack, &MOTARD, riding_style)
}

/// The tree of a bike type's clip set. `riding_style` is the blueprint's `RidingStyle` (0 to 2; a larger number is 1, the plain
/// style of the sets that have one).
pub fn graph_for(pack: &Pack, set: &ClipSet, riding_style: u8) -> Result<Graph> {
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
    let riding = blend(standard, aero, Value::remap(TUCK, (0.0, 1.0), (0.0, 1.0)));

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

    // Additive shake of the body, low and high speed.
    let shake =
        blend(looped(pack, set.shake[0])?, looped(pack, set.shake[1])?, Value::remap(SPEED, NOISE_SWAP, (0.0, 1.0)));
    let root = Node::Additive {
        base: Box::new(walking),
        delta: Box::new(shake),
        weight: Value::remap(SPEED, NOISE_RAMP, (0.0, NOISE_ALPHA)),
    };
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
