# NIS scenes: the scene chunk, the bundles and the directory

What a NIS (non-interactive sequence, a cutscene) is made of on disk: the `AnimScene` chunk, the entity records, the
bundle that holds them, the directory that finds a bundle by name, and the ICE camera set that goes with each scene.
The behaviour (how a scene is started, placed, timed and ended) is in [nis-player.md](nis-player.md).

Evidence tags, as in [docs/README.md](../README.md#evidence-tags):

- **[verified]**: measured on the install with a Python scan of all 87 `NIS/Scene_*_BundleB.bun` files, the
  `GLOBAL/InGameB.bun` directory and camera packs. Numbers are quoted.
- **[decomp]**: read in the CC0 decompilation (dbalatoni13/nfsmw). The file is named.
- **[unconfirmed]**: a hypothesis, or a rule whose use is not proven. Items that need Ghidra are marked
  **[unconfirmed – needs Ghidra]** and listed in [nis-player-2.md](nis-player-2.md), section 15.

## 0. Summary

1. The install has **87 NIS scenes**, one bundle each: `NIS/Scene_<name>_BundleB.bun`. The scene name is the text
   between the first and second underscore of the file name. **[verified]**
2. Every bundle has one `AnimScene` chunk (`0x80037020`) with a header record and, in 26 bundles only, one to three
   entity records. The 26 bundles are the **arrest** scenes. **[verified]**
3. Only the 26 arrest bundles carry characters: skinned solids, EAGL skeletons, texture packs and geometry. The other 61
   scenes hold an animation bank (`EAGLAnimations`) and, in most cases, a `CarpEventSequence`. `IntroNis05`,
   `EndingNis04` and `IntroNisBL01` are examples of the second group. **[verified]**
4. The header is not 152 bytes. It is 0x40 bytes of fixed fields followed by the scene's description text, so it is
   68 to 188 bytes long (the size is `0x40` plus the description, NUL-terminated, rounded up to 4 bytes; true in all 87).
   **[verified]**. The entity record is 152 bytes, but it starts with 8 bytes of `0x11` alignment padding, so the
   data is 144 bytes (`0x90`). **[verified]**
5. The scene's name hash is the case-sensitive hash `h = h * 33 + c` over the ASCII name, starting from `0xFFFFFFFF`,
   modulo 2^32. It matches the header hash in 87 of 87 scenes, and the camera-pack keys match it too. **[verified]**
6. The directory `AnimDirectory` (`0x80037050`, in `GLOBAL/InGameB.bun`) lists the 87 scenes and their bundle file
   names. Its two children are the load table (`0x37060`, 18,440 bytes) and the candidate table (`0x37070`, 4,180
   bytes). **[verified]**, **[decomp]** for the structs.
7. The camera sets are in `GLOBAL/InGameB.bun`: pack `0x8003B200` (NIS) has 73 of the 87 scenes, pack `0x8003B201`
   (FMV) has 4 more. Ten scenes have no camera set at all, including `IntroNisBL01`. **[verified]**
8. Every scene that has a camera set has a `Track 0`, and that track's length in seconds equals `VanishFrame / 30`
   for 69 of the 77 scenes that have a track in pack 0 or 1. **[verified]**
9. All 805 bank animations in the NIS bundles are Compound (type 15) with an FPS attribute of 30. **[verified]**

## 1. Where a scene's data lives **[verified]**

| Item | Where | Chunk id | Count | Size |
|---|---|---|---|---|
| Scene chunk (header and entities) | each `NIS/Scene_<name>_BundleB.bun` | `0x80037020` (`AnimScene (NisScene)`) | 87 | 1 per bundle |
| Scene header | inside the scene chunk | `0x00037030` | 87 | 68 to 188 bytes |
| Entity record | inside the scene chunk | `0x00037040` | 64 records in 26 bundles | 152 bytes on disk |
| Animation banks | inside the bundle | `0x00E34010` (`EAGLAnimations`) | 111 chunks, 805 animations | ELF objects, see [eagl-animation-bank.md](eagl-animation-bank.md) |
| Skeletons | inside the bundle | `0x00E34009` (`EAGLSkeletons`) | 64 chunks in 26 bundles | ELF objects, see [eagl-animation-skeleton.md](eagl-animation-skeleton.md) |
| Geometry (skinned models) | inside the bundle | `0x80134000` (`GeometryPack`) | 64 chunks in 26 bundles | see [models.md](models.md) |
| Texture packs | inside the bundle | `0xB3300000` (`TexturePack`) | 90 chunks, all in the 26 arrest bundles | see [models.md](models.md) |
| Event sequence | inside the bundle | `0x0003B811` (`CarpEventSequence`) | 59 chunks | not decoded here |
| Directory | `GLOBAL/InGameB.bun` | `0x80037050` (`AnimDirectory`) | 1 | 4,808-byte container |
| Load table | inside the directory | `0x00037060` (`SceneLoadData`) | 1 | 18,440 bytes |
| Candidate table | inside the directory | `0x00037070` (`SceneCandidateData`) | 1 | 4,180 bytes |
| NIS camera set | `GLOBAL/InGameB.bun` | `0x8003B200` (`ICENisCameras`) | 1 chunk, 74 keys, 84 tracks | section 8 |
| FMV camera set | `GLOBAL/InGameB.bun` | `0x8003B201` (`ICEFmvCameras`) | 1 chunk, 4 keys, 7 tracks | section 8 |

Names come from the repo's `tools/bchunk_names.py` (community names, checked against `SpeedChunks.hpp`) **[decomp]**.

### 1.1 Bundle contents by family **[verified]**

The 87 bundles fall into four content shapes:

| Shape | Bundles | Contents |
|---|---|---|
| Arrest (full character scene) | 26 | Header, 2 or 3 entity records, 2 or 3 skeletons, 3 or 4 texture packs, 2 or 3 geometry packs, 1 or 2 animation banks, 1 event sequence |
| Animation only, with event sequence | 33 | Header, 1 animation bank, 1 event sequence (for example `IntroNis05`, `EndingNis04`, all race starts with `Carp`) |
| Animation only, no event sequence | 28 | Header, 1 animation bank (for example `IntroNisBL01`, the `FMV*` scenes, the world moments) |
| Empty of animation | 2 | Header only: `FMVNoCars`, `Flythrough` |

The 26-bundle arrest set is exactly the set of bundles with skeletons. Entity count equals skeleton count in every
bundle. **[verified]** Animation banks per bundle: 1 bank in 59 bundles, 2 in 26 (the arrests), 0 in 2. The bundle
sizes are 1,289,984 to 1,729,920 bytes for the arrests and as small as 92 bytes for the smallest scene.

## 2. The AnimScene chunk tree **[verified]**

```
80037020 AnimScene (NisScene)           one per bundle; the root of the scene
├─ 00037030 AnimSceneHeaderData         one; 68 to 188 bytes
├─ 00000000 Padding                     present in 24 of the 26 arrest bundles; no data
└─ 00037040 AnimSceneEntityData ×N      N = 0 (61 bundles), 2 (14) or 3 (12); 152 bytes each
```

The scene chunk's children are the only children that the scene loader reads. The loader (`LoaderAnimSceneData`)
looks at the header and entity ids and ignores the rest **[decomp: `AnimScene.cpp`]**. The ids `0x00037045` (entity
clip data) and `0x00037047` (entity UCAP data) are in the decomp's chunk list but no MW PC NIS file uses them.
**[verified: 0 counts]**

## 3. Scene header (`0x00037030`) **[verified; field names decomp: `NisScene` in `AnimScene.hpp`]**

The header record is the fixed 0x40-byte `NisScene` struct followed by the description text. All numbers are
little-endian.

| Offset | Type | Field | Meaning | Values in the install |
|---|---|---|---|---|
| 0x00 | u32 | `mSceneNameHash` | name hash (section 0, item 5) | equal to the hash of the name in 87 of 87 |
| 0x04 | char[16] | `mSceneName` | scene name, NUL-padded | e.g. `ArrestF02`, `IntroNis05` |
| 0x14 | u32 | `Description` (pointer) | 0 in the file; the loader points it at the text | always 0 |
| 0x18 | i32 | `SceneType` | placement and car rules, see [nis-player.md](nis-player.md) section 6 | 0: 25 scenes, 2: 26, 3: 4, 4: 32; value 1 never used |
| 0x1C | i32 | `HaveLayout` | 1 for the arrests, 0 elsewhere | 1: 26 scenes, 0: 61 |
| 0x20 | i32 | `HaveCarAnimation` | 1 means the scene drives cars from animation | 1 in 85 scenes; 0 in `FMVNoCars` and `Flythrough` |
| 0x24 | i32 | `NumberOfCars` | number of car channels the scene uses | 1 (18 scenes), 2 (53), 3 (3), 4 (10), 5 (1), 8 (1), 10 (1) |
| 0x28 | i32 | `StartFrame` | first frame that plays | 0 in all 87 |
| 0x2C | i32 | `VanishFrame` | last frame; the scene's length is this / 30 s | 33 (`IntroNisBL07End`) to 4000 (`FMV17to21`, `FMVroc02`) |
| 0x30 | char[16] | `SeeulatorOverlayName` | overlay drawn during the scene, empty if none | `FLYBY` in 12 scenes, empty in 75 |
| 0x40 | char[] | description | text, NUL-terminated, then padding to a 4-byte boundary | e.g. "Cop walking perp in hand cuffs, cop shoves perp, cop puts perp into car (mellow)(int:1)" |

Notes:

- The description is inside the record, after the fixed part. Its length rule (0x40 plus the aligned text) holds in all
  87 headers **[verified]**. The 152-byte size in the older notes is the size of two arrest headers
  (`ArrestF02`, `ArrestF02b`) only.
- `FLYBY` scenes: `Flythrough`, `IntroNisBL03`, `IntroNisBL04`, `IntroNisBL05`, `IntroNisBL06`, `IntroNisBL10`,
  `IntroNisBL11`, `IntroNisBL12`, `IntroNisBL13`, `IntroNisBL14`, `IntroNisDD`, `IntroNisDDEnd`. The effect of the
  overlay is in [nis-player-2.md](nis-player-2.md), section 11.
- The decomp's struct sets the overlay name at 0x30 and ends it at 0x40, so the description starts exactly there
  **[decomp]** and in the file **[verified]**.

## 4. Entity record (`0x00037040`) **[verified; struct names decomp: `BasicCharacterAnimEntityInfo`, `PropAnimEntityInfo`]**

Each record is 152 bytes. The first 8 bytes are `0x11` padding (the loader aligns the data to 16 bytes **[decomp:
`AnimScene.cpp`, `GetAlignedData(16)`]**). The data is the 144 bytes below. Offsets here are from the start of the data,
that is 8 bytes after the start of the record. All 64 records are type 1.

| Offset | Type | Field | Meaning | Values in the install |
|---|---|---|---|---|
| 0x00 | u32 | `mTypeID` | entity kind: 1 BasicCharacter, 2 Prop, 3 WorldAnimNode | 1 in all 64 |
| 0x04 | u32 | instance name hash | name of the actor inside the scene | not a string in the bundle; no name resolves |
| 0x08 | u32 | parent name hash | parent entity, 0 for the scene root | 0 in all 64 |
| 0x0C | u32 | play flags | bit set that picks the play mode | 0x20 in all 64 |
| 0x10 | f32 x16 | local matrix | placement in scene space, row 4 = (0 0 0 1) | translation in floats 12 to 14; z = 0 in all 64 |
| 0x50 | u32 x4 | LOD name hashes | the skinned model's LOD levels | 3 non-zero and 1 zero in all 64; not resolved to strings |
| 0x60 | u32 | translation anim hash | bank animation for the root translation | 0 in all 64 |
| 0x64 | u32 | quaternion anim hash | bank animation for the rotations | 0 in all 64 |
| 0x68 | u32 | scale anim hash | bank animation for scale | 0 in all 64 |
| 0x6C | u32 | skeletal anim hash | the actor's clip, e.g. `ArrestF02_FCop_s` | non-zero in all 64; resolves to a bank animation of the same bundle |
| 0x70 | u32 | skeleton name hash | the skeleton the clip drives | resolves to a skeleton of the same bundle in all 64 |
| 0x74 | f32 | play speed | multiplier on the clip's time | 1.0 in all 64 |
| 0x78 | f32 | play delay | seconds before the clip starts | 0.0 in all 64 |
| 0x7C | u32 | world object type hash | used only for props | 0 in all 64 |
| 0x80 | u32 x2 | loop start, loop end | loop range in frames, used only with the loop flag | 0, 0 in all 64 |
| 0x88 | u32 x2 | padding | unused | 0 in all 64 |
| 0x90 | 8 bytes | tail | beyond the struct; not read by the decomp | 0 in all 64 |

### 4.1 Actors in the arrest scenes **[verified]**

Each arrest has two actors (a cop and the perp) or three (the cop, the perp and the cuffs). The skeleton tells them
apart.

| Skeleton | Bones | Actor | Example clip (frames) |
|---|---|---|---|
| `Bip21` | 48 | female cop (`F` in the clip name, for example `ArrestF02_FCop_s`) | 119 (`ArrestF02`) |
| `Bip20` | 48 | male cop (`M`, for example `ArrestM01_MCop_s`) | 94 (`ArrestM01`) |
| `Bip23` | 48 | perp (`ArrestF02_Perp_s`) | 119 |
| `Bip24` | 3 | cuffs (`ArrestF02_Cuffs_s`), a three-bone prop-like actor | 119 |

Entity counts by skeleton over the 64 records: `Bip23` 26, `Bip20` 14, `Bip21` 12, `Bip24` 12. The clip frame
counts of the arrests run from 94 to 433 frames (for example `ArrestF06`: 433 frames against a scene of 340 frames).
So the clip length is not a fixed fraction of the scene length. **[verified]**

Translations of the actors in scene space (floats 12 to 14 of the local matrix): for `ArrestF02` the cop is at
(1.597, −4.608, 0), the perp at (1.350, −4.023, 0) and the cuffs at (1.523, −3.960, 0). Every actor's z is 0.
**[verified]** The meaning of the axes is in [nis-player.md](nis-player.md), section 5.

### 4.2 Props **[verified: none]**

No record has type 2 (Prop) in the install. The prop code exists in the decomp (`CPropAnimEntity`, 0x60-byte info, a
static model with no animation) **[decomp: `AnimEntity_Prop.cpp`]**, but no scene uses it. The cuffs in the arrests are
type 1 with a skeleton.

### 4.3 Other entity types **[decomp]**

The factory (`CAnimEntityFactory::CreateAnimEntity`) makes types 1 and 2 only; type 3 (WorldAnimNode) returns nothing.
**[decomp: `AnimEntity.cpp`]** The cars are not entity records. They are channels that the scene binds to race or NIS
cars by name (`car1`..`car8`, `cop1`..`cop8`), described in [nis-player.md](nis-player.md), section 7.

## 5. Scene bundle: what the names inside mean **[verified]**

A scene's animation names follow one pattern, `<Scene><Part>_<suffix>`:

| Name pattern | Meaning | Example |
|---|---|---|
| `<Scene>ZPM_Car<i>_t` and `_q` | car channel `i` (1 to 8): translation and quaternion | `IntroNis05ZPM_Car1_q` |
| `<Scene>ZPM_Car<i>_s` | car channel `i`, third channel (not read by the car code) | `IntroNis05ZPM_Car1_s` |
| `<Scene>ZPM_Cop<i>_t`, `_q`, `_s` | cop car channel `i` (1 to 8) | `EndingNis04ZPM_Cop3_q` |
| `<Scene>_<Actor>_s` | character clip; actor is `FCop`, `MCop`, `Perp` or `Cuffs` | `ArrestF02_Perp_s` |

The car channels in the install: `_q` and `_t` exist for every car channel found (the scene code uses only these two).
The `_s` channel exists for some scenes only (for example `IntroNis05`, not `ArrestF02`). The base name (`<Scene>ZPM_Car<i>`
with no suffix) is not in any bank; the code only tests that a hash is not zero **[decomp: `AnimScene.cpp`,
`CreateCarAnimationControllers`]**.

Car channel frame counts (the `_q` channel) for six examples are in section 7.

## 6. Directory `AnimDirectory` (`0x80037050`, in `GLOBAL/InGameB.bun`) **[verified; structs decomp: `AnimDirectory.hpp`]**

The container has two children.

### 6.1 Load table (`0x00037060`, 18,440 bytes)

| Offset | Type | Field | Meaning |
|---|---|---|---|
| 0x0000 | u32 | scene count | 87 |
| 0x0004 + 8 × i | 8 bytes × 256 | scene slot i | scene hash (u32), then four bytes: shared-file count, shared-file start, scene-file count, scene-file start |
| 0x0804 | u32 | file count | 87 |
| 0x0808 + 64 × j | 64 bytes × 256 | file name j | e.g. `NIS\Scene_ArrestF02_BundleB.bun`, NUL-padded |

- The 87 slots are in alphabetical order (`GenericStart`, `IntroNis01`, ...). Each slot names one scene file (count 1,
  start = slot index). Shared-file counts are 0 for every NIS scene. **[verified]**
- The directory's scene hashes are the 87 NIS names' hashes; each slot's file name contains `_<name>_` **[verified]**.
- The loader reads a scene's files from this table, shared files first, then scene files **[decomp: `AnimPlayer.cpp`,
  `AnimLoader_NextStep`]**. The size rule of the loader is in [nis-player.md](nis-player.md), section 4.

### 6.2 Candidate table (`0x00037070`, 4,180 bytes)

This table is the "scene mapping" that the older docs call `AnimDirectorySceneMappingData`. Its C++ struct is
`CAnimCandidateData` **[decomp: `AnimCandidates.hpp`]**. Seven u32 counts come first, then the arrays:

| Offset | Array | Element size | Count in install |
|---|---|---|---|
| 0x0000 to 0x0018 | counts: race start, race end, online race start, forward track masks, reverse track masks, moments, special car lists | 4 | 1, 0, 0, 0, 0, 41, 53 |
| 0x001C | race start candidates | 12 | 1 used (see below) |
| 0x007C | race end candidates | 8 | 0 |
| 0x00BC | online race start candidates | 20 | 0 |
| 0x015C | forward track race masks (120 slots) | 12 | 0 |
| 0x06FC | reverse track race masks (120 slots) | 12 | 0 |
| 0x0C9C | moment entries (64 slots) | 8 | 41 |
| 0x0E9C | special car list entries (55 slots) | 8 | 53 |

**Moment entries (41).** Each is a scene hash and a marker kind (the `eMomentMarkerTypes` enumeration, 0 to 20)
**[decomp]**. They are: all 26 arrests (kind 12, `BustedMomementMarker`), `IntroNisToll01` (kind 13, `TollboothMarker`),
and the 14 world moments, each with its own kind (for example `WMGasStation` 1, `WMRadioTower` 2, `WMWaterTower` 4,
`WMPorch` 5, `WMBoat` 10, `WMAmphitheatre` 19). **[verified]** The pursuit-breaker spec uses the same table for the
marker of each moment ([pursuit-breaker-moment.md](pursuit-breaker-moment.md), section 3).

**Special car lists (53).** Each entry is a scene hash and one of twelve car list codes (`NONE` = -1, `ONE_COP` = 0,
`LOTS_OF_COPS` = 1, `TRAFFIC_MIX1` to `TRAFFIC_MIX9` = 2 to 10, `ONE_SPORT_COP` = 11) **[decomp]**. All 26 arrests use
code 0 (`ONE_COP`). The other 27 entries are for race starts, endings and FMVs, and 7 of those 27 hash to no scene in
the install. **[verified]** For what the code does with the code, see [nis-player.md](nis-player.md), section 7.

**Race start candidates (1 used).** The only race-start row in the install names the hash of `ArrestF02`
(camera track 0, flags 0x43, three to four cars, config 6). A race-start scene is not an arrest, so this row is
odd. Its meaning is **[unconfirmed]**; the race-start chooser is not in the CC0 decomp.

**Race end, online, and track masks** are all empty in this install. **[verified]**

## 7. Scene timing and length **[verified: counts; decomp: rules in nis-player.md]**

Every scene has `StartFrame` 0. Its length is `VanishFrame` frames, which is `VanishFrame / 30` seconds **[verified
against the camera track, section 8]**. Six examples:

| Scene | Type | Vanish (frames) | Length (s) | Car `_q` frames (channel) | Clip frames (actor) |
|---|---|---|---|---|---|
| `ArrestF02` | 2 | 232 | 7.73 | 2 (`car1`), 2 (`cop1`) | 119 (cop, perp, cuffs) |
| `IntroNis05` | 0 | 180 | 6.00 | 81 (`car1`), 91 (`car2`) | none (no skeletons) |
| `IntroNisDD` | 4 | 1290 | 43.00 | 676 (`car1`), 676 (`car2`) | none |
| `IntroNisBL01` | 4 | 480 | 16.00 | 2 (`car1`), 2 (`car2`) | none |
| `EndingNis04` | 4 | 150 | 5.00 | 101 (`car1`) | none |
| `FMVroc02` | 4 | 4000 | 133.33 | 2251 (`car1`), 2251 (`car2`) | none |

The decomp's length rule is a single rule: the scene ends at `VanishFrame / 30` seconds, not at the longest clip
**[decomp: `AnimScene.cpp`, `BindToGame`]**. The clips are not required to match: in `ArrestF06` the clip (433 frames)
is longer than the scene (340), and in `ArrestF02` it is shorter (119 against 232).

## 8. Camera sets for the scenes **[verified: counts and key bytes; decomp: layout in animation.md]**

The camera set of a scene is the chunk in pack `0x8003B200` (NIS) or `0x8003B201` (FMV) whose key is the scene's name
hash. Its tracks are named `Track 0`, `Track 1`, and so on. The layout of the pack, the camera record and the key is in
[formats/animation.md](../formats/animation.md) (section "ICE camera records" and "ICE camera keys").

### 8.1 Coverage **[verified]**

| Pack | Chunk id | Keys (scenes) | Tracks | Scenes matched |
|---|---|---|---|---|
| NIS | `0x8003B200` | 74 | 84 | 73 NIS scenes: the 26 arrests, the 14 world moments, and 33 others (race starts, career and D-Day starts, endings, `Flythrough`, `GenericStart`) |
| FMV | `0x8003B201` | 4 | 7 | `FMV17to21`, `FMV20T1`, `FMVTest`, `FMVroc02` |
| MKR | `0x8003B202` | 6 | 60 | none of the NIS scenes |
| Replay | `0x8003B203` | 1 (key `0x38D40DDF`) | 36 | none of the NIS scenes; the replay set is named in [formats/animation.md](../formats/animation.md) |

**No camera set** for 10 scenes: `FMVBLIntro`, `FMVNoCars`, `FMVPIN11X06`, `FMVPSA`, `IntroNisBL01`, `IntroNisBL08`,
`ReplayBurnout`, `ReplayCorner`, `ReplaySlide`, `ReplaySpeed` (no set under their own name hash). **[verified]** So `IntroNisBL01` has an animation bank and cars but no camera track in the install.

### 8.2 Track length and scene length **[verified]**

The length of `Track 0`, in seconds, equals `VanishFrame / 30` for **69 of the 77** scenes that have a camera track in
pack 0 or 1. The eight that do not:

| Scene | Track 0 length (s) | Vanish / 30 (s) |
|---|---|---|
| `FMVTest` | 1.0 | 20.0 |
| `Flythrough` | 1.0 | 45.0 |
| `WMBoat`, `WMFishingHouse`, `WMPorch`, `WMStripMall`, `WMTorusRoofShop`, `WMTrailerPark` | 3.33 | 3.0 |

The six world moments with 3.33 s tracks are the moment scenes that the pursuit-breaker spec describes
([pursuit-breaker-moment.md](pursuit-breaker-moment.md), section 4). **[verified]**

### 8.3 Space and key bytes **[verified: first key of each example track]**

The first key of `Track 0` in six examples (key byte 0x04 is the eye's space, 0x05 the look point's space):

| Scene | Track 0 length (s) | Keys | Eye and look space | First eye (x, y, z) | Lens (mm) | Simulation speed (%) |
|---|---|---|---|---|---|---|
| `ArrestF02` | 7.733 | 4 | 3 (scene), 3 | (2.539, −5.565, 1.365) | 37.5 | 100 |
| `IntroNis05` | 6.000 | 3 | 3, 3 | (−2.730, −19.738, 0.735) | 60.0 | 100 |
| `IntroNisDD` | 43.000 | 48 | 1 (world), 1 | (1038.395, −11.174, 246.477) | 112.5 | 100 |
| `IntroNisBL13` | 29.200 | 25 | 1, 1 | (2009.257, 1796.898, 152.146) | 54.1 | 87 |
| `EndingNis04` | 5.000 | 17 | 1, 1 | (−1005.152, 2618.682, 209.270) | 86.2 | 100 |
| `WMGasStation` | 3.000 | 1 | 3, 3 | (−4.824, −8.368, 5.223) | 24.0 | 100 |

Across all keys of pack 0 the space pairs are (scene, scene) 250 times, (world, world) 434 times, (car, car) 33 times and
two mixed (world eye, car look) keys. **[verified]**

The space bytes match the placement rule of the scenes (section 4 of [nis-player.md](nis-player.md)): scenes of types
0, 2 and 3 use the scene's own frame, and the scenes of type 4 use world coordinates. The key kind byte at 0x00 of these
first keys is 12 or 13; the kinds the existing docs list are 0 (no tangents) and do not include 12 or 13.
**[verified; meaning unconfirmed]**

## 9. Animation banks in the scene bundles **[verified]**

The bundle banks use the layout in [eagl-animation-bank.md](eagl-animation-bank.md). The NIS totals are:

| Quantity | Value |
|---|---|
| `EAGLAnimations` chunks in the 87 bundles | 111 |
| Bank animations | 805 |
| Animation type id | 15 (Compound) in all 805 |
| Child channel types | DeltaQuat (11) 330, DeltaLerp (10) 475, DeltaF3 (20) 64, DeltaF1 (21) 36 |
| FPS attribute | raw value `0x3030301E` in all 805; its first byte is 0x1E (30). The decomp's one-byte reader gives 0x30 instead, so the byte order is open (see [eagl-animation-bank.md](eagl-animation-bank.md), section 10.1) |
| Frame counts | 2 to 2,251 |
| Animations per bank | 1 bank: 59 bundles; 2 banks: 26 (arrests) |

Bank entries are 2 frames long for the car channels in several arrests. A two-frame clip is a hold pose.

## 10. Summary by family **[verified]**

| Family | Scenes | Type (0x18) | Entities | Cars (number) | Camera set | Overlay |
|---|---|---|---|---|---|---|
| Arrest (`ArrestF*`, `ArrestM*`, with `b` variants) | 26 | 2 | 2 or 3 | 2 | NIS pack, track 0 | none |
| Race start, intro (`IntroNis01`–`09`, `GenericStart`, `IntroNisToll01`) | 11 | 0 | 0 | 2 to 8 | NIS pack | none |
| Race start, intro type 4 (`IntroNis10`) | 1 | 4 | 0 | 4 | NIS pack | none |
| Career and blacklist race starts (`IntroNisBL01`–`BL16`, `BL06End`, `BL07End`) | 18 | 4 | 0 | 2 (`BL11`: 10) | NIS pack, except `BL01` and `BL08` (none) | `FLYBY` in 9 |
| D-Day (`IntroNisDD`, `IntroNisDDEnd`) | 2 | 4 | 0 | 2 | NIS pack | `FLYBY` |
| Endings (`EndingNis03`, `EndingNis04`) | 2 | 4 | 0 | 3 | NIS pack | none |
| FMV (`FMV*`) | 8 | 4 | 0 | 0 to 5 | FMV pack (4), none (4) | none |
| Flythrough | 1 | 4 | 0 | 0 | NIS pack | `FLYBY` |
| Replays (`ReplayBurnout`, `ReplayCorner`, `ReplaySlide`, `ReplaySpeed`) | 4 | 3 | 0 | 1 | none under the scene name | none |
| World moments (`WM*`) | 14 | 0 | 0 | 1 | NIS pack | none |

Totals: 87 scenes; 26 with entities; 85 with car animation; 73 + 4 with a camera set.

## 11. Six example scenes **[verified]**

| Scene | Bundle content | Header | Entities | Car channels | Camera | Notes |
|---|---|---|---|---|---|---|
| `IntroNis05` | 1 bank (6 anims), event sequence; no skeleton, no geometry | type 0, car 1 to 2 channels, 4 cars, 180 frames, no overlay | 0 | `car1`, `car2` (`_q`, `_s`, `_t`) | `Track 0`, 6.0 s, 3 keys, scene space | "Old Intro NIS 19" |
| `IntroNisDD` | 1 bank (30 anims), event sequence | type 4, 2 cars, 1290 frames, `FLYBY` | 0 | `car1`, `car2`, `cop1`..`cop8` | `Track 0`, 43.0 s, 48 keys, world space | D-Day race start |
| `IntroNisBL01` | 1 bank (8 anims), no event sequence; no skeleton, no geometry | type 4, 2 cars, 480 frames, no overlay | 0 | `car1`, `car2`, `cop2`, `cop3` (`_q`, `_t`) | **none** in any pack | "Career Racestart Number 01" |
| `EndingNis04` | 1 bank (23 anims), event sequence; no skeleton, no geometry | type 4, 3 cars, 150 frames | 0 | `car1`, `cop1`..`cop6`, `cop8` | `Track 0`, 5.0 s, 17 keys, world space | "End Game Outro" |
| `ArrestF02` | 2 banks, 3 skeletons, 3 geometry packs, 4 texture packs, event sequence | type 2, layout 1, 2 cars, 232 frames | 3 (`Bip21` cop, `Bip23` perp, `Bip24` cuffs) | `car1` (`_q`, `_t`, 2 frames), `cop1` (`_q`, `_s`, `_t`) | `Track 0`, 7.733 s, 4 keys, scene space | arrest; 48, 48 and 3 bones |
| `FMVroc02` | 1 bank (27 anims), event sequence | type 4, 2 cars, 4000 frames | 0 | `car1`, `car2`, `cop1`..`cop7` | FMV pack `Track 0`, 133.3 s, world space | "Mia meets hero" |

Two points from the table matter for the player spec. First, `IntroNis05`, `EndingNis04` and `IntroNisBL01` have no
actor skeletons, so their actors (other than the cars) are not in the bundle: their character content is absent from
the file. Second, `IntroNisBL01` has no camera set. **[verified]**

## 12. Corrections to the existing docs **[verified]**

- [formats/animation.md](../formats/animation.md), "NIS scene chunk": "64 of 87 scenes have these" (the entity
  records). The count is 26 scenes holding 64 records. The 64 is the number of records, not of scenes.
- The same section gives the header as 152 bytes. It is 0x40 bytes plus the description, 68 to 188 bytes in the
  install. The 152-byte size belongs to the two `ArrestF02` headers only.
- The same section says the entity record is "152 B per scene actor". It is 152 bytes on disk, with 8 bytes of
  alignment padding. The data is 144 bytes.
- The same section names `0x00037060` `AnimDirectorySceneLoadData` (18,440 B) and `0x00037070`
  `AnimDirectorySceneMappingData` (4,180 B). The sizes are right. The second is `CAnimCandidateData`, a table of
  moment, special-car and race candidates, not a file mapping.
- [specs/busted-sequence.md](busted-sequence.md), section 4: the arrest scene is chosen by the perp car's heat level in
  the decomp (`IPerpetrator::GetHeat()`), not by a "class" value. The fall-through between heat 2, 3 and the default
  table gives the same final names as the spec describes. **[decomp: `AnimChooseArrest.cpp`]**
- The same section says the arrest's camera track is chosen by the cutscene system. The decomp sets the track to 0 for
  every arrest and ignores the result of the camera-count call **[decomp: `AnimChooseArrest.cpp`]**.

## 13. Open questions (data) **[unconfirmed]**

- The race-start candidate row names `ArrestF02` (section 6.2). The race-start chooser, where this row would be read,
  is not in the CC0 decomp. **[unconfirmed – needs Ghidra]**
- The 7 special-car entries whose scene hashes match no installed scene. Their scenes may be in another build.
- The meaning of key kinds 12 and 13 (section 8.3).
- The meaning of the car channel `_s` (never read by the car code).
- Which record of the 152 bytes is the prop type's (0x60) tail: no prop exists in the install, so it cannot be checked.
- The `CarpEventSequence` contents (59 chunks) are not decoded here.

## 14. Method and sources

- Install scan: a Python 3 script run with `python -I` that reads all 87 `NIS/Scene_*_BundleB.bun`, the InGameB
  directory and camera packs, and the ELF objects inside each bundle. It uses the repo's `tools/chunkdump.py` parser and
  `tools/bchunk_names.py` names. The scripts are in the reader's scratch folder and are not committed.
- Checks run: header hash against name (87/87); entity skeleton and animation hashes against the bundle's own names
  (64/64 skeleton, 64/64 animation); directory slots against the NIS file names (87/87); camera set keys against scene
  hashes (73 + 4); track length against `VanishFrame / 30` (69/77).
- Decomp files read (CC0, `dbalatoni13/nfsmw`, `src/Speed/Indep/Src/`):
  `Animation/AnimScene.hpp` (10.5 KB) and `.cpp` (26.7 KB); `Animation/AnimEntity.hpp` (1.4 KB) and `.cpp` (1.1 KB);
  `Animation/AnimEntity_BasicCharacter.hpp` (3.8 KB) and `.cpp` (14.4 KB); `Animation/AnimEntity_Prop.hpp` (1.8 KB) and
  `.cpp` (2.1 KB); `Animation/AnimDirectory.hpp` (3.7 KB) and `.cpp` (2.0 KB); `Animation/AnimCandidates.hpp` (9.4 KB)
  and `.cpp` (6.7 KB, first part); `Animation/AnimChooseArrest.cpp` (8.9 KB); `Animation/AnimChooser.hpp` (0.6 KB);
  `Animation/AnimLocator.cpp` (3.5 KB); `Animation/AnimPlayer.hpp` (4.0 KB) and `.cpp` (17.5 KB); `Animation/AnimCtrl.cpp`
  (8.5 KB, `AdvanceAnimTime`); `Sim/Activities/NISActivity.cpp` (30.0 KB, partial: see the player spec);
  `Sim/Activities/QuickGame.cpp` (7.1 KB, NIS parts); `Interfaces/SimActivities/INIS.h`;
  `Interfaces/Simables/INISCarControl.h`; `Camera/ICE/ICEAnimScene.hpp`; `Camera/ICE/ICEManager.hpp` and `.cpp`
  (1.2 KB, mostly constructor); `Generated/Events/EPlayRaceNIS.hpp`; `Generated/Events/EPlayEndNIS.hpp`;
  `Generated/Messages/MNISComplete.h`. The ICE implementation files (`ICEAnimScene.cpp`, `ICEData.cpp`, `ICERender.cpp`,
  `ICEMover.cpp`) are 0 bytes in the CC0 tree.
- Repo docs read: [formats/animation.md](../formats/animation.md), [specs/eagl-animation-bank.md](eagl-animation-bank.md),
  [specs/eagl-animation-skeleton.md](eagl-animation-skeleton.md), [specs/eagl-animation-codecs.md](eagl-animation-codecs.md),
  [specs/pursuit-breaker-moment.md](pursuit-breaker-moment.md), [specs/busted-sequence.md](busted-sequence.md) section 4.
- Install files: `NIS/` (87 bundles), `GLOBAL/InGameB.bun`. Read only.
