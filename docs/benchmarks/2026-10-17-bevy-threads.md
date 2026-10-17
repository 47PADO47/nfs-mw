# Benchmark: Bevy threading and pipelined rendering, 2026-10-17

Question: does Bevy's `multi_threaded` (ECS executor, task pools, parallel `CommandEncoder::finish`) or
`PipelinedRenderingPlugin` lower wall-clock frame time on the Bevy renderer? The shipped build has neither
(see [bevy-backend.md](../bevy-backend.md) "Threads", which rejected `multi_threaded` on total CPU time).

## Setup

- Machine: RTX 4070 SUPER, Vulkan, Windows. Release builds of `nfsmw`.
- Command: `view-world --renderer bevy --drive --no-vsync --bench-seconds 20`, run once per variant back to back.
- Variants:
  - **base**: the current build.
  - **mt**: `bevy_ecs/multi_threaded` and `bevy_render/multi_threaded`.
  - **pipe**: mt plus `PipelinedRenderingPlugin`, added right after `RenderPlugin`.
- Two interleaved rounds (base, mt, pipe, then pipe, mt, base). Build processes from other sessions were running
  during some runs; the count is logged per run.
- The bench window is a visible window, not `--screenshot`. `--bench-seconds` conflicts with `--screenshot`, so the
  run could not be offscreen. It took focus briefly on each run.
- Added `NFSMW_BENCH_SERIES=<file>` to `bench` (kept in this change): writes every frame's time, warm-up included,
  so first-drive hitches can be seen. The percentiles still exclude the first 2 s.

## Results (frame time, ms)

| Variant | Round | fps | p50 | p95 | p99 | worst |
| --- | --- | --- | --- | --- | --- | --- |
| base | 1 | 31.2 | 30.69 | 44.25 | 53.71 | 81.55 |
| base | 2 | 36.3 | 26.02 | 35.63 | 41.90 | 82.37 |
| mt | 1 | 32.1 | 28.03 | 46.30 | 70.80 | 169.00 |
| mt | 2 | 38.2 | 24.37 | 36.01 | 47.70 | 115.00 |
| pipe | 1 | 38.9 | 23.24 | 40.72 | 58.42 | 93.39 |
| pipe | 2 | 41.8 | 22.74 | 33.38 | 44.59 | 87.92 |

Hitches from the frame series:

- Frames over 100 ms after the first 3 s: base 0 and 0, mt 3 and 1, pipe 0 and 0.
- Frames over 100 ms in the first 10 s (the first-drive compile window): base 3, mt 11, pipe 6 (round 1 series).
- Worst frame in the first 3 s: 520 to 715 ms in every variant. These are first-use pipeline and shader costs.

## Reading

- **mt alone does not win.** The median is slightly better in both rounds, but p99 and worst are worse in round 1
  and the hitch count is higher. Run-to-run spread on base is about 5 fps, so the median difference is inside noise.
- **pipe is about 15% faster at the median** (p50 23 ms against 26 to 31 ms) in both rounds. This is a real
  signal but not a clean one: it rests on two rounds, and it is only valid if the races below are fixed.
- **Absolute numbers do not match the 2026-10-10 benchmark** (311 fps there, 31 to 42 fps here). The base build
  here is 36 fps at best. The window here logged as 1280x720 borderless, not 1920x1080, and `HEAD` has changed since
  then. Only the relative comparison above is meaningful. The discrepancy is not explained by this run.

## Races under pipelined rendering

`PipelinedRenderingPlugin` runs the render world on its own thread, concurrent with the next main-world frame. The
render world reads the shared `BlackboxBridge` directly in its render-schedule systems, not from an extracted
snapshot. Found by reading the code:

1. **Settings skew.** `fsr3_pass` (`fsr3/node.rs`), `fsr1_pass` (`post/fsr1.rs`) and `scale.rs` read
   `shared.settings` (render scale, upscaler, sharpness) at render time. The main world can change those while
   frame N renders. Texture sizes come from the camera extracted at the sync point. A scale change mid-drive can
   mismatch the sizes the pass uses, and that can produce a wgpu validation error or a wrong-size upscale for a frame.
2. **Effect and UI layer skew.** `effects_main_pass` clones `shared.effects`, and `ui_pass` takes `shared.ui_ops`
   then clones `shared.ui_layer`. Both are latest-value reads, so frame N can draw layer N+1. A UI texture the layer
   references may still have its `Update` op undrained, which shows as a one-frame stale or missing texture.
3. **Ordering is preserved for ops.** `ui_ops` is taken under the lock in FIFO order, so no op is lost or reordered.
   The race is between the ops and the layer, not within the ops.
4. **Not a race.** `extract_world` (the `ExtractSchedule` snapshot) runs on the main thread while the render thread
   is parked, so it is exclusive with the previous render frame.

The fix is to snapshot everything the render systems read (settings, effects layer, focus, UI ops and layer) in
`extract_world`, the same way effects already are, and to drain `ui_ops` at extract. That is a design change in
`libs/blackbox-bevy-render` and is not done here. Pipelined rendering should not ship before it.

## Recommendation

1. **Do not enable `multi_threaded` alone.** It does not improve frame time on this machine.
2. **Pipelined rendering is worth doing, but not yet.** The gain is about 15% at the median, but the render-side
   bridge reads are racy. Snapshot them in extract first, then rerun pipe against base with at least three rounds on
   a quiet machine. Record first-drive hitches from `NFSMW_BENCH_SERIES` as well.
3. **Cargo change: reverted.** The `multi-threaded` and `pipelined-rendering` features and the plugin line were
   removed from the tree after measurement, because neither should ship as-is. To reproduce, add
   `multi-threaded = ["bevy_ecs/multi_threaded", "bevy_render/multi_threaded"]` to `libs/blackbox-bevy-render`, and
   `app.add_plugins(bevy_render::pipelined_rendering::PipelinedRenderingPlugin)` after `RenderPlugin` in
   `plugin.rs`.

## Not done

- The machine was shared with other sessions' builds, so some runs were noisy. The pipe round 2 run saw 2 to 3
  build processes.
- Only one scene and one machine. Only two rounds per variant.
- Hitch analysis covers the first 10 s and the steady state only, not a full first drive through every material.
- Commit status: the `bench.rs` series dump is uncommitted in the worktree, along with this doc.
