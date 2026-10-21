# EAGL animation codecs: delta-compressed channels

Status: **decoded for every codec the install uses**; DeltaQ is partly decoded; DeltaQFast and DeltaSingleQ are not used by the install. This spec gives the byte layout and the decode rule for the delta codecs, precise enough to implement them bit-exactly. The bank and evaluator mapping work is in [eagl-animation.md](eagl-animation.md).

Evidence tags (see [docs/README.md](../README.md#evidence-tags)):

- **[verified]**: measured on the install by a Python prototype. The numbers are quoted.
- **[decomp]**: from the CC0 decompilation; the file is cited.
- **[exe]**: supplied by the project supervisor from Ghidra on `speed.exe` 1.3. Not re-checked by the author of this spec.
- **[inferred]**: consistent with the evidence, not proven.
- **[unconfirmed – needs Ghidra]**: the decomp file that defines the rule is empty or stubbed.

## 1. Where the codecs sit **[decomp] [verified]**

A bank holds named animations. Each named animation is one channel object. Every channel object starts with the same 4-byte header: a u16 codec type, then a u16 target checksum. The codec types, as numbered in the decomp:

| Type | Name | Leaves in the install (all .bun files) | Covered here |
|---|---|---|---|
| 10 | DeltaLerp | 475 | yes, section 6 |
| 11 | DeltaQuat | 343 | yes, section 7 |
| 15 | Compound | container | yes, section 2 |
| 17 | DeltaQ | 2 | short, section 11 |
| 18 | DeltaQFast | 0 | not used, section 12 |
| 19 | DeltaSingleQ | 0 | not used, section 12 |
| 20 | DeltaF3 | 92 | yes, section 9 |
| 21 | DeltaF1 | 40 | yes, section 8 |

The counts come from scanning the 101 `.bun` files in the install. 88 of them contain animation banks. Each count is the number of leaf channels, after walking compound channels.

A bank name ends in `_q`, `_s` or `_t`. The suffix does not select the codec. In `NIS\Scene_ArrestF02_BundleB.bun`, channels named `_s` are DeltaQuat channels. **[verified]**

## 2. The compound channel (type 15) **[decomp] [verified]**

The airliner channel `ANM_AirlinerA_10_XO_AirlinerA_1b_LB_10_q` is a compound channel. The bank entries point to compound objects, and each compound points to one leaf codec object. The layout of a compound object:

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16 | codec type (15) |
| +0x02 | u16 | target checksum |
| +0x04 | pointer | attribute block (see below) |
| +0x08 | u16 | number of leaf channels (1 in every airliner case) |
| +0x0A | u16 | number of frames |
| +0x0C | pointer array | one pointer per leaf channel |

Pointers in the `.data` section of the ELF object are offsets into that section after relocation.

**Attribute block.** The compound's +0x04 pointer points to a 12-byte block that sits just before the leaf object in `.data`. In the airliner it reads `01 00 00 00 | 01 00 01 00 | 1E 30 30 30`. **[verified: bytes]** Its contents are not decoded. **[unconfirmed]** It is not the codec header. An earlier version of this spec took it for one; that was wrong.

**Evaluation.** Every leaf writes into the same output buffer. The decomp loops over the leaves from the last to the first. **[decomp: FnCompoundChannel::Eval]**

## 3. Bones, dof indices and slots **[decomp] [verified]**

The output of a channel is a buffer of floats. A **dof index** names one float of that buffer. The decomp divides a dof index by 12 to get the bone index (`bone = dof div 12`). The remainder is the slot inside the bone (`slot = dof mod 12`). **[decomp: the bone-mask code]**

On the airliner, every channel uses bone 1 and the slots are:

| Dof index | Bone | Slot | Channel | Value |
|---|---|---|---|---|
| 12 to 14 | 1 | 0 to 2 | `_s` (constant) | (1, 1, 1) |
| 16 to 19 | 1 | 4 to 7 | `_q` (quaternion) | animated |
| 20 to 22 | 1 | 8 to 10 | `_t` (translation) | animated |

**[verified]** Slots 3 and 11 are never written by any install channel. **[verified]** The slot names (scale 0 to 2, quaternion 4 to 7, translation 8 to 10) are **[inferred]** from the channel names and the values.

Where the dof index list is stored depends on the codec:

- DeltaChan family (types 10 and 11): the list sits inline, starting at +0x0A of the channel header. A quaternion channel has one index per quaternion. A DeltaLerp channel has one index per scalar.
- DeltaF1 and DeltaF3 (types 21 and 20): the list is a separate array, pointed to from +0x04 of the header. A channel with no animated bones has no list.

## 4. The DeltaCompressedData block (types 10 and 11) **[decomp] [verified]**

The DeltaChan channel header (types 10 and 11), with offsets from the start of the channel object:

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16 | codec type (10 or 11) |
| +0x02 | u16 | target checksum |
| +0x04 | pointer | data block (below) |
| +0x08 | u16 | number of frames |
| +0x0A | u16 array | dof indices (inline) |

The data block:

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16 | number of dofs (numDofs) |
| +0x02 | u8 | quantisation bits: 8, 16 or 4 |
| +0x03 | u8 | padding |
| +0x04 | 12 bytes × numDofs | DofInfo records (below) |
| after DofInfo | rows | one row per frame (below) |

Each **DofInfo** record is 12 bytes: a minimum (f32), a range (f32) and a start value (f32). **[decomp: DeltaCompressedData::DofInfo] [verified: the floats are plausible on the airliner]**

The delta rows follow the DofInfo records. Row stride by quantisation:

- **8 bits:** a row is numDofs bytes. Dof i uses byte i. **[verified]**
- **16 bits:** a row is 2 × numDofs bytes. Dof i uses the little-endian u16 at byte 2i. **[verified: NIS DeltaLerp channels]**
- **4 bits:** a row is ⌈numDofs / 2⌉ bytes. An even dof uses the low nibble of its byte, an odd dof uses the high nibble. **[decomp only; no 4-bit channel in the install]**

The decoded delta of dof i in a row is raw × range_i + min_i, where raw is the unscaled integer (0 to 255, 0 to 65535 or 0 to 15). **[decomp] [verified for 8 and 16 bits]**

## 5. Decode rule for types 10 and 11 **[decomp] [verified]**

1. The start value of dof i is the start field of its DofInfo record. That is frame 0.
2. For frame f ≥ 1, the value of dof i is the value at frame f − 1 plus the decoded delta of row f − 1. Row j is the step from frame j to frame j + 1.
3. A channel with N frames has N rows, and the last row is never used. **[verified: on the airliner `_q` channel, the delta rows end exactly at the next object]**
4. The decomp keeps an incremental state and resets to frame 0 when the requested frame goes backwards. A stateless decode from frame 0 gives the same values. **[decomp: DeltaChan.cpp]**

**Time to frame.** Let t be the requested time in frames (a float).

1. The floor frame is the integer part of t, truncated toward zero.
2. The fraction is t minus the floor frame.
3. A floor frame below 0 becomes 0. A floor frame at or beyond the last frame becomes the last frame (N − 1). **[decomp: EvalToPrevValues clamps]**

**Interpolation.** Let f be the floor frame and g = f + 1.

1. If the fraction is zero, or g ≥ N, the result is the value at f with no lerp.
2. Otherwise the result is the value at f plus the fraction times the decoded delta of row f, for each component.
3. The interpolation is linear, component by component. For quaternions (type 11) there is no renormalisation. **[decomp: DecompressValuesIndexed with a scale; FnDeltaQuatChan::EvalSQT]**

## 6. DeltaLerp (type 10) **[decomp] [verified]**

A DeltaLerp channel uses section 4 and section 5 exactly. Each dof is one scalar. The dof index list has one entry per dof. Each dof writes its own output slot.

**[verified]** `WMWaterTowerZPM_Car1_t` in `NIS\Scene_WMWaterTower_BundleB.bun`: 16-bit rows, 3 dofs, dof indices 20, 21, 22, 47 frames. Mean step 0.0072 of the channel's range, maximum 0.033. Smooth.
**[verified]** `WMWaterTowerZPM_Car1_s` in the same bundle: 16-bit, 3 dofs, dof indices 12, 13, 14. The steps are zero, so the channel is constant.

## 7. DeltaQuat (type 11) **[decomp] [verified]**

A DeltaQuat channel uses section 4 and section 5. Each quaternion takes four consecutive dofs, so a channel with N_q quaternions has 4 × N_q dofs and N_q dof indices. The four components are stored in dof order 0 to 3. The decomp does not name the order. **[inferred]** The last component is W: in the airliner's frame 0 it is the largest (0.9857), so it is the scalar part.

The quaternion is not rebuilt from three components. All four are stored and decoded directly. There is no W reconstruction for types 10 and 11.

**[verified] Airliner `ANM_AirlinerA_10_XO_AirlinerA_1b_LB_10_q`** (51 frames, 8 bits, dof index 16):

- |q| between 0.999774 and 1.000216 over all frames.
- Mean angle between consecutive frames 0.1209 rad. Maximum 0.1838 rad, between frames 25 and 26.

**[verified] The other 12 DeltaQuat channels in `TRACKS\L2RA.BUN`** (8 bits): |q| between 0.99957 and 1.00041 over all channels. Mean step 0.0024 to 0.156 rad. Maximum step 0.0074 to 0.39 rad. All smooth.

**[verified] Channels in `NIS\Scene_ArrestF02_BundleB.bun`** (8 bits, 119 frames): |q| between 0.99854 and 1.00154. Mean step 0.027 to 0.043 rad. One channel (`ArrestF02_FCop_s`) has a single step of 1.80 rad and another (`ArrestF02_Cuffs_s`) has 0.85 rad. Those are probably real cuts in the cutscene, not decode errors. **[inferred]**

## 8. DeltaF1 (type 21) **[decomp] [verified]**

**Header** (object size 0x14):

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16 | codec type (21) |
| +0x02 | u16 | target checksum |
| +0x04 | pointer | dof index list (one u16 per bone) |
| +0x08 | pointer | key-time table (null in every install channel) |
| +0x0C | u16 | number of frames |
| +0x0E | u16 | number of bones |
| +0x10 | u8 | bin length power p |
| +0x11 | u8 | number of constant bones |
| +0x12 | 2 bytes | padding |

**DofInfo** starts at +0x14: one 12-byte record per bone.

| Offset | Type | Field |
|---|---|---|
| +0x00 | f32 | physical minimum |
| +0x04 | f32 | physical range |
| +0x08 | u16 | quantised minimum |
| +0x0A | u16 | quantised range |

**Scales.** These are the decomp's constants, with 1/65535 written as 1.5259022e-5 and 1/255 as 0.003921569. **[decomp: FnDeltaF1::InitBuffersAsRequired]** For each bone, with P the physical range from the DofInfo record:

1. Physical scale: P × (1/65535).
2. Delta minimum: quantised minimum × P × (2/65535) − P.
3. Delta range: quantised range × P × (2/65535) × (1/255).

**Bins.** The frames are cut into bins of 2^p frames. Bin k holds frames k × 2^p up to k × 2^p + 2^p − 1. The bins start after the DofInfo records. Each bin has:

- one physical key per bone, a u16 each, and
- 2^p − 1 delta rows, each numBones bytes long, one byte per bone.

A bin is padded to an even size. Bin size in bytes = numBones × 2 + (2^p − 1) × numBones, rounded up to an even number. **[decomp] [verified on `ANM_Blimp_1b_02_XO_Blimp_1b_02_t`]**

**Constant bones.** After the last bin come the constant bones. First a u16 dof index per constant bone, then one f32 per constant bone, aligned to 4 bytes. The last bin is partial when the frame count is not a multiple of 2^p. It holds its physical key and (r − 1) delta rows, where r is the frame count modulo 2^p. **[decomp: GetConstBoneIdx] [verified on the airliner `_t` channel: the last bin ends at the next object]**

**Decode rule.**

1. Frame f lives in bin b = f shifted right by p, at position k = f modulo 2^p.
2. The value of a bone at position 0 of its bin is the physical minimum plus the stored u16 times the physical scale.
3. The value at position k is the value at position 0 plus the sum of the decoded deltas of rows 0 to k − 1 in that bin. Each decoded delta is the stored u8 times the delta range plus the delta minimum. This is the cumulative rule (hypothesis a in section 10).
4. The interpolation between frame f and frame f + 1 follows section 5. If f + 1 is in the same bin, the step is row k of the bin. If it is in the next bin, the step ends at the next bin's physical key.
5. Each constant bone takes its f32 value in every frame.

**[verified] Measured on `ANM_Blimp_1b_02_XO_Blimp_1b_02_t`** (2 bones, 41 frames, p = 3): mean step 0.045 of the bone's range, maximum 0.107. Boundary error mean 0.044, maximum 0.070. Smooth. The boundary error is the size of one typical delta. See section 10.

**[verified] `ArrestF02_FCop_s`** in the NIS bundle: 1 bone, 119 frames. Mean step 0.0001 of range. The channel is nearly constant.

## 9. DeltaF3 (type 20) **[decomp] [verified]**

**Header** (object size 0x14): the same fields as DeltaF1 (section 8), at the same offsets. The dof index list is still a pointer at +0x04.

**DofInfo** is 36 bytes per bone:

| Offset | Type | Field |
|---|---|---|
| +0x00 | 3 × f32 | physical minimum, per component |
| +0x0C | 3 × f32 | physical range, per component |
| +0x18 | 3 × u16 | quantised minimum, per component |
| +0x1E | 3 × u16 | quantised range, per component |

**Scales.** **[exe]** Supplied by the supervisor from speed.exe 1.3. **[decomp: FnDeltaF3::InitBuffersAsRequired agrees]** For each component c, with P_c the physical range:

1. Physical scale: P_c × (1/65535).
2. Delta minimum: quantised minimum_c × P_c × (2/65535) − P_c.
3. Delta range: quantised range_c × P_c × (2/65535) × (1/255).

**Bins.** The layout is the DeltaF1 layout, with three components per bone. A bin holds one physical key per bone (three u16, 6 bytes) and 2^p − 1 delta rows. Each row is 3 bytes per bone (one byte per component). The row stride is 3 × numBones bytes. Bin size = numBones × 6 + (2^p − 1) × 3 × numBones, rounded up to an even number. **[decomp] [verified on the airliner `_t` channel]**

**Constant bones.** As in DeltaF1, but each constant bone has three f32 values (one per component). The dof index of a constant bone starts its three slots.

**Decode rule.** The same as DeltaF1, applied to each component. Each channel writes three consecutive slots starting at its dof index.

**[verified] Airliner `_s` (DeltaF3, 0 animated bones, 1 constant bone):** the constant is (1, 1, 1) at dof 12, so bone 1's scale is unity.

**[verified] Airliner `_t` (DeltaF3, 1 bone, dof index 20, 51 frames, p = 3):** the phys keys of six bins form one smooth cycle. X goes 0.0039 to 0.98 and back to 0.013 of its range. The per-frame steps are large (per component: mean 121 units, maximum 400 units, in the file's own units). This is expected for a fast sweep over a 4.6-unit range; the units are not stated in the file.

## 10. Hypothesis tests for DeltaF3 **[verified]**

The DeltaF1 and DeltaF3 decoders have two open questions. How is a delta applied inside a bin? And what is the physical key's exact form? The prototype tested three options on the airliner `_t` channel, which has 6 full bins and one bone:

- (a) **cumulative**: each frame adds the deltas of all earlier rows in its bin to the bin's physical key.
- (b) **relative**: each frame adds only the delta of the previous row to the bin's physical key.
- (c) **physical form**: either the decomp form (physical key = base + u16 × P / 65535), or a centred form (physical key = base + (2 × u16 / 65535 − 1) × P).

For each option the prototype measured the mean and maximum step between frames, and the **boundary error**: the distance between the last in-bin frame of a bin (frame 7 in a 2^3 bin) and the next bin's physical key (frame 8). Values are fractions of each component's range.

| Delta rule | Physical form | Mean step | Max step | Mean boundary | Max boundary |
|---|---|---|---|---|---|
| (a) cumulative | decomp | 0.063 | 0.103 | **0.041** | **0.082** |
| (b) relative | decomp | 0.067 | 0.620 | 0.285 | 0.620 |
| (a) cumulative | centred | 0.121 | 0.759 | 0.366 | 0.759 |
| (b) relative | centred | 0.126 | 1.320 | 0.611 | 1.320 |

**Result.** Hypothesis (a) with the decomp physical form closes best. Option (b) fails because it does not let deltas accumulate. The centred form is worse than the decomp form.

**The 4.5% closure error is not a scale error.** The mean in-bin delta is 0.041 of the range (maximum 0.103). The boundary error is 0.041 on average. So the boundary error is one typical delta. The reason is structural: a bin stores only 2^p − 1 rows, so the step from frame 7 to frame 8 is not stored. The next key is an independent sample, not the end of the cumulative path. **[inferred from the matching sizes]**

The same test on the Blimp DeltaF1 channel gives a boundary error (0.044) matching its mean step (0.045). **[verified]**

## 11. DeltaQ (type 17) **[unconfirmed – needs Ghidra]**

Only two leaves in the install use this codec. Both are the same bank, `Rotate_360_q`, in `GLOBAL\InGameB.bun` and `GLOBAL\INGAMEC.BUN`. It has one bone, 41 keys and p = 3. Cutscenes do not use it. **[verified]**

**Header.** Like DeltaF1, with these fields: key count u16 at +0x04, bone count u8 at +0x06, constant-bone count u8 at +0x07, key-time table pointer at +0x08, bone-index pointer at +0x0C, bin power u8 at +0x10. The header is 0x14 bytes. **[decomp: DeltaQ.h]**

**Min and range.** Per bone, six u16 values: three minima and three ranges, 12 bytes. The bytes at +0x14 read `FF 7F 03 80 00 00 | 00 00 07 0A FE 7F`. **[verified: bytes]**

**Keys and deltas.** The decomp declares a physical key of 6 bytes: X in 15 bits with the W sign in bit 15, then Y and Z as u16. It declares a delta of 3 bytes: X in 7 bits with the W sign in bit 7, then Y and Z as u8. The W component is recovered from the other three. **[decomp: DeltaQPhysical, DeltaQDelta]**

**What is missing.** The decomp's bodies for the dequantisation, the W recovery and the bin-size formula are empty or stubbed. The bin-size formula leaves out the key bytes. The prototype's measurements:

- With a bin stride of 28 bytes (6-byte key plus 7 deltas of 3 bytes, rounded to even), bins 0 to 4 give a smooth path. Y rises from 0 to 0.0785 and the derived W stays between 1.0 and 0.9975. |q| is 1.00000 at every key. **[verified]**
- The last bin (frame 40, a single key) reads as Z ≈ −1, which breaks the path. Either the partial-bin layout is different or that key is not on the path. **[unconfirmed]**

**Needed from Ghidra:** the dequantisation in `DeltaQPhysical::UnQuantize` and `DeltaQDelta::UnQuantize`, the W recovery in `DeltaQRecoverW`, the partial-bin layout in `DeltaQ::GetConstBoneIdx`, and `FnDeltaQ::EvalSQTMasked`, which no decomp file defines.

## 12. DeltaQFast (type 18) and DeltaSingleQ (type 19) **[verified: not used by the install]**

The scan of all banks found no leaf of either type. The decomp has layout headers (`DeltaQFast.h`, `DeltaSingleQ.h`), but the implementation files `FnDeltaQFast.cpp` and `FnDeltaSingleQ.cpp` are 0 bytes. Their decode rules are **[unconfirmed – needs Ghidra]** and are not specified here.

## 13. Edge cases **[decomp] [verified]**

- **Frame 0.** Types 10 and 11 return the DofInfo start values. Types 21 and 20 return the bin's physical key.
- **Negative time.** The floor frame is clamped to 0.
- **Time at or past the last frame.** The value at the last frame is returned, with no interpolation.
- **Looping.** The codecs never wrap. Looping is the caller's job.
- **Going backwards.** The decomp resets to the bin key or frame 0 and accumulates again. The stateless rule gives the same values. For DeltaF1 and DeltaF3 the reverse-sum switch in the decomp only changes which path is taken, not the result.
- **Key-time tables.** Every install channel has a null key-time table. The decomp has a path for tables. It is not verified here. **[decomp only]**
- **One frame.** There is no interpolation.

## 14. Worked examples **[verified]**

All values are from the airliner, `TRACKS\L2RA.BUN`, object at file offset 1,409,088.

**Example A: `_q` channel, frame 0 and frame 1.** The DeltaCompressedData block is at object offset 0x9C. Its header bytes are `04 00 08 00`: 4 dofs, 8 bits, a padding byte. The DofInfo records start at 0xA0:

| Dof | Bytes (min, range, start) | Minimum | Range | Start |
|---|---|---|---|---|
| 0 | 80 92 B2 BB, BC 6B BB 38, D9 A0 2C BE | −0.00544959307 | 8.93692195e-05 | −0.168582335 |
| 1 | E8 47 28 BC, 96 2F 9A 38, 0D D8 88 B5 | −0.01027105 | 7.35215581e-05 | −1.01956687e-06 |
| 2 | 98 C8 77 BD, 7B 89 02 3A, 69 07 C8 36 | −0.0604940355 | 0.000497959263 | 5.96132713e-06 |
| 3 | BB 2B B9 BD, 88 CC B9 39, 05 56 7C 3F | −0.0904154405 | 0.000354383374 | 0.985687554 |

The first delta row (at 0xD0) is `3D 88 8D FF`: raw deltas 61, 136, 141, 255.

- Frame 0 quaternion: (−0.168582335, −1.01956687e-06, 5.96132713e-06, 0.985687554).
- Frame 1 quaternion: (−0.168580406, −0.000273137704, 0.00972418195, 0.985639874).
- Frame 2 quaternion: (−0.168578476, −0.00098638519, 0.037368936, 0.984883427).

**Example B: `_t` channel, frame 0 and frame 1.** The DeltaF3 object is at 0x210. Its DofInfo is 36 bytes at 0x224:

- Physical minimum (x, y, z): −18.6462765, −2003.93237, 12.8326864.
- Physical range (x, y, z): 4643.01416, 3645.07251, 822.481384.
- Quantised minimum: 29945, 30526, 29389. Quantised range: 5520, 4887, 6034.

Bin 0's physical key is at 0x248: raw (258, 36170, 0), bytes `02 01 4A 8D 00 00`.

- Frame 0 translation: (−0.367529952, 7.85175266, 12.8326864).
- The first delta row is raw (124, 96, 151). Under the cumulative rule, frame 1 translation: (−19.9590455, −36.8322404, 17.7166084).

## 15. Reconciliation with `eagl-animation.md` **[verified]**

These points correct or extend the earlier spec:

- The compound's 12-byte attribute block is not the codec header. The codec header of the airliner's `_q` leaf is at 0x9C (bytes `04 00 08 00`), not at 0x90.
- The earlier 16-byte "record header" is wrong. The layouts are in sections 2 to 9 of this spec.
- The "8-byte keys" that the earlier §4 test read were the DofInfo floats and 8-bit delta rows. That is why the 16-bit reading gave no smooth motion.
- The 14-byte key and the nibble delta in the earlier §3.1 and §3.2 are not part of any codec in this spec. Those two decomp functions are not called from any codec class here. Their owner is **[unconfirmed – needs Ghidra]**.

## 16. Open questions **[unconfirmed]**

1. DeltaQ: the partial-bin layout, the dequantisation of keys and deltas, and the W recovery (section 11).
2. DeltaQFast and DeltaSingleQ: no install instance, no implementation file (section 12).
3. The meaning of the 12-byte compound attribute block (section 2).
4. The exact component order of the quaternion (section 7), and the slot meanings (section 3).
5. Units of the translation channels. The file does not state them (section 9).
6. The 4-bit row format has no install instance to check (section 4).

## 17. Sources

**Decomp** (CC0, `dbalatoni13/nfsmw`, via the GitHub API; not saved to disk), under `src/Speed/Indep/Src/EAGL4Anim/`:

- `DeltaChan.h` (11,147 bytes), `DeltaChan.cpp` (19,905 bytes): types 10 and 11, incremental decode, lerp.
- `DeltaCompressedData.h` (2,266 bytes), `DeltaCompressedData.cpp` (14,705 bytes): the data block, the 8, 16 and 4-bit rows.
- `DeltaF1.h` (4,611 bytes), `FnDeltaF1.h` (2,496 bytes), `FnDeltaF1.cpp` (15,301 bytes): type 21.
- `DeltaF3.h` (5,194 bytes), `FnDeltaF3.h` (2,516 bytes), `FnDeltaF3.cpp` (18,218 bytes): type 20.
- `DeltaQ.h` (4,196 bytes), `FnDeltaQ.h` (2,426 bytes), `FnDeltaQ.cpp` (898 bytes): type 17, with empty bodies.
- `DeltaQFast.h` (4,369 bytes), `FnDeltaQFast.h` (3,239 bytes), `FnDeltaQFast.cpp` (0 bytes): type 18, not used.
- `DeltaSingleQ.h` (4,160 bytes), `FnDeltaSingleQ.h` (2,146 bytes), `FnDeltaSingleQ.cpp` (0 bytes): type 19, not used.
- `CompoundChannel.h` (5,278 bytes): type 15.
- `AnimTypeId.h`, `AnimMemoryMap.h`, `AnimUtil.h`, `eagl4AnimBank.h`, `FnAnimMemoryMap.h`: type numbers and base header.

**Ghidra** (`speed.exe` 1.3, image base 0x400000), supplied by the supervisor **[exe]**: `FnDeltaF3::InitBuffersAsRequired` at 0x004968E0. Its DofInfo layout and scale formulas are in section 9.

**Install** (read only): `TRACKS\L2RA.BUN` (object at 1,409,088, 1,504 bytes; plus the other 13 banks), `NIS\Scene_ArrestF02_BundleB.bun`, `NIS\Scene_WMWaterTower_BundleB.bun`, `GLOBAL\InGameB.bun` and `GLOBAL\INGAMEC.BUN`.

**Prototype.** Python scripts in the session scratchpad, folder `s3`, not in the repository: `elf.py`, `scan_banks.py`, `scan_all.py`, `decode_airliner.py`, `f3_hyp.py`, `f1_lerp.py`, `deltaq_probe.py`, `deltaq_keys.py`, `worked.py`.
