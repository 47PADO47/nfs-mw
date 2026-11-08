//! The fingers on the levers in the rider's blend tree: the left hand's fingers pull the clutch lever, the right hand's pull the
//! front brake lever (the blueprint's `Hands_FingersOverride`, `docs/specs/ride6-rider-hands.md`).
//!
//! The blueprint lays a short finger clip over the riding pose on the finger bones of one hand (a `LayeredBoneBlend` whose
//! branch starts at the first bones of the fingers) and steps it by an explicit time: `race_clutch_brake_gear` for the clutch,
//! the brake clip of the rider's number of braking fingers for the front brake, each over the first 0.28 s of the clip. The weight is `Fingers_Clutch_Alpha` and `FrontFingers_Brake_Alpha`: 1 once the lever moves, smoothed.
//! The graph reads four parameters, all made by the game's rider runtime: `clutch` and `brake` (the lever, 0 to 1) and
//! `clutch_alpha` and `brake_alpha` (the layers' weights).

use anyhow::Result;
use blackbox_skeleton::{Node, Pack, Skeleton, TimeMode, Value};

/// The clutch clip (`Fingers_Clutch`) and the brake clips by the number of fingers that brake (`Fingers_Brake`; the blueprint's
/// `BrakeFingers` is 1, 2 or 4, anything else plays the two-finger clip).
pub const CLUTCH_CLIP: &str = "race_clutch_brake_gear";
pub const BRAKE_CLIPS: [(u8, &str); 3] = [
    (1, "race_clutch_brake_gear_OneFinger"),
    (2, "race_clutch_brake_gear_TwoFingers"),
    (4, "race_clutch_brake_gear_FourFingers"),
];
/// The seconds of the finger clips that play, from lever let go to pulled in: `Fingers_Clutch_Explicit` and
/// `Fingers_FrontBrake_Explicit` of `Hands_FingersOverride` go from 0 to 0.28 (`DefineFloatRuleEx(.., 0.28)`), the lever's weight
/// times 0.28 s. The clutch's own targets are 0.27 and 0.24 (a woman's rider) for a fully pulled lever; the 0.28 is the brake's.
pub const CLUTCH_SECONDS: (f32, f32) = (0.0, 0.28);
pub const BRAKE_SECONDS: (f32, f32) = (0.0, 0.28);
/// The hand bones whose fingers the layers move: the clutch is the left hand's, the front brake the right hand's.
pub const LEFT_HAND: &str = "hand_l";
pub const RIGHT_HAND: &str = "hand_r";

/// Parameter indices of the fingers, as `graph.rs` orders them.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub clutch: usize,
    pub clutch_alpha: usize,
    pub brake: usize,
    pub brake_alpha: usize,
}

/// 1 for every bone below `hand` (its fingers, not the hand itself), 0 for the rest.
pub fn finger_mask(skeleton: &Skeleton, hand: &str) -> Option<Vec<f32>> {
    let hand = skeleton.find(hand)?;
    let mask: Vec<f32> =
        (0..skeleton.len()).map(|bone| f32::from(bone != hand && skeleton.is_descendant(bone, hand))).collect();
    mask.iter().any(|m| *m > 0.0).then_some(mask)
}

/// One finger layer over `base`: `clip` at `seconds.0 + lever * (seconds.1 - seconds.0)` on the fingers of `hand`, as heavy as
/// `alpha`.
fn layer(pack: &Pack, base: Node, clip: &str, hand: &str, seconds: (f32, f32), (lever, alpha): (usize, usize)) -> Node {
    let (Some(index), Some(mask)) = (pack.clip_index(clip), finger_mask(&pack.skeleton, hand)) else { return base };
    let length = pack.clips[index].length.max(f32::EPSILON);
    // `Fraction` is the clip's length times the value: the explicit time over the length, held at the clip's ends.
    let time = Value::remap(lever, (0.0, 1.0), (seconds.0 / length, seconds.1 / length));
    let fingers = Node::Clip { clip: index, time: TimeMode::Fraction(time) };
    Node::Layer { base: Box::new(base), layer: Box::new(fingers), mask, weight: Value::param(alpha) }
}

/// `base` with the clutch fingers on the left hand and the brake fingers (`fingers` of them: 1, 2 or 4) on the right. A pack
/// that lacks a clip or a bone leaves that hand alone.
pub fn with_fingers(pack: &Pack, base: Node, fingers: u8, params: Params) -> Result<Node> {
    let brake = BRAKE_CLIPS.iter().find(|(count, _)| *count == fingers).map_or(BRAKE_CLIPS[1].1, |(_, name)| name);
    let clutched = layer(pack, base, CLUTCH_CLIP, LEFT_HAND, CLUTCH_SECONDS, (params.clutch, params.clutch_alpha));
    Ok(layer(pack, clutched, brake, RIGHT_HAND, BRAKE_SECONDS, (params.brake, params.brake_alpha)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_skeleton::{Bone, Channel, Clip, EvalContext, Track, Transform};
    use glam::Vec3;

    /// A rider with a root, two hands and a finger on each, and a clip per finger layer that moves every bone's x.
    fn pack() -> Pack {
        let bone = |name: &str, parent: Option<usize>| Bone { name: name.into(), parent, rest: Transform::IDENTITY };
        let bones = vec![
            bone("root", None),
            bone(LEFT_HAND, Some(0)),
            bone("index_01_l", Some(1)),
            bone(RIGHT_HAND, Some(0)),
            bone("index_01_r", Some(3)),
        ];
        let clip = |name: &str, x: f32| Clip {
            name: name.to_owned(),
            length: 0.6,
            tracks: (0..5)
                .map(|bone| Track {
                    bone,
                    rotation: Channel::None,
                    translation: Channel::Constant(Vec3::new(x, 0.0, 0.0)),
                    scale: Channel::None,
                })
                .collect(),
            additive: false,
            root_motion: false,
            notifies: Vec::new(),
        };
        let clips = vec![clip(CLUTCH_CLIP, 10.0), clip(BRAKE_CLIPS[1].1, 20.0)];
        Pack { skeleton: Skeleton::new(bones).unwrap(), sockets: Vec::new(), clips, meshes: Vec::new() }
    }

    const PARAMS: Params = Params { clutch: 0, clutch_alpha: 1, brake: 2, brake_alpha: 3 };

    #[test]
    fn the_mask_is_the_fingers_of_one_hand_and_nothing_else() {
        let pack = pack();
        assert_eq!(finger_mask(&pack.skeleton, LEFT_HAND), Some(vec![0.0, 0.0, 1.0, 0.0, 0.0]));
        assert_eq!(finger_mask(&pack.skeleton, RIGHT_HAND), Some(vec![0.0, 0.0, 0.0, 0.0, 1.0]));
        assert_eq!(finger_mask(&pack.skeleton, "hand_x"), None);
    }

    #[test]
    fn the_layers_move_only_the_fingers_of_their_hand_and_only_when_their_weight_is_up() {
        let mut pack = pack();
        // The riding pose under the layers: a clip with the rest of the bones at x = 1.
        pack.clips.push(Clip {
            name: "riding".into(),
            length: 1.0,
            tracks: (0..5)
                .map(|bone| Track {
                    bone,
                    rotation: Channel::None,
                    translation: Channel::Constant(Vec3::new(1.0, 0.0, 0.0)),
                    scale: Channel::None,
                })
                .collect(),
            additive: false,
            root_motion: false,
            notifies: Vec::new(),
        });
        let base = Node::Clip { clip: 2, time: TimeMode::Hold };
        let node = with_fingers(&pack, base, 2, PARAMS).unwrap();
        let at = |params: [f32; 4]| {
            let ctx = EvalContext { skeleton: &pack.skeleton, clips: &pack.clips, params: &params, time: 0.0 };
            node.evaluate(&ctx).local.iter().map(|t| t.translation.x).collect::<Vec<_>>()
        };
        assert_eq!(at([0.0, 0.0, 0.0, 0.0]), vec![1.0; 5], "the layers are off");
        assert_eq!(at([1.0, 1.0, 0.0, 0.0]), vec![1.0, 1.0, 10.0, 1.0, 1.0], "the clutch fingers are the left hand's");
        assert_eq!(at([0.0, 0.0, 1.0, 1.0]), vec![1.0, 1.0, 1.0, 1.0, 20.0], "the brake fingers are the right hand's");
    }

    #[test]
    fn a_pack_that_lacks_a_clip_leaves_that_hand_alone() {
        let pack = pack();
        let base = Node::Clip { clip: 0, time: TimeMode::Hold };
        // The four-finger brake clip is not in this pack: only the clutch layer is.
        let four = with_fingers(&pack, base.clone(), 4, PARAMS).unwrap();
        let Node::Layer { base: inner, .. } = &four else { panic!("the clutch layer is on top: {four:?}") };
        assert_eq!(**inner, base);
        // Any other number of fingers plays the two-finger clip: a brake layer over the clutch one.
        let three = with_fingers(&pack, base.clone(), 3, PARAMS).unwrap();
        let Node::Layer { base: under, .. } = &three else { panic!("{three:?}") };
        assert!(matches!(**under, Node::Layer { .. }));
        let empty = Pack { clips: Vec::new(), ..pack };
        assert_eq!(with_fingers(&empty, base.clone(), 2, PARAMS).unwrap(), base);
    }
}
