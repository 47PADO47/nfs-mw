# Maps (the city of Rockport)

MW has a single open world, internally called **L2RA**. Its two main files are in `TRACKS/`:

| File | Size | Role |
|---|---|---|
| `TRACKS/L2RA.BUN` | 1.4 MB | World **metadata**, loaded once: streaming index, visibility, AI paths, collision volumes, events, world animations |
| `TRACKS/STREAML2RA.BUN` | 533 MB | World **content**: 720 sections of textures, geometry and scenery instances, streamed in and out while driving |
| `TRACKS/L2RA/MINI_MAP_*.BIN` (267 files) | small | Minimap tiles: `CompTPKBlock` chunks, each a JDLZ-compressed one-tile TPK; see [minimap.md](minimap.md) |
| `TRACKS/L2RA/TrackMaps.bin` | | Texture pack (map screen) |
| `TRACKS/L2RA/TroughBoundary.bin` | | A single `0x00034190` chunk. Not in the decomp's chunk list; the nearest IDs are `Troughs` `0x80034180`–`WallClusters` `0x00034184` **[unconfirmed]** |

Runtime code in the decomp: `src/Speed/Indep/Src/World/TrackStreamer.cpp`, `VisibleSection.cpp`,
`Scenery.cpp`. Collision, the road network, triggers, effects, sky and the minimap are covered in
[world.md](world.md). For the tag meanings, see [evidence tags](../README.md#evidence-tags).

## L2RA.BUN: top-level chunks **[verified]**

| Chunk | What it is |
|---|---|
| `00034110 TrackStreamingSections` | **The streaming index**: 720 × 92-byte records (below) |
| `00034111 TrackStreamingInfos`, `00034112 …Barriers`, `00034113 …DiscBundle` | Streaming bookkeeping (mostly empty on PC) |
| `80034150 VisibleSectionManager` | Section boundaries (2D polygons), drivable sections, loading sections |
| `00034158 VisibleSectionOverlays` | Named overlays, e.g. `FlyBy`, `E3Demo` |
| `80034147 TrackPathManager` → `0003414A TrackPathZones` | AI / navigation zones |
| `00034146 TrackPositionMarkers` | Named positions in the world |
| `0003414D TrackPathBarriers` | Road barriers |
| `00034108 SceneryOverrideInfos`, `00034109 SceneryGroups` | Scenery override records and scenery groups that are toggled together (e.g. race barriers). Names from the decomp's `SpeedChunks.hpp`; older lists called them `SceneryGroup` / `SceneryBarrierGroups` |
| `8003410B ModelHierarchyTree` → 133 × `0003410C ModelHierarchy` | Hierarchical (multi-part) models |
| `0003B800 CarpWGrid (UWorld)` | 551 KB **world map tree** (a `UGroup` tree): collision grid + **road network** for AI and traffic. Loaded by `WWorld::Loader`; see [world.md](world.md#world-map-tree-0x3b800-decomp--verified) |
| `8003B900 BoundsPack` → 405 × `0003B901 CollisionBody / Bounds` | Collision bounds for props (`SPEED_BOUNDS_PACK`) |
| `8003B810 CarpEventSequences` → 73 × `0003B811 CarpEventSequence` | Scripted world events: `OBJECT_COLLISION`, `scaffold_crash_big`, `sfx_glass_shatter`, `onesupportdown` (the names suggest pursuit breakers) |
| `00034250 WeathermanPack` | Lighting/weather regions (`BLOOM_CP_21`, …); runtime `World/WeatherMan.cpp` |
| `8003B600 ParameterMaps` | 14 `ParameterMapLayer`s: field types/offsets + 8/16-bit quad data; runtime `World/ParameterMaps.cpp`, purpose unconfirmed |
| `000370xx`, `00E34010` | World animations, e.g. the tower cranes; see [animation.md](animation.md) |

## The streaming index **[decomp + verified]**

`TrackStreamingSection`, 0x5C bytes (`TrackStreamer.hpp` in the decomp). The fields in **bold** were
checked against all 720 records in this install.

| Offset | Type | Field | Notes |
|---|---|---|---|
| 0x00 | char[8] | **SectionName** | `A41`, `X0`, `Y6`, … |
| 0x08 | i16 | **SectionNumber** | = letter index × 100 + number (A=1 … Z=26): `X0` → 2400 |
| 0x0A | i8 ×2 | WasRendered, CurrentlyVisible | runtime |
| 0x0C | i32 | Status | runtime (0 on disk) |
| 0x10 | i32 | FileType | 1 for every record |
| 0x14 | i32 | **FileOffset** | into `STREAML2RA.BUN`; always lands on a top-level chunk, 0x800-aligned |
| 0x18 | i32 | **Size** | the last record ends exactly at end of file |
| 0x1C | i32 | CompressedSize | = Size in this install |
| 0x20 | i32 | PermSize | smaller than Size in the 436 sections that start with a texture pack; meaning not proven **[unconfirmed]** |
| 0x24 | i32 | SectionPriority | 10000 + SectionNumber for spatial sections |
| 0x28 | f32 ×2 | **Centre** (x, y) | world coordinates; (0, 0) for non-spatial sections |
| 0x30 | f32 | **Radius** | |
| 0x34 | u32 | Checksum | |
| 0x38… | | timestamps, priorities, `pMemory`, `pDiscBundle`, LoadedSize | runtime, zero on disk |

### Section families **[verified]**

| Letters | Count | Total size | Has position | Content |
|---|---|---|---|---|
| A–T (no K, N) | 605 | ~375 MB | yes | City tiles: textures + geometry + scenery + collision + events |
| V | 54 | 18.9 MB | no | Geometry + textures |
| X | 22 | 19.4 MB | no | Geometry only (`X0` is 18.7 MB); the decomp calls X/U "library" sections |
| Y | 38 | 118.7 MB | no | Textures only; the decomp calls Y/W "texture" sections |
| Z | 1 | 1.3 KB | no | Scenery only: the sky domes and the airliner |

Of the shared sets, only `Z0` places scenery; V/X/Y only hold models and textures that tile scenery uses.

**The number inside a letter** says what a tile is. The split point is the `LODOffset` in
`VisibleSectionManagerInfo` (below), 40 in MW **[verified + decomp]**:

| Number | Count | What it is |
|---|---|---|
| 1–39 | 90 | Close-up detail of a **drivable section** (only where it is split off) |
| 41–79 | 435 | Section *n*'s geometry seen from further away (= *n* + 40); where *n* has no own tile, it is the whole of *n* |
| 80–89 | 34 | Non-drivable scenery (tree lines along highways, `C81`'s petrol station); `R86`–`R89` are panoramas |
| 90–99 | 46 | **Panoramas**: low-detail backdrops (`PAN_Island_*`, `PAN_Mountains_*`, `Panorama_TC_City`), radius 1–6.5 km |

Tiles are **not streamed by distance**: which ones are loaded and drawn comes from the visible-section
tables below ([../specs/visible-sections.md](../specs/visible-sections.md)). Panoramas in particular sit in
the middle of the map (`C99`'s `Panorama_TC_City` covers x −309…1783, y −1145…938) and are only drawn from
the zones that list them.

## Visible sections: zones **[decomp layout, verified on all records]**

`80034150 VisibleSectionManager` in `L2RA.BUN` divides the map into 2D **zones** (one per drivable
section) and lists, per zone, the sections to load and draw. Children:

| Chunk | Size | Content |
|---|---|---|
| `00034151 VisibleSectionManagerInfo` | 808 B | `i32 LODOffset` (40), then `DrivableSectionsInRegion`: `i32 count` (373) + `i16[400]` section numbers |
| `00034152 VisibleSectionBoundaries` | 52,876 B | 515 variable-size boundary polygons |
| `00034155 LoadingSections` | 2,964 B | 39 × 0x4C loading groups |
| `00034153 DrivableScenerySections` | 46,904 B | 435 variable-size visible lists |

Records start with an 8-byte list node (two pointers, `0x0000000B` on disk), and payloads start right
after the chunk header (no alignment padding). All fields are little-endian.

**VisibleSectionBoundary**, `0x24 + 8 × NumPoints` bytes:

| Offset | Type | Field |
|---|---|---|
| 0x08 | i16 | SectionNumber |
| 0x0A | i8 | NumPoints (≤ 16) |
| 0x0B | i8 | PanoramaBoundary (1 on 49 boundaries) |
| 0x0C | f32 ×2 | BBoxMin (x, y) |
| 0x14 | f32 ×2 | BBoxMax |
| 0x1C | f32 ×2 | Centre |
| 0x24 | f32 ×2 × NumPoints | Points, a closed polygon |

435 boundaries belong to drivable sections (numbers 1–39); they **never overlap** (checked on a 20 m grid
over the whole map: every point is in at most one, 79 % of the bounding rectangle is covered). The other
80 belong to non-drivable sections (80–99).

**DrivableScenerySection**, `0x14 + 2 × MaxVisibleSections` bytes:

| Offset | Type | Field |
|---|---|---|
| 0x08 | u32 | pBoundary (runtime, 0 on disk) |
| 0x0C | i16 | SectionNumber |
| 0x0E | i8 | MostVisibleSections |
| 0x0F | i8 | MaxVisibleSections (record capacity) |
| 0x10 | i16 | NumVisibleSections (2–68 in MW) |
| 0x12 | i16 × Max | VisibleSections, sorted; then 2 bytes of padding |

Lists name drivable (*n*), far (*n* + 40), non-drivable and panorama tiles and the V/X/Y sets around the
zone; 5,533 of the 16,265 entries name a section that is not in the stream (a drivable number whose
content lives in its *n* + 40 tile). Every drivable section has a boundary and a *n* + 40 tile. Four
panoramas are in **no** list and are never drawn: `A91`, `A94`, `C99`, `O93`.

The 66 drivable sections **not** in `DrivableSectionsInRegion` are exactly the ones whose list holds 2–20
entries, mostly just themselves and *n* + 40 (`T14`: `T14 T54 V43`): zones a car never reaches. The
decomp never reads the region list at runtime **[verified; purpose unconfirmed]**.

**LoadingSection**, 0x4C bytes: `char[15] Name` at 0x08 (`CP1`…, `CT1`…, `TC1`…), `i8 DefaultFlag` at
0x17, `i16 NumDrivableSections` at 0x18, `i16[16] DrivableSections` at 0x1A, `i16 NumExtraSections` at
0x3A, `i16[8] ExtraSections` at 0x3C. They group neighbouring zones that load as one, plus extra tiles
(often panoramas: `CP14` adds `O90`, `D56`, `V56`).

**VisibleSectionOverlays** (`00034158`, top level): `char[40] Name` at 0x08, `i32 NumEntries` at 0x30,
then 6-byte entries `{i8 AddRemove, i8 pad, i16 DrivableSection, i16 Section}` at 0x34. MW has `FlyBy`
(500 entries) and `E3Demo` (empty); they edit visible lists while active.

## A streamed section's contents **[verified]**

A typical city tile is a run of top-level chunks:

```
B3300000 TexturePack                 one or more; the section's textures
80134000 GeometryPack  × 6..9        the section's models (see models.md)
80034100 ScenerySection × 2          placed instances of those models
0003BC00 EmitterLibrary              particle emitters
80036000 EmTriggerPack               trigger volumes
00034159 HeliSheet                   helicopter (pursuit) navigation data
00034027 SmokeableSpawners           breakable / smokeable prop spawners
0003B801 CarpWCollisionPack          static collision for this tile (see world.md)
```

Totals for the whole stream: 259,934 chunks; 20,377 solids; 477 texture packs (244 MB of pixel data);
232 MB of vertex buffers.

## Scenery: placing models in the world

```
80034100 ScenerySection
├─ 00034101 ScenerySectionHeader     contains the SectionNumber
├─ 00034102 SceneryInfos             72-byte records: which model
├─ 00034103 SceneryInstances         64-byte records: where it goes
├─ 00034105 SceneryTreeNodes         spatial tree for culling
├─ 00034106 SceneryOverrideHooks
└─ 00034107 SceneryPrecullerInfos
```

**SceneryInfo** (72 B) **[community; sizes and keys verified]**:

| Offset | Field |
|---|---|
| 0x00 | char[24] Name, e.g. `XO_StreetLightD2_1b_00`, `TRN_CT_Terrain_Colonial`, `SKYDOME_XENON` |
| 0x18 | u32[4] SolidMeshKey: `bStringHash` of the solid for each LOD (0 = none) |
| 0x28 | u32[4] mesh pointers (runtime) |
| 0x38 | f32 Radius, u32 MeshChecksum, u32 HierarchyNameHash, u32 HierarchyPointer |

**SceneryInstance** (64 B) **[community; layout verified]**:

| Offset | Field |
|---|---|
| 0x00 | BBoxMin (vec3), **world space** |
| 0x0C | BBoxMax (vec3) |
| 0x18 | u32 ExcludeFlags: which views draw it ([../specs/scenery-visibility.md](../specs/scenery-visibility.md)) |
| 0x1C | i16 PrecullerInfoIndex, i16 LightingContextNumber |
| 0x20 | Position (vec3) |
| 0x2C | 3×3 rotation, 9 × i16, **÷ 8192**, row-major |
| 0x3E | i16 SceneryInfoNumber (index into this section's `SceneryInfos`) |

**The rotation** **[verified]**: rows are the object's x, y and z axes in world space, so
`world = x·row0 + y·row1 + z·row2 + Position` (the D3D row-vector convention). Rows can include scale
(row lengths 0.09–3.0) and mirroring (determinant −1). Evidence: transforming each solid's local bounds
this way reproduces the instance's stored world box. On the 59,529 instances with a non-identity rotation,
this convention fits better than the transposed one 45,838 times, the reverse 4,124 times, and ties the
rest. The remaining error is about 0.14 m of box padding.

**World-space instances** **[verified]**: 10,911 instances have exclude flag `0x200`. All of them sit at
the origin with an identity rotation: their geometry is already in world coordinates (terrain, roads,
`*_DEINST` props, the sky domes). For them, the info's `Radius` is the distance from the origin to the far
side of the geometry (ratio 1.00 at the median), so the LOD rule's sphere around the origin always contains
them ([../specs/scenery-lod.md](../specs/scenery-lod.md)).

Payload alignment: `SceneryInfos` is not aligned; `SceneryInstances` is aligned to 0x10. **In the stream, the
instances chunk comes before the infos chunk** inside each `ScenerySection`. A one-pass reader must not
resolve instances as it meets them. **[verified]**

Checked across all 947 scenery sections:

- every `SceneryInfos` payload is a whole number of 72-byte records;
- every `SceneryInstances` payload is a whole number of 64-byte records (after 0x10 alignment);
- **40,574 of 40,679 (99.7%)** SolidMeshKeys match the hash of a solid somewhere in the stream. The rest
  presumably point at shared models in the global files.

**Where an instance's model lives** **[verified, whole stream]**: of the 77,776 instances in map tiles,
17,759 use a solid from their own tile, 59,963 use one from the shared V/X/Y/Z sections, **none** uses
another tile, and 54 resolve nowhere. Textures behave the same way: tile solids find 1,461 texture
references in their own tile and 21,844 in the shared sets, and 426 are missing. So the shared sets can
stay loaded while tiles stream independently.

So, to rebuild the map:

1. Pick the sections from the camera's zone ([../specs/visible-sections.md](../specs/visible-sections.md)), plus
   the shared sets. For each, load its `GeometryPack`s into a `hash → solid` table.
2. For each `SceneryInstance`, take `SceneryInfos[SceneryInfoNumber]`.
3. Skip instances the player view excludes ([../specs/scenery-visibility.md](../specs/scenery-visibility.md)).
4. Use the first non-zero `SolidMeshKey` (the highest LOD) to find the model, looking in the tile first and
   then in the shared sets. Place it with the instance's rotation and position.

This is implemented in [`blackbox-streaming`](../../libs/blackbox-streaming),
[`blackbox-scenery`](../../libs/blackbox-scenery) and `nfsmw view-world`
([../architecture.md](../architecture.md#the-streamed-city-view-world)).
[NFS-ModTools](https://github.com/NFSTools/NFS-ModTools) (`Common/Scenery/MostWantedScenery.cs`, no
license) has a C# reader.

## Inspecting it yourself

```bash
python tools/chunkdump.py "D:/Need For Speed Most Wanted Black Edition/TRACKS/L2RA.BUN" -s -d 2
python tools/chunkdump.py "D:/Need For Speed Most Wanted Black Edition/TRACKS/STREAML2RA.BUN" --summary
python tools/chunkdump.py "D:/Need For Speed Most Wanted Black Edition/TRACKS/STREAML2RA.BUN" -d 1 -n 30
```
