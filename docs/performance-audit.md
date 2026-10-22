# Performance audit, 2026-10-17

The audit of the frame loop and the load paths, the fixes that followed, and what they measured. The work ran in two
rounds. The raw numbers and the setup are in [benchmarks/2026-10-17-perf-sweep.md](benchmarks/2026-10-17-perf-sweep.md).
How to profile is in [profiling.md](profiling.md) and [profiling-bevy.md](profiling-bevy.md).

## Context

The native renderer ([ADR 0004](decisions/0004-swappable-renderers.md)) was measured as CPU-bound.

The audit traced one native frame end to end and the load paths. It looked for work that repeated every frame, work
that repeated for each car or each file, and work that ran on the frame thread but did not need to.

The "before" numbers are from the sweep's starting point, with only the benchmark tests added. The changes are the
commits on `perf/sweep` after those tests.

## Round one: findings and fixes

| Finding | Where | Fix | Commit subject |
| --- | --- | --- | --- |
| The rear-view mirror rendered a full second frame each frame, with a blocking GPU readback (the poll wait in `capture.rs`). | `libs/blackbox-render/src/gpu/capture.rs`, `crates/nfsmw/src/app/mirror.rs` | `RenderBackend::request_view_capture`. The picture is reduced (no AO, contact shadows, sun cascades, SMAA or UI). It does not block. Targets and the readback buffer are cached. | `perf(blackbox-render): non-blocking reduced view capture for the rear-view mirror` |
| The mirror's tint and bezel clip were separate work. | `crates/nfsmw/src/app/mirror.rs` | Tint and bezel clip run in one pass, with a cached mask. | `perf(nfsmw): request the mirror as a view capture and clip it with a cached mask` |
| Per-size post images were rebuilt twice a frame, as the mirror and the main view alternate. | `libs/blackbox-render/src/gpu/sized.rs`, `gpu/post/` | A two-entry size cache for scratch, bloom and treatment images. | `perf(blackbox-render): keep post images for both view sizes drawn each frame` |
| The colour pass swept every instance 16 times. | `libs/blackbox-render/src/gpu/draw_list.rs`, `gpu/frame.rs` | Draw buckets are built once per pass. Buffers and textures are bound only on change. | `perf(blackbox-render): bucket the colour pass draws once per pass` |
| The depth, motion and shadow passes set the pipeline for every draw. | `gpu/surface/prepass.rs`, `gpu/temporal/motion.rs`, `gpu/shadows/mod.rs` | The pipeline, and the textures, are bound only when they change. | `perf(blackbox-render): bind pipelines and textures only when they change in the depth passes` |
| Alpha-tested pixels were fully shaded before the discard. | `libs/blackbox-render/src/shaders/scene.wgsl` | The alpha test runs first. | `perf(blackbox-render): discard cut-out pixels before lighting them` |
| The post passes created bind groups for every draw. | `libs/blackbox-gpu-passes/src/bind_cache.rs` and the passes that use it | A bounded bind group cache. | `perf(gpu-passes): reuse the bind groups of the full-screen passes` |
| Traffic was appended after the mesh sort. Every scenery instance did a prop lookup. | `crates/nfsmw/src/scenes/world/visibility.rs`, `frame.rs`, `shadow_feed.rs`, `reflections.rs`, `rear_view.rs` | One sort after all appends. Prop lookups are skipped for non-props (main view, rear view, reflections and the shadow feed). | `perf(nfsmw): group traffic with the scenery by mesh and skip prop lookups for scenery` |
| The same install files were read, decompressed and parsed many times. | `crates/nfsmw-data/src/files.rs`, `crates/nfsmw-data/src/shared.rs` | A session memo (`memo`, `shared_unwrapped`) and shared accessors for the shared-part files. | `perf(nfsmw-data): add a session cache for data derived from install files`; `perf(nfsmw): read the shared install files and the attributes database once per session` |
| `CarTables` was rebuilt for each car. The shared-part `GEOMETRY.BIN` was re-read for each car. | `crates/nfsmw-data/src/car/tables.rs`, `car/mod.rs`, `car/folders.rs` | Stock car tables are memoised. The shared-part solids come from a pool. Car bounds are reused. | `perf(nfsmw-data): reuse the car tables, shared parts and texture packs across car loads` |
| Texture packs were read whole, though a car needs a few textures. | `libs/blackbox-tpk/src/reader/mod.rs` | `read_texture_packs_filtered` reads only the wanted textures. | `perf(tpk): read only the wanted textures of a pack` |
| JDLZ copied back-references one byte at a time. | `libs/ea-compress/src/jdlz.rs` | Back-references are copied in chunks (`extend_from_within`). | `perf(ea-compress): copy JDLZ back-references in chunks` |
| Traffic and cop models were loaded synchronously in the middle of a drive. Cop performance was simulated for each spawn. | `crates/nfsmw/src/scenes/world/ai/model_loader.rs`, `ai/models.rs` | A worker-thread model loader. A per-type performance memo. | `perf(nfsmw): load traffic and cop models on a worker thread` |
| Per-tick allocations in props, AI, wall contacts, ray casts, cracked windows and minimap tile names. | `scenes/world/props.rs` and `props/`, `ai/`, `drive/walls.rs`, `drive/cracked.rs`, `hud/minimap.rs`, `libs/blackbox-collision/src/grid.rs` | Reused scratch buffers and caches. foldhash for the hot maps (props, mixmap, FEng). | `perf(nfsmw): track live prop lifetimes and reuse the prop collision buffers`; `perf(nfsmw): reuse the AI and wall-contact scratch buffers every step`; `perf(nfsmw): drop per-frame allocations for cracked windows and minimap tiles`; `perf(collision): reuse the ray-cast candidate buffer`; `perf(mixmap,feng): use foldhash for the hot lookup maps` |
| Streaming converted vertices on the main thread. Uploads were budgeted by tile count. | `crates/nfsmw/src/scenes/world/residency.rs`, `resident.rs`, `crates/nfsmw-data/src/world/section.rs` | Vertex conversion runs on the stream workers. Uploads have an 8 MiB per frame budget, and at least one section is uploaded per frame. | `perf(nfsmw): convert streamed vertices on the workers and budget uploads by bytes` |
| Window damage re-uploaded the whole car. | `crates/nfsmw/src/scenes/world/drive/rig.rs`, `scenes/car/materials.rs` | Only the affected solids are rebuilt. | `perf(nfsmw): rebuild only the cracked solids when a window breaks` |
| The world opened its data one load after another. | `crates/nfsmw/src/scenes/world/open.rs` | Independent loads run in parallel threads. | `perf(nfsmw): open the world's independent data in parallel` |
| The front-end package files were parsed up to 4 times. | `crates/nfsmw/src/ui/catalog.rs` | Each file is parsed once per session. | `perf(nfsmw): parse each front-end package file once per session` |
| `speed.exe` was SHA-256 hashed on every launch. | `libs/game-install/src/validate.rs`, `crates/nfsmw-data/src/game.rs` | Only `check-install` hashes the executable. | `perf(game-install): check the install without hashing the executable on every launch` |
| The menu backdrop was rebuilt when the next screen showed it again. | `crates/nfsmw/src/frontend/flow.rs` | The backdrop is kept. | `perf(nfsmw): keep the menu backdrop when the next screen shows it again` |
| No profiling profile, no game-side spans, and the system allocator. | `Cargo.toml` (`[profile.profiling]`, `[profile.dist]`), `crates/nfsmw/Cargo.toml`, `crates/nfsmw/src/app/render/mod.rs` and the frame phases | `[profile.profiling]` and `[profile.dist]`. `bevy_log` spans for the frame phases. mimalloc is the default allocator (3-5 % less CPU per frame). | `build: add profiling and dist profiles and an optional mimalloc allocator`; `feat(nfsmw): name the frame phases with profiler spans`; `perf(nfsmw): make mimalloc the default allocator` |

## Round one: measurements

Headless Vulkan, 1920x1080, release build. The figures are CPU time per frame in milliseconds, p50 and p95. They
include the world update, the mirror and the render submit. Setup and commands are in
[benchmarks/2026-10-17-perf-sweep.md](benchmarks/2026-10-17-perf-sweep.md).

| Scene and mirror | Before (p50 / p95) | After (p50 / p95) | After, with mimalloc (p50 / p95) |
| --- | --- | --- | --- |
| hill, mirror off | 0.465 / 0.691 | 0.322-0.467 / 0.471-0.681 | 0.312 / 0.515-0.543 |
| hill, mirror on | 1.500 / 1.657 | 0.706-0.711 / 0.767-0.829 | 0.684-0.687 / 0.726-0.755 |
| city, mirror off | 0.493 / 1.732 | 0.401-0.402 / 0.533-0.548 | 0.381-0.386 / 0.476-0.509 |
| city, mirror on | 1.804 / 1.992 | 0.929-0.942 / 1.045-1.097 | 0.887-0.894 / 0.937-0.999 |

The GPU p50 with the mirror off did not change: 0.16 ms on hill, 0.12 ms on city. Before this work, the GPU numbers
with the mirror on were not valid, because the mirror captures overwrote the timing slots. They now match the mirror-off
numbers.

Load timing (`load_timing`, release build, real install):

| Load step | Before | After |
| --- | --- | --- |
| GLOBALB unwrap | 5.5 ms | 2.5 ms |
| CarTables | 43.2 ms, then 10-11 ms | 7.4 ms, then 0.1 ms |
| 5 cars, total | 849.8 ms | 45.1 ms |
| 5 cars, per car | 155-191 ms | 3.9-5.1 ms after the first (the first takes 26.9 ms) |
| PhysicsData | 20.6 ms, then 13.8 ms | 0.2 ms, then 0.1 ms |
| Gameplay data | 32-49 ms | 20-21 ms |
| Global textures | 13.9 ms | 3.9 ms |

## Cache verdict

- **In-memory session cache: yes.** Entries are keyed by the resolved path, size and modification time. A mod overlay
  or an edited file therefore invalidates its entry.
- **Disk cache of decoded assets: no.** Textures are already BC-compressed with stored mips, so a disk cache would
  gain little for textures. Derived data must not be redistributed ([licensing.md](licensing.md)). Mod overlays make
  invalidation risky.
- **`wgpu::PipelineCache`: added in round two, Vulkan only.** In wgpu it works on Vulkan only. DX12 has a driver-side
  cache, and DX12 and GL are unchanged. See the round two table below.

## Round two: findings and fixes

Round two takes the items deferred at the end of round one. It also fixes one regression from round one: a cop model
that failed to load stayed in the running pursuit. The rows are the changes in the tree. The "Where" column names the
crate or area, not every file.

| Finding | Where | Fix |
| --- | --- | --- |
| The post chain ran a final resolve copy after the last pass. | Post chain (`blackbox-render`, `blackbox-gpu-passes`) | The last post pass writes the output. A resolve runs only when the result must convert, scale or follow FSR 3. FXAA alone went from 2 to 1 full-screen passes. FSR 1 went from 3 to 2. |
| FSR 3 intermediate images were fp32. | `fsr3-wgpu` | Intermediates use fp16 (R16Float and Rg16Float storage) where the adapter supports it. fp32 is the fallback. Depth stays fp32. |
| Opaque lit draws did not reuse the depth that the prepass had already written. | `blackbox-render` depth passes | When the prepass depth is reused, these draws use an Equal depth test with depth writes off. |
| The UI pass rebuilt and uploaded its vertices every frame. The mirror did a UI swap even where its view captures never draw UI. | `blackbox-render` UI pass; the mirror in `nfsmw` | The UI pass reuses its staging buffers and skips the upload when the layer is unchanged. The mirror skips its UI swap when the backend reports `Capabilities::view_capture_without_ui`. |
| Vulkan compiled every pipeline from scratch on each start. | Vulkan: `blackbox-render`, `blackbox-gpu-passes`, `fsr3-wgpu` | `wgpu::PipelineCache`, one file per adapter, under `<local data>/nfsmw/cache/pipelines`. It is saved after frame 300 and when the renderer is dropped. One scoped unsafe call reads back only bytes the renderer wrote. Fallback is on: a driver mismatch or a damaged file starts empty. DX12 and GL are unchanged. |
| CPU decode of non-BC textures ran on the frame thread. | Stream loading in `nfsmw` | Decode runs on the stream loader threads. |
| Off-frustum shadow-only casters were added to the main and rear lists. | Native renderer scene lists in `nfsmw` | They are not added when the backend takes a separate caster list (`Capabilities::separate_shadow_casters`, native renderer). |
| Parsed MixMaps were not kept. | `nfsmw-data`, `blackbox-mixmap` | The parsed MixMap is memoised per session. |
| Traffic and cop AI copied the other cars for each car. | AI in `nfsmw`; `blackbox-roads` | Other cars are passed as iterators to `lane_clear`, `earliest_oncoming` and the occlusion trail. There are no per-car copies. |
| GPU resources of a replaced scene were never freed (a leak). The menu scene was reloaded after a race. | Scene lifecycle: menu, world, asset viewer | Resources are freed when a scene is replaced. The menu scene is parked during free roam and restored without a reload. |
| A cop model that failed to load stayed in the running pursuit. This was a round one regression from async model loading. | Pursuit in `nfsmw` | A failed model is dropped from the running pursuit. |

## Round two: measurements

The round two numbers and the setup are in
[benchmarks/2026-10-17-perf-sweep.md](benchmarks/2026-10-17-perf-sweep.md#round-two). They come from one session that
ran the round one tip and round two side by side. With the mirror on, the CPU p50 ranges are lower on both scenes:
hill 0.739-0.885 ms to 0.707-0.713 ms, and city 1.003-1.126 ms to 0.963-1.089 ms. The city ranges overlap, so the
gain there is small. The machine was noisier than in round one, so compare within one session. Load timing did not
change.

## Remaining follow-ups

- Support SUVs (`copsuv`, `copsuvl`, `copcross`) are never requested by the pursuit support path. This was already the
  case before the sweep.
- Bevy's pipelines are not touched by the pipeline cache.
- Streamed sections requested before the first residency update still decode on the main thread on non-BC GPUs.

## Round three: stutter first (2026-10-22)

The first two rounds were judged by a benchmark that draws a parked car with the renderer's defaults. The game's own
trace showed something else: a p50 of 3 ms but a p90 of 50 ms, with stalls of up to 4 s on the frame thread. Round
three measures the whole app (`--bench-seconds`, now hidden, with hitch counts) and starts with the stalls. Numbers:
[benchmarks/2026-10-22-perf-round-three.md](benchmarks/2026-10-22-perf-round-three.md).

| Finding | Fix |
| --- | --- |
| The release profile had no LTO and 16 codegen units; dev builds compiled our decoders, physics and renderer at level 1 with Cranelift. | Thin LTO and one codegen unit for release; the hot crates at level 3 with LLVM in dev. |
| The save screens read the career tables on the frame thread, and the car lot ran the 5,400-step performance simulation for each car, again for Quick Play and at race starts. | The tables load on a thread; car performance, the gameplay data and the career tables are memoised per session; the lot is measured in parallel. |
| Movies were read whole and their sound decoded before the first frame, on the frame thread; each frame decoded and copied the picture there. | Video and sound workers; buffered reads; a bounded queue of recycled frame buffers. |
| Bank sounds and NIS scenes decoded or loaded on first use, on the frame thread. | An audio decode worker fed as soon as the car is known; NIS bundles, cameras and takes load on threads. |
| The menu backdrop read and uploaded its car and room inside `init`. | A worker reads them; the frame thread uploads the room, then the car. |
| The streamer started after the whole world load, no tile was asked for before the shared sets arrived, and the shared sets uploaded in one frame. | The loading thread starts the streamer and asks for the shared sets and the start tiles together; the shared upload is spread over hidden frames. |
| The upload budget counted BC textures as RGBA, dropped requests were still read, and placements were worked out on the frame thread. | Real byte counts, cancel flags, placement prepared on the stream workers, bookkeeping only on change. |
| Pipelines were built on first use inside a frame (264 ms). | `RenderBackend::prewarm` builds what the current settings need ahead of the frame. |
| Parallel opens could build one memo entry twice; surface capabilities were queried twice a second. | A `OnceLock` per memo key; capabilities on change. |

Next in this round: the steady frame (reflection views, HUD composition, draw submission), then the GPU passes and the
load path.

## How to measure

Set `NFSMW_GAME_DIR` to the install first. Then run:

```sh
cargo test --release -p nfsmw --bin nfsmw world_frame_costs -- --ignored --nocapture
cargo test --release -p nfsmw-data --test real_install load_timing -- --ignored --nocapture
```

For timelines, the frame phases are named spans. Open them in Tracy as described in [profiling-bevy.md](profiling-bevy.md).
Build profiles (`release`, `profiling`, `dist`) and the allocator switch are in [profiling.md](profiling.md).
