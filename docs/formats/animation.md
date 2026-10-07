# Animation data

MW uses EA's **EAGL4** animation system ("EA Graphics Library", version 4). The runtime is in the
decomp under `src/Speed/Indep/Src/EAGL4Anim/` (the codecs) and `src/Speed/Indep/Src/Animation/` (the game
side: `AnimBank`, `AnimSkeleton`, `AnimScene`, `AnimPlayer`, …).

**This is the least-documented area.** No open-source tool parses MW animation data. NFS-ModTools
handles models, textures and scenery, but not animation. The findings below were all made in this
project. For the tag meanings, see [evidence tags](../README.md#evidence-tags).

## Where animation data lives **[verified]**

| Location | Chunks | What |
|---|---|---|
| `NIS/Scene_*_BundleB.bun` (87 files) | `AnimScene (NisScene)`, `EAGLSkeletons` ×64, `EAGLAnimations` ×111, `CarpEventSequence` (59 files), plus textures and models | **NIS** cutscenes ("non-interactive sequences"): arrests (`ArrestF02`, `ArrestM04`, …), intros (`IntroNis01`, `IntroNisBL05`), endings, world-map scenes (`WMWaterTower`, …) |
| `GLOBAL/InGameB.bun` (the same chunks are also in `Ingameb.lzc` and `INGAMEC.BUN`) | `AnimDirectory` (→ `AnimDirectorySceneLoadData`, `AnimDirectorySceneMappingData`), `EAGLSkeletons`, `EAGLAnimations`, `ICECameraGroup (ICETracks)` ×85, `ICEShakeTracks`, ICE camera packs `8003B200`–`8003B203` (decomp: NIS / FMV / MKR / replay cameras) | In-game shared animations, plus **ICE** camera tracks (cinematic cameras) |
| `TRACKS/L2RA.BUN` | `WorldAnimEntities` ×11, `WorldAnimTreeMarkers` ×11, `WorldAnimInstances`, `WorldAnimInstanceEntry / WorldAnimCtrl`, `EAGLAnimations` ×13 | Animated world objects, e.g. `ANM_TowerCraneArm_01_XO_TowerCraneArm_1b_01_q` |
| `GLOBAL/GLOBALB.BUN`, `GlobalB.lzc` | `CarPartAnimHookupTable` (`0x34608`), `CarPartAnimHideTable` (`0x34609`) | Car-part animation hookup / hide tables (decomp `SPEED_CARPART_ANIMHOOKUP_TABLE` / `ANIMHIDE_TABLE`; contents unconfirmed) |

## EAGL skeletons and animations are ELF object files **[verified]**

The payload of every `EAGLSkeletons` (`0x00E34009`) and `EAGLAnimations` (`0x00E34010`) chunk is
8 bytes of `0x11` alignment padding followed by a complete **ELF32 relocatable object file**:

```
7F 45 4C 46  01 01 01 00 ...      "\x7fELF", 32-bit, little-endian
e_type    = 1 (ET_REL)            relocatable object, not an executable
e_machine = 8 (EM_MIPS)           even on PC, a leftover of the PS2-era toolchain
e_flags   = 0x20924000
```

All 188 EAGL objects in the install (64 skeletons + 124 animation banks) follow this pattern. EA's
offline tools produce these "objects", and at load time the game links them in memory. The decomp has
the loader: `eagl4supportdlopen.cpp` (a small `dlopen`) and `eagl4supportsympool.cpp` (symbol lookup).

### Sections

| Section | Skeleton | Animation |
|---|---|---|
| `.data` | bone records | animation bank + compressed channel data |
| `.shstrtab` | section names | section names |
| `.strtab` | symbol names | symbol names |
| `.symtab` | one symbol per bone + skeleton | the bank symbol |
| `.rel.data` | — | pointer fixups inside `.data` |

### Symbols tell you what's inside

The symbol names follow the pattern `__<Type>:::<name>`:

| Symbol | Meaning | Example |
|---|---|---|
| `__Skeleton:::<rig>` | the skeleton object | `__Skeleton:::Bip24` |
| `__Bone:::<rig>.<bone>` | one bone; its value is an offset into `.data`, and bones are spaced 16 bytes apart | `__Bone:::Bip21.Bip21 L Thigh`, `__Bone:::Bip24.Cuff02` |
| `__AnimationBank:::bank` | entry point of an animation bank | |
| `__EAGL_TOOLLIB_VERSION:::EAGL_TOOLLIB_VERSION-3` | tool version marker (weak symbol) | |

Totals: 2,532 bone symbols, 64 skeletons, 124 banks. The bone names (`Bip21 Spine 1`, `Bip21 R Upperarm`,
`Bip21 Cheek L`, …) are **3ds Max Character Studio Biped** names, so the characters were rigged with
Biped in 3ds Max. Character skeletons include facial bones (brows, cheeks, jaw, eyes).

### Relocations

Every relocation in all 188 objects is type **2 = `R_MIPS_32`**: a 32-bit absolute pointer (4,772 in
total). To "load" an object without the game:

1. Read `.data`.
2. For each `.rel.data` entry `(r_offset, r_info)`: `word_at(r_offset) += address_of(symbol(r_info >> 8))`.
   With a single `.data` section, that's simply "add the base address of `.data`".

After this, every pointer inside the bank is a valid in-memory pointer, matching how the runtime
structs in `EAGL4Anim/*.h` expect to see them.

### Animation names and codecs

Bank names (from `.data` strings) look like `ArrestF02_Cuffs_s`, `ArrestF02ZPM_Car1_q`, `Rotate_360_q`.
The suffix probably selects the channel codec. The decomp has one runtime class per codec:

| Decomp files | Kind (from the names) |
|---|---|
| `DeltaQ`, `DeltaQFast`, `DeltaSingleQ`, `FnDeltaQ*` | delta-compressed quaternion rotations |
| `DeltaF1`, `DeltaF3`, `FnDeltaF1/F3` | delta-compressed float / vec3 channels (translation, scale) |
| `StatelessQ`, `StatelessF3` | random-access (stateless) quaternion / vec3 |
| `RawPoseChannel`, `RawLinearChannel`, `RawStateChan`, `RawEventChannel` | uncompressed |
| `PoseAnim`, `CompoundChannel`, `PhaseChan`, `CsisEventChannel` | pose blending, multi-channel, phase (cycles), sound/event channels |
| `Skeleton`, `SkeletonData`, `BoneMask`, `IK` | skeleton side |

Mapping the `_q` / `_s` suffixes to specific codecs is **[unconfirmed]**.

## NIS scene chunk

```
80037020 AnimScene (NisScene)
├─ 00037030 AnimSceneHeaderData        152 B scene header; contains the scene name, e.g. "ArrestF02"
└─ 00037040 AnimSceneEntityData ×N     152 B per scene actor/element (64 of 87 scenes have these)
```

The names come from the decomp's `src/Speed/Indep/Src/Misc/SpeedChunks.hpp`
(`BCHUNK_SPEED_ANIM_SCENE_HEADER_DATA`, `BCHUNK_SPEED_ANIM_SCENE_ENTITY_DATA`) **[decomp]**. The list
also defines `0x00037045` (entity clip data) and `0x00037047` (entity "ucap" data), which no MW PC
NIS file uses **[verified]**. The structs are presumably in `src/Speed/Indep/Src/Animation/AnimScene.hpp`
/ `AnimWorldScene.hpp` **[unconfirmed]**.

`AnimDirectory` (`0x80037050`) in `InGameB.bun` has two children, `0x00037060`
`AnimDirectorySceneLoadData` (18,440 B) and `0x00037070` `AnimDirectorySceneMappingData` (4,180 B)
**[decomp names; sizes verified]**.

## Looking at it yourself

Extract one animation bank and read it with standard ELF tools:

```bash
python tools/chunkdump.py "D:/Need For Speed Most Wanted Black Edition/NIS/Scene_ArrestF02_BundleB.bun" --find 0x00E34010
python tools/chunkdump.py "D:/Need For Speed Most Wanted Black Edition/NIS/Scene_ArrestF02_BundleB.bun" --extract 0x0014C320 -o anim.chunk
# drop the 8-byte chunk header and the 8 bytes of 0x11 padding:
python -c "open('anim.o','wb').write(open('anim.chunk','rb').read()[16:])"
readelf -a anim.o      # if you have binutils; it recognizes the file as a MIPS relocatable object
```

## Suggested next steps

1. Write an EAGL ELF loader (`.data` + `R_MIPS_32` fixups), as above.
2. Map the 16-byte bone records and the bank header using `EAGL4Anim/Skeleton.h`, `SkeletonData.h`,
   `eagl4AnimBank.h` and `AnimMemoryMap.h` from the decomp.
3. Port the decoders (`FnDeltaQ.cpp`, `FnDeltaF3.cpp`, …) to Python and sample poses per frame.
4. Export the skeleton plus keyframes to glTF/FBX. Pair them with the NIS bundle's own `GeometryPack`
   (e.g. `FCop01.BIN`). Those models are skinned: in `Scene_ArrestF02` the cop and the perp have 48 bones,
   and every material uses effect 2 = `WorldBoneShader` with 60-byte vertices that carry blend weights and
   indices. **[verified]** See [models.md](models.md).
