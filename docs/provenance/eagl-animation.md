# EAGL animation (`libs/blackbox-anim`)

- **Spec:** [specs/eagl-animation.md](../specs/eagl-animation.md), with its parts
  [skeleton](../specs/eagl-animation-skeleton.md), [bank](../specs/eagl-animation-bank.md),
  [codecs](../specs/eagl-animation-codecs.md), [channels](../specs/eagl-animation-channels.md) and
  [channels, part 2](../specs/eagl-animation-channels-2.md). Format index: [formats/animation.md](../formats/animation.md).

## Method

- The install was read as bytes only. Each file's bChunk tree was walked, and each `EAGLAnimations` chunk (id
  `0x00E34010`) and each `EAGLSkeletons` chunk was taken as one ELF32 relocatable object. The objects were checked
  for their magic and their 8-byte padding.
- Relocations were applied by hand. Each `R_MIPS_32` entry adds the symbol's value to the word it points at. In the
  install every relocation targets `.data`, so every pointer is then an offset into `.data`.
- Layouts were proved by census, not by reading one file. The bank, skeleton and codec layouts were checked against
  every object in the 87 NIS bundles, the track file and the inflated `.bun` and `.lzc` files (105 files, nested JDLZ
  inflated). Counts are quoted in the specs.
- The codec and evaluator facts come from the decomp's headers and the few bodies it has. Where a body is empty, the
  spec says so and marks the behaviour **[unconfirmed – needs Ghidra]**.
- The prototype scripts lived in the session scratchpad and are not in the repository.
- The implementation was written from the specs. The decomp files were not kept on disk.

## Sources read

Decomp (CC0, `dbalatoni13/nfsmw`, read through the GitHub API and not saved to disk):

- `src/Speed/Indep/Src/EAGL4Anim/`, bank, channel and codec side: the headers `eagl4AnimBank.h`, `eagl4FnAnimBank.h`,
  `FnAnim.h`, `FnAnimFactory.h`, `AnimTypeId.h`, `AnimMemoryMap.h`, `FnAnimMemoryMap.h`, `Attribute.h`,
  `AttributeId.h`, `AttributeDictionary.h`, `AnimationTarget.h`, `Event.h`, `EventHandler.h`, `CompoundChannel.h`
  (5,278 bytes), `DeltaChan.h` (11,147 bytes), `DeltaCompressedData.h` (2,266 bytes), `DeltaF1.h` (4,611 bytes),
  `FnDeltaF1.h` (2,496 bytes), `DeltaF3.h` (5,194 bytes), `FnDeltaF3.h` (2,516 bytes), `DeltaQ.h` (4,196 bytes),
  `FnDeltaQ.h` (2,426 bytes), `DeltaQFast.h`, `DeltaSingleQ.h`, `FnCycle.h`, `FnRawLinearChannel.h`,
  `RawLinearChannel.h`, `RawPoseChannel.h`, `PoseAnim.h`, `StatelessQ.h`, `StatelessF3.h`, `FnStatelessQ.h`,
  `FnStatelessF3.h`, `SkeletonData.h`, `ScratchBuffer.h`, `AnimUtil.h`, `ClipSet.h` (empty).
- Bodies read: `CompoundChannel.cpp` (5,345 bytes), `CsisEventChannel.cpp` (3,613 bytes), `DeltaCompressedData.cpp`
  (14,705 bytes), `FnDeltaF1.cpp` (15,301 bytes), `FnDeltaF3.cpp` (18,218 bytes), `FnDeltaQ.cpp` (898 bytes, empty
  bodies), and the dispatch functions `NewFnAnimAux` and `InitAnimMemoryMapAux` in `MemoryPoolManager.cpp`.
- Zero-byte `.cpp` files, so their behaviour is not known from the decomp: `FnStatelessQ.cpp`, `FnStatelessF3.cpp`,
  `StatelessQ.cpp`, `StatelessF3.cpp`, `RawPoseChannel.cpp`, `FnRawPoseChannel.cpp`, `RawEventChannel.cpp`,
  `RawStateChan.cpp`, `PhaseChan.cpp`, `FnPoseAnim.cpp`, `PoseAnim.cpp`, `FnDeltaQFast.cpp`, `FnDeltaSingleQ.cpp`,
  `FnEventBlender.cpp`, `FnPoseBlender.cpp`, `FnRunBlender.cpp`, `FnTurnBlender.cpp`, `AnimUtil.cpp`, `ClipSet.cpp`.
- `src/Speed/Indep/Src/EAGL4Anim/`, skeleton side: `eagl4supportdlopen.h` (9,629 bytes) and `.cpp` (6,874),
  `eagl4supportsympool.h` (1,898) and `.cpp` (3,686), `eagl4supportconspool.h` (2,648) and `.cpp` (1,169),
  `eagl4supportdef.h` (1,744), `eagl4supportbase.h` (320) and `.cpp` (591), `Skeleton.h` (2,915) and `Skeleton.cpp`
  (0 bytes), `SkeletonData.h` (2,337), `BoneMask.h` (955) and `.cpp` (0), `eagl4runtimetransform.h` (4,855) and `.cpp`
  (0), `PosePalette.h` (1,757), `System.h` (401), `eagl4realmath.h` (guard only).
- `src/Speed/Indep/Src/Animation/` (game side): `AnimSkeleton.hpp` (1,635 bytes) and `.cpp` (4,431), `AnimCtrl.cpp`
  (8,532), `WorldAnimCtrl.cpp` (10,583), `AnimPart.cpp` (2,109), `AnimInternal.cpp` (604), `AnimBank.hpp` and `.cpp`,
  `AnimScene.cpp`, `AnimEntity_WorldEntity.cpp`, `AnimEntity_BasicCharacter.cpp`.
- Also `Libs/Support/Utility/UTypes.h` (3,883 bytes; the vector and matrix types) and `Src/Misc/SpeedChunks.hpp` (the
  chunk ids `0x00E34009` and `0x00E34010`).

Ghidra (`speed.exe` 1.3, image base `0x400000`; read for behaviour, the facts are recorded in the spec):

- The channel dispatch `FUN_0049dc20` and its three evaluators, at `0x499620`, `FUN_004995d0` and `0x499660`
  ([eagl-animation.md](../specs/eagl-animation.md) section 3).
- `FnDeltaF3::InitBuffersAsRequired` at `0x004968E0`, the DofInfo layout and scale formulas
  ([eagl-animation-codecs.md](../specs/eagl-animation-codecs.md) section 9 and its sources; supplied by the supervisor).

Install (read only, `D:\Need For Speed Most Wanted Black Edition`):

- `TRACKS\L2RA.BUN`: 13 `EAGLAnimations` chunks, among them the airliner's object at offset 1,409,088.
- `NIS\Scene_*_BundleB.bun`: all 87 bundles. Skeleton and animation chunks in them were walked.
- `GLOBAL\InGameB.bun`, `GLOBAL\Ingameb.lzc` and `GLOBAL\INGAMEC.BUN` (the same skeleton and animation chunks; one copy
  of `INGAMEC` differs in 8 bytes).
- A raw byte search of all 2,084 files under the install for the ELF-after-fill pattern. It found the same 67 skeleton
  and 127 animation chunks as the chunk walk.

Repository docs read: [formats/animation.md](../formats/animation.md), the earlier [eagl-animation.md](../specs/eagl-animation.md)
(superseded in part by the bank spec's section 12), and [tools/chunkdump.py](../../tools/chunkdump.py) for the chunk format.

## Implemented by

- `libs/blackbox-anim/src/`:
  - `object/elf.rs` and `object.rs`: the ELF objects and their relocations;
  - `skeleton.rs`, `pose.rs`: the skeleton, the bind pose, the pose buffer and the world matrices;
  - `bank.rs`, `compound.rs`, `clip.rs`: the banks and their names, the compound channels and the sampling of a clip into poses;
  - `codec/`: DeltaLerp and DeltaQuat (the DeltaChan header and the DeltaCompressedData block in `codec/rows.rs`), and
    DeltaF3 and DeltaF1 (the binned layout in `codec/bins.rs`);
  - `type_id.rs`: the codec type numbers.
- `crates/nfsmw-data/src/anim.rs`: reads the EAGL objects of a bundle (the install side; the format crate takes bytes).
- `crates/nfsmw/src/commands/dump_anim.rs` (the `dump-anim` command) and `crates/nfsmw/src/scenes/anim_view/` (the
  `view-anim` viewer: one skeleton drawn as sticks, animated by one of its clips).
- `crates/nfsmw/src/scenes/world/nis/cars.rs` and `player.rs`: the NIS player samples the scene's animation channels
  for its cars ([nis-player](nis-player.md)).

## Checked against the game by

- Synthetic tests in `libs/blackbox-anim/src/tests/` (bank, clip, codec, codec_f, compound, object, pose, skeleton,
  type_id). They build their own bytes with `elf_builder.rs` and copy nothing from the install.
- Real-install tests, ignored unless `NFSMW_GAME_DIR` is set: `crates/nfsmw-data/tests/real_install/anim_objects.rs`,
  `anim_banks.rs`, `anim_skeletons.rs`, `anim_codecs.rs`, `anim_codecs_f.rs` and `anim_clips.rs`. `skin.rs` checks the
  skin weights of the arrest solids (each weight stays inside its rig) and that a rigid split keeps every triangle.
- The census numbers in the specs were produced by the scripts above (for example 848 compound records, all at
  30 FPS, and 2,538 skeleton bone records).
- The inverse bind matrices of all 2,532 bones match the inverse of the bind pose, with a worst error of 8.4e-7
  ([eagl-animation-skeleton.md](../specs/eagl-animation-skeleton.md) section 0).

## Known differences and open items

- **Not in the install:** the stateless, raw, phase, cycle, CSIS and pose channel types (`0`, `1`, `2`, `3`, `14`,
  `16`, `22` to `25`). The install has no instance of any of them, so their layouts and evaluators are not checked
  **[unconfirmed – needs Ghidra]**. The functions to find are listed in
  [eagl-animation-channels-2.md](../specs/eagl-animation-channels-2.md) section 17.
- **Not decoded:** DeltaQ (type 17, three uses in the install; the codec spec calls it partly decoded). DeltaQFast
  (18) and DeltaSingleQ (19) are not used by the install and have no implementation.
- **Skeleton pose:** the bodies of `PoseSQTToGlobal`, `PoseLocalToGlobal`, `PoseSQTToSkin`, `GetStillPose`,
  `BuildBoneMask` and `MirrorPose` are empty in the decomp. The pose composition follows the call site and the bind
  pose check. The 10 values per bone in the pose buffer, the two extra floats and the 12 zero bytes of each index
  record are open ([eagl-animation-skeleton.md](../specs/eagl-animation-skeleton.md) section 8).
- **Names and binding:** the string hash (`bStringHash` in `bWare`) was not read, and the meaning of the 16-bit target
  checksum is not confirmed. So binding an animation to a skeleton from its header alone is not proven.
- **Timing:** every compound in the install has the FPS attribute set to 30. Whether the game applies it at run time
  is not confirmed. The car and character clips in a NIS run at half the scene rate through their time scale
  ([nis-player-2.md](../specs/nis-player-2.md) section 16; the effect is **[unconfirmed]**).
- **Blenders and Cycle** are built by game code, not stored in banks. Their evaluators are open (the list above).
- **Not checked in a running game:** how the airliner moves in the world. The status line of the spec was written before
  the animation work and says the airliner is not yet animated; it has not been updated against a running game.
- **Not read:** the EAGL factory dispatch body (`FnAnimFactory.cpp` is not in the decomp) and `speed.exe` beyond the
  addresses listed above.
