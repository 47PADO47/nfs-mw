//! The characters' skeleton and clips of the real install. Needs `CHAMELEON_GAME_DIR` (or `MW2012_GAME_DIR`) and `--ignored`.

use chameleon_bundle::{Bundle, ResourceKind};

use crate::{AnimationList, Pose, Skeleton, bone_hash};

fn characters() -> Option<Vec<u8>> {
    let dir = std::env::var_os("CHAMELEON_GAME_DIR").or_else(|| std::env::var_os("MW2012_GAME_DIR"))?;
    std::fs::read(std::path::Path::new(&dir).join("CHARACTERS/ALL_CHARS.BNDL")).ok()
}

fn load(bytes: &[u8]) -> (Skeleton, AnimationList) {
    let bundle = Bundle::parse(bytes).unwrap();
    let find = |kind: ResourceKind| bundle.entries().iter().position(|e| e.kind == kind.id()).unwrap();
    let skeleton = Skeleton::parse(&bundle.resource(find(ResourceKind::Skeleton)).unwrap().blocks[0]).unwrap();
    let list = AnimationList::parse(&bundle.resource(find(ResourceKind::AnimationList)).unwrap().blocks[0]).unwrap();
    (skeleton, list)
}

/// The skeleton is the 46-bone tree whose bones the game names (`Root`, the belt bones, `Hips`, `Head`, both hands), and the
/// list holds 28 clips at 30 keys a second.
#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR or MW2012_GAME_DIR)"]
fn the_characters_skeleton_and_clips_read() {
    let Some(bytes) = characters() else { return };
    let (skeleton, list) = load(&bytes);
    assert_eq!(skeleton.bones.len(), 46);
    for name in [
        "Root",
        "SeatbeltBottomRight",
        "SeatbeltBottomLeft",
        "SeatbeltTop",
        "HarnessTopRight",
        "Hips",
        "Neck",
        "Head",
        "LeftArm",
        "LeftForeArm",
        "LeftHand",
        "RightArm",
        "RightForeArm",
        "RightHand",
        "LeftFoot",
        "RightFoot",
    ] {
        assert!(skeleton.find(name).is_some(), "no bone {name} (hash {:08x})", bone_hash(name));
    }
    assert_eq!(skeleton.find("Root"), Some(0));
    assert_eq!(list.clips.len(), 28);
    assert!(list.clips.iter().all(|clip| clip.rate == 30.0 && clip.bones == 46));
    // The steering clip: 31 keys, one second; the long look over the shoulder: 112 keys.
    assert_eq!((list.clips[3].keys, list.clips[19].keys), (31, 112));
    assert!((list.clips[3].seconds() - 1.0).abs() < 1e-6);
    assert!(list.by_id(0x1779c7).is_some(), "the car pose clips are looked up by id");
}

/// A clip's deltas are rotations in the rest world axes: posed so, the hand bones land within a millimetre of the hand
/// positions every key of every clip stores.
#[test]
#[ignore = "reads the game install (CHAMELEON_GAME_DIR or MW2012_GAME_DIR)"]
fn clips_posed_put_the_hands_where_the_clips_say() {
    let Some(bytes) = characters() else { return };
    let (skeleton, list) = load(&bytes);
    let hands = [skeleton.find("LeftHand").unwrap(), skeleton.find("RightHand").unwrap()];
    let mut pose = Pose::new(skeleton.bones.len());
    let mut worst = 0.0f32;
    for clip in &list.clips {
        for key in 0..clip.keys {
            let mut deltas = skeleton.rest_deltas();
            skeleton.sample(clip, key as f32 / clip.rate, &mut deltas);
            skeleton.pose(&deltas, &mut pose);
            for (track, &hand) in hands.iter().enumerate() {
                let stored = clip.positions[key][track];
                let miss = (0..3).map(|k| (pose.positions[hand][k] - stored[k]).powi(2)).sum::<f32>().sqrt();
                worst = worst.max(miss);
            }
        }
    }
    assert!(worst < 0.005, "a posed hand is {worst} m from the position the clip stores");
}
