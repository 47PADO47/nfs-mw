# Maps (the city of Rockport)

MW has a single open world, internally called **L2RA**. Its two main files are in `TRACKS/`:

| File | Size | Role |
|---|---|---|
| `TRACKS/L2RA.BUN` | 1.4 MB | World **metadata**, loaded once: streaming index, visibility, AI paths, collision volumes, events, world animations |
| `TRACKS/STREAML2RA.BUN` | 533 MB | World **content**: 720 sections of textures, geometry and scenery instances, streamed in and out while driving |
| `TRACKS/L2RA/MINI_MAP_*.BIN` (267 files) | small | Minimap tiles: `CompTPKBlock` chunks, each a JDLZ-compressed one-tile TPK; see [world.md](world.md#minimap) |
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

### Section families **[verified layout; purpose of V/X/Y/Z is unconfirmed]**

| Letters | Count | Total size | Has position | Content |
|---|---|---|---|---|
| A–T (no K, N) | 605 | ~375 MB | yes | City tiles: textures + geometry + scenery + collision + events |
| V | 54 | 18.9 MB | no | Geometry + textures |
| X | 22 | 19.4 MB | no | Geometry only (`X0` is 18.7 MB) |
| Y | 38 | 118.7 MB | no | Textures only |
| Z | 1 | 1.3 KB | no | Scenery only |

The spatial tiles probably stream by distance (Centre/Radius). V/X/Y/Z are probably shared sets loaded
alongside them, such as shared textures. **[unconfirmed]**

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

1. For each section, load its `GeometryPack`s into a `hash → solid` table.
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
