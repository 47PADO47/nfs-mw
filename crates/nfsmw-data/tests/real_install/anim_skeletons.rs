//! The EAGL skeletons of the NIS bundles. Each bone's inverse bind matrix must undo its bind world matrix, so the
//! skinning matrices of the bind pose are the identity (`docs/specs/eagl-animation-skeleton.md` section 4.1).

use blackbox_anim::{Object, Skeleton};
use glam::Mat4;

use crate::{install, unwrapped};

/// The id of an `EAGLSkeletons` chunk.
const EAGL_SKELETONS: u32 = 0x00E3_4009;
/// The object inside the chunk payload starts on a 16-byte boundary.
const OBJECT_ALIGN: usize = 16;
const TOLERANCE: f32 = 1e-4;

/// The largest absolute difference between the 16 elements of two matrices.
fn max_error(actual: &Mat4, expected: &Mat4) -> f32 {
    actual.to_cols_array().iter().zip(expected.to_cols_array()).map(|(a, e)| (a - e).abs()).fold(0.0, f32::max)
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_nis_skeletons_skin_their_bind_pose_to_the_identity() {
    let Some(dir) = install() else { return };
    let mut skeletons = 0;
    let mut bones = 0;
    let mut worst = 0.0f32;
    for name in dir.files_in("NIS") {
        let lower = name.to_ascii_lowercase();
        if !(lower.starts_with("scene_") && lower.ends_with("_bundleb.bun")) {
            continue;
        }
        let data = unwrapped(&dir, &format!("NIS/{name}"));
        for chunk in blackbox_chunk::find_all(&data, EAGL_SKELETONS) {
            let object = Object::read(chunk.aligned_payload(OBJECT_ALIGN))
                .unwrap_or_else(|e| panic!("{name} at {:#x}: {e}", chunk.offset));
            let skeleton = Skeleton::read(&object).unwrap_or_else(|e| panic!("{name} at {:#x}: {e}", chunk.offset));
            skeletons += 1;
            bones += skeleton.bones.len();
            for (index, bone) in skeleton.bones.iter().enumerate() {
                assert!(bone.parent.is_none_or(|parent| parent < index), "{name}: {} has a later parent", bone.name);
            }
            for (index, skin) in skeleton.skinning(&skeleton.bind_pose()).iter().enumerate() {
                let error = max_error(skin, &Mat4::IDENTITY);
                worst = worst.max(error);
                assert!(
                    error <= TOLERANCE,
                    "{name}: {} (bone {index}) skins the bind pose off by {error}",
                    skeleton.name
                );
            }
        }
    }
    eprintln!("{skeletons} skeletons, {bones} bones, worst bind skinning error {worst:e}");
    assert_eq!(skeletons, 64, "skeleton chunks in the NIS bundles");
    assert_eq!(bones, 2532, "bone records in the NIS bundles");
}
