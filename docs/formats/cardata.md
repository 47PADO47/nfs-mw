# Car data: type info, parts database, presets, paint, vinyls, tuning

Car data is split between bChunk tables in `GLOBAL/GlobalB.lzc` (identity, parts, presets, materials,
bounds), per-car folders (models and textures) and AttribSys (physics, tuning and wheel placement). How
the pieces combine into a drawn car is specified in [specs/car-assembly.md](../specs/car-assembly.md).
For the tag meanings, see [evidence tags](../README.md#evidence-tags).

## Where it lives **[verified]**

| Data | File / chunk | Size / count |
|---|---|---|
| Car type table | `GlobalB.lzc` → `00034600 CarTypeInfos` | 18,936 B = 8 B of `0x11` padding (payload aligned to 0x10) + **91 × 0xD0** records |
| Parts database | `GlobalB.lzc` → `80034602 CarPartPack (DBCarParts)` | 253,628 B (children below) |
| Slot types | `GlobalB.lzc` → `00034607 CarPartTypeNameTable (SlotTypes)` | 1,272 B = **139 × 8** default pairs + **10 × 16** overrides |
| Animation hookup / hide | `00034608`, `00034609` | 140 B, 256 B |
| Preset cars | `GlobalB.lzc` → `00030220 PresetRides` | 53,792 B = **82 × 0x290** |
| Car collision bounds | `GlobalB.lzc` → `8003B900 BoundsPack` | 86 bounds (see [world.md](world.md)) |
| Light materials (car shading) | `GlobalB.lzc` → 156 × `00135200 LightMaterials` | 168 B each |
| Models | `CARS/<CAR>/GEOMETRY.BIN` (100) + shared `CARS/WHEELS`, `BRAKES`, `PLATES`, `ROOF`, `SPOILER*` | `GeometryPack`, see [models.md](models.md) |
| Textures | `CARS/<CAR>/TEXTURES.BIN` (93 + 7 empty), `CARS/TEXTURES.BIN` (930 shared, incl. `DUMMY_SKIN1…8`) | TPK, see [textures.md](textures.md) |
| Vinyls | `CARS/<CAR>/VINYLS.BIN` (47 + 50 empty), `PREVINYL.BIN` (39 + 27 empty) | **TPK** (bitmap vinyls) |
| Physics, tuning, wheel placement | `GLOBAL/attributes.bin` | AttribSys, see below |

The live `GlobalB.lzc` is stored uncompressed by mod tools; `GLOBALB.LZC.bacc` is the JDLZ original
([install-layout.md](../install-layout.md)). `GLOBALB.BUN` (2.8 MB) has the same chunk types with
different content: 84 car types and a 233,004-byte parts pack. Whether the PC game loads it at all is
**[unconfirmed]**. The 7 extra types in `GlobalB.lzc` are add-on cars: `FXXEVO`, `LEVIN`, `SF90`, `SKYLINEZT`,
`TRUENO`, `TRUENOCP`, `TRUENOID` **[verified]**. The eighth add-on folder, `COROLLAE88`, has no
CarTypeInfo entry (it does have an `ecar` collection). The parts pack's string table starts with
`Binary by MaxHwoy | Automated`: this install's database was rewritten by the Binary mod tool
**[verified]**.

## CarTypeInfo (0xD0) **[decomp + community; values verified]**

Field names from the decomp's `CarTypeInfo` (`World/CarInfo.hpp`), order cross-checked with Nikki.

| Offset | Type | Field | M3 GTR |
|---|---|---|---|
| 0x00 | char[16] | CarTypeName | `BMWM3GTR` |
| 0x10 | char[16] | BaseModelName (solid prefix; lower-cased it keys the `ecar` collection) | `BMWM3GTR` |
| 0x20 | char[32] | GeometryFilename | `CARS\BMWM3GTR\GEOMETRY.BIN` |
| 0x40 | char[16] | ManufacturerName | `BMW` |
| 0x50 | u32 | CarTypeNameHash = `bStringHash(CarTypeName)` (all 91 ✔) | `0x6C401512` |
| 0x54 | f32 | HeadlightFOV | 1.92 |
| 0x58 | u8 ×4 | padHighPerformance, NumAvailableSkinNumbers, WhatGame (1 = MW), ConvertableFlag | 0, 0, 1, 0 |
| 0x5C | u8 ×3 | WheelOuterRadius, WheelInnerRadiusMin, WheelInnerRadiusMax (inches; rim-size range offered in the shop) | 26, 17, 20 (all cars) |
| 0x60 | vec4 ×3 | HeadlightPosition, DriverRenderingOffset, InCarSteeringWheelRenderingOffset | (1.28, 0, 0.32), … |
| 0x90 | i32 | Type (index) | 22 |
| 0x94 | i32 | UsageType: 0 racing, 1 cop, 2 traffic, 3 wheels, 4 universal | 0 |
| 0x98 | u32 | CarMemTypeHash | |
| 0x9C | u8[5] ×2 | MaxInstances, WantToKeepLoaded | |
| 0xA8 | f32[5] | MinTimeBetweenUses | |
| 0xBC | u8[10] | AvailableSkinNumbers | |
| 0xC6 | u8 | DefaultSkinNumber | 1 |
| 0xC7 | u8 | **Skinnable** (body painted through a composite skin) | 1 |
| 0xC8 | i32 | padding | 0 |
| 0xCC | u32 | **DefaultBasePaint**: part name hash of the stock paint | `0xC7F2884E` (`METAL_L1_COLOR02`) |

Skinnable: 47 types; none of them has a `<CAR>_SKIN1` texture. Of the 44 others, 38 ship one
**[verified]**. Stock paints by family: 36 `GLOSS_*`, 11 `METAL_*`, 33 `TRAFFIC_*`, 11 `COP_*`.
No CarTypeInfo field places wheels **[verified]**; see `ecar` below.

## Parts database (`0x80034602`) **[verified]**

Children in file order (the decomp's loader walks them in exactly this order):

| Chunk | Decomp name | Community name | Size | Content |
|---|---|---|---|---|
| `00034603` | `SPEED_CARPART_PACK` | DBCarParts_Header | 60 | header |
| `00034606` | `…_STRING_TABLE` | DBCarParts_Strings | 26,480 | NUL-terminated strings, addressed in 4-byte units |
| `0003460C` | `…_ATTRIBUTETABLE_TABLE` | DBCarParts_Offsets | 15,460 | i16 stream of per-part attribute lists |
| `00034605` | `…_ATTRIBUTES_TABLE` | DBCarParts_Attribs | 14,968 | 1,870 × 8 B + 8 B padding |
| `0003460A` | `…_MODELNAMEHASH_TABLE` | DBCarParts_Structs | 13,032 | 543 × 24 B model tables |
| `0003460B` | `…_TYPENAMEHASH_TABLE` | DBCarParts_Models | 428 | 106 × u32 + 4 B padding |
| `00034604` | `…_PARTS_TABLE` | DBCarParts_Array | 183,128 | 13,080 × 14 B + 8 B padding |
| `0003460D` | — | DBCarParts_Custom | 8 | zero |

Field layouts: **[decomp]** (`World/CarInfo.hpp`), all counts and the rules below **[verified]** on the
install with a scratch reader.

**Header (60 B).** 0x00 8 B list links (0); 0x08 u32 Version = **6**; then runtime pointer / count pairs,
pointers 0 on disk: 0x18 NumAttributeTables (0 on disk), 0x20 NumAttributes **1,870**, 0x28
NumTypeNames **106**, 0x30 NumModelTables **543**, 0x38 NumParts **13,080**.

**Part (14 B).**

| Offset | Type | Field |
|---|---|---|
| 0x0 | u16, u16 | PartNameHash low, high (hash = high << 16 \| low) |
| 0x4 | i8 | PartID (`CAR_PART_ID`, see [Slots](#slots)) |
| 0x5 | u8 | bits 0–4 GroupNumber, bits 5–7 **UpgradeLevel** (0 = stock) |
| 0x6 | i8 | BaseModelNameHashSelector: 0 none, 1 car type name, 2 `BRAND_NAME` attribute |
| 0x7 | u8 | CarTypeNameHashIndex (into the type-name table) |
| 0x8 | u16 | NameOffset (× 4 into the string table): display / authoring name, e.g. `BODY_00`, `WHEEL`, `METAL_L1_COLOR02` |
| 0xA | u16 | AttributeTableOffset (index in i16 units into `0x3460C`; `0xFFFF` = none) |
| 0xC | u16 | ModelNameHashTableOffset (index into the model tables; `0xFFFF` = no model) |

The part name hash is `bStringHash` of an authoring name, e.g. `BMWM3GTR_BASE`, `BMWM3GTR_WHEEL`,
`BMWM3GTR_FRONT_BRAKE`, `METAL_L1_COLOR02`; it equals `bStringHash(display name)` for only 526 parts,
so treat it as an opaque id. Selectors: 11,950 × 1, 966 × 0, 164 × 2 (aftermarket rims). Upgrade levels:
8,330 at 0, the rest 1–7.

**Attribute (8 B).** u32 name hash (`bStringHash` of the attribute name), u32 value read as u32, i32,
f32 or string offset (× 4) depending on the attribute. Attributes are shared between parts (only 1,870
distinct). Names seen: `LANGUAGEHASH` 377, `CV` 368, `TEXTURE_NAME` 249, `NAME` 224, `TEXTURE` 155,
`BRAND_NAME` 113, `RED`/`GREEN`/`BLUE` ~80 each, `LIGHT_MATERIAL_NAME` 74, `SPEECHCOLOUR` 16,
`KITNUMBER` 7, `INNER_RADIUS`, `OUTER_RADIUS`, `SPOKE_COUNT`, `GLOSS`, `REMAP`, `NUMCOLOURS`,
`NUMREMAPCOLOURS`, `SHAPE`, `SIZE`, `CARBONFIBRE`, `USEMARKER2`, `EXCLUDEDECAL`.

**Attribute lists (`0x3460C`).** At `AttributeTableOffset × 2`: i16 count, then count i16 indices into
the attribute array. The first match by name wins.

**Model table (24 B).** i8 Templated, u8 pad, u16 MiddleStringOffset (× 4 into the strings; `0xFFFF` =
none), then 5 u32 entries for LOD A–E (`0xFFFFFFFF` = no model). Plain tables (222 parts) hold solid
name hashes directly; templated ones (12,074) hold string offsets (× 4), and the solid hash is built as
`bStringHash` chained over: selector base hash → middle string → entry string → `_A`…`_E`. Examples:
`BMWM3GTR` + `_KIT00` + `_BODY` + `_A`; `BMWM3GTR` + `_BASE` + `` + `_A`; brand `BBS` +
`_STYLE01_18_25` + `_A`.

**Type names.** 106 `bStringHash`es: the 91 car type names plus shared groups `ROOF`, `PAINT`, `VINYL`,
`WINDOW_TINT`, `WHEELS`, `BRAKES`, `PLATES`, `SPOILER`, `SPOILER_PORSCHES`, `SPOILER_CARRERA`,
`SPOILER_HATCH`, `CUSTOM_HUD`, `WHEEL_MANUFACTURERS`, and two unnamed ones (`0x02A05578` = decal textures,
`0x596AB315` = HUD colours).

## Slots

A car holds one part per slot (`CAR_SLOT_ID`, 139 values) **[decomp]**. Slots 0–75 name models.

| Slots | Names | Part id |
|---|---|---|
| 0 | `BASE` | same as slot |
| 1–22 | `DAMAGE_FRONT_WINDOW`, `DAMAGE_BODY`, `DAMAGE_COP_LIGHTS`, `DAMAGE_COP_SPOILER`, `DAMAGE_FRONT_WHEEL`, `DAMAGE_{LEFT,RIGHT}_{BRAKELIGHT,HEADLIGHT}`, `DAMAGE_HOOD`, `DAMAGE_BUSHGUARD`, `DAMAGE_FRONT_BUMPER`, `DAMAGE_RIGHT_DOOR`, `DAMAGE_RIGHT_REAR_DOOR`, `DAMAGE_TRUNK`, `DAMAGE_REAR_BUMPER`, `DAMAGE_{REAR_LEFT,FRONT_LEFT,FRONT_RIGHT,REAR_RIGHT}_WINDOW`, `DAMAGE_LEFT_DOOR`, `DAMAGE_LEFT_REAR_DOOR` | same |
| 23–42 | `BODY`, `FRONT_BRAKE`, `FRONT_LEFT_WINDOW`, `FRONT_RIGHT_WINDOW`, `FRONT_WINDOW`, `INTERIOR`, `LEFT_BRAKELIGHT`, `LEFT_BRAKELIGHT_GLASS`, `LEFT_HEADLIGHT`, `LEFT_HEADLIGHT_GLASS`, `LEFT_SIDE_MIRROR`, `REAR_BRAKE`, `REAR_LEFT_WINDOW`, `REAR_RIGHT_WINDOW`, `REAR_WINDOW`, `RIGHT_BRAKELIGHT`, `RIGHT_BRAKELIGHT_GLASS`, `RIGHT_HEADLIGHT`, `RIGHT_HEADLIGHT_GLASS`, `RIGHT_SIDE_MIRROR` | same |
| 43–51 | `DRIVER`, `SPOILER`, `UNIVERSAL_SPOILER_BASE`, `DAMAGE0_FRONT`, `DAMAGE0_FRONTLEFT`, `DAMAGE0_FRONTRIGHT`, `DAMAGE0_REAR`, `DAMAGE0_REARLEFT`, `DAMAGE0_REARRIGHT` | same |
| 52–65 | `ATTACHMENT0`–`9`, `ROOF`, `HOOD`, `HEADLIGHT`, `BRAKELIGHT` | same |
| 66, 67 | `FRONT_WHEEL`, `REAR_WHEEL` | 67 `WHEEL` |
| 68–75 | `SPINNER`, `LICENSE_PLATE`, `DECAL_{FRONT_WINDOW,REAR_WINDOW,LEFT_DOOR,RIGHT_DOOR,LEFT_QUARTER,RIGHT_QUARTER}` | same |
| 76, 77, 78 | `BASE_PAINT`, `VINYL_LAYER0`, `PAINT_RIM` | 76 `PAINT`, 79 `VINYL`, 78 `RIM_PAINT` |
| 79–82 | `VINYL_COLOUR0_0`…`3` | 77 `VINYL_PAINT` |
| 83–130 | `DECAL_<zone>_TEX0`…`7` (6 zones × 8) | 80 `DECAL_TEXTURE` |
| 131–138 | `WINDOW_TINT`, `CUSTOM_HUD`, `HUD_BACKING_COLOUR`, `HUD_NEEDLE_COLOUR`, `HUD_CHARACTER_COLOUR`, `CV`, `WHEEL_MANUFACTURER`, `MISC` | 81, 82, 83, 83, 83, 84, 85, 86 |

## SlotTypes (`0x00034607`) **[decomp layout; verified]**

- **Defaults:** 139 pairs of u32 (one per slot) at payload offset 0: the type-name hashes searched for
  that slot, in order. `0xFFFFFFFF` = the car's own type name, 0 = none. Non-trivial ones: 44 →
  (car, `SPOILER`), 62 → `ROOF`, 66/67 → (car, `WHEELS`), 68 → `SPINNER`, 69 → `PLATES`, 76 and 78–82 →
  `PAINT`, 77 → `VINYL`, 83–130 → `0x02A05578`, 131 → `WINDOW_TINT`, 132 → `CUSTOM_HUD`, 133–135 →
  `0x596AB315`; every other slot is (car, none).
- **Overrides:** from payload offset 1,112 (0x458) to the end, 16 B each: u32 car type name hash, u32
  slot, u32[2] types. The 10 in this install all retarget slot 44: `911TURBO`, `TT`, `997S`, `CAYMANS` →
  `SPOILER_PORSCHES`; `CARRERAGT`, `FORDGT` → `SPOILER_CARRERA`; `A3`, `GTI`, `CLIO`, `PUNTO` →
  `SPOILER_HATCH`.

## PresetRides (`0x00030220`, 0x290 each) **[decomp layout; verified]**

| Offset | Type | Field |
|---|---|---|
| 0x00 | 8 B | list links (0) |
| 0x08 | char[32] | CarTypeName |
| 0x28 | char[32] | PresetName (`BL10`, `CE_GTRSTREET`, `RAZORMUSTANG`, `M3GTRCAREERSTART`, `CS_CAR_01`…) |
| 0x48 | u64 | FEKey |
| 0x50 | u64 | VehicleKey (low 32 bits = an AttribSys collection hash, e.g. `0xCD88083F` = `bmwm3gtr`) |
| 0x58 | u32, i32 | FilterBits, PhysicsLevel (0 in all 82) |
| 0x60 | u32[139] | part name hash per slot: 0 = empty, 1 = keep stock, else the part |
| 0x28C | 4 B | padding |

Records start at payload offset 0. `CE_GTRSTREET` (BMWM3GTR) lists exactly the 81 parts the stock rule
of [car-assembly.md §1](../specs/car-assembly.md#1-stock-part-selection) produces; `CS_CAR_15`
(911 Turbo) differs from stock only in paint **[verified]**.

## Paint parts and LightMaterials **[verified]**

Paint parts (part id 76, type `PAINT`, 202 of them) carry `RED`, `GREEN`, `BLUE`, `GLOSS` (0–255),
`LIGHT_MATERIAL_NAME`, `BRAND_NAME` (family) and `SPEECHCOLOUR`. Examples:

| Part | Level | RGB | Gloss | Light material |
|---|---|---|---|---|
| `METAL_L1_COLOR02` (M3 GTR stock) | 2 | 79, 79, 79 | 128 | `METPAINTSILVER` |
| `GLOSS_L1_COLOR75` (911 Turbo stock) | 1 | 0, 16, 59 | 128 | `REGPAINTBLUE` |
| `GLOSS_L1_COLOR01` | 1 | 255, 255, 255 | 128 | `REGPAINTWHITE` |
| `COP_L1_COLOR01` | 1 | 130, 130, 130 | 128 | `DRIVERHEAD` |

`LightMaterials` (`0x00135200`, 168 B payload = the decomp's `eLightMaterial`):

| Offset | Type | Field |
|---|---|---|
| 0x00 | 12 B | runtime (platform info, list links) |
| 0x0C | u32 | NameHash = `bStringHash(Name)` (all 156 ✔) |
| 0x10 | u32 | Version (3) |
| 0x14 | char[28] | Name (`CARBONFIBER`, `CHROME`, `MAGSILVER`, `METPAINTSILVER`, `REGPAINT*`, `WINDSHIELD`, `WINDSHIELD_TINT_L*_*`, `CUSTOMPAINT_1…20`, …) |
| 0x30 | f32 ×30 | DiffuseMinScale, DiffuseMinR/G/B, DiffuseMaxScale, DiffuseMaxR/G/B, DiffuseMinA, DiffuseMaxA, SpecularPower, SpecularMinScale, SpecularMinR/G/B, SpecularMaxScale, SpecularMaxR/G/B, EnvmapPower, EnvmapMinScale, EnvmapMinR/G/B, EnvmapMaxScale, EnvmapMaxR/G/B, MetallicScale, SpecularHotSpot |

`METPAINTSILVER`: diffuse scale 0.75 / 1.0, specular power 3, scale 0.7 / 0.3, env power 0.15, scale
3.5 / 0.2, metallic 1, all colours 1. `WINDSHIELD`: diffuse alpha 0.4 / 0.4, env power 1, scale 3 / 0.5.
There is no `CARSKIN` material: it is a placeholder swapped for the paint's material at draw time.

## SolidMarkers (`0x0013401A`) **[decomp layout; verified]**

Position markers attached to a solid. The payload is aligned to 0x10 (absolute file offset); then
0x50-byte records (`ePositionMarker`) fill the rest. `SolidInfo.NumPositionMarkers` (0x1B) is **0 on
disk**: derive the count from the chunk size.

| Offset | Type | Field |
|---|---|---|
| 0x00 | u32 | NameHash (`bStringHash`, e.g. `FRONT_BRAKE`) |
| 0x04 | i32 | iParam0 (0 on cars) |
| 0x08 | f32 ×2 | fParam0, fParam1 (0 on cars) |
| 0x10 | f32[16] | Matrix, row-major, rows = x/y/z axes then translation |

Census over every car `GEOMETRY.BIN` (incl. shared folders): `LEFT/RIGHT_EXHAUST` 1,296/1,153,
`FRONT_BRAKE` 871, `LEFT/RIGHT_REVERSE`, `LEFT/RIGHT_BRAKELIGHT`, `LEFT/RIGHT_HEADLIGHT`,
`COPLIGHT{WHITE,RED,BLUE,BRIGHTRED,BRIGHTBLUE,ORANGE}`, `SPOILER` 176, `REAR_BRAKE` 166, `ROOF_SCOOP` 150,
`SPOILER2` 112, `CENTRE_BRAKELIGHT`, `HOOD` and `TRUNK` (cop damage bodies), `EXHAUST`, window and
side-mirror markers (`GTO`, `COPGTO*`, `LANCEREVO8`), door open/closed markers (`COPMIDSIZEINT` only). Where they sit: light, exhaust,
spoiler and roof-scoop markers on `<CAR>_BASE` / `KIT00_BODY`; `FRONT_BRAKE` / `REAR_BRAKE` on wheel
solids (stock and all 164 aftermarket rims). **No wheel-position and no licence-plate markers exist.**

M3 GTR: `BMWM3GTR_KIT00_FRONT_TIRE_A`–`D` carry one `FRONT_BRAKE` marker at (0, 0.046, 0);
`KIT00_BODY_*` carry `LEFT/RIGHT_REVERSE` (−1.967, ±0.440, 0.659) and `LEFT/RIGHT_EXHAUST` (−2.100,
±0.286, 0.157); `BASE_*` carry headlights (2.162, ±0.581, 0.473), brakelights (−1.930, ±0.652, 0.645)
and the reverse lights.

## AttribSys `ecar` (presentation) **[verified values; field meaning decomp]**

Collection key = lookup2 hash of the lower-cased `BaseModelName` (`bmwm3gtr` = `0xCD88083F`); parents
supply missing fields (`bmwm3gtr` → `0xA6ABC921` → `default`). 100 collections, incl. `cops`,
`default` and `corollae88`. Fields used for car assembly (class layout in the decomp's
`Generated/AttribSys/Classes/ecar.h`):

| Field | Type | M3 GTR |
|---|---|---|
| `TireOffsets[4]` | vec4: x, y, z, tyre radius (FL, FR, RR, RL) | (1.615, ±0.88, 0, 0.33), (−1.12, ±0.89, 0, 0.33) |
| `FECompressions[2]` | front-end compression, front / rear | 0.21, 0.21 |
| `TireSkidWidth[4]` | tyre width per wheel | 0.235, 0.235, 0.255, 0.255 |
| `TireSkidWidthKitScale[7]` | vec2 per kit | all (1, 1) |
| `KitWheelOffsetFront[6]` / `Rear[6]` | u8, mm per kit | all 0 (911 Turbo: 0, 0, 75, 55, 65, 0) |
| `CamberFront`, `CamberRear` | f32 | 0.28, 0.20 |
| `WheelSpokeCount` | i8; negative = mirror left wheels | 8 (negative on `911turbo`, `corvette`, `monaro`, `rx8`, `slr`) |
| `RideHeight` (in), `WheelWell` (in), `ExtraRearTireOffset` | in-game only; the last only on 7 trucks | −2.0, 22.5 (inherited), — |

## Vinyls and decals

MW's vinyls are **bitmap textures** in per-car `VINYLS.BIN` / `PREVINYL.BIN` TPKs. The vector-vinyl
chunk family `0x8003CE00`–`0x0003CE13` (`VS_*` in the decomp) does **not** occur in any MW PC file;
it belongs to later games **[verified]**. Vinyl and decal choices are parts (slots 77 and 83–130); with
none chosen, decal models show the fully transparent `DEFAULTALPHA` texture **[verified texture;
replacement rule decomp]**.

## Performance and tuning: AttribSys **[verified class list]**

No performance chunks exist in GlobalB: the Underground-era `PERFORMANCE_CONFIG_TABLE` / career IDs
`0x34A00`–`0x34B00` are absent. Physics and tuning live in `attributes.bin` classes
([attributes.md](attributes.md)):

| Class | Fields / collections | Role (from the names; **[unconfirmed]** except `ecar`) |
|---|---|---|
| `pvehicle` | 66 / 121 | per-vehicle root: links to the components below |
| `chassis`, `engine`, `transmission`, `tires`, `brakes`, `induction`, `nos` | 19/97, 7/89, 9/89, 9/95, 3/90, 7/69, 8/6 | components and upgrade levels |
| `ecar` | 49 / 100 | car-level presentation: wheel placement, camber, body motion, cameras, effects (above) |
| `rigidbodyspecs`, `damagespecs`, `collisionreactions` | 23/22, 16/14, 4/31 | physics body, damage |
| `aivehicle`, `acceltrans`, `shiftpattern` | 9/23, 5/28, 24/25 | AI driving, acceleration curves, shifting |
| `presetride`, `junkman`, `engineaudio`, `turbosfx` | 7/25, 7/1, 40/70, 6/18 | presets, junkman tokens, engine sound mapping |

## Tools

| Tool | What | License |
|---|---|---|
| [SpeedReflect/Nikki](https://github.com/SpeedReflect/Nikki) | Read/write CarTypeInfo, DBModelPart, PresetRide, SlotType/SlotOverride, Collision (bounds), Material, SunInfo, Track, STRBlock, FNGroup, TPK for MW | C#, **MIT** |
| [SpeedReflect/Binary](https://github.com/SpeedReflect/Binary) (+ fork [nlgxzef/Binarius](https://github.com/nlgxzef/Binarius)) | GUI and end-scripts on top of Nikki; rewrote this install's parts pack | C#, **GPL-3.0** |
| [NFSTools/GlobalLib](https://github.com/NFSTools/GlobalLib) / [NFSTools/Binary](https://github.com/NFSTools/Binary) | Older versions (README: "outdated") | MIT / GPL-3.0 |
| [NFSTools/VaultLib](https://github.com/NFSTools/VaultLib) | AttribSys tuning (legacy VPAK pointer fix-ups, layouts) | MIT |
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) | `World/CarInfo.cpp`, `CarRender.cpp`, `CarSkin.cpp`, `CarPartID.h`, generated AttribSys class headers | CC0-1.0 |
