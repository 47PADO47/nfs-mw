# World systems: collision, road network, triggers, effects, sky, minimap

This page covers the non-visual world data. Streaming, scenery placement and the `L2RA` chunk overview
are in [maps.md](maps.md). Chunk names follow [`tools/bchunk_names.py`](../../tools/bchunk_names.py),
which now takes them from the decomp's `SpeedChunks.hpp`. For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

## Overview

| Chunk | Where (count) **[verified]** | What | Best reference |
|---|---|---|---|
| `0003B800 CarpWGrid (UWorld)` | `L2RA.BUN` (1 × 551,432 B) | World "map" tree: collision grid + **road network** | decomp `World/Common/WWorld.cpp`, `WGrid.cpp`, `WRoadNetwork.cpp` (144 KB), `WRoadElem.h`; noclip reads path vertices |
| `0003B801 CarpWCollisionPack` | `STREAML2RA.BUN` (390, 5.8 MB) | Static world collision per section | decomp `WCollisionPack.cpp`, `WCollisionAssets.cpp`, `WCollisionMgr.cpp`, `WCollision.h` |
| `8003B900 BoundsPack` → `0003B901 CollisionBody / Bounds` | `L2RA.BUN` (405), `GlobalB.lzc` (86) | Prop bounds (L2RA) / car bounds (GlobalB) | Nikki `Collision` (car bounds, MIT) |
| `8003B810 CarpEventSequences` → `0003B811` | `L2RA.BUN` (73) | Scripted world events (pursuit breakers, …) | decomp `Libs/Support/Miscellaneous/CARP.h` |
| `80036000 EmTriggerPack` (`36001` header, `36002` tree, `36003` triggers) | stream (415) | Trigger volumes per section | decomp `World/EventManager.cpp` |
| `80034147 TrackPathManager` → `0003414A TrackPathZones`; `0003414D TrackPathBarriers` | `L2RA.BUN` | AI / navigation zones and barriers | decomp `World/TrackPath.cpp` |
| `00034146 TrackPositionMarkers` | `L2RA.BUN` (15,748 B) | Named positions | decomp `World/TrackPositionMarker.cpp` |
| `00034159 HeliSheet` | stream (435) | Helicopter navigation sheets | decomp `World/HeliSheet.cpp` (registers a `bChunkLoader`) |
| `00034027 SmokeableSpawners` | stream (366) | Breakable / smokeable prop spawners | decomp `Misc/ResourceLoader.cpp`; noclip calls it "DestructiblesList" |
| `0003BC00 EmitterLibrary` | stream (290) | Particle emitters placed per section | decomp `Ecstasy/EmitterSystem.cpp` |
| `00034250 WeathermanPack` | `L2RA.BUN` (10,028 B) | Weather/lighting regions (`BLOOM_CP_21`, …) | decomp `World/WeatherMan.cpp` |
| `00034202 SunInfos`, `00034201 TrackInfos` | `GlobalB.lzc` (1,360 B / 5,184 B) | Sun parameters, track table | decomp `World/Sun.cpp`, `TrackInfo.cpp`; Nikki `SunInfo`, `Track` |
| `8003B600 ParameterMaps` | `L2RA.BUN` (14 layers) | Gridded parameter layers (field types/offsets, 8/16-bit quad data) | decomp `World/ParameterMaps.cpp` |
| `8003B000 SplinePack (QuickSplines)` | `L2RA.BUN` (empty) | — | — |

Other world chunks named in the decomp do **not** occur in MW PC data **[verified]**: `TrackRoutes`,
`TrafficIntersections`, `TrackCops`, the track camera chunks `0x80034405`–`0x00034492` (MW uses the
ICE cameras in `InGameB.bun` instead, see [animation.md](animation.md)), and all light / flare packs
(see [Light flares and light packs](#light-flares-and-light-packs)).

## World map tree (`0x3B800`) **[decomp + verified]**

`WWorld::Loader` takes the 16-byte-aligned payload as one blob ("CARP data"). The blob is a **`UGroup`
tree** (`Libs/Support/Utility/UGroup.hpp`): tagged groups and data records whose u32 tags combine a
two-character type and a two-character index (`MAKE_UDATA_TYPE('CD') | 'at'`). The payload begins with
the tags `CDat`, `Map `, `RNgp`, `CGcn` **[verified]**. Per the decomp:

- `CDat` / `CGrd`: the collision grid (`fMin`, `fNumRows`, `fNumCols`, `fEdgeSize`), with `CGcn` grid
  nodes (`WGrid::Init`).
- `RNgp`: the **road network**, i.e. roads, segments, nodes, intersections, lanes and profiles. Record
  sizes come from `WRoadElem.h`: `WRoad` 0x8, `WRoadNetworkInfo` 0xE, `WRoadLane` 0x4, `WRoadProfile`
  0x40, `WRoadSegment` 0x16, `WRoadNode` 0x20, `WRoadIntersection` 0x40.

AI driving, traffic and cop routing use this (`WRoadNetwork.cpp`, `WPathFinder.cpp`). noclip parses
the vertex list only, to snap its camera to roads.

## Collision packs (`0x3B801`) **[decomp + verified]**

The payload starts (16-byte aligned) with `bChunkCarpHeader` (`bWare/Inc/bChunk.hpp`):

| Offset | Field |
|---|---|
| 0x00 | i32 CrpSize: size of the CARP blob that follows |
| 0x04 | i32 SectionNumber: the streaming section, e.g. 101 = `A1` ([maps.md](maps.md#the-streaming-index-decomp--verified)) |
| 0x08 | i32 Flags (1 = resolved/relocated in memory) |
| 0x0C | ptr LastAddress (relocation base) |

All 390 packs are size-consistent and carry 390 distinct section numbers **[verified]**. The CARP
blob holds `WCollisionInstance` / `WCollisionObject` records (both derived from `CARP::CollisionInstance` /
`CARP::CollisionObject`, 0x40 / 0x70 B), with barriers, strips and packed vertices (`WCollision.h`).
`WCollisionMgr.cpp` does the queries. **[decomp]**

## Minimap

`TRACKS/L2RA/` has 269 files: 267 `MINI_MAP*.BIN`, `TrackMaps.bin`, `TroughBoundary.bin`
**[verified]**.

- `MINI_MAP.BIN` is 64 `0003A100 CompTPKBlock` chunks. Each payload is a bare **JDLZ** blob that
  decompresses to a **17,152-byte `TexturePack`** (one tile) **[verified]**.
- `MINI_MAP_<event>.BIN` files are per-event maps. Their names match `gameplay.bin` vaults, e.g.
  `MINI_MAP_10_2_1.BIN` ↔ vault `10_2_1_sprint` ([attributes.md](attributes.md)). Files with an `_R`
  suffix match `r_*` vaults, probably reversed routes **[unconfirmed]**.
- `TrackMaps.bin` is one 63.5 MB TPK (the world-map screen).
- `TroughBoundary.bin` is a single `0x00034190` chunk. The ID is not in the decomp's list; the nearest
  are `Troughs` `0x80034180`–`WallClusters` `0x00034184` **[unconfirmed]**.

## Sky, time of day, weather

- Sky models and textures named `SKY_*` are in `GLOBAL/InGameA.bun` (e.g. `SKY_MIDDAY_A_CLOUDS_A`,
  `SKY_MIDDAY_A_GRADIENT_A`, `SKY_NEXGEN_CAP_A`, 17 names). The stream adds `SKY_1B_00`,
  `SKY_POND_REFLECTION`, … **[verified]**.
- `World/SkyRender.cpp` looks them up by `bStringHash` and draws them with `skyshader` / `IDI_SKYBOX_FX`
  ([shaders.md](shaders.md)) **[decomp]**. `FRONTEND/PLATFORMS/NextGenSky.BIN` is a `GeometryPack`.
- Lighting/TOD data: AttribSys `timeofdaylighting` (13 fields, 8 collections), `visuallook`,
  `visuallookeffect`, `visuallooktransition`, `visualrgbtweaker`; plus `SunInfos`
  ([attributes.md](attributes.md)). Rain: `world`, `CARSFX_Rain`, `RainDropShader`.

## Particles

Emitter **placements** are the per-section `EmitterLibrary` chunks. Emitter **definitions** are AttribSys
classes `emitterdata` (45 fields, 205 collections), `emittergroup` (128), `emitteruv`, `effects` (198),
`fuelcell_effect`, `fuelcell_emitter` **[verified]**. Runtime: `Ecstasy/EmitterSystem.cpp` **[decomp]**.
noclip's `particles.ts` approximates a few emitter types with hard-coded templates.

## Light flares and light packs

AttribSys has `light_flares_cg` (8 fields, 22 collections). **No `LightSourcesPack` (`0x80135000`) or
`LightFlaresPack` (`0x80135100`) chunk occurs anywhere in the install**: all bChunk files outside
`SOUND/` were scanned, plus `STREAML2RA.BUN` **[verified]**. Only `LightMaterials` (`0x00135200`, 156 per
GlobalB file) exist. Where world light flares come from (scenery/solid markers? AttribSys?) is open.
NFS-ModTools' `LightPackReader` targets other games' packs.

## References

| Source | What | License |
|---|---|---|
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) `src/Speed/Indep/Src/World/`, `World/Common/`, `Misc/SpeedChunks.hpp`, `Libs/Support/Miscellaneous/CARP.h`, `Libs/Support/Utility/UGroup.hpp`, `Ecstasy/EmitterSystem.cpp` | Loaders and struct layouts for everything above | CC0-1.0 |
| [magcius/noclip.website](https://github.com/magcius/noclip.website) `src/NeedForSpeedMostWanted/` | WebGL world viewer: regions, scenery, collision chunk id, path vertices, particle approximations | MIT, with a note that some files were reverse-engineered rather than clean-room |
| [SpeedReflect/Nikki](https://github.com/SpeedReflect/Nikki) | `Collision`, `SunInfo`, `Track` editors for GlobalB | MIT |
| [NFSTools/NFS-ModTools](https://github.com/NFSTools/NFS-ModTools) | Scenery/geometry/TPK readers (no collision, AI or emitter support) | no LICENSE file |
| [TsyVM/MWEncyclopedia](https://github.com/TsyVM/MWEncyclopedia) | Leads only (chapters C17, C18, C29, C63): no license, and contains verified errors | none |
