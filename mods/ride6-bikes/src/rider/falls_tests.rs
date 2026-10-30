use blackbox_skeleton::fall::FallPlayer;
use blackbox_skeleton::{Bone, Channel, Clip, FallInput, Pack, Skeleton, Track, Transform};
use glam::Vec3;

use super::*;

/// The 21 clips of the fall set, each one holding its own number as the x of the one bone's translation, with the lengths the
/// export has for the ones that matter to the timing.
const CLIPS: [(&str, f32); 21] = [
    ("race_ragdoll_Back_GetUp", 1.0),
    ("race_ragdoll_FaceDown_GetUp", 1.0),
    ("race_ragdoll_fall_front_loop_center", 0.967),
    ("race_ragdoll_fall_front_loop_center_L", 0.967),
    ("race_ragdoll_fall_front_loop_center_R", 0.967),
    ("race_ragdoll_HighSide_FaceUp_L", 1.133),
    ("race_ragdoll_HighSide_FaceUp_R", 1.133),
    ("race_ragdoll_HighSide_Front", 1.4),
    ("race_ragdoll_HighSide_High_FaceDown_L", 1.8),
    ("race_ragdoll_HighSide_High_FaceDown_R", 1.8),
    ("race_ragdoll_HighSide_Side_L", 1.133),
    ("race_ragdoll_HighSide_Side_R", 1.133),
    ("race_ragdoll_Rolling_Loop_L", 1.4),
    ("race_ragdoll_Rolling_Loop_R", 1.4),
    ("race_ragdoll_Side_GetUp_L", 2.533),
    ("race_ragdoll_Side_GetUp_R", 2.533),
    ("race_ragdoll_Sliding_FaceDown_Loop_L", 1.4),
    ("race_ragdoll_Sliding_FaceDown_Loop_R", 1.4),
    ("race_ragdoll_Sliding_FaceUp_Loop_L", 2.0),
    ("race_ragdoll_Sliding_FaceUp_Loop_R", 2.0),
    ("ragdoll_lowside_center_R", 0.467),
];

fn pack(without: &[&str]) -> Pack {
    let bone = Bone { name: "root".into(), parent: None, rest: Transform::IDENTITY };
    let clips = CLIPS
        .iter()
        .filter(|(name, _)| !without.contains(name))
        .enumerate()
        .map(|(i, (name, length))| Clip {
            name: (*name).to_owned(),
            length: *length,
            tracks: vec![Track {
                bone: 0,
                rotation: Channel::None,
                translation: Channel::Constant(Vec3::new(100.0 + i as f32, 0.0, 0.0)),
                scale: Channel::None,
            }],
            additive: false,
            root_motion: false,
            notifies: Vec::new(),
        })
        .collect();
    Pack { skeleton: Skeleton::new(vec![bone]).unwrap(), sockets: Vec::new(), clips, meshes: Vec::new() }
}

fn clip_x(pack: &Pack, name: &str) -> f32 {
    100.0 + pack.clip_index(name).unwrap() as f32
}

/// The x of the bone `seconds` into a fall of `kind` with the ragdoll doing `input` the whole time.
fn x_after(pack: &Pack, set: &FallSet, kind: &str, seconds: f32, input: FallInput) -> f32 {
    let mut player = FallPlayer::new(set, kind);
    for _ in 0..(seconds * 60.0).round() as usize {
        player.step(set, &pack.clips, 1.0 / 60.0, input);
    }
    player.pose(set, &pack.skeleton, &pack.clips).local[0].translation.x
}

fn lying(face: f32, side: f32) -> FallInput {
    FallInput { face, side, speed: 5.0, airborne: 0.1 }
}

#[test]
fn the_set_names_every_clip_of_the_fall_and_fits_the_pack() {
    let pack = pack(&[]);
    let set = fall_set(&pack).unwrap();
    set.validate(&pack.clips, &pack.skeleton).unwrap();
    assert_eq!(set.launches.iter().map(|l| l.kind.as_str()).collect::<Vec<_>>(), [HIGH_SIDE, LOW_SIDE]);
    assert_eq!(set.rests.len(), 3);
    // All 21 clips are used by one tree or another.
    let text = serde_json::to_string(&set).unwrap();
    let used = (0..pack.clips.len()).filter(|i| text.contains(&format!("\"clip\":{i},"))).count();
    assert_eq!(used, 21);
    let back: FallSet = serde_json::from_str(&text).unwrap();
    assert_eq!(back, set);
}

#[test]
fn a_pack_without_a_clip_of_the_fall_is_refused_and_names_it() {
    let error = fall_set(&pack(&["race_ragdoll_Back_GetUp"])).unwrap_err().to_string();
    assert!(error.contains("race_ragdoll_Back_GetUp"), "{error}");
    assert!(fall_set(&pack(&["ragdoll_lowside_center_R"])).is_err());
}

#[test]
fn a_low_side_opens_with_the_right_centre_clip_and_a_high_side_with_the_clips_the_blueprint_picks() {
    let pack = pack(&[]);
    let set = fall_set(&pack).unwrap();
    // Low side: the right clip only, whatever the rider's side.
    let low = x_after(&pack, &set, LOW_SIDE, 0.02, lying(0.0, -1.0));
    assert_eq!(low, clip_x(&pack, "ragdoll_lowside_center_R"));
    // High side: `HighSide_Front` with the rider upright, the `_L` clips for a positive side and the `_R` ones for a negative.
    let front = x_after(&pack, &set, HIGH_SIDE, 0.02, lying(0.0, 0.0));
    assert_eq!(front, clip_x(&pack, "race_ragdoll_HighSide_Front"));
    let left = x_after(&pack, &set, HIGH_SIDE, 0.02, lying(0.0, 1.0));
    assert_eq!(left, clip_x(&pack, "race_ragdoll_HighSide_Side_L"));
    let right = x_after(&pack, &set, HIGH_SIDE, 0.02, lying(0.0, -1.0));
    assert_eq!(right, clip_x(&pack, "race_ragdoll_HighSide_Side_R"));
    // The face value picks within the set: positive the `High_FaceDown` clip, negative the `FaceUp` one (the names are the
    // blueprint's; the sign is what its bindings give).
    let positive = x_after(&pack, &set, HIGH_SIDE, 0.02, lying(1.0, 1.0));
    assert_eq!(positive, clip_x(&pack, "race_ragdoll_HighSide_High_FaceDown_L"));
    let negative = x_after(&pack, &set, HIGH_SIDE, 0.02, lying(-1.0, -1.0));
    assert_eq!(negative, clip_x(&pack, "race_ragdoll_HighSide_FaceUp_R"));
}

#[test]
fn the_high_side_hands_over_to_the_tumble_a_blend_before_its_dominant_clip_ends() {
    let pack = pack(&[]);
    let set = fall_set(&pack).unwrap();
    // Face 1, side 1: the dominant clip is `High_FaceDown_L`, 1.8 s long: 1.2 s of launch, then 0.6 s of fade, then the tumble
    // alone (sliding face up; the side blend is at its end, the right clip).
    let input = lying(1.0, 1.0);
    let before = x_after(&pack, &set, HIGH_SIDE, 1.1, input);
    assert_eq!(before, clip_x(&pack, "race_ragdoll_HighSide_High_FaceDown_L"), "still launching");
    let halfway = x_after(&pack, &set, HIGH_SIDE, 1.5, input);
    let (from, to) =
        (clip_x(&pack, "race_ragdoll_HighSide_High_FaceDown_L"), clip_x(&pack, "race_ragdoll_Sliding_FaceUp_Loop_R"));
    assert!(halfway > from.min(to) && halfway < from.max(to), "{halfway} between {from} and {to}");
    let after = x_after(&pack, &set, HIGH_SIDE, 2.0, input);
    assert!((after - to).abs() < 1e-3, "{after} {to}");
}

#[test]
fn the_tumble_is_the_ground_blend_by_face_and_side_and_the_flying_one_when_he_is_high() {
    let pack = pack(&[]);
    let set = fall_set(&pack).unwrap();
    let t = |face: f32, side: f32, airborne: f32| {
        x_after(&pack, &set, "generic", 0.02, FallInput { face, side, speed: 9.0, airborne })
    };
    let x = |name: &str| clip_x(&pack, name);
    // Face down, side -1: the left clip of the face-down slide (alpha 0.5 * -1 + 0.5 = 0). Face up, side +1: the right one.
    assert_eq!(t(-1.0, -1.0, 0.0), x("race_ragdoll_Sliding_FaceDown_Loop_L"));
    assert_eq!(t(1.0, 1.0, 0.0), x("race_ragdoll_Sliding_FaceUp_Loop_R"));
    // On a side with the face neither way: the rolling, left for a negative side, right for a positive (alpha = side).
    assert_eq!(t(0.0, -1.0, 0.0), x("race_ragdoll_Rolling_Loop_L"));
    assert_eq!(t(0.0, 1.0, 0.0), x("race_ragdoll_Rolling_Loop_R"));
    // 2.5 m up and more: the front fall, centre or by side.
    assert_eq!(t(-1.0, 0.0, 3.0), x("race_ragdoll_fall_front_loop_center"));
    assert_eq!(t(-1.0, 1.0, 3.0), x("race_ragdoll_fall_front_loop_center_R"));
    assert_eq!(t(-1.0, -1.0, 3.0), x("race_ragdoll_fall_front_loop_center_L"));
}

#[test]
fn he_gets_up_by_how_he_lies_once_he_is_still() {
    let pack = pack(&[]);
    let set = fall_set(&pack).unwrap();
    let still = |face: f32, side: f32| FallInput { face, side, speed: 0.1, airborne: 0.1 };
    let x = |name: &str| clip_x(&pack, name);
    assert_eq!(x_after(&pack, &set, "generic", 1.5, still(-0.9, 0.0)), x("race_ragdoll_FaceDown_GetUp"));
    assert_eq!(x_after(&pack, &set, "generic", 1.5, still(0.9, 0.0)), x("race_ragdoll_Back_GetUp"));
    // On a side the get-up is chosen by the side: alpha 0.5 * side + 0.5.
    assert_eq!(x_after(&pack, &set, "generic", 0.5, still(0.0, -1.0)), x("race_ragdoll_Side_GetUp_L"));
    assert_eq!(x_after(&pack, &set, "generic", 0.5, still(0.1, 1.0)), x("race_ragdoll_Side_GetUp_R"));
    // Not between the bands, not while moving, not while flying.
    let player = |input: FallInput| {
        let mut player = FallPlayer::new(&set, "generic");
        player.step(&set, &pack.clips, 0.02, input);
        player
    };
    assert!(!player(still(-0.4, 0.0)).resting());
    assert!(!player(FallInput { speed: 0.6, ..still(0.9, 0.0) }).resting());
    assert!(!player(FallInput { airborne: 1.2, ..still(0.9, 0.0) }).resting());
}
