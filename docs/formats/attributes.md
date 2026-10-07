# AttribSys attributes (`VPAK`) — Need for Speed: Most Wanted (2005)

**AttribSys** is MW's database of **gameplay and tuning data**: cars, physics, AI, pursuit, races,
audio parameters, effects. It is not bChunk data. A file is a **`VPAK`** pack of one or more
**vaults**, and each vault holds typed **classes** (schemas) and **collections** (instances), keyed by
32-bit name hashes. For the tag meanings, see [evidence tags](../README.md#evidence-tags).

> **Status:** everything on this page is **[verified]** with the Rust reader
> [`blackbox-attrib`](../../libs/blackbox-attrib): every vault, class, collection and value of the three
> packs (and of the 2005 originals kept as `*.bak`) decodes, every parent and every non-null `RefSpec`
> resolves. Game-specific record and enum types are read as raw bytes (see [Values](#values-verified)).

## Files **[verified]**

| File | Size | Vaults | Contents |
|---|---|---|---|
| `GLOBAL/attributes.bin` | 668,696 | 1 (`db`) | **57 class definitions** + 2,376 collections (table below) |
| `GLOBAL/gameplay.bin` | 2,097,088 | **272** (`10_2_1_sprint`, `11_4_1_tollbooth`, `race_bin_*`, `challenge_*`, `gpcore`, `escape_the_cops`, …) | 6,293 collections of class `gameplay`: race events, Blacklist race bins, challenges, milestones, script handlers |
| `GLOBAL/gameplay.lzc` | 2,097,104 | 272 | the same pack in a `RAWW` wrapper (16-byte header, see [bchunk.md](bchunk.md)) |
| `GLOBAL/FE_ATTRIB.bin` | 111,008 | 1 (`frontend`) | 446 collections of class `frontend` |

`*.bak` files are mod-tool backups of the 2005 originals; they read with the same code:
`attributes.bak` (689,728 bytes, 57 classes, 2,309 collections), `fe_attrib.bak` (411 collections),
`gameplay.bak` and the JDLZ-compressed `gameplay.lzc.bak` (272 vaults, 6,293 collections each). The
installed `attributes.bin` was rewritten by a mod tool (its string table starts with
`NFS-VltEd 4.6.0.0 by nfsu360`).

Class definitions exist only in `attributes.bin`; the other packs contain collections of classes
defined there, so it has to be loaded first. Race-event vault names match the per-event minimap
files `TRACKS/L2RA/MINI_MAP_<id>.BIN` ([world.md](world.md#minimap)).

## Pack (`VPAK`) **[community + verified]**

| Offset | Type | Field |
|---|---|---|
| 0x00 | char[4] | `VPAK` |
| 0x04 | u32 | vault count |
| 0x08 | u32 | string-block offset (vault names, NUL-terminated) |
| 0x0C | u32 | string-block size |
| 0x10 | 20 B × count | `{u32 nameOffset (into string block), u32 binSize, u32 vltSize, u32 binOffset, u32 vltOffset}` |

Offsets are file-relative (after unwrapping `RAWW`/`JDLZ`). The data is 0x80-aligned (e.g.
`attributes.bin`: bin at 0x80, vlt at 0x584D8).

## Vault blobs **[verified]**

Each vault is two blobs. Pointers inside them are stored as 0 and patched at load time from the
`PtrN` chunk ([below](#pointer-fix-ups-ptrn-community--verified)).

- **`.vlt`**: the structure, a list of chunks `{u32 id, u32 size (header included), payload}`. The id
  is a FourCC stored reversed (the bytes read `NpeD` for `DepN`). Every MW vault has exactly
  `DepN`, `StrN`, `DatN`, `ExpN`, `PtrN`, in that order, filling the blob; chunks are padded to 16
  bytes. VaultLib also knows `EndC` and `Vers` (later games).
  - `DepN`: `u32 count`, `u32 hash[count]`, `u32 nameOffset[count]`, NUL-terminated names. Always two
    dependencies, `<vault>.vlt` and `<vault>.bin` (`db.vlt` → `0x18228ADE`, `db.bin` → `0x610CE10B`).
  - `StrN`: 8 zero bytes.
  - `DatN`: the export records (class, collection and database headers).
  - `ExpN`: the export table.
  - `PtrN`: pointer fix-ups.
- **`.bin`**: the payload. It starts with a `StrE` chunk (NUL-terminated strings: collection names
  and string values; 1,989 strings in `attributes.bin`), followed by raw data: class definitions,
  collection layout blocks, out-of-line values, the type-name table.

## Pointer fix-ups (`PtrN`) **[community + verified]**

12-byte records `{u32 fixupOffset, u16 kind, u16 index, u32 destination}`, ended by a `PtrEnd`:

| Kind | Name (VaultLib) | Meaning |
|---|---|---|
| 0 | `PtrEnd` | end of the table |
| 1 | `PtrNull` | the pointer at `fixupOffset` stays null |
| 2 | `PtrSetFixupTarget` | the following `fixupOffset`s are in dependency `index` (0 = `.vlt`, 1 = `.bin`) |
| 3 | `PtrDepRelative` | the pointer at `fixupOffset` points to `destination` in dependency `index` |
| 4 | `PtrExport` | never used in MW |

Every MW vault has one `SetFixupTarget(1)` group (pointers inside the `.bin`, e.g. string pointers of
layout blocks), then one `SetFixupTarget(0)` group (pointers in the `.vlt` export records). All
fix-ups are `PtrDepRelative` with index 1: **every pointer points into the `.bin`**. No `PtrNull`
occurs, and every patched slot holds 0 in the file.

| File | Fix-ups in `.bin` | Fix-ups in `.vlt` |
|---|---|---|
| `attributes.bin` | 2,876 | 4,710 |
| `gameplay.bin` (272 vaults) | 13,941 | 23,843 |
| `FE_ATTRIB.bin` | 446 | 666 |

## Exports (`ExpN`) **[community + verified]**

`u32 count`, then 20-byte entries `{u32 id, u32 type, u32 0, u32 size, u32 offset}`. The offset
points into the `.vlt` (`DatN`).

| Type hash | Export | Export id | Counts in `attributes.bin` |
|---|---|---|---|
| `0x5E970CBC` | `Attrib::ClassLoadData` | the class key (57 of 57) | 57 |
| `0x8E112EB7` | `Attrib::CollectionLoadData` | `hash("<class>/<collection>")` (all 1,680 whose name is known) | 2,376 |
| `0xCBBC628F` | `Attrib::DatabaseLoadData` | `0xF1DFAC8D` | 1 |

`gameplay.bin` and `FE_ATTRIB.bin` export only collections. Field names below are the decomp's
**[decomp]**; reading order follows VaultLib's `LegacyBase` **[community]**; all offsets
**[verified]**.

### `Attrib::DatabaseLoadData` (0x188 bytes)

| Offset | Field | MW value |
|---|---|---|
| 0x00 | `mNumClasses` | 88 (a reserve; there are 57 classes) |
| 0x04 | `mDefaultDataSize` | 0x11D |
| 0x08 | `mNumTypes` | 94 |
| 0x0C | `mTypenames` → `.bin` | 94 NUL-terminated type names, back to back |
| 0x10 | `u32 size[mNumTypes]` | value size per type, same order |

The type table names every type fields use: `EA::Reflection::{Bool, Int8…Int64, UInt8…UInt64, Float,
Double, Text, …}` (1–8 bytes), `Attrib::Key` (4), `Attrib::StringKey` (16), `Attrib::RefSpec` (12),
`Attrib::Blob` (8), `Attrib::Types::{Vector2, Vector3, Vector4, Matrix, Quaternion, Colour, …}`, and
game types such as `EffectLinkageRecord` (32), `UpgradeSpecs` (16), `AxlePair` (8), `GRace::Type` (4).

### `Attrib::ClassLoadData` (0x1C bytes)

| Offset | Field | Notes |
|---|---|---|
| 0x00 | `mClass` | class key |
| 0x04 | `mCollectionReserve` | e.g. 222 for `pvehicle` (121 collections) |
| 0x08 | `mNumDefinitions` | field count |
| 0x0C | `mDefinitions` → `.bin` | `Attrib::Definition[mNumDefinitions]` |
| 0x10 | `mLayoutSize` | size of the per-collection layout block (`pvehicle`: 0x4E) |
| 0x14 | `mLayoutKeyShift` | always 0 |
| 0x18 | `mLayoutCount` | in-layout fields **without** the not-searchable flag (57 of 57 classes); `emitterdata` has 44 in-layout fields but 5 |

`Attrib::Definition` (16 bytes): `{u32 key, u32 type, u16 offset, u16 size, u16 maxCount, u8 flags,
u8 log2(alignment)}`. `offset` is only meaningful for in-layout fields; `size` is one value's size;
`maxCount` is an array's capacity (1 otherwise). Flags: 0x01 array, 0x02 in layout, 0x04 bound, 0x08
not searchable. Seen combinations: 0x04 (454 fields), 0x06 (391), 0x05 (280), 0x0E (55), 0x07 (34).

### `Attrib::CollectionLoadData` (0x20 + 4 × types + 12 × entries bytes)

| Offset | Field | Notes |
|---|---|---|
| 0x00 | `mKey` | collection key |
| 0x04 | `mClass` | class key |
| 0x08 | `mParent` | parent collection key (same class), 0 = none |
| 0x0C | `mTableReserve` | always equal to `mNumEntries` |
| 0x10 | `mTableKeyShift` | always 0 |
| 0x14 | `mNumEntries` | |
| 0x18 | `mNumTypes` | |
| 0x1C | `mLayout` → `.bin` | layout block; null only for classes without in-layout fields (193 collections) |
| 0x20 | `u32 types[mNumTypes]` | the type keys the entries use |
| … | `AttribEntry[mNumEntries]` | 12 bytes each |

`AttribEntry`: `{u32 key, u32 data, u16 typeIndex, u8 nodeFlags, u8 entryFlags (0)}`. `typeIndex`
indexes `types[]` and always matches the field's type. **`data` holds the value itself when the field
is not an array and its size is ≤ 4** (node flag 0x20, "by value"; a `Text` stored this way is a
pointer with a `.vlt` fix-up, 8,456 of them in `gameplay.bin`); **otherwise it is a pointer** to the
value in the `.bin` (node flag 0x02 for arrays). `attributes.bin`: 1,303 by-value entries, 1,829
arrays, 640 other pointers; the flags agree with the definitions on every entry of the three packs.

**Layout block:** every in-layout field sits at its definition's `offset`, so each collection carries
all of them (`mLayoutSize` bytes). For `pvehicle` it holds `TENSOR_SCALE` (0x00), `MODEL` (0x10),
`DefaultPresetRide` (0x20), `CollectionName` (0x24), the `*_upgrades` counts, `MASS` (0x38),
`VerbalType` (0x40), `HornType` (0x4C), `TrafficEngType` (0x4D), matching the decomp's generated
`_LayoutStruct` **[decomp + verified]**.

## Values **[verified]**

| Type | Size | Encoding |
|---|---|---|
| `EA::Reflection::Bool`, `Int8`, `UInt8` | 1 | |
| `Int16`, `UInt16` / `Int32`, `UInt32`, `Float` / `Int64`, `UInt64`, `Double` | 2 / 4 / 8 | little-endian |
| `EA::Reflection::Text` | 4 | pointer to a NUL-terminated string (in `StrE`) |
| `Attrib::Key` | 4 | a name hash |
| `Attrib::StringKey` | 16 | `{u64 hash64, u32 hash32 = hash(string), pointer string}` |
| `Attrib::RefSpec` | 12 | `{u32 class, u32 collection, u32 0}` (the game caches a pointer in the last word) |
| `Attrib::Blob` | 8 | `{u32 size, pointer data}`; the data is usually compressed ([below](#blobs-lua-bytecode-verified)) |
| `Attrib::Types::Vector2/3/4` | 8/12/16 | floats |
| `Attrib::Types::Matrix` | 64 | 16 floats |
| enums (`GRace::Type`, `eDRIVE_BY_TYPE`, …) | 4 | a `u32` |
| records (`AxlePair`, `UpgradeSpecs`, `EffectLinkageRecord`, …) | various | game structs, field layouts in the decomp's `MWAttribUserTypes.h`; `AxlePair` is `{f32 front, f32 rear}` |

**Arrays** (`Attrib::Array`): an 8-byte header `{u16 capacity, u16 count, u16 itemSize, u16 flags}`,
then `count` items `itemSize` apart. In a layout block the capacity is the definition's `maxCount`
(the decomp's `_Array_TORQUE` + `TORQUE[9]`; 2,368 of 2,368 in-layout arrays); out of line,
capacity = count (3,734 of 3,734 in the three packs). Flag 0x8000 ("aligned at 16") puts the items one
header later, at +16. It occurs on the in-layout `ecar` `TireOffsets` (`Vector4[4]`, 100 collections),
where the decomp's layout has `_Pad_TireOffsets[8]` between header and items, and on 11 out-of-line
`Attrib::Types::Matrix` arrays in `shiftpattern`, where the next value starts exactly 16 + count × 64
bytes on. The low 15 bits are 0 in the files (the game stores a type index there at run time)
**[decomp + verified]**.

## Inheritance **[verified]**

A collection's own values are its layout block plus its entries. A field it has no entry for comes
from the parent, then the grandparent, and so on (in-layout fields are always its own). 2,214 of the
2,376 collections in `attributes.bin` have a parent, and all parents resolve; with all three packs
loaded, all parents and all 2,029 non-null `RefSpec`s resolve (in `attributes.bin` alone, 1,768 of
1,823 do; the rest point to `frontend` and `gameplay` collections).

**Example**, `pvehicle/bmwm3gtr`: lineage `bmwm3gtr` → `racers` → `cars` → `default`. `MASS` 1350,
`MODEL` "BMWM3GTR", `TENSOR_SCALE` (1, 2, 1, 0). `chassis`, `engine`, `tires`, `transmission`,
`brakes` are one-element `RefSpec` arrays to the `bmwm3gtr` collection of that class, `induction`
refers to `induction/default`, `nos` has two levels. Following them: `engine` `TORQUE` = 149, 179,
222, 266, 283, 275, 255, 235, 228, `RED_LINE` 8500, `MAX_RPM` 9500; `transmission` `GEAR_RATIO` =
3.79, 0, 3.4, 2.085, 1.5, 1.25, 1.09, 0.91 (reverse, neutral, 1st–6th **[unconfirmed]**),
`FINAL_GEAR` 3.85; `chassis` `WHEEL_BASE` 2.725, `FRONT_WEIGHT_BIAS` 54; `tires` `RIM_SIZE` (19, 19).

## Blobs: Lua bytecode **[verified]**

The only `Attrib::Blob` values are the `gameplay` field `bytecode`, all 259 in the `gpcore` vault of
`gameplay.bin`: **253 `HUFF`** ([huff.md](huff.md); 67,346 → 88,795 bytes), 5 `JDLZ`, and one empty
blob (`messagehandler`). All decompress with `ea-compress`, and every one starts with
`1B 4C 75 61 50 01 04 04 04 06 08 09 09 04` and the float 3.14159265e7: a **Lua 5.0 compiled chunk**
(`\x1bLua`, version 0x50, little-endian, 4-byte int/size_t/instruction, 4-byte `float` numbers). The
collections are state-graph handlers, named `stategraph_<graph>_<state>_handler_<event>` (for example
`stategraph_basicrace_raceover_handler_notifyraceabandoned`). Decompiling them is future work.

## Name hash **[verified]**

All names (vaults' contents, classes, fields, collections, types) are hashed with **Bob Jenkins'
lookup2** (`hash()`; golden ratio `0x9E3779B9`, 12-byte blocks) with **initval `0xABCDEF00`**:
VaultLib `VLT32Hasher.Hash(s, 0xABCDEF00)`. Checked against the decomp's `symbols/vlt.txt`:
**13,207 of 13,208** names hash to the listed value. Examples: `pvehicle` → `0x4A97EC8F`,
`gameplay` → `0x5CEA9D46`. This is **not** the `bStringHash` used by bChunk data
([models.md](models.md#name-hashing)). (VaultLib maps the empty string to 0; plain lookup2 gives
`0x82FC1624`.)

The files carry few names: `StrE` names 1,680 of the 2,376 collections in `attributes.bin`, but only
13 of the 1,169 field keys and 3 of the 57 class keys. Name dictionaries: decomp `symbols/vlt.txt`
(13,208 entries, CC0) and Attribulator `Resources/hashes.txt` (45,903 lines, no license).

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

## Reading it in Rust

[`libs/blackbox-attrib`](../../libs/blackbox-attrib) reads all of the above from bytes:

```rust
use blackbox_attrib::Database;

let mut db = Database::open(&attributes_bin)?; // classes first
db.load(&gameplay_bin)?; // VPAK, optionally RAWW/JDLZ/HUFF-wrapped
let car = db.collection("pvehicle", "bmwm3gtr").unwrap();
let mass = car.get_f32("MASS"); // Some(1350.0), inherited values included
let engine = car.follow("engine").unwrap(); // RefSpec → collection
let torque = engine.get("TORQUE").and_then(|v| v.as_array());
```

- `Database::load_vault(name, vlt, bin)` takes a bare vault pair; `db.names()` maps hashes back to the
  strings found in the files, and `Names::add_lines` adds an external list (e.g. `vlt.txt`).
- Values are decoded at load time into `Value` (numbers, `Text`, `StringKey`, `RefSpec`, vectors,
  `Blob` with `decompress()`, arrays); other types stay `Value::Raw`.
- Record layouts live in `src/layout/` per AttribSys generation and are detected per vault from the
  export table. Only the legacy (2005) layout exists; Carbon-and-later vaults (16-byte export entries,
  0x20-byte `ClassLoadData`) fail with `Error::UnsupportedLayout` until a `modern.rs` is added.
- Tests: `cargo test -p blackbox-attrib` (synthetic packs) and, with `NFSMW_GAME_DIR` set,
  `cargo test --release -p blackbox-attrib -- --ignored` (every number on this page).

## Tools

| Tool | What | Language / license |
|---|---|---|
| [NFSTools/VaultLib](https://github.com/NFSTools/VaultLib) | Full read/write of legacy (pre-2006) and modern AttribSys; `VaultLib.Support.MostWanted` has MW's custom types. Parts of `blackbox-attrib` are ported from it (see NOTICE) | C#, **MIT** |
| [NFSTools/Attribulator](https://github.com/NFSTools/Attribulator) | CLI export/import as text/YAML; `MostWantedProfile` loads `attributes.bin`, `fe_attrib.bin`, `gameplay.bin` | C#, **no LICENSE file** |
| [NFSTools/vltedit](https://github.com/NFSTools/vltedit), [NFSTools/vpak](https://github.com/NFSTools/vpak) | Arushan's original VLTEdit / VPAK (2005–06) source, "distributed with permission" | C#, no LICENSE file |
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) | `symbols/vlt.txt`, `Src/Generated/AttribSys/Classes/*.h`, AttribSys runtime under `src/Speed/Indep/Tools/AttribSys/` | C++, CC0-1.0 |

## Next steps

1. Decode the game record types (`AxlePair`, `UpgradeSpecs`, `EffectLinkageRecord`, …) from the
   decomp's `MWAttribUserTypes.h` in the game crate, on top of `Value::Raw`.
2. Document the `gameplay` class (race events, Blacklist bins) and the Lua 5.0 state-graph scripts.
3. Add the modern (Carbon+) layout when another game is supported.
