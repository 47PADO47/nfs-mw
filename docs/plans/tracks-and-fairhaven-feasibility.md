# Feasibility: RIDE 6 tracks and the full Fairhaven map

- **Status:** research with working evidence (2026-11-07). Nothing is built in the game; the proofs are Python scripts in
  `tools/` and exports in `C:\r6work\`.
- **Question:** can the rewrite offer (1) the tracks of RIDE 6 and (2) the whole open city of NFS: Most Wanted (2012),
  "if not too expensive"? Formats: [chameleon-world.md](../formats/chameleon-world.md), [ride6-tracks.md](../formats/ride6-tracks.md).
  How it was learned: [provenance/chameleon-world.md](../provenance/chameleon-world.md). Rules: external content is **streamed
  from the original files**, converted only when streaming is impossible, every feature toggleable, nothing from a game in the
  repository ([licensing.md](../licensing.md)).

## 1. Recommendation

| Target | Verdict | Why, in one line |
|---|---|---|
| **Fairhaven, render only** (fly through the whole city) | **Do it** | Proven: the format is simple, units are self-contained, a district exports in 1.6 s; streamable from the install with no conversion (about 7 agent-days) |
| **Fairhaven + collision** (drive free roam, no traffic) | **Do it, after the first stage** | Proven: the collision soup is the car's soup format and decodes for all 169 units (1.65 M triangles); the driving code needs a ground source and a surface mapping (about 7 agent-days) |
| **Fairhaven + roads and AI** (traffic, police, pursuits) | **Do not commit yet: a 3-day spike first** | The nav data (`RoadData`, `AiData`, `TrafficLaneData`, `NavigationMesh`) is only partly seen and the 2005 AI wants its own lane model; high risk (8 to 10 agent-days if the spike succeeds) |
| **Fairhaven + events** (races, safe houses, career) | **Do not** | The 2005 career is tied to the Rockport vault, rank and region data; a second city would need a new game mode, not a port (15+ agent-days, low value) |
| **RIDE 6 tracks** | **Do it in stages, one track first (Okayama), then all** | Proven: ground, kerbs and the whole lap export; the data is regular across 39 tracks; it must be converted (UE cooked, no Rust reader), at about 100 to 500 MB per track |

Both targets need one host feature that does not exist: **a second world** (section 2). Build it once, for Fairhaven's streamed
source and for converted RIDE 6 track packs, rather than twice.

## 2. How the 2005 game loads a world, and what a second one needs

### What exists

- `WorldIndex::open(dir, track)` ([`nfsmw-data/src/world/index.rs`](../../crates/nfsmw-data/src/world/index.rs)) already takes a track
  name: it reads `TRACKS/<track>.BUN` (streaming index, visible sections, collision grid, road network, track zones and barriers,
  prop bounds, event sequences, model hierarchies, scenery groups and overrides) and names `TRACKS/STREAM<track>.BUN` as the stream.
  The name comes from the constant `DEFAULT_TRACK = "L2RA"`, used by `scenes/world/open.rs` (four places), the assets viewer and a few
  readers that name `TRACKS/L2RA.BUN` themselves (the weather and wind maps, the clouds, the audio environment, the mesh animation,
  `minimap.rs`, the career's world map). Everything is one world, hard-wired ([architecture.md](../architecture.md#the-streamed-city-view-world)).
- The streamer ([`world/streamer.rs`](../../crates/nfsmw-data/src/world/streamer.rs), `scenes/world/residency/`) reads byte ranges of one
  stream file by section, converts on loader threads and uploads within a byte budget; zones and visible lists decide what is
  loaded and drawn ([visible-sections.md](../specs/visible-sections.md)).
- A mod can **replace files** through the overlay (`TRACKS/L2RA.BUN` as a whole, which replaces Rockport) and **add** a scenery pack
  (`scenery/*.toml`: placements loaded near an anchor, optional static collision) and **interiors** (an entry zone and a place):
  [api-content.md](../modding/api-content.md#scenery-and-map-packs), [mod-world-content.md](../specs/mod-world-content.md). The mod API says
  plainly that a new district (new sections, road network, collision, nav data) is "specified but not promised", and the road
  network registry is reserved. `bb.world` reads the track and section name, sets rain and traffic, and has no world selection.
  A native mod cannot provide a scene: the `Scene` trait is the host's.

So a mod can add objects to Rockport and cannot add a world. A whole alternative world is host work.

### What a second world needs from the host, layer by layer

| # | Layer | The 2005 world's source | A second world must supply | Where it plugs in |
|---|---|---|---|---|
| 1 | **Selection and loading** | `DEFAULT_TRACK`, `WorldScene::load`, `Options` | a world id (config key and Gameplay menu row, `--world`), a loading thread, a loading screen, no L2RA literals | `scenes/world/open.rs`, `settings/`, `frontend/options*.rs` |
| 2 | **Streaming and residency** | 720 sections of one stream file, 435 zones with visible lists | a source of "pieces near the camera": for Fairhaven 169 units found by the `PVS` rectangles (unit files, resources shared by id); for a track pack a few large pieces | `nfsmw-data/src/world/streamer.rs`, `residency/` |
| 3 | **Scenery rendering** | `blackbox-solid` + `-scenery` instances, TPK textures, pre-lit shading | static meshes (float positions, UVs, strips), materials by family (terrain blend, road, building, alpha-tested), BC1/3/7 textures, level of detail from the model | `libs/blackbox-gfx` meshes and textures; a shading class per technique family |
| 4 | **Sky, fog, light, time of day** | `SKYDOME` scenery, `GlobalB` lighting, `timeofdaylighting` | a sky and a sun; Fairhaven has no dome among its models (probably a post effect), so the host's own sky is used | `scenes/world/sky/`, `lighting/` |
| 5 | **Collision** | one `CarpWCollisionPack` per tile in a `CollisionWorld`, grid of 64 m cells | triangles with a **surface kind** per face: ground rays (tyres, camera, road placement), walls, props | `blackbox-collision`, `scenes/world/ground.rs`, `drive/`; the soups are 1.65 M triangles for Fairhaven |
| 6 | **Surface types** | `simsurface` grips by the pack's surface byte | a map from the new data's surface id to a grip, sound and tyre effect (Fairhaven: the face flag word; RIDE 6: `PM_T_*` and `SurfaceType1..10`) | `nfsmw-data/src/car/physics`, `blackbox-vehicle` `Ground` |
| 7 | **Road network** | `RNgp` lanes, segments, nodes in `L2RA.BUN`; track zones; helicopter sheet | the same *structure* from other data: lanes with widths, directions and links for traffic, police, racers and the GPS | `blackbox-roads::RoadNetwork` |
| 8 | **Spawn and placement** | `road.rs` finds asphalt in the collision, `reset`/`tp` search the nearest street | a start point and "nearest road" over the new collision (needs the surface kind of layer 6) | `scenes/world/road.rs`, `start.rs` |
| 9 | **Minimap and world map** | `TRACKS/L2RA/MINI_MAP*.BIN` tiles and a calibration in the `GlobalB` track table | a map picture (Fairhaven's `MAPS/*.BNDL` textures; a track's outline from its spline) and a calibration | `nfsmw-data/src/minimap.rs`, `hud/minimap.rs`, `frontend/world_map/` |
| 10 | **Events, markers, triggers** | `gameplay.bin` vault (races, zones, shops) and `EmTriggerPack` per section | race start grids, finish lines, checkpoints, entry zones; for tracks a start grid, laps and a pit lane; Fairhaven's `TriggerData` is not decoded | `gameplay/`, `scenes/world/race/`, `entry_zones.rs` |
| 11 | **Props, animation, particles** | prop bounds, event sequences, mesh animations, emitters | Fairhaven's `PropObject`, `CompoundObject`, `WorldObject`, groundcover; RIDE 6's instanced barriers and grass | `scenes/world/props/`, `mesh_animation.rs` |
| 12 | **Audio and weather maps** | environment zones, rain and wind parameter maps from `L2RA.BUN` | a neutral default (no map): both readers must tolerate a world without them | `audio/environment_context.rs`, `weather/` |
| 13 | **Mod API** | `bb.world` track name | a `content.worlds()` registry (id, name, source) and a way to choose it | `docs/modding/api-content.md` |

Layers 1 to 4 give a flying camera. 5, 6 and 8 give driving. 7 gives traffic and police. 9, 10 and 11 give a game.

## 3. Fairhaven (NFS: Most Wanted 2012)

### Where it lives and how it is built

`HAWAII/TRK_UNIT<n>.BNDL` (169 files, 3.32 GB) are the city: each is self-contained for what it draws (models, renderables, materials,
textures) plus its instance lists, collision soup and nav mesh; the zone list `PVS.BNDL` gives each unit's rectangle and district.
Shared sets sit in `DISTRICT_*`, `GLOBALRESOURCES`, `SHADERS` and `GLOBALTEXTUREDICTIONARY`. The city spans **7.25 by 11.3 km**
(70 km2 of overlapping rectangles), with **35,431 placed models, 12,290 props, 4,301 compound objects, 4,477 dynamic objects, 8.8 million
groundcover instances and 49.4 million triangles at the highest level of detail**. Counted once by resource id the whole city is
**1.07 GB** (inflated), stored as 3.3 GB because units repeat what they use (the repeats are byte-identical). Full facts:
[chameleon-world.md](../formats/chameleon-world.md).

### What the proofs show (all in `tools/`, Python, no build)

| Proof | Result |
|---|---|
| Census of all 169 units (`chameleon_census.py`) | 2.8 s for the whole city; per-unit counts, the overhead picture `C:\r6work\fairhaven\overhead.png` (rectangles by district, the roads show as dotted lines) |
| Render export of a district (`chameleon_world.py export ... 161 160`) | 820 instances, 418,024 triangles, 218 textures, 30 MB glb in **1.6 s**: `C:\r6work\fairhaven\district_161_160.glb`, preview `d161_obl.png` (a harbour district with highway, roads, quays, a pier and tower blocks, recognisable) |
| Collision decode of all units (`chameleon_collision.py stats`) | 36,169 chunks, 1,201,146 triangles + 225,442 quads, no error, 2.2 s; unit 161's faces inside its rectangle and drawn as streets and quays (`u161_col2.png`, `unit161_collision.obj`) |
| Streaming cost of a unit | reading and inflating a 16 MB unit file takes **0.07 s** in Python (30 MB inflated, 10 MB of it not textures) |

### Cost by layer (agent-days)

An agent-day is one agent working a day with specs and tests written first and no builds; the lead's build, fixes and review are
extra (about +30 %). The numbers are what the proofs say is hard: reading formats took hours here; integrating with the engine is
the cost.

| Stage | Includes | Agent-days | Risk |
|---|---|---|---|
| **F1 render only** | `libs/chameleon-world` (zone list, instance lists, models, collision soup header), `nfsmw-data` unit loader (resolve ids across unit, district and global bundles), mesh and texture upload for the technique families (diffuse first; terrain blend, road layers, alpha test, doubled sides), unit residency by camera and `PVS` rectangle with budgets, LOD from the model, the host sky and fog, `view-fairhaven` scene, toggle and menu row | **6 to 8** | low |
| **F2 + collision and driving** | face flag word to surface kind (a survey against `SURFACELIST`: 2 d), a `CollisionWorld` source for triangle soups with a ground grid (3 d), road placement and reset over it, props and groundcover left non-solid; 2012 cars from the existing content mod drive on it | **+6 to 7** | medium (surfaces) |
| **F3 + roads and AI** | spike: decode `RoadData`, `AiData`, `TrafficLaneData`, `NavigationMesh` against the drawn roads (3 d, Ghidra for the loader), then a `RoadNetwork` built from them (3 to 4 d) and traffic, police and helicopter on it (3 d) | **+9 to 10** (3 for the spike) | **high** |
| **F4 + events** | `TriggerData`, race start and finish definitions of the 2012 game, minimap and `MAPS` textures, a free-roam activity layer; no career | **+10 to 15** | high, low value |
| Total to a drivable, empty Fairhaven (F1 + F2) | | **12 to 15** | |

### Runtime and disk cost if streamed

- **Disk: nothing added.** The loader reads the install. A small index of "resource id to bundle and block" can be built at start
  (28,738 ids in the Hawaii folder, 1.1 s in Python, well under 2 MB), or cached in the config folder.
- **Per unit:** one file read, median 18.8 MB (maximum 45.6 MB); zlib blocks per resource so the loader can inflate only what it
  draws; 30 MB inflated for a typical unit of which about 10 MB is not texture; **0.07 s** to read and inflate in Python, so a
  background thread loads a unit well inside a second.
- **Memory:** a resident ring of 12 to 20 units is 0.4 to 0.6 GB of textures and meshes, less with the shared ids stored once. The
  whole city unique is 1.07 GB, so a 2 GB budget could hold all of it, but streaming by rectangle is the plan.
- **Collision:** 34 MB of soups for the whole city (decoded to triangles on the loader thread, 1.65 M triangles).
- **When streaming is not enough:** the 49 million triangles at LOD 0 mean the loader must use the model's lower levels beyond a
  distance and cull by unit rectangle; both are in the data. A converted copy is **not** needed: it would be at least 1.07 GB
  (a glb export of the whole city would be several GB of glTF) for no gain over reading the install. The only conversion the
  plan allows is an in-memory one (triangle soup to a collision grid at load).

### Risks and unknowns

1. Shading: the world shaders are a lightmapped physically-based chain (`*_Lightmap`, lit windows, planar reflections, terrain
   blending by mask). A diffuse-only first version looks flat; matching the original is open-ended. Mitigation: ship F1 with the
   renderer's existing lit shading and treat the rest as polish.
2. The unit zone list's other offsets (visibility per unit) are not decoded; the plan uses the rectangles and a radius.
3. Surface kinds for collision are unconfirmed (the flag word).
4. The nav data is the big unknown; F3 is gated by its spike.
5. Cars and traffic come from the 2012 content mod: Fairhaven has its 2012 traffic and 2012 handling only through that mod; the
   2005 police and pursuit logic would run on the 2012 road data.
6. Licensing: the formats are measured here from the user's install, the executable was only searched for strings; community
   tools are not used. Keep it spec-first: write the `docs/formats` page of each layer before its reader.

## 4. RIDE 6 tracks

### What a track is

39 map folders (Okayama, Suzuka, Monza, Imola, Mugello, Nordschleife, the Isle of Man roads, Stelvio, Kanto, Nevada, the Blue Wave
arenas...). Each is a **persistent level that streams about 25 sub-levels** by name: terrain slices and kerbs, buildings, props and
barriers placed along splines, vegetation, grass, crowd, splines (the track, the borders, racing lines, pit lane, respawn), cameras,
lighting, audio. The ground is static meshes (Okayama: 12 slices, 7 kerbs, a "runnable" tarmac mesh, far terrain) whose
materials are layered shaders; the **surface type under a wheel comes from each material instance's physical material**
(`PM_T_Tarmac` = `SurfaceType1`...). The lap is a closed **spline** of 44 points with left and right border widths per sample.
Details: [ride6-tracks.md](../formats/ride6-tracks.md).

### What the proofs show

| Proof | Result |
|---|---|
| Survey (`R6ToolParts list`, `jsonmany`) | 1,075 levels, 1,002 under `maps/Tracks/`; Okayama's 26 levels dumped in 4 s; 1,061 static-mesh actors, 48,000 instanced meshes |
| Ground export (`R6ToolParts export` of 21 meshes) | **444,437 triangles** (12 slices, 7 kerbs, runnable, terrain) in **1.5 s**, `C:\r6work\tracks\okayama_ground.glb` (34 MB as glTF, geometry only); the top view shows the 3.7 km circuit with its pit lane (`tools/glb_merge_preview.py`) |
| Splines | `TrackSpline` 3,742 m, closed loop, 44 points with tangents, curvature and border widths; two racing lines `IdealTrajectory_*` of 3.72 and 3.67 km |
| Surface mapping | material instance to `PhysMaterial` to `EPhysicalSurface::SurfaceTypeN`, read for tarmac and metal grid |
| Size of one track's own cooked assets | 622 MB for Okayama (468 MB textures, 99 MB meshes, 15 MB levels, 69 MB other), before the shared sets `ObjShared` (7,267 files), `VegShared` and `EnvShared` that every track uses |

### Why it is a conversion, not streaming

The data is UE5 cooked: IoStore containers with Oodle blocks, packages that need the game's `.usmap` for unversioned properties,
meshes and textures in engine-specific serializations. The only reader is CUE4Parse in C#. A Rust reader of UE5 packages is a project
larger than the whole track feature, so the tracks are **converted once by an external tool** (the rewrite's exporter, which the user runs
over their own install) into a compact track pack read by the host. This is the case the rule allows ("only when streaming is
impossible, a very small fast converter, small on disk, measured"): convert only what a game needs, not the cooked data.

### Cost by layer (agent-days)

| Stage | Includes | Agent-days | Risk |
|---|---|---|---|
| **R1 render, first track** | the track-pack format (spec first): meshes (positions, UV, indices), a few material classes (terrain blend baked or approximated, tarmac, kerb, generic), textures at 1024 or less as BC1/BC7, the exporter stage for one track, the host loader and `view-track` on the common second-world scene (layers 1 to 4) | **6 to 8** | medium (materials) |
| **R2 + surfaces, collision, bike driving** | the ground meshes as collision with a per-face surface from the material's `PhysMaterial`; mapping `SurfaceType1..10` to the bike data that exists (`ride6-tyre`, `ride6-chassis-round6`); placement at the start | **+4 to 5** | medium |
| **R3 + splines: lap and line** | pack the track spline, borders, markers, pit lane and respawn; lap counting and timing; a one-lane `RoadNetwork` from the spline for the racers and the minimap outline; start grid | **+5 to 6** | medium |
| **R4 + props, barriers, vegetation** | instance lists (the barriers along splines, grass, trees, crowd), shared sets, LOD | **+4 to 5** | medium (volume) |
| **R5 more tracks** | the same exporter and pack, per-track fixes | **0.5 to 1 each** (39 tracks: 20 to 40) | low per track |
| Race mode for bikes (grid, laps, results) | no such mode exists for bikes; the 2005 race rules are for cars on a road network | not estimated | |

### Disk and time of the converted pack (estimates)

Okayama's own cooked data is 622 MB; the shared sets add to it. A pack that keeps the ground at full detail (444k triangles at about
20 bytes a vertex: **10 to 15 MB**, not the 34 MB of glTF), props as instances of shared meshes, and textures limited to 1024 pixels as
BC1/BC7, should be **100 to 300 MB** per track and, with a shared mesh and texture store across tracks, much less for the second
track onward. Conversion time: the ground exported in 1.5 s; a whole track (levels, meshes, textures) is estimated at 1 to 3 minutes
with the exporter's warm cache. **Both are unmeasured**: the first R1 build must measure them and put the numbers in the docs.
Every pack is generated on the user's machine into the config folder and is toggleable with the world option.

### Risks and unknowns

1. Materials: ground is a layered, mask-blended shader with detail tiling; a pack needs a baked colour or a small set of
   shading classes. Baking per track is the main R1 effort.
2. Collision: the cooked meshes show an empty body setup; the "runnable" meshes are assumed to be the contact surface. Needs a
   check against the game (a bike on kerbs and grass).
3. Surface mapping for the mask-based terrain materials (grass, sand, gravel) is unconfirmed.
4. Bike physics need terrain surface types and slope; the specs list the three off-track surfaces and the terrain terms
   ([ride6-chassis-round6.md](../specs/ride6-chassis-round6.md)) that today have no road surface to read.
5. Race rules for bikes do not exist in the rewrite.
6. Licence and distribution: the converter reads the user's install and writes to their machine only; nothing is distributed.

## 5. Staged milestones

Build the shared **external world** first, then fill it.

| Milestone | Deliverable | Gate | Agent-days |
|---|---|---|---|
| **M0 world seam** | `--world <id>` / config / menu row; no `L2RA` literal outside the 2005 world; a `WorldSource` boundary (units or pieces near the camera, collision, roads, minimap) with the 2005 city as its first implementation; no behaviour change | all existing tests and screenshots equal | 4 to 5 |
| **M1 Fairhaven F1** | `libs/chameleon-world`, loader, render, `view-world --world fairhaven` | an offscreen flight over three districts at an acceptable frame time | 6 to 8 |
| **M2 RIDE 6 R1** | track-pack spec, the exporter's pack stage, `view-world --world okayama` | the circuit from the start line, kerbs and pit lane visible | 6 to 8 |
| **M3 Fairhaven F2** | collision source, surfaces, drive a 2012 car | a scripted drive along three roads without falling through | 6 to 7 |
| **M4 RIDE 6 R2 and R3** | surfaces, bike on the track, spline lap and timing | a KTM completes a timed lap | 9 to 11 |
| **M5 spike** | decode the Fairhaven nav data; go or no go for traffic | the lanes drawn over the roads | 3 |
| **M6 more** | R4, R5, F3 if the spike passes | per target | 4 to 5 per RIDE 6 layer; 9 to 10 for F3 |

Totals: **a flyable Fairhaven in M0 + M1 (10 to 13 agent-days)**, a **drivable Fairhaven in about 17 to 20**, **one drivable, timed RIDE 6
track in M0 + M2 + M4 (19 to 24)**, all tracks about 20 to 40 more.

## 6. What is streamed and what is converted

| Data | Source | Streamed or converted |
|---|---|---|
| Fairhaven models, renderables, materials, textures | `HAWAII/TRK_UNIT*`, `DISTRICT_*`, `GLOBALRESOURCES`, `SHADERS`, `GLOBALTEXTUREDICTIONARY` | **Streamed** by resource id; uploaded as BCn directly |
| Fairhaven collision, instance lists | the same units | **Streamed**, decoded in memory on the loader thread into the collision grid |
| Fairhaven roads and AI | `ROADDATA`, `AIDATA`, `TRAFFIC`, unit nav meshes | **Streamed** once decoded (F3); read whole at load (about 4 MB) |
| Fairhaven sky, time of day | `ENVIRONMENT.BNDL` | the host's own first; the 2012 data later |
| RIDE 6 meshes, textures, levels | IoStore containers | **Converted** by the exporter into a track pack (cooked UE5 cannot be read in Rust) |
| RIDE 6 splines, surface kinds, markers | level and material JSON | **Converted** (a few KB) |
| RIDE 6 shared sets | `ObjShared`, `VegShared`, `EnvShared` | **Converted once**, shared by all packs |
| Track packs and caches | config folder | generated on the user's machine, never committed |

## 7. Open questions for the owner

1. Is "Fairhaven as an empty free-roam city, no traffic or police at first" an acceptable first deliverable (F1 + F2 = 12 to 15
   agent-days)? The alternative, waiting for the nav spike, delays driving.
2. Should RIDE 6 tracks be circuit-only sessions (time trial, start grid, laps) or part of a career?
3. Which comes first: Fairhaven F1 or RIDE 6 R1? They share M0; Fairhaven gives a result sooner (streaming, no conversion).
4. Texture detail for converted RIDE 6 packs: 1024 pixels or full.
