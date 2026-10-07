# Maps (the city of Rockport)

MW has a single open world, internally called **L2RA**. Its two main files are in `TRACKS/`:

| File | Size | Role |
|---|---|---|
| `TRACKS/L2RA.BUN` | 1.4 MB | World **metadata**, loaded once: streaming index, visibility, AI paths, collision volumes, events, world animations |
| `TRACKS/STREAML2RA.BUN` | 533 MB | World **content**: 720 sections of textures, geometry and scenery instances, streamed in and out while driving |
| `TRACKS/L2RA/MINI_MAP_*.BIN` (267 files) | small | Minimap tiles: `CompTPKBlock` chunks with JDLZ-compressed texture data |
| `TRACKS/L2RA/TrackMaps.bin` | | Texture pack (map screen) |
| `TRACKS/L2RA/TroughBoundary.bin` | | A single `0x00034190` chunk (not in the name table) **[unconfirmed]** |

Runtime code in the decomp: `src/Speed/Indep/Src/World/TrackStreamer.cpp`, `VisibleSection.cpp`,
`Scenery.cpp`. For the tag meanings, see [evidence tags](../README.md#evidence-tags).

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
| `00034108 SceneryGroup`, `00034109 SceneryBarrierGroups` | Scenery groups that are toggled together (e.g. race barriers) |
| `8003410B ModelHierarchyTree` → 133 × `0003410C ModelHierarchy` | Hierarchical (multi-part) models |
| `0003B800 UppleUWorld` | 551 KB blob; name from the decomp's chunk list, purpose unconfirmed |
| `8003B900 CollisionVolumes` → 405 × `0003B901 CollisionBody` | Collision bodies for props |
| `8003B810 EventSystem` → 73 × `0003B811 EventHandler` | Scripted world events: `OBJECT_COLLISION`, `scaffold_crash_big`, `sfx_glass_shatter`, `onesupportdown` (the names suggest pursuit breakers) |
| `00034250 GenericRegions / Weatherman` | Lighting/weather regions (`BLOOM_CP_21`, …) |
| `8003B600 ParameterMaps` | 14 `ParameterMapLayer`s (purpose unconfirmed) |
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
0003BC00 EmitterSystem               particle emitters
80036000 EventTriggerPack            trigger volumes
00034159 HeliSheetManager            helicopter (pursuit) data
00034027 WorldBounds                 bounds / smokeable spawners
0003B801 WCollisionAssets            collision assets for this tile
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
└─ 00034107 PrecullerInfos
```

**SceneryInfo** (72 B) **[community; sizes and keys verified]**:

| Offset | Field |
|---|---|
| 0x00 | char[24] Name, e.g. `XO_StreetLightD2_1b_00`, `TRN_CT_Terrain_Colonial`, `SKYDOME_XENON` |
| 0x18 | u32[4] SolidMeshKey: `bStringHash` of the solid for each LOD (0 = none) |
| 0x28 | u32[4] mesh pointers (runtime) |
| 0x38 | f32 Radius, u32 MeshChecksum, u32 HierarchyNameHash, u32 HierarchyPointer |

**SceneryInstance** (64 B) **[community; size verified]**: BBoxMin (vec3), BBoxMax (vec3), u32
ExcludeFlags, i16 PrecullerInfoIndex, i16 LightingContextNumber, Position (vec3), packed 3×3 rotation
(9 × i16), i16 SceneryInfoNumber (index into this section's `SceneryInfos`).

Checked across all 947 scenery sections:

- every `SceneryInfos` payload is a whole number of 72-byte records;
- every `SceneryInstances` payload is a whole number of 64-byte records (after 0x10 alignment);
- **40,574 of 40,679 (99.7%)** SolidMeshKeys match the hash of a solid somewhere in the stream. The rest
  presumably point at shared models in the global files.

So, to rebuild the map:

1. For each section, load its `GeometryPack`s into a `hash → solid` table.
2. For each `SceneryInstance`, take `SceneryInfos[SceneryInfoNumber]`.
3. Use `SolidMeshKey[0]` (the highest LOD) to find the model, then place it with the instance's
   rotation and position.

[NFS-ModTools](https://github.com/NFSTools/NFS-ModTools) (`Common/Scenery/MostWantedScenery.cs`)
already does this.

## Inspecting it yourself

```bash
python tools/chunkdump.py "D:/Need For Speed Most Wanted Black Edition/TRACKS/L2RA.BUN" -s -d 2
python tools/chunkdump.py "D:/Need For Speed Most Wanted Black Edition/TRACKS/STREAML2RA.BUN" --summary
python tools/chunkdump.py "D:/Need For Speed Most Wanted Black Edition/TRACKS/STREAML2RA.BUN" -d 1 -n 30
```
