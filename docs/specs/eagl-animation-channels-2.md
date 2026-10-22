# EAGL non-delta animation channels, part 2

Sections 9 to 17 and the sources. Sections 0 to 8 (summary, install measurements, bank dispatch, Compound, StatelessQ, StatelessF3, RawPose, RawEvent, RawLinear) are in [eagl-animation-channels.md](eagl-animation-channels.md).

## 9. RawState (type 16)

Decomp: `RawStateChan.h`. `RawStateChan.cpp` is 0 bytes.

### 9.1 Record layout (declared total 0x0C) **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type and checksum |
| +0x04 | u16 | number of frames |
| +0x06 | u16 | number of keys |
| +0x08 | u8 | number of fields |
| +0x09 | u8 | key size in bytes |
| +0x0A | u16 × fields | decode table (one entry per field) |

Each decode entry packs three values: the stored bit count (as a power of two), the destination byte count and
the destination byte offset. The key data (key size bytes per key) follows the decode table. **[decomp]** The
exact packing and placement are **[unconfirmed]**.

### 9.2 Wrapper and evaluation **[unconfirmed – needs Ghidra]**

The wrapper keeps a key index (`mKeyIdx`, +0x10). `FnRawStateChan::EvalState` and `FnRawStateChan::FindTime`
are declared, and `RawStateChan::Decode` unpacks a key. Their bodies are in `RawStateChan.cpp` (0 bytes). The
output is a state record (the `State` struct is empty in the header), so its use is not known. **[unconfirmed]**

### 9.3 Install

No RawState record exists in the install. **[verified]**

## 10. Phase (type 14)

Decomp: `PhaseChan.h`. `PhaseChan.cpp` is 0 bytes.

### 10.1 Record layout (declared total 0x0C) **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type and checksum |
| +0x04 | u16 | number of frames |
| +0x06 | u16 | start time (frames) |
| +0x08 | u8 | flags: invalid, starts on the right foot, cycle animation, angle sample rate. The bit masks are not in the decomp read. **[unconfirmed]** |
| +0x09 | u8 | number of cycles |
| +0x0A | u8 × 2 | cycle entries (declared size 2; the array is longer in practice) |

Following the cycle entries are the sampled angles, one byte per sample. The angle sample rate comes from the
flags. A byte maps to a float angle; the mapping is not in the decomp read. **[unconfirmed]**

### 10.2 Wrapper and evaluation **[unconfirmed – needs Ghidra]**

The wrapper (`FnPhaseChan`) keeps an index (`mIdx`, +0x10), the current frame (`mCurrentFrame`, +0x12), a
right-foot flag (`mRight`, +0x14) and a sample-rate value (`mSampleRate`, +0x18, default 1). Its Eval writes a
phase value (one float, the `PhaseValue` struct). `FnPhaseChan::Eval`, `GetLength`, `SetAnimMemoryMap` and
`PhaseChan::FindMatchTime` are not read (0-byte `.cpp`).

### 10.3 Use in Compound **[decomp]**

Compound's EvalPhase calls every child's EvalPhase, and GetPhaseChan returns the first phase channel found
(section 3.4).

### 10.4 Install

No Phase record exists in the install. **[verified]**

## 11. Cycle (type 3)

Decomp: `FnCycle.h`. It is an FnAnim, not a bank record. It is created by the pool manager from the type number,
and set with a child and a start and end time. **[decomp]**

### 11.1 Wrapper fields **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x0C | f32 | start time |
| +0x10 | f32 | end time |
| +0x14 | f32 | length |
| +0x18 | pointer | child animation |

### 11.2 Evaluation **[unconfirmed – needs Ghidra]**

The header has `Eval`, `EvalEvent`, `EvalSQT` and `EvalPhase` with empty bodies, plus a private helper
`GetInRangeTime`. There is no `FnCycle.cpp` in the directory, so the bodies are not in the decomp read. The
probable purpose is to repeat the child between the start and end times. **[unconfirmed]**

### 11.3 Install

Not a bank record. **[verified]**

## 12. CsisEvent (type 24)

Decomp: `CsisEventChannel.h`, `CsisEventChannel.cpp` (full body), `FnCsisEventChannel.h`, `Csis.h`,
`Event.h`, `EventHandler.h`. The evaluation below is read from the decomp. **[decomp]**. The install has no
CsisEvent record and no CSIS dictionary in any NIS bank. **[verified]** So nothing below was tested on data.

### 12.1 Channel record layout (total 0x14) **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type and checksum |
| +0x04 | i32 | number of events |
| +0x08 | pointer | CSIS dictionary (section 12.2) |
| +0x0C | pointer | data index array (u16 per event) |
| +0x10 | pointer | data block (u8) |
| +0x14 | 8 bytes × N | events (section 12.3) |

### 12.2 CSIS dictionary **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | i32 | number of event entries |
| +0x04 | pointer | start value per parameter (i32 array) |
| +0x08 | pointer | bit width per parameter (u8 array) |
| +0x0C | 12 bytes × N | event entries (section 12.2.1) |

#### 12.2.1 Event entry

| Offset | Type | Field |
|---|---|---|
| +0x00 | i32 | event id (the key the handler table is indexed by) |
| +0x04 | u16 | number of parameters |
| +0x06 | u16 | interface CRC (a signature check, not used in the evaluation) |
| +0x08 | i32 | index of this event's first parameter in the two parameter arrays |

Parameter k of an event uses entry paramIdx + k in both parameter arrays. **[decomp]**

### 12.3 Event record (8 bytes) **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16 | global index (into the dictionary's event entries) |
| +0x02 | u16 | local index (into the data index array) |
| +0x04 | f32 | trigger time |

Events are expected in ascending trigger time. The cursor logic in section 12.4 assumes this. **[inferred]**

### 12.4 Evaluation **[decomp]**

The channel keeps two wrapper values: the cursor (the index of the next event to consider) and the last time it
was evaluated (starting at 0). Given the previous time p and the current time c:

1. If p is less than the last evaluated time: move the cursor back. Scan backwards from the cursor for the last
   event whose trigger time is at most p. The cursor becomes that index plus one (0 if there is none).
2. Scan forwards from the cursor while the trigger time is at most p. Let i be the first index whose trigger
   time is greater than p (or the event count).
3. Set the cursor to i.
4. If p equals c: if i is past the end, set the cursor to the last index. Scan backwards for the last event whose
   trigger time is less than c, and set the cursor to that index plus one. This makes events exactly at c fire
   in step 6.
5. Otherwise, if c is less than p (time moved backwards): fire every event from i to the end, passing time c,
   then set the cursor to 0.
6. Fire, in order, every event from the cursor while its trigger time is at most c, passing time c.
7. Set the cursor to the first index not fired; if that is past the end, set it to the last index.
8. Store c as the last evaluated time.

Note on step 5: the code fires the events after p, then restarts from index 0 in step 6. So events at or before c
can fire again after a backwards step. This is read from the code. **[decomp]** It has not been tested.

### 12.5 Firing an event **[decomp]**

Firing an event is a sequence of steps. The steps are the same for every event.

1. Look up the event id from the dictionary entry of the global index. The trigger time comes from the event
   record. The duration and parameter are set to 0.
2. Look up the parameter count and the interface CRC from the same dictionary entry.
3. Start reading the data block at the byte given by the data index array, using the local index. The bit
   cursor starts at bit 0 of that byte.
4. For each parameter in order: read w bits, where w is the bit width of that parameter. The bits are read
   least significant first, and the cursor moves on to the next byte when it passes bit 7. The cursor is not
   reset between parameters. The value is the start value plus the bits read, as an integer.
5. Call the handler for the event id, if one is registered in the caller's table. Pass the current time c (not the
   trigger time), an event record with the event id and the trigger time, and a CSIS data record. If no handler is
   registered, nothing happens. **[decomp]**

The CSIS data record carries the caller's user-data pointer, the interface CRC, the parameter count and the
parameter values (an integer array of up to 255 entries). **[decomp]**

### 12.6 What a CSIS event carries

| Field | Meaning | Status |
|---|---|---|
| Event id | integer key into the caller's handler table | **[decomp]** |
| Trigger time | time from the event record; the unit is whatever the caller passes in. Under a Compound with FPS on, the child sees frame numbers, so the unit is frames. | **[decomp]** arithmetic |
| Fire time | the current time c at the moment the event fires | **[decomp]** |
| Interface CRC | signature check for the parameter list | **[decomp]** (not used in evaluation) |
| Parameters | up to 255 integers, bit-packed in the data block | **[decomp]** |
| Meaning of the id and parameters (sound, cue, etc.) | not in the decomp read; the handler table is filled by game code | **[unconfirmed]** |

### 12.7 Install

No CSIS record and no CSIS dictionary exist in the install. **[verified]** So the meaning of the event ids and
parameters is not known for any real asset. **[unconfirmed]** Where NIS audio cues come from is therefore an open
question, not something this spec answers.

## 13. PoseAnim (type 25)

Decomp: `PoseAnim.h`, `FnPoseAnim.h`, `PosePalette.h`. `FnPoseAnim.cpp` and `PoseAnim.cpp` are 0 bytes.

### 13.1 Record layout (total 0x10) **[decomp]**

| Offset | Type | Field |
|---|---|---|
| +0x00 | u16, u16 | type and checksum |
| +0x04 | u16 | number of keys |
| +0x06 | u8 | palette index |
| +0x07 | u8 | padding |
| +0x08 | pointer | pose index array (u8 per key) |
| +0x0C | pointer | key times (u16 per key) |

### 13.2 Palettes **[decomp]**

A palette bank has an 8-byte header: skeleton checksum (u16), animation checksum (u16), number of palettes
(u16), padding (2 bytes). The palette pointers follow; their placement is not in the decomp read. **[unconfirmed]**

Each palette is 0x0C bytes: a float pointer to the pose data, a u16 pointer to the degree-of-freedom index
array, then three u8 counts (poses, quaternion DOFs, translation DOFs) and one padding byte.

### 13.3 Evaluation and install **[unconfirmed – needs Ghidra]**

`FnPoseAnim::EvalPose` selects a pose from the palette by the key time, and writes the SQT buffer. Its body is
not in the decomp read. The wrapper sits 16 bytes before the record **[decomp]**. No PoseAnim record exists in
the install. **[verified]**

## 14. Blenders (types 4 to 9), briefly

These are runtime classes built by game code. They have no bank record. **[decomp] [verified: no record in
the install]**

- **EventBlender (4):** blends between two child animations, each with a time offset, a transition start and a
  duration. The trigger type chooses which children apply: first only (0), second only (1), or both (2). The
  body is in `FnEventBlender.cpp` (0 bytes). **[unconfirmed – needs Ghidra]**
- **Graft (5):** holds an array of child animations and an output count. Its Eval calls each child on the same
  output buffer, in index order, so a later child overwrites an earlier one. **[decomp]**
- **PoseBlender (6):** blends two child poses, with a weight from a transition start, a duration and an end time.
  It can align the root bone using an XZ projection. Blends are quaternion (`BlendQ`) or linear (`Blend`), with a
  bone mask. The bodies are in `FnPoseBlender.cpp` (0 bytes). **[unconfirmed – needs Ghidra]**
- **PoseMirror (7):** Eval runs the child into a buffer. If mirroring is on, it mirrors the pose through the
  skeleton's mirror function, into the output; otherwise it passes the child's output through. **[decomp]**
  (`Skeleton::MirrorPose` is not read.)
- **RunBlender (8):** blends running cycles, using two alignment frames, two cycle times, a frequency, a speed
  range and a root quaternion. The body is in `FnRunBlender.cpp` (0 bytes). **[unconfirmed – needs Ghidra]**
- **TurnBlender (9):** blends turn cycles between a begin and an end facing. The body is in `FnTurnBlender.cpp`
  (0 bytes). **[unconfirmed – needs Ghidra]**

## 15. Evaluation status by type

| Type | Layout | Evaluation | Install |
|---|---|---|---|
| 0 RawPose | decomp | unconfirmed (0-byte `.cpp`) | none |
| 1 RawEvent | decomp | unconfirmed (0-byte `.cpp`) | none |
| 2 RawLinear | decomp | unconfirmed (no source file) | none |
| 14 Phase | decomp | unconfirmed (0-byte `.cpp`) | none |
| 15 Compound | verified | decomp, full | 848 |
| 16 RawState | decomp | unconfirmed (0-byte `.cpp`) | none |
| 22 StatelessQ | decomp | unconfirmed (0-byte `.cpp`) | none |
| 23 StatelessF3 | decomp | unconfirmed (0-byte `.cpp`) | none |
| 24 CsisEvent | decomp | decomp, full | none |
| 25 PoseAnim | decomp | unconfirmed (0-byte `.cpp`) | none |

## 16. Corrections to existing docs

1. **[verified]** [eagl-animation.md](eagl-animation.md) section 3.0a says the rotation record of the airliner
   is type 4 and the translation and scale records are type 3. Type 3 is Cycle and type 4 is EventBlender in the
   decomp enum. Neither is a bank record. In the install, every `_q` child is a delta (DeltaQuat, 279 of 280), and
   no `_s` or `_t` child is type 3 or 4. The type-4 vtable reading in section 3.0 is a runtime class, not a bank
   codec. The section should be rewritten.
2. **[verified]** [formats/animation.md](../formats/animation.md) says the name suffix probably selects the channel
   codec. It does not select one codec: `_s` holds DeltaLerp (204), DeltaF3 (78), DeltaF1 (36) and DeltaQuat (64)
   children, and `_t` holds DeltaLerp (271), DeltaF3 (12) and DeltaF1 (3).
3. **[verified]** [eagl-animation.md](eagl-animation.md) section 2 reads the field at +0x0C of a channel as a data
   record, and reads a rotation record at 0x90 for the airliner. For a Compound record, +0x0C is its first child
   pointer, and +0x04 is an attribute block. In the NIS banks checked, the block at +0x04 is the FPS block. The
   airliner reading should be checked against this layout. This spec did not check the airliner bank.

## 17. Open questions

Each item names the function or field that must be found, with its file.

1. `FnStatelessQ::Eval`, `EvalSQT`, `EvalSQTMask`, `StatelessQ::Compress2UShort`, `StatelessQ::Uncompress2Float`
   (`FnStatelessQ.cpp`, `StatelessQ.cpp`: 0 bytes). **[unconfirmed – needs Ghidra]**
2. `FnStatelessF3::Eval`, `EvalSQT`, `EvalSQTfast`, `EvalSQTMask`, `StatelessF3::UnQuantize`,
   `StatelessF3::GetArrays` (`FnStatelessF3.cpp`, `StatelessF3.cpp`: 0 bytes). Also the frame stride and the
   position of the range table. **[unconfirmed – needs Ghidra]**
3. `RawPoseChannel::Eval`, `EvalFrame`, `EvalInterpFrame`, `FnRawPoseChannel::EvalSQT`, `FnRawPoseChannel::GetLength`
   (`RawPoseChannel.cpp`, `FnRawPoseChannel.cpp`: 0 bytes). **[unconfirmed – needs Ghidra]**
4. `RawEventChannel::Eval` (`RawEventChannel.cpp`: 0 bytes), and the placement of the event array. **[unconfirmed – needs Ghidra]**
5. `RawLinearChannel::Eval`, `EvalInterpFrame`, `EvalFrame`, and the array layout (no source file in the
   directory). **[unconfirmed – needs Ghidra]**
6. `RawStateChan::Decode`, `FnRawStateChan::EvalState`, `FnRawStateChan::FindTime` (`RawStateChan.cpp`: 0 bytes).
   **[unconfirmed – needs Ghidra]**
7. `FnPhaseChan::Eval`, `FnPhaseChan::GetLength`, `PhaseChan::FindMatchTime` (`PhaseChan.cpp`: 0 bytes), and the
   flag bit masks and angle mapping. **[unconfirmed – needs Ghidra]**
8. `FnCycle::Eval`, `EvalSQT`, `EvalPhase`, `GetInRangeTime` (no source file). **[unconfirmed – needs Ghidra]**
9. `FnPoseAnim::EvalPose`, `FnPoseAnim::GetLength`, `PoseAnim::InitAnimMemoryMap` (`FnPoseAnim.cpp`,
   `PoseAnim.cpp`: 0 bytes), and the palette pointer placement. **[unconfirmed – needs Ghidra]**
10. `MemoryPoolManager::InitAnimMemoryMapAux` does not attach a CSIS dictionary. The setter `CsisEventChannel::SetCsisDictionary`
    has no caller in the decomp read. The caller must be found. **[unconfirmed]**
11. The meaning of CSIS event ids and parameters for real NIS audio. No CSIS record exists in the install, so this
    needs game-side handler code or a sample that uses CSIS. **[unconfirmed]**
12. The blender bodies in `FnEventBlender.cpp`, `FnPoseBlender.cpp`, `FnRunBlender.cpp` and `FnTurnBlender.cpp`
    (0 bytes). **[unconfirmed – needs Ghidra]**

## Sources

Decomp (dbalatoni13/nfsmw, read through the GitHub API, not saved to disk), under
`src/Speed/Indep/Src/EAGL4Anim/`:

- Headers read in full: `StatelessQ.h`, `StatelessF3.h`, `FnStatelessQ.h`, `FnStatelessF3.h`, `RawPoseChannel.h`,
  `FnRawPoseChannel.h`, `RawLinearChannel.h`, `FnRawLinearChannel.h`, `RawEventChannel.h`, `FnRawEventChannel.h`,
  `RawStateChan.h`, `CompoundChannel.h`, `PhaseChan.h`, `FnCycle.h`, `CsisEventChannel.h`, `FnCsisEventChannel.h`,
  `Csis.h`, `PoseAnim.h`, `FnPoseAnim.h`, `PosePalette.h`, `FnEventBlender.h`, `FnGraft.h`, `FnPoseBlender.h`,
  `FnPoseMirror.h`, `FnRunBlender.h`, `FnTurnBlender.h`, `Event.h`, `EventHandler.h`, `AnimMemoryMap.h`,
  `FnAnimMemoryMap.h`, `AnimTypeId.h`, `FnAnim.h`, `eagl4AnimBank.h`, `eagl4AnimBank.cpp`, `eagl4FnAnimBank.h`,
  `MemoryPoolManager.h`, `Attribute.h`, `AttributeId.h`.
- Bodies read: `CompoundChannel.cpp` (5,345 bytes), `CsisEventChannel.cpp` (3,613 bytes), and the
  `MemoryPoolManager.cpp` dispatch functions `NewFnAnimAux` and `InitAnimMemoryMapAux` (9,934 bytes in the file).
- 0-byte `.cpp` files (bodies not available): `FnStatelessQ.cpp`, `FnStatelessF3.cpp`, `StatelessQ.cpp`,
  `StatelessF3.cpp`, `RawPoseChannel.cpp`, `FnRawPoseChannel.cpp`, `RawEventChannel.cpp`, `RawStateChan.cpp`,
  `PhaseChan.cpp`, `FnPoseAnim.cpp`, `PoseAnim.cpp`, `FnEventBlender.cpp`, `FnPoseBlender.cpp`, `FnRunBlender.cpp`,
  `FnTurnBlender.cpp`.
- No source file in the directory listing for `RawLinearChannel` or `FnCycle`.

Install measurements (`D:\Need For Speed Most Wanted Black Edition`, read only), made with scripts in
`C:\Users\PADO\AppData\Local\Temp\claude\C--Users-PADO-github-nfs-mw\324026bd-a58e-44f9-b41b-bf1a34331658\scratchpad\s4\`:

- Chunk walk and ELF relocation: `eagl.py`, using `tools/chunkdump.py` for the chunk format and JDLZ inflate.
- Bank census of the 87 NIS bundles and `TRACKS/L2RA.BUN`, `GLOBAL/InGameB.bun`: `scan_all.py`.
- Full-install scan of 105 `.bun` and `.lzc` files, with nested JDLZ inflate: `scan_install_inflate.py`
  (output `scan_install_inflate.out`). The first pass without inflate gave the same totals.
- Compound child types by name suffix: `suffix_map.py`.
- Compound attribute blocks and FPS: `attr_fps.py`.

Existing docs: [formats/animation.md](../formats/animation.md), [eagl-animation.md](eagl-animation.md),
[../README.md](../README.md) (evidence tags).
