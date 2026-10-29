//! The blend tree of a supermoto ("motard") rider, built as data for [`blackbox_skeleton`].
//!
//! Every clip name is an asset of the rider animation set (`Animations/motard/`), chosen by the animation blueprint's
//! `SET Animations` function for bike type 2. Every number is a default of that blueprint's class (the variable name is
//! given); `docs/specs/ride6-rider-animation.md` lists what is data and what is a choice.

use anyhow::{Context, Result};
use blackbox_skeleton::{Graph, Node, Pack, TimeMode, Value};

/// Parameter order of the graph (the names are in [`Graph::param_names`]).
///
/// `lean` is `RiderLeaning`, `tuck` `RiderTuckedIn_Alpha`, `wheelie` the entry of the wheelie pose (`BikeWheelie` held to 8 per
/// second), `wheelie_alpha` `Wheelie_Alpha` (the minimum-to-maximum pose), `wheelie_direction` `Wheelie_Direction`, `stoppie`
/// `Stoppie_Alpha` and `stoppie_time` the stoppie clip's time over 1.27 s (`ride6-rider-round6.md`, section 4). A graph made
/// before the last three existed reads them as 0.
pub const PARAMS: [&str; 8] =
    ["lean", "speed_kmh", "tuck", "wheelie", "stoppie", "wheelie_alpha", "stoppie_time", "wheelie_direction"];
const LEAN: usize = 0;
const SPEED: usize = 1;
const TUCK: usize = 2;
const WHEELIE: usize = 3;
const STOPPIE: usize = 4;
const WHEELIE_ALPHA: usize = 5;
const STOPPIE_TIME: usize = 6;
const WHEELIE_DIRECTION: usize = 7;

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
    // `Leaning_Standard_*` is the low-speed set, `Leaning_Aero_*` the high-speed one; the tuck blends them.
    let standard = lean_set(pack, low_l, "motard_CenterPose_LowSpeed", low_r)?;
    let aero = lean_set(pack, high_l, "motard_CenterPose_HighSpeed", high_r)?;
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

    // Additive shake of the body, low and high speed.
    let shake = blend(
        looped(pack, "motard_Shake_LowSpeed")?,
        looped(pack, "motard_Shake_HighSpeed")?,
        Value::remap(SPEED, NOISE_SWAP, (0.0, 1.0)),
    );
    let root = Node::Additive {
        base: Box::new(with_stoppie),
        delta: Box::new(shake),
        weight: Value::remap(SPEED, NOISE_RAMP, (0.0, NOISE_ALPHA)),
    };
    Ok(Graph { param_names: PARAMS.iter().map(|s| (*s).to_owned()).collect(), root })
}

fn pack_length(pack: &Pack, name: &str) -> Result<f32> {
    let clip = pack.clip_index(name).with_context(|| format!("the animation {name} is not in the pack"))?;
    Ok(pack.clips[clip].length)
}
