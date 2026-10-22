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

## After P1 (stutter)

P1 moves blocking loads off the frame thread: the career tables and the car performance simulation, movies, bank
sounds and NIS scenes, the menu backdrop, the world's streaming start and shared-set upload, section placement, and
the first use of each pipeline (prewarm). Same machine and command, "user" settings, 20 s runs. The P0 and P1 builds
alternated in one session. Another workload shared the GPU during this session, so the p50 is about 7.6 ms on both
builds, against 4.3 ms in the morning; only the comparison within the session counts.

| Build | p50 | p99 | Worst | Hitches measured | Worst frame before measuring | Ready after |
| --- | --- | --- | --- | --- | --- | --- |
| P0 | 7.31, 7.58 | 11.70, 11.67 | 142.5, 18.7 | 2, 0 | 3,275 ms, 860 ms | 3.96 s, 0.94 s |
| P1 | 7.70, 7.62 | 12.36, 13.10 | 59.2, 55.2 | 1, 2 | 1,053 ms, 764 ms | 1.28 s, 0.98 s |

What it says:

- **The steady frame is unchanged**, as expected: P1 moves loads, not per-frame work.
- **The longest loading stall is shorter.** The worst frame before measuring was 0.76-1.05 s with P1, against
  0.86-3.3 s with P0. A trace of the P1 build (`--exec "trace start 30"`) has its only frames over 25 ms in the
  first 0.2 s, while the world is still hidden: 51 ms of uploads, 17 ms of audio, 8 ms of HUD presentation. None
  while driving.
- **The driving hitches are within the noise** of this session: 0-2 per 20 s on both builds.
- **Not measured here:** the loads P1 moves that this drive never reaches (the save screens' career load, movies,
  NIS scenes, the menu backdrop). They are covered by the unit tests and by the trace scopes added in P0.

The steady frame in the P1 trace (mean 7.56 ms with the shared GPU): 3.3 ms waiting on present, 2.3 ms scene update of
which 1.4 ms are the reflection views, 1.1 ms render recording plus 0.7 ms in wgpu's encoder finish, 0.5 ms HUD (0.45 ms
of it theme composition). Those are the targets of P2.
