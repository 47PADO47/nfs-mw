//! The blend tree of a supermoto ("motard") rider, built as data for [`blackbox_skeleton`].
//!
//! Every clip name is an asset of the rider animation set (`Animations/motard/`), chosen by the animation blueprint's
//! `SET Animations` function for bike type 2. Every number is a default of that blueprint's class (the variable name is
//! given); `docs/specs/ride6-rider-animation.md` lists what is data and what is a choice.

use anyhow::{Context, Result};
use blackbox_skeleton::{Axis, BlendSpace, Graph, Node, Pack, SpaceSample, TimeMode, Value};

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
pub const PARAMS: [&str; 12] = [
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

/// The tree. `riding_style` picks the lean clips: 0 the "knee" set, 1 the plain set, 2 the old-school set (the three
/// branches of `RidingStyle` in `SET Animations`).
pub fn motard_graph(pack: &Pack, riding_style: u8) -> Result<Graph> {
    let (low_l, low_r, high_l, high_r) = match riding_style {
        0 => (
            "motard_Leaning_Knee_LowSpeed_L",
            "motard_Leaning_Knee_LowSpeed_R",
            "motard_Leaning_Knee_HighSpeed_L",
            "motard_Leaning_Knee_HighSpeed_R",
        ),
        2 => (
            "motard_Old_School_standard_L",
            "motard_Old_School_standard_R",
            "motard_Old_School_aero_L",
            "motard_Old_School_aero_R",
        ),
        _ => (
            "motard_Leaning_LowSpeed_L",
            "motard_Leaning_LowSpeed_R",
            "motard_Leaning_HighSpeed_L",
            "motard_Leaning_HighSpeed_R",
        ),
    };
    // `Leaning_Standard_*` is the low-speed set, `Leaning_Aero_*` the high-speed one; the tuck blends them. The old-school
    // style sets its own centre poses (`Standard_Pose` and `AeroPose_*` of `SET Animations` for `RidingStyle == 2`): its lean
    // clips sit 13 cm further forward than the centre pose of the other two styles, the old-school centre pose with them.
    let (centre_low, centre_high) = match riding_style {
        2 if has_clip(pack, "motard_old_school_standard_pose") => {
            ("motard_old_school_standard_pose", "motard_aero_pose")
        }
        _ => ("motard_CenterPose_LowSpeed", "motard_CenterPose_HighSpeed"),
    };
    let standard = lean_set(pack, low_l, centre_low, low_r)?;
    let aero = lean_set(pack, high_l, centre_high, high_r)?;
    let riding = blend(standard, aero, Value::remap(TUCK, (0.0, 1.0), (0.0, 1.0)));

    // Sitting wheelie: minimum to maximum pose, left to right by the lean.
    let wheelie_amount = Value::param(WHEELIE_ALPHA);
    let side =
        |min: &str, max: &str| -> Result<Node> { Ok(blend(looped(pack, min)?, looped(pack, max)?, wheelie_amount)) };
    let wheelie_pose = blend(
        side("motard_wheelie_min_L", "motard_wheelie_max_L")?,
        side("motard_wheelie_min_R", "motard_wheelie_max_R")?,
        Value::remap(WHEELIE_DIRECTION, (-1.0, 1.0), (0.0, 1.0)),
    );
    let with_wheelie = blend(riding, wheelie_pose, Value::remap(WHEELIE, WHEELIE_ENTRY_RAMP, (0.0, 1.0)));

    // Stoppie: the clip is stepped by the amount, not played.
    let stoppie_time =
        Value::remap(STOPPIE_TIME, (0.0, 1.0), (0.0, STOPPIE_CLIP_SECONDS / pack_length(pack, "motard_stoppie")?));
    let stoppie_pose = node(pack, "motard_stoppie", TimeMode::Fraction(stoppie_time))?;
    let with_stoppie = blend(with_wheelie, stoppie_pose, Value::param(STOPPIE));
    let with_drift = with_standing_and_drift(pack, with_stoppie)?;

    // Additive shake of the body, low and high speed.
    let shake = blend(
        looped(pack, "motard_Shake_LowSpeed")?,
        looped(pack, "motard_Shake_HighSpeed")?,
        Value::remap(SPEED, NOISE_SWAP, (0.0, 1.0)),
    );
    let root = Node::Additive {
        base: Box::new(with_drift),
        delta: Box::new(shake),
        weight: Value::remap(SPEED, NOISE_RAMP, (0.0, NOISE_ALPHA)),
    };
    Ok(Graph { param_names: PARAMS.iter().map(|s| (*s).to_owned()).collect(), root })
}

fn has_clip(pack: &Pack, name: &str) -> bool {
    pack.clip_index(name).is_some()
}

/// The standing wheelie over the sitting one and the drift poses over everything, when the pack has those clips (they are
/// the ACL ones: a pack made without them keeps the older tree).
///
/// `Wheelie_Standing_Alpha` weighs the blend space `motard_Wheelie_Standing_BS` over the pose so far. The three-way drift
/// blend takes the left pose on the negative side and the right one on the positive: the blueprint's `Drift_L` and `Drift_R`
/// lean to the rider's left and right (the head 12 cm to either side of the centre).
fn with_standing_and_drift(pack: &Pack, base: Node) -> Result<Node> {
    let standing = [
        "motard_wheelie_Standing_anim",
        "motard_wheelie_Standing_anim_L",
        "motard_wheelie_Standing_anim_R",
        "motard_wheelie_StandingInterpolation_anim",
    ];
    let drift = ["motard_drift_pose_L", "motard_drift_pose_R"];
    if !standing.iter().chain(&drift).all(|name| has_clip(pack, name)) {
        return Ok(base);
    }
    let index = |name: &str| pack.clip_index(name).with_context(|| format!("the animation {name} is not in the pack"));
    let space = BlendSpace {
        x: Axis { min: STANDING_STEER.0, max: STANDING_STEER.1, divisions: STANDING_STEER.2 },
        y: Axis { min: STANDING_ENTRY_RANGE.0, max: STANDING_ENTRY_RANGE.1, divisions: 1 },
        samples: vec![
            SpaceSample { x: 1, y: 1, clip: index(standing[0])? },
            SpaceSample { x: 0, y: 1, clip: index(standing[1])? },
            SpaceSample { x: 2, y: 1, clip: index(standing[2])? },
            SpaceSample { x: 1, y: 0, clip: index(standing[3])? },
        ],
    };
    let standing_pose = Node::Space {
        space,
        x: Value::param(STEER),
        y: Value::param(STANDING_ENTRY),
        time: TimeMode::Loop { rate: 1.0 },
    };
    let with_standing = blend(base, standing_pose, Value::param(STANDING));
    Ok(Node::ThreeWay {
        negative: Box::new(node(pack, drift[0], TimeMode::Hold)?),
        zero: Box::new(with_standing),
        positive: Box::new(node(pack, drift[1], TimeMode::Hold)?),
        param: Value::param(DRIFT),
    })
}

fn pack_length(pack: &Pack, name: &str) -> Result<f32> {
    let clip = pack.clip_index(name).with_context(|| format!("the animation {name} is not in the pack"))?;
    Ok(pack.clips[clip].length)
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
