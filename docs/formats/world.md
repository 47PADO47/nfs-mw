# World systems: collision, road network, triggers, effects, sky, minimap

This page covers the non-visual world data. Streaming, scenery placement and the `L2RA` chunk overview
are in [maps.md](maps.md). Chunk names follow [`tools/bchunk_names.py`](../../tools/bchunk_names.py),
which now takes them from the decomp's `SpeedChunks.hpp`. For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

## Overview

| Chunk | Where (count) **[verified]** | What | Best reference |
|---|---|---|---|
| `0003B800 CarpWGrid (UWorld)` | `L2RA.BUN` (1 × 551,432 B) | World "map" tree: collision grid + **road network** | decomp `World/Common/WWorld.cpp`, `WGrid.cpp`, `WRoadNetwork.cpp` (144 KB), `WRoadElem.h`; noclip reads path vertices |
| `0003B801 CarpWCollisionPack` | `STREAML2RA.BUN` (390, 5.8 MB) | Static world collision per section | [collision.md](collision.md) |
| `8003B900 BoundsPack` → `0003B901 CollisionBody / Bounds` | `L2RA.BUN` (405), `GlobalB.lzc` (86) | Prop bounds (L2RA) / car bounds (GlobalB) | [collision.md](collision.md); Nikki `Collision` (car bounds, MIT) |
| `8003B810 CarpEventSequences` → `0003B811` | `L2RA.BUN` (73) | Scripted world events (pursuit breakers, …) | decomp `Libs/Support/Miscellaneous/CARP.h` |
| `80036000 EmTriggerPack` (`36001` header, `36002` tree, `36003` triggers) | stream (415) | Trigger volumes per section | decomp `World/EventManager.cpp` |
| `80034147 TrackPathManager` → `0003414A TrackPathZones`; `0003414D TrackPathBarriers` | `L2RA.BUN` (705 zones, 1,376 barriers) | AI / navigation zones and barriers | [road-network.md](road-network.md); decomp `World/TrackPath.cpp` |
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
  0x40, `WRoadSegment` 0x16, `WRoadNode` 0x20, `WRoadIntersection` 0x40. **Layouts and statistics, measured
  on the install (4,385 nodes, 6,538 segments, 710 profiles, 1,308 roads, no intersection records), are in
  [road-network.md](road-network.md)**, together with the `TrackPathZones` and `TrackPathBarriers` tables.

AI driving, traffic and cop routing use this (`WRoadNetwork.cpp`, `WPathFinder.cpp`). noclip parses
the vertex list only, to snap its camera to roads.

## Collision packs (`0x3B801`) **[decomp + verified]**

One `CARP` blob per streaming section with the instances, triangle strips and barriers that make the
static world solid; the collision grid in the `0x3B800` tree finds them. Layouts, the car and prop
bounds, surface types and the query rules are in [collision.md](collision.md).

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
- **The sky domes are ordinary scenery** **[verified]**. Scenery infos `SKYDOME` (349 instances across
  the tiles) and `SKYDOME_XENON` (25) place world-space models (flag `0x200`, at the origin) about 19.4 km
  across, spanning z −1,148 to 5,545. They don't follow the camera. Their textures come from
  `InGameA.bun` (`SKY_MIDDAY_A_CAP_A`, `SKY_MIDDAY_A_CLOUDS_A`; the Xenon dome uses
  `SKY_NEXGEN_CLOUDS_A`).
  - `SKYDOME` uses effect 0 with 36-byte vertices and white vertex colours; drawn pre-lit and unfogged,
    it gives the overcast midday sky.
  - `SKYDOME_XENON` uses effect 19, the only 44-byte vertex format in the stream: the common 36 bytes
    (white vertex colour) plus a second UV pair at 36. It draws the detailed next-gen clouds.
  - `SKY_SPECULAR` (textured `SKY_REFSKYSPECULARB`) looks like a layer for reflections. Drawing it in
    the player view darkens the whole dome **[unconfirmed]**.
- Lighting/TOD data: AttribSys `timeofdaylighting` (13 fields, 8 collections), `visuallook`,
  `visuallookeffect`, `visuallooktransition`, `visualrgbtweaker`; plus `SunInfos`
  ([attributes.md](attributes.md)). Rain: `world`, `CARSFX_Rain`, `RainDropShader`.

## Water

**[verified]** Water surfaces are ordinary world-space terrain solids textured with the animated
`ANM_WATERA_` texture (frames `ANM_WATERA_000`–`013`), which uses alpha usage 2 and blend 0, so it draws
opaque. 39 solids use water textures:

| Solid (info) | Tile | Extent |
|---|---|---|
| `TRN_Ocean_A` | R89 (radius 6,464 m) | x 1,373…10,032, y −9,268…1,470, z = 0 |
| `TRN_Ocean_B` | R88 (radius 4,739 m) | x 3,186…9,911, y 616…8,847, z = 0 |
| `TRN_CP_CR_Water_A_01`, `TRN_CP_BD_Pond_A_01`, park ponds | D16, O17, … | local |

Breakwaters (`TRNS_WATERBREAK_A/B`) are blended overlays (blend 1). The oceans live in two of the 37
**map-wide tiles** (radius over 1 km, 8 MB in total, also the distant panoramas). A streamer that loads tiles
by distance must keep those loaded, or the sea disappears. Animating the water (cycling the 14 frames,
`TextureAnimPack` `0xB0300100`) and its reflections are not implemented.

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
