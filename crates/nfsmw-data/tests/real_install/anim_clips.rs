//! The animation clips of one NIS bundle, read with `blackbox-anim`. Every clip is sampled at ten times across its
//! duration, composed with the skeleton it drives, and checked for finite matrices and for bones that stay within
//! reach of bone 0 (`docs/specs/eagl-animation-bank.md` sections 10 and 11).

use std::collections::BTreeMap;

use blackbox_anim::clip::{Clip, clips};
use blackbox_anim::{DOFS_PER_BONE, Object, Skeleton};

use crate::{install, unwrapped};

/// The id of an `EAGLSkeletons` chunk.
const EAGL_SKELETONS: u32 = 0x00E3_4009;
/// The id of an `EAGLAnimations` chunk.
const EAGL_ANIMATIONS: u32 = 0x00E3_4010;
/// The object inside a skeleton chunk payload starts on a 16-byte boundary.
const SKELETON_ALIGN: usize = 16;
/// The bundle to read: a cutscene with a character.
const BUNDLE: &str = "NIS/Scene_ArrestF02_BundleB.bun";
/// The number of times each clip is sampled, from its first frame to its last.
const SAMPLES: usize = 10;
/// The largest distance a bone may sit from bone 0, in metres. Characters are about 2 m tall.
const REACH: f32 = 10.0;

/// How a clip was matched to its skeleton.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Fit {
    /// The skeleton has the clip's target checksum and the bone count the clip needs.
    Checksum,
    /// Only the bone count matches.
    Bones,
    /// Nothing matches: the largest skeleton is used and the reach check is skipped.
    Largest,
}

/// The skeleton a clip drives. The target checksum is the compatibility rule (bank spec section 9), so a skeleton
/// with the clip's checksum and bone count is preferred. Then comes the bone count alone, then the largest skeleton.
fn pick_skeleton(skeletons: &[Skeleton], checksum: u16, bones: usize) -> (&Skeleton, Fit) {
    if let Some(exact) = skeletons.iter().find(|s| s.checksum == checksum && s.bones.len() == bones) {
        return (exact, Fit::Checksum);
    }
    if let Some(counted) = skeletons.iter().find(|s| s.bones.len() == bones) {
        return (counted, Fit::Bones);
    }
    let largest = skeletons.iter().max_by_key(|s| s.bones.len()).expect("the bundle has a skeleton");
    (largest, Fit::Largest)
}

/// Samples `clip` across its duration on its skeleton. Every matrix must be finite. A matched skeleton's bones must
/// also stay within [`REACH`] of bone 0; an unmatched clip is sampled on the largest skeleton only for finiteness.
fn check_clip(clip: &Clip, skeletons: &[Skeleton]) -> Fit {
    let needed = clip.max_dof().div_ceil(DOFS_PER_BONE);
    let (skeleton, fit) = pick_skeleton(skeletons, clip.checksum, needed);
    let duration = clip.duration_secs();
    eprintln!(
        "{}: {duration:.3} s, {} channels, skipped {:?}, checksum {:#x}, {needed} bones, skeleton {} ({} bones, {fit:?})",
        clip.name,
        clip.channels().len(),
        clip.skipped,
        clip.checksum,
        skeleton.name,
        skeleton.bones.len()
    );
    assert!(!skeleton.bones.is_empty(), "{}: the skeleton {} has no bones", clip.name, skeleton.name);
    let base = skeleton.bind_pose();
    for step in 0..SAMPLES {
        let secs = duration * step as f32 / (SAMPLES - 1) as f32;
        let pose = clip.sample_pose(secs, &base);
        let world = skeleton.world(&pose);
        let root = world[0].w_axis.truncate();
        for (index, matrix) in world.iter().enumerate() {
            let name = &skeleton.bones[index].name;
            assert!(
                matrix.to_cols_array().iter().all(|value| value.is_finite()),
                "{} at {secs:.3} s: bone {index} ({name}) has a non-finite matrix",
                clip.name
            );
            let distance = (matrix.w_axis.truncate() - root).length();
            if fit == Fit::Largest {
                continue;
            }
            assert!(
                distance <= REACH,
                "{} at {secs:.3} s: bone {index} ({name}) is {distance:.2} m from bone 0",
                clip.name
            );
        }
    }
    fit
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn anim_clips_sample_into_finite_poses() {
    let Some(dir) = install() else { return };
    let data = unwrapped(&dir, BUNDLE);
    let skeletons: Vec<Skeleton> = blackbox_chunk::find_all(&data, EAGL_SKELETONS)
        .into_iter()
        .map(|chunk| {
            let object = Object::read(chunk.aligned_payload(SKELETON_ALIGN))
                .unwrap_or_else(|e| panic!("{BUNDLE} at {:#x}: {e}", chunk.offset));
            Skeleton::read(&object).unwrap_or_else(|e| panic!("{BUNDLE} at {:#x}: {e}", chunk.offset))
        })
        .collect();
    assert!(!skeletons.is_empty(), "{BUNDLE} has no skeleton");
    let counts: Vec<usize> = skeletons.iter().map(|skeleton| skeleton.bones.len()).collect();
    eprintln!("{BUNDLE}: {} skeletons with bone counts {counts:?}", skeletons.len());

    let mut fits = BTreeMap::new();
    for chunk in blackbox_chunk::find_all(&data, EAGL_ANIMATIONS) {
        let object = Object::read(chunk.payload).unwrap_or_else(|e| panic!("{BUNDLE} at {:#x}: {e}", chunk.offset));
        let found = clips(&object).unwrap_or_else(|e| panic!("{BUNDLE} at {:#x}: {e}", chunk.offset));
        for clip in &found {
            *fits.entry(check_clip(clip, &skeletons)).or_insert(0usize) += 1;
        }
    }
    eprintln!("clips by skeleton match: {fits:?}");
    let total: usize = fits.values().sum();
    assert!(total > 0, "{BUNDLE} has no animation clips");
}
