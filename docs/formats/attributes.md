# AttribSys attributes (`VPAK`) — Need for Speed: Most Wanted (2005)

**AttribSys** is MW's database of **gameplay and tuning data**: cars, physics, AI, pursuit, races,
audio parameters, effects. It is not bChunk data. A file is a **`VPAK`** pack of one or more
**vaults**, and each vault holds typed **classes** (schemas) and **collections** (instances), keyed by
32-bit name hashes. For the tag meanings, see [evidence tags](../README.md#evidence-tags).

> **Status:** pack, vault, export and hash layers are **[verified]** against all four files below.
> The attribute-value encoding is described from VaultLib and the decomp, and has not been re-derived
> here.

## Files **[verified]**

| File | Size | Vaults | Contents |
|---|---|---|---|
| `GLOBAL/attributes.bin` | 668,696 | 1 (`db`) | **57 class definitions** + 2,376 collections (table below) |
| `GLOBAL/gameplay.bin` | 2,097,088 | **272** (`10_2_1_sprint`, `11_4_1_tollbooth`, `race_bin_*`, `challenge_*`, `gpcore`, `escape_the_cops`, …) | 6,293 collections of class `gameplay`: race events, Blacklist race bins, challenges, milestones |
| `GLOBAL/gameplay.lzc` | 2,097,104 | 272 | the same pack in a `RAWW` wrapper (16-byte header, see [bchunk.md](bchunk.md)); `gameplay.lzc.bak` is the original JDLZ |

`gameplay.bin` also contains 253 `HUFF`-compressed blobs ([huff.md](huff.md)); all decode with `nfsmw-compress` **[verified]**. Which vault data they hold is **[unconfirmed]**.
| `GLOBAL/FE_ATTRIB.bin` | 111,008 | 1 (`frontend`) | 446 collections of class `frontend` |

`*.bak` files are mod-tool backups. Class definitions exist only in `attributes.bin`; the other packs
contain collections of classes defined there. Race-event vault names match the per-event minimap
files `TRACKS/L2RA/MINI_MAP_<id>.BIN` ([world.md](world.md#minimap)).

## Pack (`VPAK`) **[community + verified]**

| Offset | Type | Field |
|---|---|---|
| 0x00 | char[4] | `VPAK` |
| 0x04 | u32 | vault count |
| 0x08 | u32 | string-block offset (vault names, NUL-terminated) |
| 0x0C | u32 | string-block size |
| 0x10 | 20 B × count | `{u32 nameOffset (into string block), u32 binSize, u32 vltSize, u32 binOffset, u32 vltOffset}` |

Offsets are file-relative (after unwrapping `RAWW`). The data is 0x80-aligned (e.g. `attributes.bin`:
bin at 0x80, vlt at 0x584D8). Each vault is two blobs:

- **`.vlt`**: the structure. A chunk list `char[4] id, u32 size (header included)`, in order:
  - `DepN`: dependency names, e.g. `db.vlt`, `db.bin`
  - `StrN`
  - `DatN`: export data
  - `ExpN`: export table
  - `PtrN`: pointer fix-ups

  VaultLib also knows `EndC`.
- **`.bin`**: payload referenced by the `.vlt`. It starts with a `StrE` string chunk (attribute
  strings), followed by raw data that `PtrN` fix-ups point into.

## Exports (`ExpN`) **[community + verified]**

`u32 count`, then 20-byte entries `{u32 id (name hash), u32 type, u32 0, u32 size, u32 offset}`. The
offset points into the `.vlt` stream.

| Type hash | Export | Counts in `attributes.bin` |
|---|---|---|
| `0x5E970CBC` | `Attrib::ClassLoadData` | 57 |
| `0x8E112EB7` | `Attrib::CollectionLoadData` | 2,376 |
| `0xCBBC628F` | `Attrib::DatabaseLoadData` | 1 |

- **ClassLoadData** (legacy layout, VaultLib `LegacyBase/Exports/ClassLoad.cs`): u32 class hash, u32
  collection reserve, i32 definition count, pointer to definitions, … Each definition is `{u32 key,
  u32 type, u16 offset, u16 size, u16 maxCount, u8 flags, u8 log2(alignment)}`.
- **CollectionLoadData** (`CollectionLoad.cs`): u32 key, u32 class, u32 parent, u32 table reserve, u32
  ?, u32 entry count, u32 type count, pointer to layout, u32 types[], then per entry `{u32 key, inline
  value or pointer, u16 typeIndex, u16 nodeFlags}`. A collection's **parent** provides inherited values.
- **Pointers** are not stored as absolute values. `PtrN` lists fix-ups (`PtrSetFixupTarget`,
  `PtrDepRelative`, `PtrNull`) that tell the loader which dependency (`.vlt` or `.bin`) each pointer
  is relative to.

## Name hash **[verified]**

All names (vaults' contents, classes, fields, collections, types) are hashed with **Bob Jenkins'
lookup2** (`hash()`; golden ratio `0x9E3779B9`, 12-byte blocks) with **initval `0xABCDEF00`**:
VaultLib `VLT32Hasher.Hash(s, 0xABCDEF00)`. Checked against the decomp's `symbols/vlt.txt`:
**13,207 of 13,208** names hash to the listed value. Examples: `pvehicle` → `0x4A97EC8F`,
`gameplay` → `0x5CEA9D46`. This is **not** the `bStringHash` used by bChunk data
([models.md](models.md#name-hashing)).

Name dictionaries: decomp `symbols/vlt.txt` (13,208 entries, CC0) and Attribulator
`Resources/hashes.txt` (45,903 lines, no license).

## The 57 classes in `attributes.bin` **[verified]**

Fields = definition count, collections = number of instances. All names were resolved through
`vlt.txt`. The decomp has a generated header with field names, types and offsets for every class:
`src/Speed/Indep/Src/Generated/AttribSys/Classes/<class>.h` **[decomp]**.

| Class | Hash | Fields | Collections |
|---|---|---|---|
| `acceltrans` | `0xFF77F451` | 5 | 28 |
| `aivehicle` | `0x22515733` | 9 | 23 |
| `aud_moment_strm` | `0xD2410816` | 6 | 45 |
| `aud_stitch_loop` | `0x3473EDCD` | 2 | 2 |
| `audioimpact` | `0xFBFFB107` | 7 | 131 |
| `audioscrape` | `0x11B47832` | 2 | 7 |
| `audiosystem` | `0xD3C18F03` | 20 | 9 |
| `brakes` | `0x36350867` | 3 | 90 |
| `camerainfo` | `0x93C171E4` | 9 | 69 |
| `chassis` | `0xAFA210F0` | 19 | 97 |
| `chopperspecs` | `0x5D898EE7` | 24 | 3 |
| `collisionreactions` | `0xB32682F1` | 4 | 31 |
| `controller` | `0x2DEE1998` | 152 | 21 |
| `damagespecs` | `0xC1F0B434` | 16 | 14 |
| `ecar` | `0xA5B543B7` | 49 | 100 |
| `effects` | `0xEBCEE74C` | 20 | 198 |
| `emitterdata` | `0xB30B18AF` | 45 | 205 |
| `emittergroup` | `0xABA86E60` | 4 | 128 |
| `emitteruv` | `0xE4983A7D` | 4 | 4 |
| `engine` | `0xF1F5FBC7` | 7 | 89 |
| `engineaudio` | `0x50EAB0E6` | 40 | 70 |
| `explosion` | `0x6434F1FB` | 6 | 1 |
| `fecooling` | `0x5D417978` | 11 | 1 |
| `frontend` | `0x85885722` | 55 | 0 (+ 446 in `FE_ATTRIB.bin`) |
| `fuelcell_effect` | `0x6F5943F1` | 2 | 4 |
| `fuelcell_emitter` | `0xB267A856` | 19 | 8 |
| `gameplay` | `0x5CEA9D46` | 220 | 0 (+ 6,293 in `gameplay.bin`) |
| `induction` | `0xC92A0142` | 7 | 69 |
| `infractions` | `0x2870DE90` | 1 | 8 |
| `junkman` | `0x171737E9` | 7 | 1 |
| `light_flares_cg` | `0xC7C5806D` | 8 | 22 |
| `milestonetypes` | `0xE4C3D904` | 4 | 30 |
| `music` | `0x565465F8` | 5 | 27 |
| `nos` | `0xB1669F64` | 8 | 6 |
| `ocean` | `0x093D7C56` | 5 | 1 |
| `presetride` | `0x27E73952` | 7 | 25 |
| `pursuitescalation` | `0xD6D4330B` | 4 | 1 |
| `pursuitlevels` | `0x551E22B3` | 57 | 21 |
| `pursuitsupport` | `0x77B93104` | 4 | 21 |
| `pvehicle` | `0x4A97EC8F` | 66 | 121 |
| `rigidbodyspecs` | `0x7C90BB38` | 23 | 22 |
| `shiftpattern` | `0xDB01B754` | 24 | 25 |
| `simsurface` | `0xFB111FEF` | 24 | 47 |
| `smackable` | `0xCE70D7DB` | 30 | 181 |
| `speech` | `0xC593DD47` | 28 | 133 |
| `speechtune` | `0xBC683501` | 39 | 1 |
| `system` | `0x4E8DCE05` | 3 | 1 |
| `timeofdaylighting` | `0x399ED882` | 13 | 8 |
| `tires` | `0xBD38D1CA` | 9 | 95 |
| `trafficpattern` | `0x20D08342` | 5 | 10 |
| `transmission` | `0x07A7A3E5` | 9 | 89 |
| `turbosfx` | `0x55624A85` | 6 | 18 |
| `visuallook` | `0x339F7D3D` | 8 | 5 |
| `visuallookeffect` | `0x6AB2D241` | 7 | 7 |
| `visuallooktransition` | `0x0F409AA6` | 7 | 1 |
| `visualrgbtweaker` | `0xAF1837CA` | 3 | 1 |
| `world` | `0x6D90DA55` | 33 | 1 |

The decomp also generates `lightmaterials` and `lightshaders` headers, but no MW PC pack defines those
classes **[verified]**.

## Tools

| Tool | What | Language / license |
|---|---|---|
| [NFSTools/VaultLib](https://github.com/NFSTools/VaultLib) | Full read/write of legacy (pre-2006) and modern AttribSys; `VaultLib.Support.MostWanted` has MW's custom types | C#, **MIT** |
| [NFSTools/Attribulator](https://github.com/NFSTools/Attribulator) | CLI export/import as text/YAML; `MostWantedProfile` loads `attributes.bin`, `fe_attrib.bin`, `gameplay.bin` | C#, **no LICENSE file** |
| [NFSTools/vltedit](https://github.com/NFSTools/vltedit), [NFSTools/vpak](https://github.com/NFSTools/vpak) | Arushan's original VLTEdit / VPAK (2005–06) source, "distributed with permission" | C#, no LICENSE file |
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) | `symbols/vlt.txt`, `Src/Generated/AttribSys/Classes/*.h`, AttribSys runtime under `src/Speed/Indep/Tools/AttribSys/` | C++, CC0-1.0 |

## Next steps

1. Add `tools/vpakdump.py`: pack → vaults → exports, resolve names with lookup2 + `vlt.txt`, then dump
   collections using the field offsets from the generated class headers.
2. Decode the attribute value types (`Attrib::StringKey`, refspecs, arrays) against VaultLib's
   `Types/` folder.
