# Car data: type info, parts database, presets, vinyls, tuning

Car data is split between bChunk tables in `GLOBAL/GlobalB.lzc` (identity, parts, presets, bounds),
per-car folders (models and textures) and AttribSys (all physics and tuning). For the tag meanings,
see [evidence tags](../README.md#evidence-tags).

## Where it lives **[verified]**

| Data | File / chunk | Size / count |
|---|---|---|
| Car type table | `GlobalB.lzc` → `00034600 CarTypeInfos` | 18,936 B = 8 B of `0x11` padding + **91 × 0xD0** records (`911TURBO`, `CARRERAGT`, `VIPER`, `COPMIDSIZE`, `COPHELI`, …) |
| Parts database | `GlobalB.lzc` → `80034602 CarPartPack (DBCarParts)` | 253,628 B (children below) |
| Slot types | `GlobalB.lzc` → `00034607 CarPartTypeNameTable (SlotTypes)` | 1,272 B = **159 × 8** |
| Animation hookup / hide | `00034608`, `00034609` | 140 B, 256 B |
| Preset cars | `GlobalB.lzc` → `00030220 PresetRides` | 53,792 B = **82 × 0x290** |
| Car collision bounds | `GlobalB.lzc` → `8003B900 BoundsPack` | 86 bounds (see [world.md](world.md)) |
| Light materials (car paint/shading) | `GlobalB.lzc` → 156 × `00135200 LightMaterials` | 168 B each |
| Models | `CARS/<CAR>/GEOMETRY.BIN` (100) | `GeometryPack`, see [models.md](models.md) |
| Textures | `CARS/<CAR>/TEXTURES.BIN` (93 + 7 empty) | TPK, see [textures.md](textures.md) |
| Vinyls | `CARS/<CAR>/VINYLS.BIN` (47 + 50 empty), `PREVINYL.BIN` (39 + 27 empty) | **TPK** (bitmap vinyls) |
| Physics / tuning | `GLOBAL/attributes.bin` | AttribSys, see below |

The live `GlobalB.lzc` is stored uncompressed by mod tools; `GLOBALB.LZC.bacc` is the JDLZ original
([install-layout.md](../install-layout.md)). `GLOBALB.BUN` (2.8 MB) has the same chunk types with
different content: 84 car types and a 233,004-byte parts pack. Whether the PC game loads it at all is
**[unconfirmed]**. The 7 extra types in `GlobalB.lzc` are add-on cars: `FXXEVO`, `LEVIN`, `SF90`, `SKYLINEZT`,
`TRUENO`, `TRUENOCP`, `TRUENOID` **[verified]**. The eighth add-on folder, `COROLLAE88`, has no
CarTypeInfo entry.

## Parts database (`0x80034602`) **[verified sizes; names from the decomp]**

| Chunk | Decomp name | Community name | Size |
|---|---|---|---|
| `00034603` | `SPEED_CARPART_PACK` | DBCarParts_Header | 60 |
| `00034604` | `SPEED_CARPART_PARTS_TABLE` | DBCarParts_Array | 183,128 |
| `00034605` | `SPEED_CARPART_ATTRIBUTES_TABLE` | DBCarParts_Attribs | 14,968 |
| `00034606` | `SPEED_CARPART_STRING_TABLE` | DBCarParts_Strings | 26,480 |
| `0003460A` | `SPEED_CARPART_MODELNAMEHASH_TABLE` | DBCarParts_Structs | 13,032 |
| `0003460B` | `SPEED_CARPART_TYPENAMEHASH_TABLE` | DBCarParts_Models | 428 |
| `0003460C` | `SPEED_CARPART_ATTRIBUTETABLE_TABLE` | DBCarParts_Offsets | 15,460 |
| `0003460D` | — | DBCarParts_Custom | 8 |

The decomp's names describe what the tables hold better than the community ones. Field layouts are
implemented in Nikki's `DBModelPart` / `Parts/CarParts` (`CPStruct`, `RealCarPart`) **[community]** and
in the decomp's `World/CarInfo.cpp`, `CarPartID.h`, `CarPartNames.cpp` **[decomp]**. Parts are grouped by
slot (`CAR_SLOT_ID` in VaultLib's Speed framework). The database probably also drives which decals and
vinyls fit each car **[unconfirmed]**.

## Record sizes **[community; counts verified]**

From Nikki `Support.MostWanted/Class/*.cs`:

| Record | Size | Notes |
|---|---|---|
| CarTypeInfo | 0xD0 | name at 0x00 (≤ 0xD chars). Count fits exactly (91) after alignment padding |
| PresetRide | 0x290 | name at 0x28 (≤ 0x1F chars) |
| SlotType | 0x8 | |
| LightMaterial | 0xB0 incl. 8-byte header | name at 0x1C |
| SunInfo | 0x110 | 5 records in GlobalB |

## Vinyls and decals

MW's vinyls are **bitmap textures** in per-car `VINYLS.BIN` / `PREVINYL.BIN` TPKs. The vector-vinyl
chunk family `0x8003CE00`–`0x0003CE13` (`VS_*` in the decomp) does **not** occur in any MW PC file;
it belongs to later games **[verified]**. Which vinyl or decal is allowed where comes from the parts
database and AttribSys `ecar` / `presetride` **[unconfirmed]**.

## Performance and tuning: AttribSys **[verified class list]**

No performance chunks exist in GlobalB: the Underground-era `PERFORMANCE_CONFIG_TABLE` / career IDs
`0x34A00`–`0x34B00` are absent. Physics and tuning live in `attributes.bin` classes
([attributes.md](attributes.md)):

| Class | Fields / collections | Role (from the names; **[unconfirmed]**) |
|---|---|---|
| `pvehicle` | 66 / 121 | per-vehicle root: links to the components below |
| `chassis`, `engine`, `transmission`, `tires`, `brakes`, `induction`, `nos` | 19/97, 7/89, 9/89, 9/95, 3/90, 7/69, 8/6 | components and upgrade levels |
| `ecar` | 49 / 100 | car-level presentation / customization data |
| `rigidbodyspecs`, `damagespecs`, `collisionreactions` | 23/22, 16/14, 4/31 | physics body, damage |
| `aivehicle`, `acceltrans`, `shiftpattern` | 9/23, 5/28, 24/25 | AI driving, acceleration curves, shifting |
| `presetride`, `junkman`, `engineaudio`, `turbosfx` | 7/25, 7/1, 40/70, 6/18 | presets, junkman tokens, engine sound mapping |

## Tools

| Tool | What | License |
|---|---|---|
| [SpeedReflect/Nikki](https://github.com/SpeedReflect/Nikki) | Read/write CarTypeInfo, DBModelPart, PresetRide, SlotType/SlotOverride, Collision (bounds), Material, SunInfo, Track, STRBlock, FNGroup, TPK for MW | C#, **MIT** |
| [SpeedReflect/Binary](https://github.com/SpeedReflect/Binary) (+ fork [nlgxzef/Binarius](https://github.com/nlgxzef/Binarius)) | GUI and end-scripts on top of Nikki | C#, **GPL-3.0** |
| [NFSTools/GlobalLib](https://github.com/NFSTools/GlobalLib) / [NFSTools/Binary](https://github.com/NFSTools/Binary) | Older versions (README: "outdated") | MIT / GPL-3.0 |
| [NFSTools/VaultLib](https://github.com/NFSTools/VaultLib) | AttribSys tuning | MIT |
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) | `World/CarInfo.cpp`, `CarLoader.cpp`, `CarPartNames.cpp`, generated AttribSys class headers | CC0-1.0 |
