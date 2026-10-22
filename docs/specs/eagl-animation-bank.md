# EAGL animation banks: layout, animation headers, codecs and census

What an EAGL animation bank is, how it sits in the install, how each animation's header and channels are laid out,
how the game finds an animation by name, what frame rate it samples at, and what the install actually contains.
The companion spec [eagl-animation.md](eagl-animation.md) is the older, partly wrong attempt; section 12 lists what
changed.

**Evidence tags** (as in [docs/README.md](../README.md#evidence-tags)):

- **[verified]**: measured on the install by the census scripts (quoted numbers).
- **[decomp]**: read in the CC0 decompilation (dbalatoni13/nfsmw). The file is cited each time.
- **[exe]**: from `speed.exe` through Ghidra. Not re-read here; no new exe facts are claimed.
- **[unconfirmed]**: a hypothesis or a rule whose meaning is not proven.

Where a decomp body is empty (`{}`), the rule is taken from the struct layout only and marked as such.

## 0. Summary

1. Every `EAGLAnimations` chunk (id `0x00E34010`) is an 8-byte pad of `0x11` bytes followed by an ELF32
   little-endian MIPS relocatable object. The object holds one bank symbol, `__AnimationBank:::bank`, whose value
   is the offset of the bank header in the object's `.data` section. **[verified]**
2. A bank is a sorted list of animations and their names. The bank header is 0x1C bytes. **[verified]** (layout
   matches [eagl4AnimBank.h](#sources)).
3. The type id is a u16 at offset 0 of every animation header. **In all 848 animations the type id is 15
   (Compound).** The codec of each channel is chosen by the type id of that channel's child record, not of the
   animation. Child type ids seen in the install: 10 (DeltaLerp), 11 (DeltaQuat), 17 (DeltaQ), 20 (DeltaF3) and
   21 (DeltaF1). **[verified]** for the counts, **[decomp]** for the names of the ids.
4. The names end in `_t`, `_q` or `_s`. The game reads them as DOF slots 0, 1 and 2 (translation, rotation, scale).
   The suffix does not decide the codec on its own: `_s` animations hold DeltaLerp children in 204 NIS cases and
   DeltaQuat or DeltaF3 children in the rest. **[verified]** for the counts, **[decomp]** for the slot order.
5. Each animation carries one FPS attribute, with the value 30 (byte 0x1E in memory). The game advances animation
   time at 30 steps per second, so a frame is 1/30 s. **[verified]** for the attribute, **[decomp]** for the 30.
6. Lookup by name hashes the name of every animation in every loaded bank until one matches. **[decomp]**
7. The census covers all 89 `.bun` files in the three groups. 87 files hold 125 EAGL objects, 848 animations and
   948 child channels. Every object passes the layout checks with zero errors. **[verified]** (section 11).

## 1. Method and sources

- The install was read as bytes only, with a short Python walker (scratchpad, not in the repo). Each file's
  bChunk tree was walked; every chunk with id `0x00E34010` was taken as one bank object. The 8 bytes of `0x11` and
  the ELF magic were checked for each object.
- Relocations: each R_MIPS_32 entry in the object's `.rel.data` adds the symbol's `.data` value to the 32-bit
  word it points at. All relocations in the install target `.data` (no unhandled relocation was found). After this,
  every pointer is a `.data` offset.
- Animation headers were read with the decomp's struct offsets (section 4). Child channel headers were read with
  the offsets of their own structs (section 6).
- The census script, its helper and the summary script are in the scratchpad folder `s2`. The per-row output is
  `census_rows.csv` and the per-object output `census_files.csv` in the same folder.
- Files read in the decomp: `src/Speed/Indep/Src/EAGL4Anim/` (`eagl4AnimBank.h/.cpp`, `eagl4FnAnimBank.h`,
  `FnAnim.h/.cpp`, `FnAnimFactory.h`, `AnimTypeId.h`, `FnDefaultAnimBank.h`, `AnimMemoryMap.h`, `FnAnimMemoryMap.h/.cpp`,
  `Attribute.h/.cpp`, `AttributeId.h`, `AttributeDictionary.h/.cpp`, `AnimationTarget.h`, `Event.h`, `EventHandler.h`,
  `CompoundChannel.h/.cpp`, `DeltaChan.h`, `DeltaCompressedData.h`, `DeltaF1.h`, `DeltaF3.h`, `DeltaQ.h`,
  `FnDeltaQ.h/.cpp`, `FnDeltaF1.h`, `FnDeltaF3.h`, `FnCycle.h`, `FnRawLinearChannel.h`, `RawLinearChannel.h`,
  `RawPoseChannel.h`, `PoseAnim.h`, `StatelessQ.h`, `StatelessF3.h`, `FnStatelessQ.h`, `FnStatelessF3.h`,
  `SkeletonData.h`, `ScratchBuffer.h`, `AnimUtil.h`, `ClipSet.h` (empty)). The `.cpp` files
  `FnStatelessQ.cpp`, `FnStatelessF3.cpp`, `FnDeltaQFast.cpp`, `FnDeltaSingleQ.cpp`, `FnPoseAnim.cpp`,
  `FnRawPoseChannel.cpp`, `RawPoseChannel.cpp`, `RawStateChan.cpp`, `AnimUtil.cpp` and `ClipSet.cpp` are 0 bytes
  in the decomp; statements about them come from headers only.
- Game side: `src/Speed/Indep/Src/Animation/` (`AnimBank.hpp/.cpp`, `AnimPart.hpp/.cpp`, `AnimInternal.hpp`,
  `AnimCtrl.cpp`, `AnimScene.cpp`, `AnimEntity_WorldEntity.cpp`, `AnimEntity_BasicCharacter.cpp`).

## 2. Where the banks are in the install **[verified]**

| Group | Files | Files with banks | Bank objects | Animations |
|---|---|---|---|---|
| `NIS\*.bun` | 87 | 85 (2 hold none: `Scene_FMVNoCars_BundleB.bun`, `Scene_Flythrough_BundleB.bun`) | 111 | 805 |
| `TRACKS\L2RA.BUN` | 1 | 1 | 13 | 39 |
| `GLOBAL\InGameB.bun` | 1 | 1 | 1 | 4 |
| **Total** | 89 | 87 | **125** | **848** |

- 59 NIS files hold one object, 26 hold two. The objects sit at the top level or inside containers. The
  `EAGLAnimations` chunk is a leaf with the 8-byte pad and then the ELF.
- Each object has exactly one bank symbol, `__AnimationBank:::bank`. Its symbol size field is 4; the bank header
  itself is 0x1C bytes long (section 3). **[verified]**
- Each object also has a version string symbol, `__EAGL_TOOLLIB_VERSION:::EAGL_TOOLLIB_VERSION-3`, which is not used
  by the game. **[verified]**
- The object's sections are `.data`, `.shstrtab`, `.strtab`, `.symtab` and `.rel.data`. There is no `.text` and no
  `.rodata`: names, headers and arrays are all in `.data`. **[verified]**
- Whether a bank is loaded: the game walks the bChunk tree and creates one bank per `EAGLAnimations` chunk whose
  aligned size is above zero. The bank is initialised from the aligned payload. **[decomp: `AnimBank.cpp`,
  `LoaderEAGLAnimations`]**

## 3. The bank header **[verified]**

The bank symbol points at a 0x1C-byte structure in `.data`. The decomp calls it `AnimBank`, with total size 0x1C.
**[decomp: `eagl4AnimBank.h`]**

| Offset | Size | Field | Meaning | Census value |
|---|---|---|---|---|
| 0x00 | u32 | `mSize` | size of the header, always 0x1C | 0x1C in all 125 objects |
| 0x04 | u32 | `mNumAnims` | number of animations in the bank | 2 to 30 per object (see section 11) |
| 0x08 | pointer | `mAttributeDictionary` | per-bank attribute names | null in all 125 |
| 0x0C | pointer | `mCsisDictionary` | CSIS event dictionary | null in all 125 |
| 0x10 | pointer | animation array | `mNumAnims` pointers to animation headers | always present |
| 0x14 | pointer | name array | `mNumAnims` pointers to NUL-terminated names | always present |
| 0x18 | pointer | `mStat` | runtime statistics | null in all 125 |

- The animation array and the name array are arrays of 32-bit pointers, one per animation, in that order: entry
  `i` of both arrays describes the same animation. They sit next to each other in `.data` in every object.
- The name array is sorted by byte value (strcmp order). The census checked every bank: **no bank failed the
  sort test**. The game's own binary search depends on this.
- No name appears twice in any bank. **[verified]**
- The bank header is the only place the count is stored. There is no separate count of channels.

## 4. The animation header **[verified: layout checked against 848 animations]**

Every animation starts with the same 4-byte base, the decomp's `AnimMemoryMap` (size 4). **[decomp: `AnimMemoryMap.h`]**

| Offset | Size | Field | Meaning |
|---|---|---|---|
| 0x00 | u16 | type id | which codec (section 5). The census found 15 (Compound) in all 848 |
| 0x02 | u16 | target checksum | the skeleton the animation drives (section 9) |

For a Compound (type 15) the decomp's struct continues as follows. **[decomp: `CompoundChannel.h`]**

| Offset | Size | Field | Meaning | Census |
|---|---|---|---|---|
| 0x04 | pointer | attribute block | per-animation attributes (section 10) | present in all 848, always one entry |
| 0x08 | u16 | child count | number of child channels | 1 (741 animations), 2 (28), 3 (36) in NIS; 1 everywhere else |
| 0x0A | u16 | frame count | length of the animation in frames | 2 to 2251 (section 11) |
| 0x0C | pointers | child array | one pointer per child channel (count from 0x08) | every child pointer in range |

- The compound header is 0x10 bytes long. The child pointers start at 0x0C and take 4 bytes each.
- The frame count is the animation's length. The decomp reports the same field as the animation's length in frames,
  and the census checked it against every child (section 11, check 2). **[decomp: `GetLength` in `FnCompoundChannel`]**

## 5. The type id and codecs **[verified: counts; decomp: names]**

The type id is the u16 at offset 0 of any animation or channel. The decomp's enumeration names the ids below.
**[decomp: `AnimTypeId.h`]** The table lists only the ids that the census or the code uses; the rest are in
[eagl-animation.md](eagl-animation.md) section 3.0a and are not used by the install.

| Id | Name in decomp | Install use (census) | Where it appears |
|---|---|---|---|
| 0 | RawPose | 0 | not used |
| 2 | RawLinear | 0 | not used |
| 10 | DeltaLerp | 475 child channels (NIS) | child of a compound |
| 11 | DeltaQuat | 330 (NIS), 13 (L2RA) | child of a compound |
| 15 | Compound | 848 animations (all groups) | top-level bank entry |
| 17 | DeltaQ | 1 (InGame) | child of a compound |
| 20 | DeltaF3 | 64 (NIS), 24 (L2RA), 2 (InGame) | child of a compound |
| 21 | DeltaF1 | 36 (NIS), 2 (L2RA), 1 (InGame) | child of a compound |

- The other ids (1, 3 to 9, 12 to 14, 16, 18, 19, 22 to 25) are not present in the install. Ids 22 and 23 (stateless
  rotation and vector) and 25 (pose animation) are in the decomp but do not occur. **[verified: 0 counts]**
- **The codec of each channel is picked by that channel's own type id.** The bank entry is always a Compound, which
  holds the channels. The game's factory builds one object per channel from the type id. **[decomp:
  `eagl4AnimBank.cpp`, `NewFnAnim` switches on the bank entry's type; the children's dispatch is in `FnAnimFactory.h`
  and was not read as code]**
- Type ids 3 (Cycle) and 4 (EventBlender) are not used by any bank. The old guess that `_q` is type 4 and `_s`/`_t`
  are type 3 is wrong (section 12).

## 6. Channels: child records **[verified: layouts checked by census; decomp: structs]**

Each compound child is one of the records below. Children are checked against the compound's frame count (section 11,
check 2). Not every field is decoded; the notes say which are read.

### 6.1 DeltaLerp and DeltaQuat (types 10 and 11) **[decomp: `DeltaChan.h`]**

Size 0x0C. The decomp calls this `DeltaChan`.

| Offset | Size | Field | Meaning | Census |
|---|---|---|---|---|
| 0x00 | u16 | type | 10 or 11 | as in section 5 |
| 0x02 | u16 | target checksum | same meaning as the animation's (section 9) | equal to the parent compound's checksum in all 948 children **[verified]** |
| 0x04 | pointer | compressed data | a `DeltaCompressedData` block (below) | non-null |
| 0x08 | u16 | frame count | frames in this channel | equal to the compound's frame count in all 948 children |
| 0x0A | u16 array | inline list after the frame count | DOF indices of this channel; length not in the header | first entries are small numbers, then non-index values in some banks |

The `DeltaCompressedData` block the child points to starts with a u16 count of DOFs (`mNumDofs`), then a u8 count of
quantisation bits, then the data. **[decomp: `DeltaCompressedData.h`]** The census read the count and bit width:

- DOF counts seen: 3, 4, 12, 192 (NIS, types 10 and 11).
- Quantisation bit widths seen: 8 and 16.
- The inline list after the frame count does not hold a clean DOF list in the sampled children (it gives small
  values, then values such as 5140 or 56277). Its length is not known from the header. **[unconfirmed]**

### 6.2 DeltaF3 and DeltaF1 (types 20 and 21) **[decomp: `DeltaF3.h`, `DeltaF1.h`]**

Size 0x14. These records carry their own DOF list.

| Offset | Size | Field | Meaning | Census |
|---|---|---|---|---|
| 0x00 | u16 | type | 20 or 21 | as in section 5 |
| 0x02 | u16 | target checksum | as above | equal to the parent's |
| 0x04 | pointer | DOF index list | array of u16, one per bone (count from 0x0E) | read in every child |
| 0x08 | pointer | times | optional array of key times; 0 means every frame is a key | read in every child (see the frame rule below) |
| 0x0C | u16 | frame count (`mNumFrames`) | frames if there is no times array | equal to the compound's count in all children |
| 0x0E | u16 | bone count | number of entries in the DOF index list | 1 and 13 seen in the sampled children; the full range was not counted |
| 0x10 | u8 | bin length power | the key block size (not used by the census) | not read |
| 0x11 | u8 | constant bone count | bones with constant values | not read |

- The DOF index list of every DeltaF3 and DeltaF1 child in the install holds **distinct** values within its own list
  (checked for all 64 + 36 NIS, 24 + 2 L2RA and 2 + 1 InGame children). Values seen: 8 to 490. The unit (bytes or
  entries of the pose buffer) is **[unconfirmed]**.

### 6.3 Frame rule for the children **[decomp: `GetLength` in `DeltaF1.h`, `DeltaF3.h`, `DeltaQ.h`]**

When the times pointer is null, the frame count is the count field. When it is present, the frame count is the entry
before the last of the times array plus one. The census used the same rule for every child. It did not count how
many of the 948 children took the times branch. **[decomp; census: open]**

### 6.4 Other child ids

Type 17 (DeltaQ) has one measured child (InGame). Its count is at offset 0x04 and its times pointer at 0x08. Its
frame count matched the compound. **[decomp: `DeltaQ.h`; verified: one child]**

## 7. Names and the `_t`, `_q`, `_s` suffix **[verified: census; decomp: slot order]**

Every bank entry name ends in `_t`, `_q` or `_s`. No name lacked a suffix. **[verified]**

- The game builds the name of a channel from a base name and a suffix. The base name is the car or actor name and
  the suffix picks the DOF slot: `_t` is slot 0, `_q` is slot 1, `_s` is slot 2. **[decomp: `AnimEntity_WorldEntity.cpp`,
  `GetAnimChannelHash`; `AnimScene.cpp` for the car controllers, where `_t` feeds slot 0 and `_q` slot 1]**
- The base name itself is not an animation in the bank. The car controller uses the base name only as its own key.
  **[decomp: `AnimScene.cpp`, `CreateCarAnimationControllers`]**

Counts by suffix and child codec (census, per child; a compound with several children counts once for each child):

| Group | Suffix | Animations | Child codecs observed |
|---|---|---|---|
| NIS | `_q` | 266 | DeltaQuat (11) in all 266 |
| NIS | `_t` | 271 | DeltaLerp (10) in all 271 |
| NIS | `_s` | 268 | DeltaLerp (10) 204; DeltaQuat (11) with DeltaF3 (20) 28; DeltaQuat with DeltaF3 and DeltaF1 (21) 36 |
| L2RA | `_q` | 13 | DeltaQuat (11) |
| L2RA | `_s` | 13 | DeltaF3 (20) |
| L2RA | `_t` | 13 | DeltaF3 (20) 11; DeltaF1 (21) 2 |
| InGame | `_q` | 1 | DeltaQ (17) |
| InGame | `_t` | 2 | DeltaF3 (20); DeltaF1 (21) |
| InGame | `_s` | 1 | DeltaF3 (20) |

- So `_q` is a rotation codec in every NIS and L2RA case, but the `_s` and `_t` suffixes do not select one codec.
  The suffix gives the slot; the child's type gives the codec.
- Each L2RA object has three animations, one per suffix (for example the airliner `ANM_AirlinerA_10_..._LB_10_q`,
  `LB_11_q`, `LB_12_q`, each with its `_s` and `_t`). **[verified]**

## 8. Looking an animation up by name **[decomp: `AnimBank.cpp`, `eagl4AnimBank.cpp`, `AnimCtrl.cpp`]**

Two lookups exist. The game uses the first.

1. **By name hash (used by the game).** The function that finds an animation by a 32-bit hash walks every loaded
   bank in load order, and within each bank every animation in index order. For each name it computes the string
   hash and compares it with the wanted hash. The first match gives the bank and the index. If nothing matches the
   lookup fails. The string hash is the game's `bStringHash` (defined in the bWare code, **not read here**).
   **[unconfirmed for the hash function; decomp for the walk]**
2. **By name string (decomp only).** The bank's own lookup is a binary search over the sorted name array, comparing
   strings in byte order (section 3). It returns the index or -1. The game code path shown in `AnimCtrl.cpp` does not
   call it. **[decomp]**

A controller asks for each channel by hash: the base name's hash chained with the suffix. Then it creates one channel
object per hash, in slot order. **[decomp: `AnimEntity_WorldEntity.cpp`; `AnimScene.cpp`]**

## 9. Binding to bones and targets **[decomp for the mechanism; checksum meaning unconfirmed]**

- **Target.** Each animation header (and each child) carries a u16 target checksum. The skeleton carries its own
  core checksum, and an animation is compatible with a skeleton when the two match. **[decomp: `SkeletonData.h`,
  `IsAnimCompatible`; the function bodies are empty, so the comparison is inferred from the names]**
- **Which checksum belongs to which skeleton.** Not decoded. The census shows the values and where they occur
  (section 11, table 11.4), but the hash that produces them is not read. **[unconfirmed]**
- **Bone list.** Each DeltaF3 and DeltaF1 child lists the bones (or pose entries) it drives in its DOF index list
  (section 6.2). For DeltaLerp and DeltaQuat children the list is inline and its length is not decoded (section 6.1).
  **[verified: list present and distinct; decomp: layout; the link to skeleton bone indices is unconfirmed]**
- **Per-bone data in a compound.** The decomp's `CompoundChannel` calls each of its children in turn, in reverse index
  order, passing the same output buffer, and ORs their "handled" results. **[decomp: `CompoundChannel.cpp`]** How the
  children's writes combine into one pose (overwrite or blend) is not verified.

## 10. Frame rate, duration and the FPS attribute

### 10.1 The attribute **[verified: census; decomp: attribute layout]**

- Each animation header points to an attribute block. The block has a u32 entry count, then entries of 8 bytes:
  id (u16), size (u16) and a 32-bit value. Entries are sorted by id. **[decomp: `Attribute.h`]**
- Every animation in the install has one entry, id 1 (the FPS attribute, `ID_FPS`), with size 1 (one byte). **[verified:
  805 NIS, 39 L2RA, 4 InGame, 848 in all]**
- The 32-bit value stored is 0x3030301E. In memory (little-endian) its bytes are `1E 30 30 30`. The first byte is 0x1E,
  which is **30**. **[verified]**
- The decomp's one-byte accessor reads the top byte of the value (`0x30`, which is 48). That contradicts the
  first-byte reading. The byte-order convention the PC build uses for one-byte attributes is **[unconfirmed]**. The
  30 frames per second in the next section is the stronger evidence for 30.

### 10.2 Sample rate **[decomp: `AnimCtrl.cpp`, `AdvanceAnimTime`]**

- The game advances animation time by the elapsed seconds times 30, times the controller's time scale. So animation
  time is counted in frames, at 30 per second. **[decomp]**
- The NIS scene timing uses the same rate: the start and end frames of a scene are converted with 1/30 per frame.
  **[decomp: `AnimScene.cpp`]**
- Evaluation takes the time in frames. The FPS-scaling path (the `UseFPS` switch in the compound and in
  `FnStatelessQ`/`FnStatelessF3`) multiplies the time by the FPS attribute, but no call site that switches it on was
  found in the decomp (code search over the repo). So the 30 in the attribute is not applied at run time in this
  build. **[decomp: negative search; unconfirmed that the search was complete]**

### 10.3 Duration

- The length of an animation in frames is its compound frame count (section 4). In seconds it is the frame count
  divided by 30. A 41-frame animation (`Rotate_360_q`, InGame) lasts 41/30 = 1.37 s. **[verified count; derived
  duration]**

## 11. Census **[verified]**

Scripts: `census.py`, `eagl_elf.py`, `summarize.py`, `stats.py` and the probes in the scratchpad `s2` folder.

### 11.1 Totals per group

| Group | Files | Files with banks | Objects | Animations | Frames (min / median / max) | Mean frames |
|---|---|---|---|---|---|---|
| NIS | 87 | 85 | 111 | 805 | 2 / 97 / 2251 | 213.2 |
| L2RA | 1 | 1 | 13 | 39 | 21 / 31 / 66 | 34.1 |
| InGame | 1 | 1 | 1 | 4 | 21 / 36 / 41 | 33.5 |
| **All** | 89 | 87 | 125 | 848 | | |

### 11.2 Top-level and child type counts

| Type | Name | NIS | L2RA | InGame | Total |
|---|---|---|---|---|---|
| 15 | Compound (bank entry) | 805 | 39 | 4 | **848** |
| 10 | DeltaLerp (child) | 475 | 0 | 0 | 475 |
| 11 | DeltaQuat (child) | 330 | 13 | 0 | 343 |
| 17 | DeltaQ (child) | 0 | 0 | 1 | 1 |
| 20 | DeltaF3 (child) | 64 | 24 | 2 | 90 |
| 21 | DeltaF1 (child) | 36 | 2 | 1 | 39 |
| all other ids | | 0 | 0 | 0 | 0 |

Children in total: 905 (NIS), 39 (L2RA), 4 (InGame), 948 overall.

### 11.3 Child frame counts by type

| Group | Type | Children | Min | Median | Max |
|---|---|---|---|---|---|
| NIS | 10 | 475 | 2 | 91 | 2251 |
| NIS | 11 | 330 | 2 | 103 | 2251 |
| NIS | 20 | 64 | 94 | 128.5 | 433 |
| NIS | 21 | 36 | 94 | 128.5 | 433 |
| L2RA | 11 | 13 | 21 | 31 | 66 |
| L2RA | 20 | 24 | 21 | 28.5 | 66 |
| L2RA | 21 | 2 | 41 | 53.5 | 66 |
| InGame | 17 | 1 | 41 | 41 | 41 |
| InGame | 20 | 2 | 31 | 36 | 41 |
| InGame | 21 | 1 | 21 | 21 | 21 |

### 11.4 Target checksums

| Group | Checksum (hex) | Animations |
|---|---|---|
| NIS | 1414 | 741 |
| NIS | 1658 | 52 |
| NIS | dbd5 | 12 |
| L2RA | 1414 | 36 |
| L2RA | 1400 | 3 |
| InGame | 1414 | 4 |

- Within one object the checksum is the same for every animation, except in 12 NIS arrest objects (the `Scene_Arrest*_BundleB`
  files). In those objects one bank holds animations with two different checksums, 1658 and dbd5. Example: in
  `Scene_ArrestF02_BundleB.bun`, `ArrestF02_Cuffs_s` has checksum dbd5, while `ArrestF02_FCop_s` and
  `ArrestF02_Perp_s` have 1658. All three are in the same bank object. **[verified]**
- The checksum 1414 is the common one: 85 NIS objects, 12 L2RA objects, the InGame object. **[verified]**

### 11.5 Children per animation (NIS)

- 1 child: 741 animations; 2 children: 28; 3 children: 36. In L2RA and InGame every animation has one child. **[verified]**

### 11.6 Consistency checks (all banks)

1. Relocations: no relocation of an unexpected kind and no relocation targeting a section other than `.data`.
   **0 found.** **[verified]**
2. Frame counts: every child's frame count equals its compound's frame count. **0 mismatches in 948 children.**
   **[verified]**
3. Pointer slots: every pointer in the bank's two arrays and every child pointer lies on a relocated slot, and every
   pointer is in range. **0 failures.** **[verified]**
4. Names are sorted in byte order and unique within each bank. **0 failures in 125 objects.** **[verified]**
5. Type ids are at most 25. The largest type id found is 21. **[verified]**
6. Bank size field is 0x1C in every object. **[verified]**

Result: **the layout holds for every bank in the install. No bank failed.** Every top-level entry is a Compound, so
no animation in the install needed a different header.

### 11.7 Examples (bank file, name, frame count)

Top-level entry (type 15):

| Bank file | Name | Frames |
|---|---|---|
| `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02_Cuffs_s` | 119 |
| `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02_FCop_s` | 119 |
| `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02_Perp_s` | 119 |
| `TRACKS\L2RA.BUN` | `ANM_TowerCraneArm_01_XO_TowerCraneArm_1b_01_q` | 21 |
| `GLOBAL\InGameB.bun` | `Rotate_360_q` | 41 |

Child channel by type (the frame count is the child's):

| Child type | Bank file | Name | Frames |
|---|---|---|---|
| 10 DeltaLerp | `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02ZPM_Car1_t` | 2 |
| 10 DeltaLerp | `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02ZPM_Cop1_s` | 2 |
| 10 DeltaLerp | `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02ZPM_Cop1_t` | 2 |
| 11 DeltaQuat | `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02_Cuffs_s` | 119 |
| 11 DeltaQuat | `TRACKS\L2RA.BUN` | `ANM_TowerCraneArm_01_XO_TowerCraneArm_1b_01_q` | 21 |
| 11 DeltaQuat | `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02_FCop_s` | 119 |
| 17 DeltaQ | `GLOBAL\InGameB.bun` | `Rotate_360_q` | 41 |
| 20 DeltaF3 | `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02_Cuffs_s` | 119 |
| 20 DeltaF3 | `TRACKS\L2RA.BUN` | `ANM_TowerCraneArm_01_XO_TowerCraneArm_1b_01_s` | 21 |
| 20 DeltaF3 | `GLOBAL\InGameB.bun` | `Rotate_360_t` | 41 |
| 21 DeltaF1 | `NIS\Scene_ArrestF02_BundleB.bun` | `ArrestF02_FCop_s` | 119 |
| 21 DeltaF1 | `TRACKS\L2RA.BUN` | `ANM_SailboatA_03_XO_SailboatA_VC_1b_03_t` | 66 |
| 21 DeltaF1 | `GLOBAL\InGameB.bun` | `Move_UnD_t` | 21 |

The 2-frame NIS animations (`ArrestF02ZPM_*`) are one static pose each. The 2251-frame maximum is in the NIS group.

## 12. Corrections to the older spec and the format doc

- **[eagl-animation.md] section 3.0a (the type 4 for `_q`, type 3 for `_s`/`_t` guess).** Wrong. Every bank entry is
  type 15 (Compound). The codec is on the children: `_q` is DeltaQuat (11) in every NIS and L2RA case, and `_s` and
  `_t` use DeltaLerp (10), DeltaF3 (20) or DeltaF1 (21). Types 3 and 4 do not occur. The retraction is marked in
  that file.
- **[eagl-animation.md] section 2 (the 16-byte "record header" at the channel's offset).** The field at 0x0C
  (`0x3030301e`) is the FPS attribute's inline value, not a pointer, and the frame count and byte fields listed there
  are not a header of the channel. The layout in section 4 of this spec is the one the census verified.
- **[eagl-animation.md] section 2, "the bank holds three channels".** Correct for the L2RA airliner object (three
  compounds, `_q`, `_s`, `_t`), but the channels are compound animations with one child each, not the bare records
  the older spec describes.
- **[formats/animation.md] "Animation names and codecs".** The table of codec classes is still valid as a list of
  decomp classes, but the suffix-to-codec line in that section was marked unconfirmed. The census now supports a
  narrower statement (section 7).

## 13. Open questions **[unconfirmed]**

1. The string hash used by the name lookup (`bStringHash`). It is in the bWare code, which was not read.
2. The meaning of the 16-bit target checksum: which hash of which skeleton name. Needed to bind an animation to a
   skeleton from its header alone.
3. The length of the inline DOF list in DeltaLerp and DeltaQuat children (section 6.1). The DeltaCompressedData block
   gives the DOF count, but the list's own length is not confirmed.
4. The unit of the DeltaF3 and DeltaF1 DOF index values: bones, pose floats or bytes (section 6.2).
5. The byte-order convention for one-byte attributes on PC (section 10.1), and whether the FPS value is applied at
   run time.
6. How many children take the times branch of the frame rule (section 6.3). The census did not count it.
7. The codec and frame rule for types 22, 23 and 25 (stateless and pose): not present in the install, so not measured.
   The decomp `GetNumFrames` bodies for those types are empty.
8. The `.cpp` files that are 0 bytes in the decomp (listed in section 1). Their behaviour is not stated here. None
   of them needs Ghidra for the layout in this spec; the open behaviour is the evaluators.

## Sources

- Install (read only): `D:\Need For Speed Most Wanted Black Edition\NIS\*.bun` (87 files),
  `TRACKS\L2RA.BUN`, `GLOBAL\InGameB.bun`.
- Decomp (CC0, dbalatoni13/nfsmw, read through the GitHub API, not saved to disk): `src/Speed/Indep/Src/EAGL4Anim/`
  and `src/Speed/Indep/Src/Animation/` files listed in section 1.
- Census scripts and outputs: scratchpad folder `s2` (`eagl_elf.py`, `census.py`, `summarize.py`, `stats.py`,
  `probe4.py`, `probe5.py`, `census_rows.csv`, `census_files.csv`).
- Existing docs: [eagl-animation.md](eagl-animation.md) (superseded in part, see section 12),
  [formats/animation.md](../formats/animation.md).
- Not read: `bWare` string hash, the EAGL factory dispatch body (`FnAnimFactory.cpp` is not in the decomp), and
  `speed.exe` (no Ghidra run for this spec).
