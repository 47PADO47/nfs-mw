//! The rider's fall and get-up animation as data for [`blackbox_skeleton::fall`]: which clip plays when, as the rider blueprint's
//! animation graph chooses them (`docs/specs/ride6-rider-fall-animation.md`).
//!
//! Every clip is an asset of `riderShared/Animations/Ragdoll/`. The trees are those of the animation blueprint
//! `RiderGP_racing_Master_animBlueprint`: the state machine `Falling` (states `Side`, `Falling`, `FaceDown_end`, `FaceUp_end`,
//! `Side_end`) with its sub-machine `Fall_Side` (states `Dispatcher`, `HighSide`, `LowSide`). The blends are the property bindings
//! of that graph (`RagdollFace`, `RagdollSide`, `Ragdoll_FlyingAmount` into the `Alpha` of each blend node), the thresholds the
//! transition rules of its ubergraph, the crossfades the durations of its baked transitions.

use anyhow::{Context, Result};
use blackbox_skeleton::fall::{Launch, PARAM_FACE, PARAM_FLYING, PARAM_SIDE, PARAM_TIME, Rest};
use blackbox_skeleton::{FallSet, Graph, Node, Pack, TimeMode, Value};

/// The names of the kinds of fall the launches are for (`ERiderFallTypeEnum`: `VE_HighSide` 2, `VE_LowSide` 3; the game maps its
/// own kinds to them). The generic fall (1) has none: its `Dispatcher` state shows a one-frame standing pose, which the machine
/// leaves at once, so it starts in the tumble.
pub const HIGH_SIDE: &str = "high_side";
pub const LOW_SIDE: &str = "low_side";

/// Parameter order of every tree.
const PARAMS: [&str; 4] = [PARAM_TIME, PARAM_FACE, PARAM_SIDE, PARAM_FLYING];
const FACE: usize = 1;
const SIDE: usize = 2;
const FLYING: usize = 3;

/// `Falling` state machine, transition `Side` to `Falling`: `bAutomaticRemainingTimeRule` with a crossfade of 0.6 s.
const LAUNCH_BLEND: f32 = 0.6;
/// The rules of the three transitions out of `Falling`: `Ragdoll_isLowSpeed` is `RagdollSpeed < 0.5` (m/s: the pelvis velocity in
/// cm/s times 0.01), `Ragdoll_isFlying` is `AirborneDistance > 1`.
const LOW_SPEED: f32 = 0.5;
const FLYING_ABOVE: f32 = 1.0;
/// `Ragdoll_FlyingAmount` is `FuzzifyRamp(AirborneDistance, 1, 2.5)`.
const FLYING_RAMP: (f32, f32) = (1.0, 2.5);
/// `IsFallen_Alpha` is `SmoothFloat_Interpolate(IsFallen, 3)`: the weight of the fall pose over the riding pose. The smoothing is
/// read as a first-order approach at this rate (1/s); the library function was not traced.
const FADE_RATE: f32 = 3.0;
/// The face value from which the rider lies face down or face up (`Falling` to `FaceDown_end`: `RagdollFace < -0.6`; to
/// `FaceUp_end`: `RagdollFace > 0.6`) and the band of a side (`-0.2 < RagdollFace < 0.2`). `RagdollFace` is within -1..1.
const FACE_DOWN: f32 = -0.6;
const FACE_UP: f32 = 0.6;
const SIDE_BAND: f32 = 0.2;
/// The crossfades into the get-ups: `Falling` to `FaceDown_end` and `FaceUp_end` 1.0 s, to `Side_end` 0.2 s.
const GET_UP_BLEND: f32 = 1.0;
const GET_UP_SIDE_BLEND: f32 = 0.2;

fn clip(pack: &Pack, name: &str, time: TimeMode) -> Result<Box<Node>> {
    let index = pack.clip_index(name).with_context(|| format!("the animation {name} is not in the pack"))?;
    Ok(Box::new(Node::Clip { clip: index, time }))
}

fn once(pack: &Pack, name: &str) -> Result<Box<Node>> {
    clip(pack, name, TimeMode::Once { rate: 1.0 })
}

fn looped(pack: &Pack, name: &str) -> Result<Box<Node>> {
    clip(pack, name, TimeMode::Loop { rate: 1.0 })
}

fn graph(root: Node) -> Graph {
    Graph { param_names: PARAMS.iter().map(|n| (*n).to_owned()).collect(), root }
}

/// `AlphaScaleBias` of a two-way blend: the alpha is `scale * value + bias`, held to 0..1 (the default of these nodes is scale 1,
/// bias 0; the side blends of the sliding and the get-up use 0.5 and 0.5, so that -1..1 covers the whole blend).
fn scaled(index: usize, scale: f32, bias: f32) -> Value {
    Value::Param { index, scale, bias, min: 0.0, max: 1.0 }
}

fn blend(a: Box<Node>, b: Box<Node>, weight: Value) -> Box<Node> {
    Box::new(Node::Blend { a, b, weight })
}

/// A left and a right clip by the side the rider lies on (`TwoWayBlend`, alpha `0.5 RagdollSide + 0.5`).
fn by_side(a: Box<Node>, b: Box<Node>) -> Box<Node> {
    blend(a, b, scaled(SIDE, 0.5, 0.5))
}

/// `Dispatcher` to `HighSide`: the seven clips of `HighSide`, `ThreeWayBlend_21` on `RagdollSide` over two sets of three on
/// `RagdollFace` (`ThreeWayBlend_20`, the `_L` clips, in front; `ThreeWayBlend_19`, the `_R` clips, behind) and `HighSide_Front` in
/// the middle.
fn high_side(pack: &Pack) -> Result<Node> {
    let face_set = |face_down: &str, side: &str, face_up: &str| -> Result<Box<Node>> {
        Ok(Box::new(Node::ThreeWay {
            negative: once(pack, face_up)?,
            zero: once(pack, side)?,
            positive: once(pack, face_down)?,
            param: Value::param(FACE),
        }))
    };
    Ok(Node::ThreeWay {
        negative: face_set(
            "race_ragdoll_HighSide_High_FaceDown_R",
            "race_ragdoll_HighSide_Side_R",
            "race_ragdoll_HighSide_FaceUp_R",
        )?,
        zero: once(pack, "race_ragdoll_HighSide_Front")?,
        positive: face_set(
            "race_ragdoll_HighSide_High_FaceDown_L",
            "race_ragdoll_HighSide_Side_L",
            "race_ragdoll_HighSide_FaceUp_L",
        )?,
        param: Value::param(SIDE),
    })
}

/// `Falling`: `TwoWayBlend_43` on `Ragdoll_FlyingAmount` between the ground blend (`ThreeWayBlend_23` on `RagdollFace`: sliding face
/// up, rolling, sliding face down, each a left and a right clip by `RagdollSide`) and the flying one (`ThreeWayBlend_22` on
/// `RagdollSide`: the front fall loops).
fn tumble(pack: &Pack) -> Result<Node> {
    let ground = Node::ThreeWay {
        negative: by_side(
            looped(pack, "race_ragdoll_Sliding_FaceDown_Loop_L")?,
            looped(pack, "race_ragdoll_Sliding_FaceDown_Loop_R")?,
        ),
        // `TwoWayBlend_42` has scale 1 and bias 0: the rolling takes the right clip only for a positive side.
        zero: blend(
            looped(pack, "race_ragdoll_Rolling_Loop_L")?,
            looped(pack, "race_ragdoll_Rolling_Loop_R")?,
            scaled(SIDE, 1.0, 0.0),
        ),
        positive: by_side(
            looped(pack, "race_ragdoll_Sliding_FaceUp_Loop_L")?,
            looped(pack, "race_ragdoll_Sliding_FaceUp_Loop_R")?,
        ),
        param: Value::param(FACE),
    };
    let flying = Node::ThreeWay {
        negative: looped(pack, "race_ragdoll_fall_front_loop_center_L")?,
        zero: looped(pack, "race_ragdoll_fall_front_loop_center")?,
        positive: looped(pack, "race_ragdoll_fall_front_loop_center_R")?,
        param: Value::param(SIDE),
    };
    Ok(Node::Blend { a: Box::new(ground), b: Box::new(flying), weight: Value::param(FLYING) })
}

/// The data of the rider's fall animation for the clips of `pack`.
pub fn fall_set(pack: &Pack) -> Result<FallSet> {
    // `LowSide`: `TwoWayBlend_41` between `ragdoll_lowside_center_L` and `_R` with its alpha fixed at 1: the right clip only.
    let low_side = Node::Clip {
        clip: pack
            .clip_index("ragdoll_lowside_center_R")
            .context("the animation ragdoll_lowside_center_R is not in the pack")?,
        time: TimeMode::Once { rate: 1.0 },
    };
    let side_get_up = Node::Blend {
        a: once(pack, "race_ragdoll_Side_GetUp_L")?,
        b: once(pack, "race_ragdoll_Side_GetUp_R")?,
        weight: scaled(SIDE, 0.5, 0.5),
    };
    Ok(FallSet {
        launches: vec![
            Launch { kind: HIGH_SIDE.to_owned(), graph: graph(high_side(pack)?) },
            Launch { kind: LOW_SIDE.to_owned(), graph: graph(low_side) },
        ],
        launch_blend: LAUNCH_BLEND,
        tumble: graph(tumble(pack)?),
        // The transitions of `Falling` in the order of the machine: `FaceDown_end`, `Side_end`, `FaceUp_end`.
        rests: vec![
            Rest {
                face_above: -2.0,
                face_below: FACE_DOWN,
                graph: graph(*once(pack, "race_ragdoll_FaceDown_GetUp")?),
                blend: GET_UP_BLEND,
            },
            Rest { face_above: -SIDE_BAND, face_below: SIDE_BAND, graph: graph(side_get_up), blend: GET_UP_SIDE_BLEND },
            Rest {
                face_above: FACE_UP,
                face_below: 2.0,
                graph: graph(*once(pack, "race_ragdoll_Back_GetUp")?),
                blend: GET_UP_BLEND,
            },
        ],
        low_speed: LOW_SPEED,
        flying_above: FLYING_ABOVE,
        flying_ramp: FLYING_RAMP,
        fade_rate: FADE_RATE,
    })
}

#[cfg(test)]
#[path = "falls_tests.rs"]
mod tests;
