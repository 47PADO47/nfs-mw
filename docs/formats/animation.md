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

## ICE chunk dispatch (speed.exe 1.3) **[verified: Ghidra]**

The ICE chunk ids are consumed by two identical dispatch functions in `speed.exe`
(image base `0x400000`). Each one takes the chunk header pointer as its stack argument and reads the
id from `[arg]`:

| Site | Test | Action |
|---|---|---|
| `0x004788E0` (compare at `0x4788E6`) | `id == 0x0003B211` | `CALL 0x00478890` (shake tracks) |
| same | `0x8003B200 <= id <= 0x8003B203` | `CALL 0x004787C0` (camera packs, `ECX = 0x008EE320`) |
| `0x0047FB40` (compare at `0x47FB46`) | `0x8003B200 <= id <= 0x8003B203` | `CALL 0x0047F9F0` (the `0x3B211` branch goes to `0x47FB6F`, not yet read) |

Confirmed ids, all checked as immediates in the dispatch: **`0x0003B211` ICEShakeTracks** and the four
camera packs **`0x8003B200`, `0x8003B201`, `0x8003B202`, `0x8003B203`** (the range test).

**Not confirmed as immediates in `speed.exe`** (searched, no hits): `0x0003B200`, `0x8003B204`
(Generic, outside the range test), `0x8003B209` (CameraShakeData) and `0x0003B210` (ICECameraGroup).
They may use another dispatch or live in `server.dll`; open question.

Pack kind inside `0x004787C0`: the handler computes `id - 0x8003B200` and jumps through a 4-entry table at
`0x00478874` for indices 0..3. Entries map to `kind = 0, 1, 2, 3` and anything else to `kind = 4`. The
decomp names for those kinds (NIS, FMV, MKR, Replay, Generic) are not checked here.

The handler then loops over the records. Each record goes through `0x004786E0`, which appends a
20-byte entry (pointer plus kind) to one of four arrays on the manager object at `ECX`:

| Kind | Count field (offset in manager) | Limit |
|---|---|---|
| 0 | `+0x14` | 256 (`0x100`) |
| 1 | `+0x18` | 10 |
| 2 | `+0x1C` | 50 (`0x32`) |
| 3 | `+0x20` | 50 |

### ICE camera records **[verified: Ghidra, speed.exe 1.3]**

All offsets are payload-relative (payload = chunk header `+8`). Decoded from `0x004787C0` (camera
packs), `0x004786E0` (entry append), `0x0046E760` (duplicate-key lookup), `0x004789C0` /
`0x0046D2E0` / `0x00477EE0` (time search and evaluator), and `0x0047F9F0` (teardown).

**Pack (chunk `0x8003B200..0x8003B203`, `kind = id - 0x8003B200`).** The payload is a sequence of
standard chunks (`id`, `size`, payload). The engine ignores their ids. Each sub-chunk payload is:

| Offset | Type | Field |
|---|---|---|
| `+0x00` | u32 | Dedup key. Passed to `0x46E760`; a key already registered is merged into the existing entry |
| `+0x04` | i32 | Record count `N`. `N <= 0` means no records |
| `+0x08` | | `N` camera records, back to back |

**Camera record** (header `0x28` bytes, then elements):

| Offset | Type | Field |
|---|---|---|
| `+0x00..+0x08` | | Runtime list links, written by the engine (`0x004787C0`) |
| `+0x08` | | Runtime owner pointer (written on link) |
| `+0x14` | i16 (signed) | Element count `M`. Read with `MOVSX` |
| `+0x16` | u8 | Read by the teardown `0x47F9F0` as an "owned" flag before `free`. Not confirmed as a file byte |
| `+0x28` | | `M` elements, stride `0x84` |

The next record starts at `record + 0x28 + M*0x84`. A record with a negative `M` would walk backward,
so the reader rejects it.

**Camera element** (stride `0x84`, element `i` at `record + 0x28 + i*0x84`):

| Offset | Type | Field |
|---|---|---|
| `+0x00` | u8 | Kind (`nType`). The tangent routine (`0x47F490`) gives a key of kind 0 no tangents. The full key table is in "ICE camera keys" below |
| `+0x10` | f32 | Time key. `0x46D2E0` scans backward for the last element with key `<=` the current time |
| `+0x14`, `+0x4C`, `+0x54` | f32 | Read by `0x477EE0` in a component-wise difference test: tangent length, dutch and lens (the continuity test of neighbouring keys) |

Position, target and FOV are pinned below ("ICE camera keys"); the bytes after the key table are the same data.

**Shake tracks (`0x0003B211`, handler `0x00478890`).** The payload starts with `u32 N` at `+0x00`.
Records start at `+0x04`. Each record has a `0x20`-byte header:

| Offset | Type | Field |
|---|---|---|
| `+0x00..+0x0C` | | Runtime links and owner, written by the engine |
| `+0x0C` | i16 (signed) | Element count `M` |
| `+0x20` | | `M` elements, stride `0x18` (24 bytes) |

The next record starts at `record + 0x20 + M*0x18`. The shake walker does not call the duplicate
lookup. It links into the owner object at `manager+0x10`.

**Not pinned:** the 24-byte shake element.

## ICE camera keys **[decomp: `speed.exe` 1.3 and the CC0 `ICEData` header; fields checked against the install]**

A camera set's key (`+0` of the sub-chunk) is the hash of a scene name (`bStringHash`, case-sensitive, seed `~0`,
`h*33 + c`). Its records are the scene's tracks. The 0x28-byte record header is the ICE track object: `+0xC` start
(float), `+0x10` length in seconds (float), `+0x14` key count (i16), `+0x17` name (14 bytes, `Track 0`...). The scene
loader finds a track by the name `Track <n>`. Each 0x84-byte key:

| Offset | Type | Field |
|---|---|---|
| `0x00` | u8 | kind; 0 = no tangents |
| `0x01` | u8 | smooth (unused by the evaluator read) |
| `0x02` / `0x03` | u8 | cubic flag for eye (also dutch, lens) / for the look point |
| `0x04` / `0x05` | u8 | space of the eye / of the look point: 0 car, 1 world, 2 hybrid, 3 scene |
| `0x06` .. `0x0A` | u8 | constrain to cars, constrain to world, ignore orientation, car-space lag, overlay |
| `0x0C` | u32 | shake type |
| `0x10` | f32 | the key's time, 0 to 1 along the track |
| `0x14` | f32 x2 | tangent length at the start and at the end |
| `0x1C` | f32 x3 x2 | eye at the start (x y z) then at the end |
| `0x34` | f32 x3 x2 | look point, same layout |
| `0x4C` | f32 x2 | dutch (roll) |
| `0x54` | f32 x2 | lens, millimetres |
| `0x5C` | f32 x2 | near clip |
| `0x64`, `0x6C`, `0x74` | f32 x2 | noise amplitude, noise frequency, focal distance |
| `0x7C`, `0x7E`, `0x80` | u8 x2 | aperture, letterbox percent, simulation speed percent |

Scene space (3) is the scene's own frame: `+y` is the way the camera looks and `+z` is up; the NIS origin and heading
place it in the world (`docs/specs/pursuit-breaker-moment.md` section 3). Evaluation (`blackbox_chunk::ice::Track`):
cubic Hermite per interval with tangents `chord * tangent_length`, field of view `2 atan(15.96 / lens)`, dutch cut to
an integer 16-bit angle, simulation speed `percent * 0.01` clamped to at least 0.01. Not applied: the smoothing of a
key's tangents with its neighbours' (the engine does it for keys that continue each other), noise, depth of field,
the hybrid and car spaces, and the lens-to-angle conversion of the lens tangent (the rewrite curves the lens, the
engine curves the angle; the two agree when the lens does not change).

**Known gaps in the dispatch:** `0x0047FB6F` (the shake branch of the second dispatch) is not read.
`0x0047FB40` is not a function start in the Ghidra database, so the second site was not fully
decoded.

## Looking at it yourself

Extract one animation bank and read it with standard ELF tools:

```bash
python tools/chunkdump.py "$NFSMW_GAME_DIR/NIS/Scene_ArrestF02_BundleB.bun" --find 0x00E34010
python tools/chunkdump.py "$NFSMW_GAME_DIR/NIS/Scene_ArrestF02_BundleB.bun" --extract 0x0014C320 -o anim.chunk
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
