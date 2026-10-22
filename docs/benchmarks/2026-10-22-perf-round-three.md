# Benchmark: performance round three, 2026-10-22

Baseline for the third performance round. The plan behind it: stutter first, then the steady frame cost. The earlier
rounds are in [2026-10-17-perf-sweep.md](2026-10-17-perf-sweep.md) and [performance-audit.md](../performance-audit.md).

## Why a new benchmark

The earlier `world_frame_costs` test draws a parked car with the renderer's defaults: no reflections, shadows, upscaler
or traffic, no HUD, no app schedule and no present. It reads about 1 ms of CPU a frame, while the only in-game trace
(a debug build, 2026-10-20) showed a p90 of 50 ms. This round measures the whole app instead: `--bench-seconds`, which
now runs in a hidden window off the desktop, reports hitches and the time until the scene is ready, and reports CPU
time and memory on Windows too.

## Setup

- Machine: AMD Ryzen 7 9800X3D, NVIDIA GeForce RTX 4070 SUPER, Windows 10, Vulkan.
- Build: `cargo build --release -p nfsmw` at `perf3/p0-bench` (thin LTO, one codegen unit).
- Scene: the city at 2150,1399, driving `BMWM3GTR` with throttle and a slight steer for the whole run, so the car
  streams new tiles and meets traffic. HUD on, 1920x1080, vsync off, unlocked.
- "user": the player's own config (reflections High, shadows Medium, motion blur, FSR 3 at render scale 200, 16x
  anisotropic, traffic 20).
- "low": an empty config folder and `--graphics-preset low`.

```sh
NFSMW_GAME_DIR=... NFSMW_BENCH_SERIES=series.csv nfsmw view-world --drive --at 2150,1399 \
  --drive-script "40:throttle=1,steer=0.12" --bench-seconds 25 --no-vsync --max-fps unlocked --hud
APPDATA=<empty folder> ... the same ... --graphics-preset low
```

## Baseline

Frame time in milliseconds over 25 s of measuring, after 2 s of warm-up. Hitches are frames over 25 ms.

| Preset | p50 | p95 | p99 | Worst | Mean | Hitches measured | Hitches before measuring (worst) | Ready after | CPU ms/frame | Main thread busy | GPU busy |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| user | 4.30 | 7.00 | 13.33 | 39.99 | 4.77 | 7 | 4 (3,502 ms) | 4.33 s | 4.43 | 89 % | 26 % |
| low | 4.11 | 4.98 | 6.63 | 14.78 | 4.27 | 0 | 2 (933 ms) | 1.08 s | 4.23 | 99.9 % | 33 % |

What it says:

- **The frame is CPU-bound in both presets.** The main thread is busy almost all the time and the GPU a third of it.
  The p50 hardly moves between the presets, so about 4 ms of each frame is work that the graphics settings do not
  change: simulation, traffic, HUD, the app schedule and the draw submission.
- **The user's settings add the tail.** Their p99 is twice the low preset's, and they hitch while driving (reflections
  on High re-render all six cube faces every frame; streaming uploads).
- **Loading stalls.** Before measuring, one frame took 3.5 s with the user's settings and 0.9 s with the low preset:
  the world's build step and the first uploads run on the frame thread.
