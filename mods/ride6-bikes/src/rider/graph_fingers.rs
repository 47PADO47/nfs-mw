//! The fingers on the levers in the rider's blend tree: the left hand's fingers pull the clutch lever, the right hand's pull the
//! front brake lever (the blueprint's `Hands_FingersOverride`, `docs/specs/ride6-rider-hands.md`).
//!
//! The blueprint lays a short finger clip over the riding pose on the finger bones of one hand (a `LayeredBoneBlend` whose
//! branch starts at the first bones of the fingers) and steps it by an explicit time: `race_clutch_brake_gear` for the clutch,
//! the brake clip of the rider's number of braking fingers for the front brake, over the first 0.27 s (clutch) and 0.28 s (brake) of
//! the clip. The weight is `Fingers_Clutch_Alpha` and `FrontFingers_Brake_Alpha`: 1 once the lever moves, smoothed.
//! The graph reads six parameters, all made by the game's rider runtime: `clutch` and `brake` (the lever, 0 to 1), `clutch_alpha`
//! and `brake_alpha` (the layers' weights), `brake_fingers` (which brake clip: 0 one finger, 1 two, 2 four; the player's
//! `brake_fingers` setting, the blueprint's `BrakeFingers` of the riding style page) and `clutch_fingers` (0 the clutch clip, which
//! pulls with the index and middle fingers, 1 the left hand of the four-finger clip, which pulls with all four; the player's
//! `clutch_fingers` setting, ours: RIDE 6 has no such option and always plays the clutch clip).

use anyhow::Result;
use blackbox_skeleton::{Node, Pack, Skeleton, TimeMode, Value};

/// The clutch clip (`Fingers_Clutch`) and the brake clips by the number of fingers that brake (`Fingers_Brake`; the blueprint's
/// `BrakeFingers` is 1, 2 or 4, anything else plays the two-finger clip). The graph's `brake_fingers` parameter is the index into
/// this table.
pub const CLUTCH_CLIP: &str = "race_clutch_brake_gear";
pub const BRAKE_CLIPS: [(u8, &str); 3] = [
    (1, "race_clutch_brake_gear_OneFinger"),
    (2, "race_clutch_brake_gear_TwoFingers"),
    (4, "race_clutch_brake_gear_FourFingers"),
];
/// The seconds of the finger clips that play, from lever let go to pulled in. `Fingers_Clutch_Explicit` of `Hands_FingersOverride`
/// goes from 0 to 0.27 (0.24 for a woman's rider: the target of a pulled `VehicleClutchRemap`); `Fingers_FrontBrake_Explicit` is the
/// brake lever's weight times 0.28 (`DefineFloatRuleEx(.., 0.28)`).
pub const CLUTCH_SECONDS: (f32, f32) = (0.0, 0.27);
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
    /// Which brake clip plays: the index into [`BRAKE_CLIPS`].
    pub brake_fingers: usize,
    /// Which clutch clip plays: 0 [`CLUTCH_CLIP`], 1 the left hand of the four-finger clip.
    pub clutch_fingers: usize,
}

/// 1 for every bone below `hand` (its fingers, not the hand itself), 0 for the rest.
pub fn finger_mask(skeleton: &Skeleton, hand: &str) -> Option<Vec<f32>> {
    let hand = skeleton.find(hand)?;
    let mask: Vec<f32> =
        (0..skeleton.len()).map(|bone| f32::from(bone != hand && skeleton.is_descendant(bone, hand))).collect();
    mask.iter().any(|m| *m > 0.0).then_some(mask)
}

/// The finger clip `clip` stepped from `seconds.0` to `seconds.1` by the lever; `None` for a clip the pack lacks.
fn stepped(pack: &Pack, clip: &str, seconds: (f32, f32), lever: usize) -> Option<Node> {
    let index = pack.clip_index(clip)?;
    let length = pack.clips[index].length.max(f32::EPSILON);
    // `Fraction` is the clip's length times the value: the explicit time over the length, held at the clip's ends.
    let time = Value::remap(lever, (0.0, 1.0), (seconds.0 / length, seconds.1 / length));
    Some(Node::Clip { clip: index, time: TimeMode::Fraction(time) })
}

/// `fingers` over `base` on the finger bones of `hand`, as heavy as `alpha`; no layer for a hand the pack has no bones for.
fn layer(pack: &Pack, base: Node, fingers: Node, hand: &str, alpha: usize) -> Node {
    let Some(mask) = finger_mask(&pack.skeleton, hand) else { return base };
    Node::Layer { base: Box::new(base), layer: Box::new(fingers), mask, weight: Value::param(alpha) }
}

/// The brake fingers: the clip of each number of fingers, chosen by the `brake_fingers` parameter. A clip the pack lacks plays the
/// two-finger one in its place; a pack without the two-finger clip has no brake fingers.
fn brake_fingers(pack: &Pack, lever: usize, fingers: usize) -> Option<Node> {
    let two = stepped(pack, BRAKE_CLIPS[1].1, BRAKE_SECONDS, lever)?;
    let options: Vec<Node> = BRAKE_CLIPS
        .iter()
        .map(|(_, clip)| stepped(pack, clip, BRAKE_SECONDS, lever).unwrap_or_else(|| two.clone()))
        .collect();
    match options.iter().all(|option| *option == two) {
        true => Some(two),
        false => Some(Node::Select { options, index: Value::param(fingers) }),
    }
}

/// The clutch fingers: the clutch clip (index and middle) or, by the `clutch_fingers` parameter, the four-finger clip, whose left hand
/// pulls with all four. A pack without the four-finger clip has the clutch clip alone.
fn clutch_fingers(pack: &Pack, lever: usize, fingers: usize) -> Option<Node> {
    let two = stepped(pack, CLUTCH_CLIP, CLUTCH_SECONDS, lever)?;
    let Some(four) = stepped(pack, BRAKE_CLIPS[2].1, CLUTCH_SECONDS, lever) else { return Some(two) };
    Some(Node::Select { options: vec![two, four], index: Value::param(fingers) })
}

/// `base` with the clutch fingers on the left hand (two or four of them, by the `clutch_fingers` parameter) and the brake fingers
/// (one, two or four, by the `brake_fingers` parameter) on the right. A pack that lacks a clip or a bone leaves that hand alone.
pub fn with_fingers(pack: &Pack, base: Node, params: Params) -> Result<Node> {
    let clutched = match clutch_fingers(pack, params.clutch, params.clutch_fingers) {
        Some(fingers) => layer(pack, base, fingers, LEFT_HAND, params.clutch_alpha),
        None => base,
    };
    Ok(match brake_fingers(pack, params.brake, params.brake_fingers) {
        Some(fingers) => layer(pack, clutched, fingers, RIGHT_HAND, params.brake_alpha),
        None => clutched,
    })
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
        let clips = vec![
            clip(CLUTCH_CLIP, 10.0),
            clip(BRAKE_CLIPS[1].1, 20.0),
            clip(BRAKE_CLIPS[0].1, 30.0),
            clip(BRAKE_CLIPS[2].1, 40.0),
        ];
        Pack { skeleton: Skeleton::new(bones).unwrap(), sockets: Vec::new(), clips, meshes: Vec::new() }
    }

    const PARAMS: Params =
        Params { clutch: 0, clutch_alpha: 1, brake: 2, brake_alpha: 3, brake_fingers: 4, clutch_fingers: 5 };

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
        let base = Node::Clip { clip: 4, time: TimeMode::Hold };
        let node = with_fingers(&pack, base, PARAMS).unwrap();
        let at = |params: [f32; 6]| {
            let ctx = EvalContext { skeleton: &pack.skeleton, clips: &pack.clips, params: &params, time: 0.0 };
            node.evaluate(&ctx).local.iter().map(|t| t.translation.x).collect::<Vec<_>>()
        };
        // The two-finger clip is the middle one: the parameter is 1.
        assert_eq!(at([0.0, 0.0, 0.0, 0.0, 1.0, 0.0]), vec![1.0; 5], "the layers are off");
        assert_eq!(
            at([1.0, 1.0, 0.0, 0.0, 1.0, 0.0]),
            vec![1.0, 1.0, 10.0, 1.0, 1.0],
            "the clutch fingers are the left hand's"
        );
        assert_eq!(
            at([0.0, 0.0, 1.0, 1.0, 1.0, 0.0]),
            vec![1.0, 1.0, 1.0, 1.0, 20.0],
            "the brake fingers are the right hand's"
        );
        // One finger and four fingers are the other two clips.
        assert_eq!(at([0.0, 0.0, 1.0, 1.0, 0.0, 0.0])[4], 30.0);
        assert_eq!(at([0.0, 0.0, 1.0, 1.0, 2.0, 0.0])[4], 40.0);
        // The clutch plays the clutch clip, or the left hand of the four-finger one.
        assert_eq!(at([1.0, 1.0, 0.0, 0.0, 1.0, 1.0])[2], 40.0);
        assert_eq!(at([1.0, 1.0, 0.0, 0.0, 1.0, 1.0])[4], 1.0, "the right hand stays put");
    }

    #[test]
    fn the_clutch_clip_plays_to_its_pulled_second_and_the_brake_clip_to_its_own() {
        let pack = pack();
        let Some(Node::Clip { time: TimeMode::Fraction(time), .. }) = stepped(&pack, CLUTCH_CLIP, CLUTCH_SECONDS, 0)
        else {
            panic!("the clutch clip is stepped")
        };
        // The pulled lever is 0.27 s of the clip (0.6 s long here).
        assert!((time.eval(&[1.0]) - 0.27 / 0.6).abs() < 1e-6 && time.eval(&[0.0]) == 0.0);
        assert!((CLUTCH_SECONDS.1 - 0.27).abs() < 1e-6 && (BRAKE_SECONDS.1 - 0.28).abs() < 1e-6);
    }

    #[test]
    fn a_pack_that_lacks_a_clip_leaves_that_hand_alone_and_a_missing_count_plays_two_fingers() {
        let mut pack = pack();
        let base = Node::Clip { clip: 0, time: TimeMode::Hold };
        // The one- and four-finger brake clips are not in this pack: the two-finger one stands for them, with no choice to make.
        pack.clips.truncate(2);
        let node = with_fingers(&pack, base.clone(), PARAMS).unwrap();
        let Node::Layer { base: under, layer, .. } = &node else { panic!("the brake layer is on top: {node:?}") };
        assert!(matches!(**layer, Node::Clip { .. }), "no Select over one clip: {layer:?}");
        assert!(matches!(**under, Node::Layer { .. }), "the clutch layer is under it");
        // Without the two-finger clip there are no brake fingers, and without the clutch clip no clutch fingers.
        pack.clips.truncate(1);
        let clutch_only = with_fingers(&pack, base.clone(), PARAMS).unwrap();
        let Node::Layer { base: inner, .. } = &clutch_only else { panic!("{clutch_only:?}") };
        assert_eq!(**inner, base);
        let empty = Pack { clips: Vec::new(), ..pack };
        assert_eq!(with_fingers(&empty, base.clone(), PARAMS).unwrap(), base);
    }
}
