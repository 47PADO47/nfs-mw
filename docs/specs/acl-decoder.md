# ACL animation decoder (`libs/blackbox-acl`)

Part of the [skeletal animation work](skeletal-animation.md). Status: **implemented and checked** against ACL's own decoder on
158 real clips. Sources, licence and checks: [provenance/acl-decoder.md](../provenance/acl-decoder.md). The decoder is generic
(`&[u8]` in, transforms out; no game names). The RIDE 6 side (which clips, the track to bone map, the reference pose) is
`mods/ride6-bikes/src/rider/acl_clips.rs`.

## 1. Why

RIDE 6 (Unreal Engine 5.6.1) stores 158 of the rider's 700 animation sequences with the Animation Compression Library
([ACL](https://github.com/nfrechette/acl), MIT) through its Unreal plugin; the rest use Unreal's own codecs, which CUE4Parse reads
without help. CUE4Parse reads ACL through a *native* library built from the ACL sources (CUE4Parse-Natives, CMake). It has no managed
decoder, and on the development PC the library was not built (no ACL sources in the checkout, and the build died on a path that was
too long), so every ACL sequence failed to export. Nothing else reads them either.

The ACL sequences of the **supermoto ("motard") rider** are the eight of the riding graph, which the pack therefore lacked
(`docs/formats/ride6-rider.md` lists them), plus one of the 22 fall and get-up sequences:

| Sequence | What it is (data) |
|---|---|
| `motard_wheelie_Standing_anim`, `_L`, `_R`, `motard_wheelie_StandingInterpolation_anim` | The four samples of the blend space `motard_Wheelie_Standing_BS`: the rider stands on the pegs in a wheelie |
| `motard_wheelie_Entry_anim` | The `Wheelie_Entry` clip of the blueprint |
| `motard_drift_pose_L`, `_R` | The `Drift_L` / `Drift_R` poses of the blueprint |
| `motard_old_school_standard_pose` | `Standard_Pose` of the old-school riding style (`RidingStyle == 2`) |
| `race_ragdoll_FaceDown_GetUp` | The get-up from lying face down (the other 21 fall and get-up sequences are not ACL) |

The other 149 ACL sequences are the cinematics (podium, celebration, pit, hospitality) and the enduro, scooter, race and naked
riders' sets; the same code decodes them and they are not part of the pack.

## 2. What the clips use

Every one of the 158 buffers: **revision 10** (ACL 2.1), algorithm 0 (uniformly sampled), track type `qvvf` (transform), rotations
`quatf_drop_w_variable`, translations and scales `vector3f_variable` but **no scale data** (the scale of every bone is the reference
pose's), default scale 1, **non-trivial default sub-tracks** (the skeleton's reference pose fills them), 30 samples a second, none
additive. Between 1 and 98 segments; 19 of the clips have keyframes stripped and 56 are wrap-optimised (looping). 65 clips have the
81 tracks of the rider skeleton (the 8 of the riding graph among them), 93 the 235 tracks of the cinematic skeleton. The Unreal
sequence's `CompressedTrackToSkeletonMapTable` maps a track to a bone: an identity map for the cinematic clips, and for the rider's
clips the identity except for the face bones (tracks `50, 51, 52, 54, 55, 56, 65` to `72` take bones `51, 50, 56, 55, 52, 54, 71`
to `70`), which is why the map comes with every buffer.

## 3. The buffer

Little endian. Offsets are bytes. The buffer is what Unreal calls `CompressedByteStream` of `FACLCompressedAnimData`; its length
is the first word. Everything is relative to the start of the buffer unless it says "from the transform header" (byte 32).
The decoder reads revisions 9 and 10 only and refuses a database (streamed keyframes).

```text
 0  u32 size           4 u32 hash                       raw buffer header
 8  u32 tag 0xac11ac11 12 u16 revision (9, 10)  14 u8 algorithm (0)  15 u8 track type (12)
16  u32 track count    20 u32 sample count      24 f32 sample rate  28 u32 flags
```

Flags: bit 0 has scale; bit 1 default scale (0 or 1); bit 2 scale format (0 float, 1 variable); bit 3 translation format; bits 4-7
rotation format (0 four floats, 2 three floats `w` dropped, 3 variable bit rate `w` dropped); bit 8 has database; bit 9 trivial
default sub-tracks; bit 10 stripped keyframes; bit 30 wrap optimised; bit 31 has metadata.

**The transform header** (from byte 32, 52 bytes), `u32` each: segment count; animated variable sub-track count; animated rotation,
translation, scale counts; constant rotation, translation, scale counts; database offset (`0xFFFFFFFF`: none); then offsets, from the
transform header, to the segment headers, the sub-track types, the constant data, the clip range data. After it, with several
segments, the segment start indices (`u32` each, the first sample of each segment).

**Segment headers** (16 bytes, 20 with stripped keyframes): bits of one stored pose, bits of its rotation part, bits of its
translation part, offset (from the transform header) to the segment data, and with stripped keyframes the bit set of the stored
samples (bit `31 - n` is the segment's sample `n`; a segment holds at most 32 samples then).

**Sub-track types**: all rotation words, then the translation words, then the scale words (only with scale); one `u32` per 16
tracks, track `t` at bits `(15 - t % 16) * 2` (most significant first): 0 default, 1 constant, 2 animated. The counts of each
kind must equal the header's.

**Constant data**: the constant rotations, then the translations, then the scales. A rotation is stored in groups of four
tracks, component-wise (`x` of the four, `y` of the four, `z` of the four, floats; the last group has as many as are left);
`w` is rebuilt. With four-float rotations: 16 bytes each, as they are. Translations and scales: three floats each, in track order.

**Clip range data** (animated sub-tracks with a variable format): for the animated rotations, groups of four, component-wise as
above, six floats' worth per track (`min x y z`, `extent x y z`); then for each animated translation, then each animated scale
(only when the translation format is variable; scales follow), three floats of minimum and three of extent, track after track.

**Segment data** (from its offset): one byte per animated rotation, **padded to a multiple of four**, then one byte per animated
translation, then per animated scale, each the number of bits of one component (0 constant bit rate, 1 to 23, 31 uncompressed
floats); aligned to 2; with several segments the **segment range data** (for the rotations in groups of four, byte-wise: `min x`
of four tracks, `min y`, `min z`, `extent x`, `y`, `z`; for the translations and scales six bytes per track, `min x y z`,
`extent x y z`); aligned to 4; then the stored poses, bit packed.

**A pose** (`pose bits` wide, poses back to back with no padding): every animated rotation, then every animated translation,
then every animated scale, in track order; each of three components of its track's bit count, most significant bit first
(32 bits when the byte says uncompressed: a float; four floats when the rotation format is full, three when `w` is dropped
without a variable rate). The first bit of a pose is the most significant bit of a byte.

## 4. Decoding a sample

A request is a pair of sample numbers `k0, k1` and a weight `alpha` (`k1 = k0 + 1`, or 0 past the end of a wrap-optimised clip, or
`k0` at the end of a clamped one); a frame is `(f, f + 1, 0)`.

**Stripped keyframes.** With `local0`, `local1` the sample numbers inside their segments and `S0`, `S1` the stored bit sets, take
the nearest stored sample at or before `k0` and at or after `k1`:
`below = S0 & (!0 << (31 - local0))` gives `local0' = 31 - trailing_zeros(below)`; `above = S1 & (!0 >> local1)` gives
`local1' = leading_zeros(above)`. The new weight is `(p - k0') / (k1' - k0')` with `p = k0 + alpha` (and the clip's sample count
in place of `k1' - k0'` when `k1'` wrapped to 0). The pose to read is the number of set bits of `S` before `local'`. Without
stripping a pose is the sample's own number in its segment. A segment's last sample is never stripped.

**One animated rotation** (variable format) at stored pose `p` of segment `g`, `j`-th animated rotation, `bits` its byte:

```text
v          = bits == 31 : three floats                         (no range expansion)
           = bits == 0  : (hi << 8 | lo) / 65535 per component, hi and lo the segment range bytes of the track at rows 0/4, 8/12,
                          16/20 (needs several segments); the segment range is not applied
           = otherwise  : q / (2^bits - 1) per component
v          = v * (extent/255) + (min/255)       segment range, bits not 0 or 31 and several segments
v          = v * clip_extent + clip_min         clip range, bits not 31
w          = sqrt(|1 - x^2 - y^2 - z^2|)         then the quaternion is normalised
```

**An animated translation or scale** is the same with the segment range bytes laid out per track (min then extent, `/255`), the
constant bit rate value three little-endian `u16` over 65535, and no `w`. Four-float or three-float rotations and float vectors
have no ranges. **A constant** is read as stored (rotations: `w` rebuilt, normalised). **A default** takes the caller's reference
value (the skeleton's), or the identity, zero and the header's scale when the clip says its defaults are trivial. **Without scale
data** every scale is the header's default scale, as ACL's writer does.

**Interpolation.** Translation and scale `a + (b - a) alpha`; rotation `nlerp` with the shorter arc:
`normalise(b * s * alpha + (a - a * alpha))`, `s = sign(a . b)`. A weight of 0 reads one pose only.

**Time.** A clip holds `num_samples` poses `1 / sample_rate` apart; with wrap optimisation the last pose of the animation is the
first and is not stored, so the animation has `num_samples + 1` frames and lasts `num_samples / sample_rate`; otherwise
`(num_samples - 1) / sample_rate`. Unreal reports that frame count (`NumFrames`), which is what the export tool's frame list uses.

## 5. The Unreal wrapper

- The sequence's `CompressedDataStructure` is a `FACLCompressedAnimData` whose byte array is the buffer above. UE 5.5+ appends one
  boolean (`bCompressionFailed`) after the array in the asset; CUE4Parse reads it.
- Track `t` animates bone `CompressedTrackToSkeletonMapTable[t].BoneTreeIndex` of the skeleton the sequence belongs to; a bone with
  no track keeps the reference pose.
- The default sub-tracks are the skeleton's reference pose (`ReferenceSkeleton.FinalRefBonePose` of the bone), also the scale when
  there is scale data. CUE4Parse takes the same when the default-scale bit is 1; for an additive sequence it first sets that bit to
  0 and lets the defaults be the identity, zero and zero scale.
- Unreal's own decompression settings are not read: the decoder normalises rotations like ACL's default settings.

## 6. The export and the import (RIDE 6)

- `R6ToolRider acl <list> <folder>` (the user's scratch tool, on top of CUE4Parse) writes, per sequence of the list,
  `<name>.acl` (the buffer) and `<name>.meta.json` (`name`, `num_frames`, `length`, `rate_scale`, `additive`, `base_name`,
  `base_frame`, `root_motion`, `track_to_bone`, `notifies`). It needs no native library and no key.
- `R6ToolRider anims <list> <out.r6ra>` writes the sequences that Unreal's own codecs store (the 21 fall and get-up clips).
- `ride6-rider-import <export folder> <out>` reads `riding.r6ra`, every other `*.r6ra` of the folder, and the `acl/` folder, decodes the
  ACL buffers with `blackbox-acl` into the same raw clips and builds one pack. Additive ACL sequences are refused for now.

## 7. API

```rust
let tracks = blackbox_acl::Tracks::parse(bytes)?;               // headers and layout, borrowed
tracks.num_tracks(); tracks.num_frames(); tracks.sample_rate(); tracks.duration();
let pose = tracks.decode_frame(frame, Some(&reference_pose))?;  // Vec<Qvv> per track: rotation xyzw, translation, scale
let pose = tracks.sample(seconds, Some(&reference_pose))?;      // interpolated, time clamped to the clip
let all  = tracks.decode_all(Some(&reference_pose))?;           // every frame
```

## 8. What is not covered

Revisions before 9; databases; scalar (float) track lists; the optional metadata block; the hash. Scale tracks are implemented
from the source and tested with the test encoder only (none of the RIDE 6 clips animates a scale).
