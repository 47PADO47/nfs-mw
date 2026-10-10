# Benchmark: Bevy renderer vs native, 2026-10-10

Frame-time comparison of the two renderers, and a Tracy breakdown of where the Bevy renderer's frame goes. How to
run and capture is in [profiling-bevy.md](../profiling-bevy.md); the background is in
[bevy-backend.md](../bevy-backend.md).

## Setup

- Machine: RTX 4070 SUPER, Windows 10, Vulkan.
- Scene: `view-world --drive --no-vsync`, borderless 1920x1080, driving at once. Car BMW M3 GTR, world city.
- Native: `--renderer blackbox`. Bevy: `--renderer bevy`. Both from `cargo build --release -p nfsmw --features
  renderer-bevy` (no `trace` feature) for the benchmark.
- Benchmark: `--bench-seconds 20` (frame times after the scene loads).
- Trace: `cargo build --release -p nfsmw --features trace`, captured 30 s with Tracy 0.14.1 (wire protocol 82).

## Frame times (no tracing)

| Renderer | frames | fps | p50 | p95 | p99 | worst |
| --- | --- | --- | --- | --- | --- | --- |
| blackbox (native) | 20,721 | 1036 | 0.92 ms | 1.11 ms | 1.21 ms | 1.85 ms |
| bevy | 6,214 | 311 | 3.18 ms | 3.40 ms | 3.72 ms | 7.48 ms |

The Bevy renderer is 3.3x slower at the median, a gap of about 2.3 ms per frame.

## Where the Bevy frame goes (traced)

Traced run with `bevy_ecs/debug` on, so system names show: 6,015 render frames in 30 s. Inclusive time per frame, so
the numbers overlap and do not sum to the frame:

| Span | ms per frame | Notes |
| --- | --- | --- |
| `RenderApp` (render thread) | 4.41 | Pipelined: runs alongside the main thread. |
| `RenderGraph` | 1.73 | |
| `finish_command_buffers` | 0.94 | `CommandEncoder::finish` is 16 calls per frame, 42 µs each. |
| `prepare_preprocess_bind_groups` | 0.63 | Largest single render system. Most of its time is `create_bind_group`. |
| `prepare_clusters_for_gpu_clustering` | 0.16 | |
| `nfsmw::hud::plugin::present` | 0.08 | The HUD. |
| `nfsmw::devtools::run_ui` | 0.06 | |
| `blackbox_bevy_render::apply` | 0.02 | The blackbox bridge: small. |

Per-frame call counts, from the trace: `Queue::write_buffer` about 574, `StagingBuffer::new` about 585,
`Device::create_bind_group` about 398, `Queue::submit` 2 per frame.

## Reading

- The gap to native is per-frame resource churn in Bevy's pipeline: bind groups, buffer uploads and the command
  encoding they force. Bevy's own GPU preprocessing (`prepare_preprocess_bind_groups`) is the largest single
  contributor.
- The blackbox bridge and the HUD are small (under 0.1 ms each). The cost is Bevy's, not the game's.
- Tracy adds overhead to every span, so the traced numbers are higher than the untraced ones. Use the frame-time
  table for absolute fps and the trace for proportions.

## Limits

- One run per renderer, one scene, one machine. The fps figures will move between runs, so treat the ratio as the
  finding, not the exact numbers.
- Names are unique only within a run; each call count is per render frame.

## Experiment: CPU instance buffer builder (rejected)

Bevy's `PbrPlugin` defaults `use_gpu_instance_buffer_builder` to `true`, which is the path that runs
`prepare_preprocess_bind_groups`. Setting it to `false` in `libs/blackbox-bevy-render/src/plugin.rs` gave
810 fps (p50 1.20 ms), but the picture was wrong: trees, buildings and the road were missing, and the sky and ground
rendered as flat grey-blue (`target/bench/shot-nfsmw-bevy-cpuinst.png` against `shot-nfsmw-bevy.png`, mean
difference 89 of 255). The speed came from drawing less of the scene, so it is not a fix. The change was
reverted.

Open question: whether this is a bug in our material or instancing path that the CPU path exposes, or a limit of
the CPU path itself. Until that is answered, the GPU builder stays on.

### Static investigation (2026-10-10): inconclusive

A read of `bevy_pbr`, `bevy_camera` and `bevy_render` 0.20.0 did **not** find the cause: nothing the CPU builder
requires is missing from our entities, and no Bevy limitation was confirmed. Three hypotheses were ruled out:

- **`NoFrustumCulling`** is not a CPU/GPU difference. Both builders run `check_visibility_cpu_culling`
  (`bevy_camera/src/visibility/mod.rs`) in `CheckVisibility`; `NoFrustumCulling` skips only the frustum test
  (~line 823) and the entity is still marked visible (~846).
- **Missing `Transform`** is not required. The CPU extractor `extract_meshes_for_cpu_building`
  (`bevy_pbr/src/render/mesh.rs` ~1811-1921) reads `GlobalTransform` via `transform.affine()` (~1886) — the same
  source as the GPU builder (~2223) — and never reads `Transform`.
- **The custom vertex attribute / `specialize`** has no builder-specific branch; the draw path is identical for
  both (`mesh.rs` ~4708-4737).

Two CPU-only divergences remain as unconfirmed suspects:

- The sorted batcher (`bevy_render/src/batching/mod.rs` ~219-244) leaves `batch_range` at the queue default
  `0..1` for a failed item (`bevy_pbr/src/material.rs` ~1342, ~1450); `DrawMesh` then skips it.
- The CPU path falls back to `MaterialBindingId::default()` (`mesh.rs` ~1873-1875) where the GPU path skips the
  entity (~1473). If a queued item has no `RenderMaterialBindings` entry this would draw with a default/empty
  material — a plausible explanation for the *flat grey-blue* colour.

### Deferred: how to debug it (needs a GPU)

This can't be settled by static reading; it needs runtime instrumentation on the RTX host (focus-safe, offscreen).
On a throwaway branch with `use_gpu_instance_buffer_builder: false`:

1. In `apply`, count `RenderMeshInstancesCpu` entries against the instance-pool size.
2. Count queued phase items per opaque bin and per transparent batch after `batch_and_prepare_*`.
3. Count `DrawMesh` `Skip` results by reason (empty `batch_range`, missing vertex slice, missing material binding).
4. Capture one RenderDoc frame: are the tree/building/road draws issued at all, and with which instance indices
   and `MeshUniform` values?

Verify visually with offscreen `--screenshot` + `cargo xtask img-diff` against the GPU-path baseline (reference
mean ~1.6 at `2152,1399`, per [bevy-backend.md](../bevy-backend.md) "The real city"). Flipping back to the GPU
builder is a one-line revert. Potential upside if it is our bug: ~311 → ~810 fps.

## Experiment: no clustering on the scene camera (rejected)

`ClusterConfig::None` on the scene camera crashed the app at startup (a system failed on a missing resource, then
exit code 1). `ClusterConfig::Single` runs but is no faster: 311 fps, p50 3.18 ms, the same as before. Clustering is
not the cost, so the change was reverted.

## Dev build: workspace crates at opt-level 1 (kept)

Dev builds of the Bevy renderer ran at 176 fps (p50 5.41 ms). With `[profile.dev] opt-level = 1` for the
workspace's own crates, the same run reaches 272 fps (p50 3.64 ms). Dependencies were already at opt-level 2.
Committed as `perf(build)`.

## Scene lighting

The scene creates no Bevy shadow-casting lights; lighting goes through `blackbox_gfx` lights uniformly into our
material. So shadow views do not explain the per-frame bind groups. The `Camera 0` main view is the one view
measured.

## Next

Three options, to be decided:

1. **Find why the CPU builder breaks the world.** If it is our bug, the CPU path may be far faster than the 311
   fps baseline, and it is Bevy's supported path. This is measured by the same benchmark plus the screenshot diff.
2. **Cache the bind groups in `prepare_preprocess_bind_groups`.** Bevy 0.20 rebuilds them for every view and phase
   on every frame. Caching needs a vendored patch of `bevy_pbr`, which is a maintenance cost.
3. **Reduce per-frame writes** in our own systems, so `Queue::write_buffer` and `StagingBuffer::new` (about 575 and
   585 calls per frame) shrink. This does not touch Bevy's bind-group rebuilds.

   **Low value (2026-10-10 static check).** Our own per-frame `write_buffer` calls are few — the effects globals
   plus one `batch.upload` per effect batch (`systems/effects.rs`), the UI's three writes
   (`blackbox-gpu-passes/src/ui.rs`), and the soft-particle params: on the order of a dozen per frame, not ~575.
   The ~575 writes and ~398 `create_bind_group` calls are Bevy's `prepare_preprocess_bind_groups` rebuilding
   per-batch resources every frame, scaling with the ~400 draw batches. So the write count is cut either by
   option 1/2 above or by **reducing the batch count** (fewer distinct materials — e.g. texture atlasing — or a
   custom instanced pipeline that bypasses `gpu_preprocess`), not by trimming our own uploads.
