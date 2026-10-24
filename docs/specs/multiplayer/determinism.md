# Determinism audit of the world sim (D1 to D8)

Status: written 2026-10-21 by reading the code. **Steps 1 to 5 of the fix order are applied** (2026-10-22), in the scene and
not in an extracted `nfsmw-sim`; each row says what is done and what is not. The requirements D1
to D8 come from [sync model](sync-model.md) section 6. This file checks the single-player world against them and gives the fix
planned for each finding. The fixes belong to the `nfsmw-sim` extraction ([integration](integration.md) section 4). They change
single-player behaviour only where a finding says so, and the frame-rate trace test (`drive/sim/tests/frame_rate.rs`) guards
each one.

Paths are relative to `crates/nfsmw/src/scenes/world/` unless they start with `libs/`.

**Clean.** `blackbox-vehicle`, `blackbox-roads`, `blackbox-pursuit`, `blackbox-race`, `blackbox-damage` and
`blackbox-collision` use no `Instant`, `SystemTime`, `rand` or threads. Every sort in them is a stable `sort_by` with
`total_cmp`. The car integration is fixed-step (D1, as [physics timing](../../physics-timing.md) says).

The problems are in the scene code that will become `nfsmw-sim`.

## D2: wall clock and frame time

| Finding | Where | Effect | Planned fix |
|---|---|---|---|
| ~~Police, sight, pursuit camera, roadblocks and cop health run once per frame with `steps * STEP`, also on frames with 0 ticks~~ | `simulation.rs` (`police_step`) | | **Done:** they run inside the tick loop with `STEP`, after `physics_step`. `ai/pursuit/tests/frame_rate.rs` checks a chase at 30, 60 and 144 FPS and after a stall |
| ~~The sight test is sampled at frame boundaries~~ | `ai/collide.rs` | | **Done:** `update_sight` now gets `STEP` once per tick |
| ~~The race engine and racer emulation tick on float `sim_dt` per frame~~ | `race/flow.rs` (`race_tick`) | | **Done:** a running race steps once per tick inside the tick loop, so a crossing is tested at the tick it happens. Loading, results and challenge races stay on frame time (a challenge is event-driven) |
| ~~Speedbreaker depletion uses real frame time; the speedbreaker ramp, cinematic moments and `game_speed` scale the tick rate~~ | `drive/time_scale.rs`, `moment_world.rs`, `simulation.rs` | | **Done as a session rule:** `SessionRules::time_scale_allowed` (`session_rules.rs`). Off, the tick runs at 1.0 whatever `game_speed` and the moments ask, and the speedbreaker is barred for every player (`Drive::bar_speedbreaker`). A per-car speedbreaker waits for features.md. Single player keeps all of it |
| ~~The car spawns after a 10 s wall-clock wait~~ | `drive_input.rs` (`RoadWait`) | | **Done:** a session (`lockstep_streaming`) counts 600 updates, single player keeps the 10 s of wall time |
| The whole sim pauses until local streaming is ready; each AI car steps only if its collision is resident | `simulation.rs:50,59-60`, `ai/simulation.rs:89-93` | Local streaming state gates the shared world | **Partly done.** The gate already waits for every player's surroundings (co-op). `SessionRules::lockstep_streaming` makes tiles join in lockstep (`residency/lockstep.rs`): all requested tiles together, in tile order. The world host can wait before each tick until every wanted tile is in and none is in flight, the traffic's support tiles included (`HostConfig::settle_streaming`, [world host](world-host.md)): without the support tiles a host's traffic could wait on the disk where another's did not, which made two hosts part about once in four runs. The scene itself does not wait, and clients do not yet hold only their own car (`StreamingStall`) |
| ~~Race opponent names are seeded from `SystemTime`~~ | `race/start.rs` | | **Done:** `SessionRules::name_seed` is the session seed; single player keeps the clock |

## D3: hash order

Hash maps are handled well. Iterations that matter sort their ids first (`props/phases.rs:57-62`, `props.rs:233-238`,
`props/scenery/explosion.rs:85-86`); the rest are lookups, counts or `retain` calls with no cross-entry effects (`ai/cop_manager/roam.rs`,
`ai/pursuit_budget.rs`, `race/mods.rs`, `libs/blackbox-roads/src/path.rs`).

Three to check during the extraction:

| Where | Question | Planned fix |
|---|---|---|
| `props/phases.rs:23` (foldhash `FixedState`) | Each body is handled alone, then sorted. Looks safe | **Not done:** keep; add a comment saying why |
| `props/streaming.rs:46` (`std` `HashMap`, random seed) | Safe only if a prop id belongs to one tile | **Not done:** assert it in debug builds |
| `residency.rs` (`unload_unwanted`) | Tile removal order follows a random hash; removals look independent | **Done:** the keys are sorted before removing |

## D4: random numbers

| Finding | Where | Effect | Planned fix |
|---|---|---|---|
| ~~One `SplitMix` stream for the whole traffic world~~ | `ai/streams.rs` | | **Done:** `Streams` holds navigation, population, police, roadblock and air streams, each the world seed mixed with a system id by the SplitMix64 finaliser. The default world seed is the old shared seed `0x6E66_736D_7721`. A session will pass its own seed |
| ~~The police draw 4 numbers from that stream every frame~~ | `ai/police/world.rs`, `ai/police/mod_feed.rs` | | **Done:** they draw once per tick, from the police stream |
| Local fixed seeds | `ai/manager.rs:184` (`SplitMix(1)`), `libs/blackbox-roads/src/gps.rs:70` (`SplitMix(0)`) | Deterministic | Keep |
| View-only generators | `ai/sirens.rs`, `libs/blackbox-pursuit/src/speech`, particles, `career/pursuit.rs` case ids | No sim effect | Keep on the client |

## D5: entity order and identity

| Finding | Where | Effect | Planned fix |
|---|---|---|---|
| ~~AI cars are stepped and collide in `Vec` order, which is spawn order~~ | `ai/entities.rs` | | **Done:** every car takes an entity id from its world as it joins (`TrafficWorld::admit`); the list stays in id order (joins append, removals keep the order), so cars step and collide in id order. `ai/pursuit/tests/entities.rs` |
| ~~AI keys come from a process-wide counter that model uploads also advance; the key is the tie-break of roadblock stealing~~ | `keys.rs`, `ai/car/accessors.rs` | | **Done:** each world owns its render keys (`keys::CarKeys`, shared with its models), so two worlds in one process number their cars apart. `AiCar::key` is the entity id; the roadblock steal and director and the 911 call read it. Render keys are presentation only |
| Which models exist depends on a loader thread's timing, and spawns pick among loaded models | `ai/scene.rs`, `ai/model_loader.rs`, `ai/population.rs` | The world depends on local load timing | **Partly done:** `SessionRules::blocking_models` finishes every model load in the frame that asks for it (one loader thread, first in first out), so the same requests give the same models. Picking from the full catalogue with client stand-ins is not done |
| ~~AI crash damage, despawning and prop lifetimes read the local camera, aspect ratio and camera mode~~ | `camera_choice.rs`, `simulation.rs`, `ai/collide.rs`, `props/lifetime.rs` | | **Done:** the crash gate and a car's own damage measure from the nearest player's view, prop lifetimes and despawns from every player's view. `SessionRules::views` picks the views: single player passes this machine's camera (unchanged); a session passes each player's car chase camera at a fixed aspect, with no camera mode, cut scene or free camera. The cops' spawn window and the helicopter follow the lead's view, as the police follow the lead |
| ~~Ids of props without a spawner follow tile arrival order~~ | `props.rs` (`unspawned_id`) | | **Done:** the id is the high bit, the tile and the index in the tile; a tile or index too large for the 15 and 16 bits falls back to the arrival counter |
| Fixed phase order: props, player, traffic, player against traffic, finish steps | `simulation.rs` | Deterministic, but the player always resolves first | **Partly done** (ADR 0010): the player cars step in id order, then the AI. The pairs are the chased player against traffic, then the others against traffic, then player pairs; strict id order for the pairs is not done |

## D6: threads

No `rayon` or parallel iterators in a step. `std::thread::scope` in `open.rs` runs at load time only. The model loader thread
reaches the sim only through D5's load timing.

## D7 and D8

They belong to the protocol (`sim_build` in `Auth`) and the client prediction (quantised inputs; `InputState` is `f32` today,
`libs/blackbox-vehicle/src/input/mod.rs:11`). Nothing to audit until `nfsmw-net` exists.

## Order of the fixes

1. **Done.** Move police, sight, roadblocks, cop health and the race engine into the tick loop (D2). This also makes
   single-player AI independent of the frame rate. The chase frame-rate test covers traffic and police.
2. **Done.** Split the AI random stream per system (D4).
3. **Done.** Per-world entity ids and id-order stepping; the camera inputs become a set of views (D5).
4. **Done as far as the scene allows.** Session-only rules (`SessionRules`, owned by the world, defaults equal to single
   player): no time scaling, seeded streams and names, the players' own views, blocking model loads, lockstep streaming (D2,
   D3, D5). Server-side model choice from the full catalogue and a sim that waits for every tile in flight are not done.
   The `session` console command sets the rules and prints the state hash.
5. **Done.** `WorldScene::state_hash` (FNV-1a over 64 bits, `state_hash.rs`) covers the players' cars, the traffic and its
   streams, the police, the pursuit and the race, in id order. `scenes/world/determinism_tests.rs` runs a scripted free-roam
   drive with traffic and patrol cops twice in one process from the same start and compares the hash every second: 150
   simulated seconds, not 10 minutes, so both runs fit in about 21 s of test time (installed game, ignored without
   `NFSMW_GAME_DIR`). Before lockstep streaming the test failed twice in about twelve runs, both on the first run after a
   build (cold disk cache); with it, it passed the four
   runs since, one of them the first after a build. `ai/pursuit/tests/state_hash.rs` checks the traffic part without the
   install. The test now asks for the traffic's support tiles before it waits, as the world host does (D2 above); it still
   failed once in six runs on 2026-10-22, the first run after a build, so a cold-cache cause remains.
6. **Done (2026-10-22)** for the real world behind `WorldSim`: `scenes/world/net/tests/installed.rs` drives two
   `WorldSimHost`s from one seed with the same scripted input for 40 simulated seconds (two players, 16 cars with cops)
   and compares the events every tick and the hash and entity views every second. It passed the seven runs since the
   support-tile fix, the first after a build among them ([world host](world-host.md)).
