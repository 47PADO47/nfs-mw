# ACL animation decoder: to be reimplemented in Rust

Part of the [skeletal animation work](skeletal-animation.md). Status: **not started; documented as needed**.

## Why

Eight of the rider's motard riding clips of RIDE 6 are stored with the Animation Compression Library (ACL), an open-source
(MIT) format used by Unreal Engine. The importer reads the other 66 clips; these eight cannot be read today:

- the standing wheelie, the wheelie entry and the standing interpolation,
- the drift poses,
- `old_school_standard_pose`.

The unreadable clips are listed in `docs/formats/ride6-rider.md`. Until they are decoded, the rider falls back to the
nearest readable clip for those states.

## Why nothing here decodes them

CUE4Parse reads ACL through a native library built from the ACL sources (CUE4Parse-Natives, built with CMake). On the
development PC the build fails: no C++ compiler is configured for CMake, and the ACL sources are not part of the CUE4Parse
checkout (they are a download).

## What to do

1. Port the ACL decoder to Rust as a generic module of `libs/blackbox-skeleton` (for example `src/acl/`): no game names,
   takes `&[u8]` and returns the library's own clip type (`clip.rs`). Pure Rust, no native build.
2. Work from the published ACL format and its MIT-licensed reference decoder (a separate download of the ACL repository into
   a scratch folder outside this repository, kept out of the tree). Record the version of ACL, the Unreal Engine 5.6
   container fields (compressed-tracks blob in the animation sequence) and the licence in `docs/provenance/acl-decoder.md`.
3. The decoder must cover the bit-rate and range-reduction modes that the RIDE 6 clips use, plus the track types of a
   skeleton clip (rotation, translation, scale). Verify against the 66 clips that CUE4Parse already decodes by re-encoding
   comparison is not possible, so verify on the eight by checking continuity, bone lengths and the pose at frame 0.
4. Extend `ride6-rider-import` to emit the eight clips and update the rider animation graph (`docs/specs/ride6-rider-animation.md`)
   to use them for the standing wheelie, the wheelie entry and the drift poses.

## Decision of the project owner

The owner decided on 2026-10-28 not to clone the ACL repository now: the need is documented here and the work is deferred.
