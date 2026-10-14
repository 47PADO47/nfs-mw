//! ICE camera, EAGL animation and WorldAnim chunk ids.
//!
//! Status: names are taken from `tools/bchunk_names.py` (CC0 `dbalatoni13/nfsmw` chunk list).
//! `ICE_SHAKE_TRACKS` and `ICE_NIS_CAMERAS`..`ICE_REPLAY_CAMERAS` are confirmed as immediates in the
//! `speed.exe` dispatch (see `docs/provenance/ice-camera-dispatch.md`). The rest are unverified.
//! Layouts: `docs/formats/animation.md`.

/// `ICECatalog`: the ICE camera catalogue.
pub const ICE_CATALOG: u32 = 0x0003_B200;
/// ICE camera packs by use: NIS, FMV, MKR (marker cameras), replay, generic.
pub const ICE_NIS_CAMERAS: u32 = 0x8003_B200;
pub const ICE_FMV_CAMERAS: u32 = 0x8003_B201;
pub const ICE_MKR_CAMERAS: u32 = 0x8003_B202;
pub const ICE_REPLAY_CAMERAS: u32 = 0x8003_B203;
pub const ICE_GENERIC_CAMERAS: u32 = 0x8003_B204;
/// Camera shake parameters.
pub const ICE_CAMERA_SHAKE_DATA: u32 = 0x8003_B209;
/// `ICECameraGroup` (ICETracks): one camera track group.
pub const ICE_CAMERA_GROUP: u32 = 0x0003_B210;
/// `ICEShakeTracks`: per-track shake curves.
pub const ICE_SHAKE_TRACKS: u32 = 0x0003_B211;

/// `AnimDirectory`: index of shared in-game animation scenes.
pub const ANIM_DIRECTORY: u32 = 0x8003_7050;
/// EAGL skeleton and animation-bank ELF objects (`docs/formats/animation.md`).
pub const EAGL_SKELETONS: u32 = 0x00E3_4009;
pub const EAGL_ANIMATIONS: u32 = 0x00E3_4010;

/// Animated world objects.
pub const WORLD_ANIM_TREE_MARKERS: u32 = 0x0003_7110;
pub const WORLD_ANIM_EVENT_DIRECTORY_DATA: u32 = 0x0003_7140;
pub const WORLD_ANIM_INSTANCE_ENTRY: u32 = 0x0003_7150;
pub const WORLD_ANIM_PIPE_INSTANCE_ENTRY: u32 = 0x0003_7160;
pub const WORLD_ANIM_PIPE_SECTION_LIST: u32 = 0x0003_7170;

/// BBGAnim pack chunks.
pub const BBG_ANIM_PACK_HEADER: u32 = 0x0003_7210;
pub const BBG_ANIM_BLOCK_HEADER: u32 = 0x0003_7220;
pub const BBG_ANIM_PIPE_KEYFRAMES: u32 = 0x0003_7230;
pub const BBG_ANIM_KEYFRAMES: u32 = 0x0003_7240;
pub const BBG_ANIM_INSTANCE_NODE: u32 = 0x0003_7250;
pub const BBG_ANIM_INSTANCE_TREE: u32 = 0x0003_7260;
pub const BBG_ANIM_END_PACK_HEADER: u32 = 0x0003_7270;
