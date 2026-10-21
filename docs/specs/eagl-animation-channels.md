# EAGL non-delta animation channels

What the non-delta EAGL channel types are, how each one is laid out in a bank, and how each one computes its
output. "Non-delta" here means the types 0, 1, 2, 14, 15, 16, 22, 23, 24 and 25 that store plain or
stateless data. The delta types (10, 11, 17, 20, 21) are out of scope except where Compound uses them. The
blender types 4 to 9 are covered briefly in section 14, and the cycle type 3 in section 11.

Read first: [docs/formats/animation.md](../formats/animation.md) (where the banks sit in the install) and
[eagl-animation.md](eagl-animation.md) (the ELF container and the bank structure so far).

**Status:** the layouts below come from the decomp headers. Most evaluation bodies are in `.cpp` files that are
0 bytes in the decomp, so most evaluation rules are **[unconfirmed – needs Ghidra]**. The install does not
contain any stateless, raw, phase, CSIS or pose channel, so none of those layouts could be checked against
data. The only channel types that exist in the install are the delta types and Compound.

## 0. Summary

1. **The install has no non-delta channel.** Across all 105 `.bun` and `.lzc` files (nested JDLZ blobs
   inflated), the bank records are types 10, 11, 17, 20, 21 and 15 (Compound) and nothing else. There is no
   StatelessQ, StatelessF3, RawPose, RawEvent, RawLinear, RawState, Phase, CsisEvent or PoseAnim record.
   **[verified]**
2. **Compound is the only non-delta type with a full evaluation.** Its evaluation body is in the decomp
   (`CompoundChannel.cpp`). It visits its children from last to first, and multiplies time by the FPS value
   when UseFPS is on. **[decomp]**
3. **CsisEvent is fully specified from the decomp.** Its evaluation body and its parameter unpacking are in
   `CsisEventChannel.cpp`. An event carries an integer event id (from a dictionary), a trigger time, and up to
   255 integer parameters. The id indexes a table of handlers supplied by the caller. **[decomp]**. No CSIS
   channel exists in the install, so the meaning of the ids is not confirmed. **[unconfirmed]**
4. **Every Compound in the install is at 30 FPS.** All 848 Compound records carry one attribute, FPS (id 1),
   value 30. Their frame counts run from 2 to 2251, which is 0.07 s to 75 s. **[verified]**
5. **Compound children are stored inline.** The child pointers sit in an array that starts at +0x0C of the
   Compound record; the array is not a separate block. In all 848 Compounds the child count is 1 (784 of
   them), 2 (28) or 3 (36). **[verified]**
6. **The suffix does not pick the codec.** The `_s` channels hold DeltaLerp (204), DeltaF3 (78), DeltaF1 (36)
   and DeltaQuat (64) children. The `_q` channels hold DeltaQuat (279 of 280) and DeltaQ (1). **[verified]**
7. **Types 3 and 4 are not bank codecs.** The docs in [eagl-animation.md](eagl-animation.md) section 3.0a say
   `_q` is type 4 and `_s`/`_t` are type 3. Type 3 is Cycle and type 4 is EventBlender in the decomp enum, and
   neither is a bank record. See section 16. **[verified against the enum; the earlier doc is wrong]**
8. **Blenders and Cycle are built by game code, not stored.** Their classes do not derive from the bank record
   type, and no bank in the install contains them. **[decomp] [verified]**

## 1. Install measurements **[verified]**

| Measurement | Result |
|---|---|
| `.bun` and `.lzc` files scanned | 105 (nested JDLZ inflated) |
| Bank records found, by type | 10: 475, 11: 343, 15: 856, 17: 3, 20: 94, 21: 41 (node visits, roots and children) |
| Types 0, 1, 2, 12, 13, 14, 16, 18, 19, 22, 23, 24, 25 | 0 in every file |
| Banks with a non-null attribute dictionary or CSIS dictionary pointer (87 NIS files) | 0 |
| Compound records | 848 (87 NIS files, `TRACKS/L2RA.BUN`, `GLOBAL/InGameB.bun`) |
| Compound attribute blocks | 848, each with count 1 |
| The one attribute in every block | id 1 (FPS), size 1, value 30 |
| Compound frame counts | min 2, max 2251 |
| Compound child counts | 1: 784, 2: 28, 3: 36 |
| Compound checksum, `IntroNis05` bank | 0x1414 on the root, its children and all bank records |

The relocation check agrees with the earlier doc: every relocation in every object is R_MIPS_32, and every
symbol that a relocation refers to has value 0. So after the add, each pointer is an offset into `.data`.
**[verified]**

## 2. Bank records and dispatch

### 2.1 The bank header **[decomp] (fields checked on the install)**

The bank symbol points at a 0x1C-byte record:

| Offset | Type | Field |
|---|---|---|
| +0x00 | u32 | size of the bank record (0x1C). **[verified]** |
| +0x04 | i32 | number of animations. **[verified]** |
| +0x08 | pointer | attribute dictionary (0 in every NIS bank). **[verified]** |
| +0x0C | pointer | CSIS dictionary (0 in every NIS bank). **[verified]** |
| +0x10 | pointer | array of pointers to animation records, one per animation. **[verified]** |
| +0x14 | pointer | array of pointers to name strings, sorted by name. **[verified]** |
| +0x18 | pointer | statistics (0 on disk). |

The names are sorted, and the lookup is a binary search over the name array. **[decomp]**

### 2.2 The record header **[decomp] [verified]**

Every animation record starts with a four-byte header:

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16 | animation type (the `AnimTypeId` value, section 2.3). **[verified]** |
| +0x02 | u16 | target checksum. **[verified]** |

### 2.3 The type table

| Id | Name | Stored as a bank record? | Install count |
|---|---|---|---|
| 0 | RawPose | yes | 0 |
| 1 | RawEvent | yes | 0 |
| 2 | RawLinear | yes | 0 |
| 3 | Cycle | no, runtime class (section 11) | not in banks |
| 4 | EventBlender | no, runtime class (section 14) | not in banks |
| 5 | Graft | no, runtime class (section 14) | not in banks |
| 6 | PoseBlender | no, runtime class (section 14) | not in banks |
| 7 | PoseMirror | no, runtime class (section 14) | not in banks |
| 8 | RunBlender | no, runtime class (section 14) | not in banks |
| 9 | TurnBlender | no, runtime class (section 14) | not in banks |
| 10 | DeltaLerp | yes (delta, out of scope) | 475 |
| 11 | DeltaQuat | yes (delta, out of scope) | 343 |
| 12, 13 | KeyLerp, KeyQuat | not in scope | 0 |
| 14 | Phase | yes | 0 |
| 15 | Compound | yes | 856 |
| 16 | RawState | yes | 0 |
| 17 | DeltaQ | yes (delta, out of scope) | 3 |
| 18, 19 | DeltaQFast, DeltaSingleQ | not in scope | 0 |
| 20 | DeltaF3 | yes (delta, out of scope) | 94 |
| 21 | DeltaF1 | yes (delta, out of scope) | 41 |
| 22 | StatelessQ | yes | 0 |
| 23 | StatelessF3 | yes | 0 |
| 24 | CsisEvent | yes | 0 |
| 25 | PoseAnim | yes | 0 |

### 2.4 Turning a record into a runtime evaluator **[decomp]**

Two paths exist. Both come from the decomp.

1. **The bank-level helper** (`AnimBank::NewFnAnim`) handles only types 22, 23, 25 and 15. Every other type
   returns nothing.
2. **The pool manager** (`MemoryPoolManager::NewFnAnimAux`, for a record) handles every type:
   - For types 22, 23 and 25, the runtime wrapper is not allocated. It is the memory 24 bytes (StatelessQ,
     StatelessF3) or 16 bytes (PoseAnim) before the record. The record and its wrapper share one block.
   - For all other types, the wrapper is taken from the pool block for that type, built with the type's
     constructor, and given the record.

Initialisation of a freshly loaded bank (`MemoryPoolManager::InitAnimMemoryMapAux`) does the following:

- Compound: initialises its children recursively. **[decomp]**
- RawPose, StatelessQ, StatelessF3, PoseAnim: call the type's own initialiser. The bodies are not in the
  decomp read (0-byte `.cpp`), so what they do is **[unconfirmed – needs Ghidra]**. The initialisers are
  `RawPoseChannel::InitAnimMemoryMap`, `StatelessQ::InitAnimMemoryMap`, `StatelessF3::InitAnimMemoryMap` and
  `PoseAnim::InitAnimMemoryMap`.
- RawEvent, RawLinear, Phase, RawState, CsisEvent: nothing. **[decomp]**. For CsisEvent this means the bank's
  CSIS dictionary is not attached to the channel here. Something else must attach it (section 12.5).
  **[unconfirmed]**

## 3. Compound (type 15)

Evaluation is fully in the decomp (`CompoundChannel.h`, `CompoundChannel.cpp`, `FnCompoundChannel`).
**[decomp]**

### 3.1 Record layout (total 0x10 with one child) **[decomp] (layout verified on the install)**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type (15) and target checksum. **[verified]** |
| +0x04 | pointer | attribute block (section 3.2). **[verified]** |
| +0x08 | u16 | number of children. **[verified]** |
| +0x0A | u16 | number of frames. **[verified]** |
| +0x0C | pointer × N | child records, stored inline, N = child count. **[verified]** |

### 3.2 Attribute block **[verified layout]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u32 | attribute count (1 in every Compound). **[verified]** |
| +0x04 | 8 bytes × count | entries: u16 id, u16 size, then a 4-byte value field. |

The FPS attribute has id 1 (`AttributeId::ID_FPS`). In every Compound in the install it has size 1, and the
first byte of its value field is 0x1E = 30. The other three bytes of that field read 0x30 in the two blocks
checked. **[verified]**

### 3.3 Wrapper

The runtime wrapper (`FnCompoundChannel`, 0x1C bytes) holds a pointer to the child wrappers, a flag for
UseFPS, and a copy of the FPS value (0 until read). The children are wrapped lazily on first use, in reverse
order: the last child is wrapped first. **[decomp]**

### 3.4 Evaluation

The same pattern applies to every output method (SQT, pose, weights, 2D velocity, state, phase, events). The
only difference is which child method is called.

1. If the child wrappers do not exist yet, create them (section 3.3).
2. If UseFPS is on, multiply the input time by FPS (30 in the install). For the Eval and EvalEvent methods,
   both the previous time and the current time are multiplied. For the others only the current time is.
3. Call the same method on every child, in order from child index N-1 down to child index 0.
4. Combine the results by OR: the method returns true if any child returned true.

For Eval (the dof-writing path), the children all write into the same output buffer, in the order given in
step 3. **[decomp]**

Two methods differ from the pattern:

- **FindTime:** scale the start time by FPS (if UseFPS). Search children from last to first; on the first
  success, divide the result time by FPS (if UseFPS) and return true. **[decomp]**
- **GetPhaseChan:** return the first non-null phase channel found, searching children from last to first.
  **[decomp]**

### 3.5 Length

GetLength returns the frame count. If UseFPS is on, it divides by FPS, so the length in seconds is the frame
count divided by 30 in the install. **[decomp]** (arithmetic on the measured frame counts, section 1).

### 3.6 Setting FPS

UseFPS(u) stores the flag. If u is true and the FPS value is still 0, it reads the FPS attribute from the
attribute block. Otherwise it leaves the value alone. **[decomp]**

## 4. StatelessQ (type 22)

Decomp: `StatelessQ.h`, `FnStatelessQ.h`. The `.cpp` files for both are 0 bytes.

### 4.1 Record layout (total 0x18) **[decomp] (layout not measured on the install)**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type and checksum |
| +0x04 | pointer | attribute block |
| +0x08 | pointer | key times (u16 per key) |
| +0x0C | pointer | bone index array (u8) |
| +0x10 | pointer | companion data pointer (`mF3Ptr`; meaning not confirmed) **[unconfirmed]** |
| +0x14 | u16 | number of keys |
| +0x16 | u8 | number of animated bones |
| +0x17 | u8 | number of constant bones |

The record is random-access: a key time table selects the key. The 16-bit quaternion components are packed,
and the key stores one quaternion per animated bone. Constant bones are stored once. **[unconfirmed]**

### 4.2 Evaluation **[unconfirmed – needs Ghidra]**

The output is the SQT (scale, rotation, translation) buffer for the bones in the bone mask. The wrapper keeps a
previous-key index (`mPrevKey`, +0x10), a use-FPS flag (+0x12), an FPS value (+0x13) and a bone-mask pointer
(+0x14). Rules to find in `FnStatelessQ::Eval`, `FnStatelessQ::EvalSQT` and `FnStatelessQ::EvalSQTMask`, all in
`FnStatelessQ.cpp` (0 bytes): the key search, the interpolation, and the use of the previous key as a start.

### 4.3 Quantisation **[unconfirmed – needs Ghidra]**

`StatelessQ::Compress2UShort` maps a float to 16 bits, and `StatelessQ::Uncompress2Float` maps back. Their
range and the rounding are not in the decomp read (`StatelessQ.cpp`, 0 bytes).

### 4.4 Install

No StatelessQ record exists in the install. **[verified]** So no key time, bone count or companion data was
seen.

## 5. StatelessF3 (type 23)

Decomp: `StatelessF3.h`, `FnStatelessF3.h`. The `.cpp` files for both are 0 bytes.

### 5.1 Record layout (total 0x18) **[decomp] (layout not measured on the install)**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type and checksum |
| +0x04 | pointer | attribute block |
| +0x08 | pointer | key times (u16 per key) |
| +0x0C | pointer | degree-of-freedom index array (u16) |
| +0x10 | u16 | number of keys |
| +0x12 | u8 | number of animated degrees of freedom (bones) |
| +0x13 | u8 | number of constant degrees of freedom |
| +0x14 | 4 bytes | padding |

### 5.2 Range table **[decomp]**

The header defines a 0x20-byte range entry: four floats of minimum values, then four floats of range (the
fourth of each is probably unused). One entry per degree of freedom. **[decomp]** The position of the range
table relative to the record is not in the decomp read. **[unconfirmed]**

### 5.3 Frame data **[decomp] (the element type is confirmed; the layout is not)**

Each frame stores 16-bit signed values. Constant values are stored as floats. The frame stride is not in the
decomp read. **[unconfirmed]**

### 5.4 Decoding a vector **[unconfirmed – needs Ghidra]**

`StatelessF3::UnQuantize` takes a range entry and one frame's values, and writes a three-component vector. The
formula is in `StatelessF3.cpp` (0 bytes). The question "is the translation in metres" cannot be answered from
the decomp. A translation track was not found in the install, so it cannot be answered from the data either.

### 5.5 Evaluation **[unconfirmed – needs Ghidra]**

`FnStatelessF3::EvalSQT`, `EvalSQTfast` and `EvalSQTMask`, all in `FnStatelessF3.cpp` (0 bytes), are not read.
The wrapper's fields match StatelessQ's (+0x10 previous key, +0x12 use-FPS, +0x13 FPS, +0x14 bone mask).

### 5.6 Install

No StatelessF3 record exists in the install. **[verified]**

## 6. RawPose (type 0)

Decomp: `RawPoseChannel.h`, `FnRawPoseChannel.h`. The `.cpp` files for both are 0 bytes.

### 6.1 Record layout (total 0x10) **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type and checksum |
| +0x04 | i32 | signature size |
| +0x08 | i32 | frame size (floats per frame) |
| +0x0C | i32 | number of frames |

Following the header are two integer signature arrays (non-interpolated and interpolated) and a float data
array. The sizes come from the header. **[decomp]** The arrangement is not confirmed. **[unconfirmed]**

The header defines a channel kind: 0 quaternion, 1 Euler angles, 2 translation. **[decomp]** How the kind
is used is not in the decomp read. **[unconfirmed]**

### 6.2 Evaluation **[unconfirmed – needs Ghidra]**

`RawPoseChannel::Eval`, `RawPoseChannel::EvalFrame`, `RawPoseChannel::EvalInterpFrame`, all in
`RawPoseChannel.cpp` (0 bytes). The wrapper has a flag (`mInterp`, default true, +0x10) that chooses between
interpolated and stepped frames. **[decomp]** The interpolation is not read.

### 6.3 Install

No RawPose record exists in the install. **[verified]**

## 7. RawEvent (type 1)

Decomp: `RawEventChannel.h`, `FnRawEventChannel.h`, `Event.h`, `EventHandler.h`. `RawEventChannel.cpp` is 0 bytes.

### 7.1 Record layout (total 0x08) **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type and checksum |
| +0x04 | i32 | number of events |
| +0x08 | 16 bytes × N | events (see 7.2). The placement is not in the decomp read; the header size suggests it. **[unconfirmed]** |

### 7.2 Event record **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | i32 | event id |
| +0x04 | f32 | trigger time |
| +0x08 | f32 | duration |
| +0x0C | f32 | parameter |

### 7.3 Wrapper

The wrapper (`FnRawEventChannel`) keeps a cursor (`mCurrentIdx`, +0x10) and the time the cursor was last
advanced (`mCurrentTime`, +0x14). Its event method calls the channel's Eval with the cursor and the handler
table. **[decomp]** The signature is the same as CsisEvent's (section 12.4), but the algorithm is **[unconfirmed – needs Ghidra]**
(`RawEventChannel::Eval`, 0-byte `.cpp`).

### 7.4 Install

No RawEvent record exists in the install. **[verified]**

## 8. RawLinear (type 2)

Decomp: `RawLinearChannel.h`, `FnRawLinearChannel.h`. No `.cpp` for RawLinear is in the directory listing, so
the bodies of the channel methods are empty in the header. **[unconfirmed]**

### 8.1 Record layout (total 0x08) **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type and checksum |
| +0x04 | u16 | number of degrees of freedom |
| +0x06 | u16 | number of frames |

Following the header are a u16 degree-of-freedom index array and a float data array. The arrangement and
stride are not in the decomp read. **[unconfirmed]**

### 8.2 Length **[decomp]**

The wrapper's GetLength returns the frame count as a float. This is a count of frames, not seconds. There is no
FPS scaling in this path.

### 8.3 Evaluation **[unconfirmed – needs Ghidra]**

`RawLinearChannel::Eval`, `RawLinearChannel::EvalInterpFrame` and `RawLinearChannel::EvalFrame`. Their bodies
are not in the decomp read.

### 8.4 Install

No RawLinear record exists in the install. **[verified]**


Sections 9 to 17 and the sources are in [eagl-animation-channels-2.md](eagl-animation-channels-2.md).
