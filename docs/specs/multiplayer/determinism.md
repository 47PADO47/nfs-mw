# Determinism audit of the world sim (D1 to D8)

Status: written 2026-10-21 by reading the code. **Steps 1 and 2 of the fix order are applied** (police, sight, roadblocks,
cop health and a running race tick with the physics; one random stream per traffic system); the rest is planned. The requirements D1
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
| ~~One `SplitMix` stream for the whole traffic world~~ | `ai/streams.rs` | | **Done:** `Streams` holds navigation, population, police, roadblock and air streams, each the world seed mixed with a system id by the SplitMix64 finaliser. The default world seed is the old shared seed `0x6E66_736D_7721`. A session will pass its own seed |
| ~~The police draw 4 numbers from that stream every frame~~ | `ai/police/world.rs`, `ai/police/mod_feed.rs` | | **Done:** they draw once per tick, from the police stream |
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

1. **Done.** Move police, sight, roadblocks, cop health and the race engine into the tick loop (D2). This also makes
   single-player AI independent of the frame rate. The chase frame-rate test covers traffic and police.
2. **Done.** Split the AI random stream per system (D4).
3. Server entity ids and id-order stepping; the camera inputs become a set of views (D5).
4. Session-only rules: no time scaling, server-side residency, server-side model choice (D2, D5).
5. A headless test that runs 10 minutes of scripted free roam twice and compares state hashes ([integration](integration.md)
   section 4, step 6).
