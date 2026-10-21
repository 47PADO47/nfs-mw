# Determinism audit of the world sim (D1 to D8)

Status: **audit and plan only; nothing here is applied.** Written 2026-10-21 by reading the code (no run). The requirements D1
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
| Police, sight, pursuit camera, roadblocks and cop health run **once per frame** with `steps * STEP`, also on frames with 0 ticks | `simulation.rs:82-122` | Results depend on how ticks group into frames | Call them inside the tick loop with `STEP`, after `physics_step`. Single player then matches any frame rate, as the car already does |
| The sight test is sampled at frame boundaries (`sight_clock += dt`) | `ai/collide.rs:255-258` | As above | Move it to the tick loop with the others |
| The race engine and racer emulation tick on float `sim_dt` per frame | `race/flow.rs:33,64,70` | Finish times and the grid release depend on frame boundaries | Step the race on ticks (`blackbox-race` already takes `dt`; pass `STEP` per tick). Loading and results screens stay on frame time |
| Speedbreaker depletion uses real frame time; the speedbreaker ramp, cinematic moments and `game_speed` scale the tick rate | `drive/time_scale.rs:25-38,67-77`, `moment_world.rs:41-47`, `simulation.rs:58-71` | A session cannot slow time for one player | In a session, time scaling is off. Speedbreaker becomes a per-car effect or is disabled (features.md decides), and moments play on the client only. Single player keeps them |
| The car spawns after a 10 s wall-clock wait | `drive_input.rs:20` (field in `mod.rs`) | Wall clock in the spawn | Count ticks instead; the wait belongs to the client's loading state |
| The whole sim pauses until local streaming is ready; each AI car steps only if its collision is resident | `simulation.rs:50,59-60`, `ai/simulation.rs:89-93` | Local streaming state gates the shared world | The server keeps collision resident around every player ([integration](integration.md) section 2). Clients hold only their own car (`StreamingStall`) |
| Race opponent names are seeded from `SystemTime` | `race/start.rs:102,198` | Names only, but replays differ | Seed from the session seed at the caller |

## D3: hash order

Hash maps are handled well. Iterations that matter sort their ids first (`props/phases.rs:57-62`, `props.rs:233-238`,
`props/scenery/explosion.rs:85-86`); the rest are lookups, counts or `retain` calls with no cross-entry effects (`ai/cop_manager/roam.rs`,
`ai/pursuit_budget.rs`, `race/mods.rs`, `libs/blackbox-roads/src/path.rs`).

Three to check during the extraction:

| Where | Question | Planned fix |
|---|---|---|
| `props/phases.rs:23` (foldhash `FixedState`) | Each body is handled alone, then sorted. Looks safe | Keep; add a comment saying why |
| `props/streaming.rs:46` (`std` `HashMap`, random seed) | Safe only if a prop id belongs to one tile | Assert it in debug builds |
| `residency.rs:349-359` | Tile removal order follows a random hash; removals look independent | Sort the keys before removing |

## D4: random numbers

| Finding | Where | Effect | Planned fix |
|---|---|---|---|
| **One `SplitMix` stream for the whole traffic world** (seed `0x6E66_736D_7721`): navigation, spawns, model picks, stagger, roadblock lottery, helicopter, pursuit budget | `ai/mod.rs:241`, consumers in `ai/simulation.rs`, `ai/population.rs`, `ai/spawn.rs`, `ai/seek_spawn.rs`, `ai/search.rs`, `ai/pursuit.rs`, `ai/pursuit_budget.rs`, `ai/roadblock/*`, `ai/air.rs` | Any change in one consumer shifts every other decision | One stream per system, derived from a world seed by a fixed mix: `SplitMix(mix(world_seed, SYSTEM_ID))`. Systems: navigation, population, police, roadblocks, air. Single player passes today's seed as `world_seed` |
| **The police draw 4 numbers from that stream every frame**, also on 0-tick frames | `ai/police/world.rs:73-84`, `ai/police/mod_feed.rs:82` | The whole AI stream depends on the frame rate | Fixed by moving the police into the tick loop (D2) and giving them their own stream |
| Local fixed seeds | `ai/manager.rs:184` (`SplitMix(1)`), `libs/blackbox-roads/src/gps.rs:70` (`SplitMix(0)`) | Deterministic | Keep |
| View-only generators | `ai/sirens.rs`, `libs/blackbox-pursuit/src/speech`, particles, `career/pursuit.rs` case ids | No sim effect | Keep on the client |

## D5: entity order and identity

| Finding | Where | Effect | Planned fix |
|---|---|---|---|
| AI cars are stepped and collide in `Vec` order, which is spawn order; impulses are resolved one after another | `ai/mod.rs:140`, `ai/simulation.rs:61`, `ai/collide.rs:95-160` | Deterministic only if spawns happen in the same order | Give every entity a server id at spawn (ADR 0009 section 1). Keep the `Vec` sorted by id, or step in id order |
| AI keys come from a process-wide counter that model uploads also advance; the key is the tie-break of roadblock stealing | `keys.rs:21`, `ai/car.rs:178`, `ai/models.rs:314-358`, `ai/roadblock/steal.rs:31` | Ties depend on render-side work | Use the server entity id as the tie-break; keep render keys for the renderer |
| Which models exist depends on a loader thread's timing, and spawns pick among loaded models | `ai/scene.rs:64-77`, `ai/model_loader.rs:150`, `ai/population.rs:242-249` | The world depends on local load timing | The server picks models from the full catalogue; a client shows a stand-in until the model arrives |
| AI crash damage, despawning and prop lifetimes read the local camera, aspect ratio and camera mode | `ai/collide.rs:130`, `ai/car/hits.rs:28`, `ai/population.rs:257-275`, `props/lifetime.rs:145`, `ai/scene.rs:11-24` | A camera changes gameplay | Replace the camera with "the nearest player's view" (the set of player positions) on the server; single player passes its one view |
| Ids of props without a spawner follow tile arrival order | `props.rs:147-149`, `residency.rs:291` | Contact candidate order follows the id | Derive the id from the tile and the index inside it |
| Fixed phase order: props, player, traffic, player against traffic, finish steps | `simulation.rs:164-192` | Deterministic, but the player always resolves first | With several players: all player cars in id order, then AI, then pairs in id order (sync model section 4) |

## D6: threads

No `rayon` or parallel iterators in a step. `std::thread::scope` in `open.rs` runs at load time only. The model loader thread
reaches the sim only through D5's load timing.

## D7 and D8

They belong to the protocol (`sim_build` in `Auth`) and the client prediction (quantised inputs; `InputState` is `f32` today,
`libs/blackbox-vehicle/src/input/mod.rs:11`). Nothing to audit until `nfsmw-net` exists.

## Order of the fixes

1. Move police, sight, roadblocks, cop health and the race engine into the tick loop (D2). This also makes single-player AI
   independent of the frame rate, a fix worth having on its own. Extend the frame-rate trace test to cover traffic and police.
2. Split the AI random stream per system (D4).
3. Server entity ids and id-order stepping; the camera inputs become a set of views (D5).
4. Session-only rules: no time scaling, server-side residency, server-side model choice (D2, D5).
5. A headless test that runs 10 minutes of scripted free roam twice and compares state hashes ([integration](integration.md)
   section 4, step 6).
