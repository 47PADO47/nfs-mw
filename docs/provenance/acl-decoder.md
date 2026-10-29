# Provenance: the ACL decoder (`libs/blackbox-acl`)

| | |
|---|---|
| Module | `libs/blackbox-acl` (generic: no game names), used by `mods/ride6-bikes/src/rider/acl_clips.rs` |
| Spec | [specs/acl-decoder.md](../specs/acl-decoder.md) |
| Date | 2026-10-29 |

## Sources read

| Source | Licence | What for |
|---|---|---|
| [nfrechette/acl](https://github.com/nfrechette/acl), commit `3ee568542eca4428e1041908b4a644b98e7885bd` (develop, 2025-09-13): `includes/acl/core/impl/compressed_headers.h`, `core/impl/compressed_tracks.impl.h`, `core/impl/variable_bit_rates.h`, `core/impl/interpolation_utils.impl.h`, `core/impl/time_utils.impl.h`, `decompression/decompression_settings.h`, `decompression/impl/{decompression,animated_track_cache,constant_track_cache}.transform.h`, `decompression/impl/steps/animated_unpack_vec3.h`, `math/{vector4_packing,quatf}.h` | MIT | The buffer layout, the bit packing, the range expansion, the keyframe stripping and the interpolation. The ACL code is MIT, so it may be ported: the notice is kept at the top of `libs/blackbox-acl/src/lib.rs` and the project is in [NOTICE](../../NOTICE). Nothing is vendored: the clone was made into a scratch folder outside the repository. |
| CUE4Parse (`UE4/Assets/Exports/Animation/ACL`, `ACL/CompressedHeaders.cs`, `CUE4Parse-Natives/ACL`) | MIT | How Unreal wraps the buffer (`FACLCompressedAnimData`: the compressed byte stream is the `compressed_tracks` buffer itself; the track to bone map is `CompressedTrackToSkeletonMapTable`; the default sub-tracks are the skeleton's reference pose; an additive sequence gets default scale 0). CUE4Parse has no managed ACL decoder: it calls a native library built from the ACL sources. |
| The RIDE 6 clips (the user's own install) | not redistributed | The 158 `FACLCompressedAnimData` buffers of the rider's animations were dumped by the export tool and used to test the decoder; none is committed. |

No decompiled code was read.

## How the result was checked

- Unit tests (`cargo test -p blackbox-acl`): a small test-only encoder (`src/encoder.rs`) writes buffers of the layout of the
  spec (one or several segments, quantised and uncompressed sub-tracks, stripped keyframes, a looping clip) and the decoder
  reads them back to the quantisation step.
- Against ACL itself (`libs/blackbox-acl/src/real_data.rs`, ignored: it needs the user's clips): ACL's own C++ decoder was
  built from the same commit (and RTM, MIT, https://github.com/nfrechette/rtm) in a scratch folder, a ten-line harness
  decompressed every sample of the 158 buffers with `decompress_tracks` and wrote the transforms as text, and the Rust decoder
  matched them: **158 clips, 6,437,563 track samples, largest difference 1.5e-5** (the float resolution at 100 cm). The clips
  cover 1 to 98 segments, with (19 clips) and without stripped keyframes, wrap-optimised (56) and not. None of the 158 has scale data, so the
  scale blocks are tested with the encoder only.
- Against the export tool's own decoder: CUE4Parse with the native library built from the same ACL commit decoded the eight
  riding clips, and `mods/ride6-bikes` (`acl_clips_tests.rs`, ignored) matches it to 3e-7 rad and 4e-6 cm. CUE4Parse leaves the
  quaternions of the frames it interpolates between stripped keyframes a little short of unit length (by 0.2 percent in the
  wheelie clips: it happens after ACL, whose own decoder gives unit quaternions there); the importer normalises rotations, so the pack does not carry the difference.

## What is not covered

Revisions before 9 (ACL 2.0 and older), compressed databases (streamed keyframes), scalar (float track) buffers, the optional
metadata block, the buffer hash check. The Unreal ACL plugin's own decompression settings are not read: the decoder follows
ACL's default settings (rotations normalised), which is what CUE4Parse's native library uses.
