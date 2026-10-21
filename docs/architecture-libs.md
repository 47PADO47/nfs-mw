# Libraries (`libs/`)

The crates under [`libs/`](../libs) hold everything that also works for other EA Black Box games or other
reimplementations. Game differences go into per-version layout tables, chosen from the data or passed in by the caller.
Each lib has its own README and can move to a shared utilities repo unchanged. The split between `libs/` and
`crates/` is in [architecture.md](architecture.md#two-halves-libs-and-crates).

| Crate | Job | Version handling |
|---|---|---|
| [`ea-compress`](../libs/ea-compress) | JDLZ, HUFF, RAWW | self-describing headers |
| [`blackbox-attrib`](../libs/blackbox-attrib) | AttribSys gameplay databases (`VPAK`, vaults, classes, collections, inheritance) | `layout/` per AttribSys generation (legacy = MW), detected per vault |
| [`blackbox-carparts`](../libs/blackbox-carparts) | Car types, the parts database, slot types, presets, light materials | `layout::MOST_WANTED` (parts pack v6, 139 slots) passed by the caller |
| [`blackbox-hash`](../libs/blackbox-hash) | `bStringHash` | — |
| [`blackbox-chunk`](../libs/blackbox-chunk) | Zero-copy bChunk trees; chunk ids by domain | — |
| [`blackbox-tpk`](../libs/blackbox-tpk) | Texture packs (plain and compressed forms), CPU decoding | `layout/` per TPK version (5 = MW) |
| [`blackbox-solid`](../libs/blackbox-solid) | Solids: groups, every vertex buffer, indices, position markers | `layout/` per `SolidInfo` version (0x16 = MW) |
| [`blackbox-streaming`](../libs/blackbox-streaming) | The track streaming index | `layout::MOST_WANTED` passed by the caller |
| [`blackbox-scenery`](../libs/blackbox-scenery) | Scenery infos and instances; visibility rule | `layout::MOST_WANTED` passed by the caller |
| [`blackbox-collision`](../libs/blackbox-collision) | World collision packs, the collision grid, car and prop bounds, a ray-cast query | — (one layout so far) |
| [`blackbox-damage`](../libs/blackbox-damage) | Persistent visual damage per impact zone (panel, paint, glass levels) and punctured tires; impacts in, levels out | — (zones and glazing passed by the caller) |
| [`blackbox-carp`](../libs/blackbox-carp) | Embedded CARP tagged groups shared by collision, roads and event packages | — |
| [`blackbox-sequence`](../libs/blackbox-sequence) | Event systems, states, action queues and timed event lists | — (event handlers and tag hashes supplied by the game) |
| [`blackbox-feng`](../libs/blackbox-feng) | FEng user-interface packages and fonts: reader, script and message runtime, a retained `UiTree`; no rendering | — (one format version) |
| [`blackbox-text`](../libs/blackbox-text) | Language string tables (`LANGUAGES/*.bin`) | — |
| [`blackbox-anim`](../libs/blackbox-anim) | EAGL4 animation: objects, skeletons, animation banks, channel codecs and pose evaluation (being built) | — (bytes in, no layouts yet) |
| [`blackbox-nis`](../libs/blackbox-nis) | EA NIS cutscene scenes: header, characters, scene clock, placement | — (one format version) |
| [`ea-audio`](../libs/ea-audio) | EA audio: SCHl streams, ABK sound banks, MPF/MUS music and its node graph, `.big` speech, `.gin` engine loops; EA-XA, EA-XAS and MicroTalk decoders | self-describing headers |
| [`blackbox-ginsu`](../libs/blackbox-ginsu) | The Ginsu granular engine-sound synthesiser and the tables of a `.gin` file | — |
| [`blackbox-carsound`](../libs/blackbox-carsound) | Car sound controllers: telemetry in, engine mix out, on a fixed 60 Hz tick | — (tuning passed by the caller) |
| [`blackbox-minimap`](../libs/blackbox-minimap) | The minimap: the compressed map tiles, the world-to-map projection, the view around the player (tiles, scroll, turn) and the placement of blips | — (one format version) |
| [`blackbox-mixmap`](../libs/blackbox-mixmap) | The sound system's dynamic mixer: `MIXMAPS/*.mxb` parser and a deterministic evaluator (published values in, per-object volume, pitch and filter slots out) | — (one format version) |
| [`blackbox-aems`](../libs/blackbox-aems) | AEMS module banks inside `.abk` files: a reader and an interpreter of the event-sound graphs (which samples play, how loud and how high); voices and objects are a `Host` trait the caller implements | — (one format version) |
| [`blackbox-movie`](../libs/blackbox-movie) | EA VP6 movies (`.vp6`): demuxer and video decoder (the MIT `nihav-vp6`); no audio yet | — |
| [`blackbox-pursuit`](../libs/blackbox-pursuit) | Pursuit rules as pure functions: heat, the infraction log and its detection, cop tactic geometry, and the police helicopter (flight model, controller, sight, behaviours) | — (fines, heat tables and per-level tuning passed by the caller) |
| [`blackbox-race`](../libs/blackbox-race) | Race rules as pure code: gates (point and segment tests), the course with ordered checkpoints, laps and the finish, the race clock (staging, countdown, time limit and bonus), ranking, the lap knockout rule, speed-trap points, the catch-up controller and a `Race` that ties them together | — (limits, laps and gains passed by the caller) |
| [`blackbox-speedfx`](../libs/blackbox-speedfx) | High-speed screen effects as curves: speed streaks, radial blur, shake and a nitrous FOV kick, with smoothing | — (tuning passed by the caller) |
| [`blackbox-particles`](../libs/blackbox-particles) | Particle emitters: spawn and update rules (cone spray, drag, gravity, keyed size, angle and colour curves) as a deterministic simulation that yields sprites | — |
| [`blackbox-vehicle`](../libs/blackbox-vehicle) | Deterministic fixed-step vehicle physics: rigid body, engine and gearbox, suspension, tires, steering, aero; driven by plain parameter structs and a `Ground` ray-cast trait | — (parameters passed by the caller) |
| [`blackbox-gfx`](../libs/blackbox-gfx) | The renderer-neutral graphics interface: shared types, the `RenderBackend` trait, capabilities, graphics settings and the pure `resolve` | — |
| [`blackbox-gfx-testkit`](../libs/blackbox-gfx-testkit) | Test support for any renderer: procedural scenes, image metrics and digests (no game data, no GPU API) | — |
| [`blackbox-gpu-passes`](../libs/blackbox-gpu-passes) | Reusable wgpu passes (UI layer, world effects, soft particles, fullscreen filter, FSR 1) that any render loop can call with its own encoder | — |
| [`blackbox-render`](../libs/blackbox-render) | Backend-neutral renderer (wgpu inside); implements the `blackbox-gfx` types and `RenderBackend`; headless mode | — |
| [`blackbox-bevy-render`](../libs/blackbox-bevy-render) | Optional Bevy 0.20 renderer implementing `RenderBackend` as plugins of the game's `App` (world path only so far, [bevy-backend.md](bevy-backend.md)); a leaf crate behind the `renderer-bevy` feature | Bevy render crates |
| [`fsr3-wgpu`](../libs/fsr3-wgpu) | AMD FidelityFX Super Resolution 3.1 temporal upscaler in WGSL for wgpu (fp32, no subgroup operations); used by the native renderer (`blackbox-render`) and the Bevy renderer | — |
| [`blackbox-roads`](../libs/blackbox-roads) | The road network (`RNgp`: nodes, segments, lane profiles, roads), lane geometry, closest-segment search and the road navigator (a cursor that follows a lane through junctions) | — (one format version) |
| [`blackbox-driver`](../libs/blackbox-driver) | Controllers of computer-driven cars: the simple traffic driver, the adaptive-PID racer driver, reverse logic, stuck detection; plain numbers in, gas/brake/steer out | — (tuning passed by the caller) |
| [`blackbox-scene`](../libs/blackbox-scene) | Uploading solids and textures to the renderer; boxes; frustum culling | — |
| [`game-install`](../libs/game-install) | Finding, validating and reading an install, case-insensitively | driven by a `GameSpec` |
