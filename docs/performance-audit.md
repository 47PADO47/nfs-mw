# Performance audit, 2026-10-17

The audit of the frame loop and the load paths, the fixes that followed, and what they measured. The raw numbers and
the setup are in [benchmarks/2026-10-17-perf-sweep.md](benchmarks/2026-10-17-perf-sweep.md). How to profile is in
[profiling.md](profiling.md) and [profiling-bevy.md](profiling-bevy.md).

## Context

The native renderer ([ADR 0004](decisions/0004-swappable-renderers.md)) was measured as CPU-bound.

The audit traced one native frame end to end and the load paths. It looked for work that repeated every frame, work
that repeated for each car or each file, and work that ran on the frame thread but did not need to.

The base for the "before" numbers is commit `c3d1a951`. The changes are the commits after it on `perf/sweep`.

## Findings and fixes

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

## Measurements

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
- **`wgpu::PipelineCache`: deferred.** In wgpu it works on Vulkan only. DX12 has a driver-side cache.

## Deferred and follow-ups

- Drop the final resolve copy in the post chain.
- FSR 3 intermediates in fp16.
- Use an `Equal` depth test when the prepass depth is reused.
- Reuse UI vertices.
- Use `wgpu::PipelineCache` on Vulkan.
- Decode CPU textures on the stream workers when BC is not supported.
- Off-frustum shadow-only casters are still drawn by the native renderer. Bevy needs them. This needs a backend
  capability flag.
- Keep the menu scene alive during a race. This needs a `Scene` hook. Menu scenes also never free their GPU resources
  when they are replaced. That is a leak to fix.
- Memoise the parsed MixMaps.
- Traffic think still copies the other bodies for each car.

## How to measure

Set `NFSMW_GAME_DIR` to the install first. Then run:

```sh
cargo test --release -p nfsmw --bin nfsmw world_frame_costs -- --ignored --nocapture
cargo test --release -p nfsmw-data --test real_install load_timing -- --ignored --nocapture
```

For timelines, the frame phases are named spans. Open them in Tracy as described in [profiling-bevy.md](profiling-bevy.md).
Build profiles (`release`, `profiling`, `dist`) and the allocator switch are in [profiling.md](profiling.md).
