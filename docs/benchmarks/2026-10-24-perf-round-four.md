# Benchmark: performance round four, 2026-10-24

Measurements for the fourth performance round ([performance-audit.md](../performance-audit.md#round-four-rendering-time-2026-10-24)).
Earlier rounds: [2026-10-22-perf-round-three.md](2026-10-22-perf-round-three.md).

## Setup

- Machine: AMD Ryzen 7 9800X3D, NVIDIA GeForce RTX 4070 SUPER, Windows 10, Vulkan.
- Builds: `cargo build --release -p nfsmw` at `origin/main` 288b0a79 ("main") and at the top of the round-four stack
  ("perf4"), alternated in one session.
- Drive: the city at 2150,1399, `BMWM3GTR`, throttle and a slight steer, HUD on, vsync off, unlocked, 20 s measured.
  "user" is the player's config, "low" an empty config folder with `--graphics-preset low`.
- Menu: `play --skip-boot`, the main menu over the backdrop. A benchmark run never caps the menu or reuses its still
  frames, so this row shows the frame without them.
- Every run is hidden and `--muted` (the audio keeps running, silently).

```sh
NFSMW_GAME_DIR=... nfsmw view-world --drive --at 2150,1399 --drive-script "40:throttle=1,steer=0.12" \
  --bench-seconds 20 --no-vsync --max-fps unlocked --hud --muted
nfsmw play --skip-boot --bench-seconds 20 --no-vsync --max-fps unlocked --muted
```

## Shared machine, 2026-10-24 evening

A game ran on the same GPU during the whole session. The GPU base render of one binary moved between 2.6 and 24 ms
from run to run, so the frame times below say little; the CPU time per frame, which leaves out waiting, is the most
comparable column.

| Run | Build | p50 ms | p99 ms | CPU ms/frame | GPU base render ms |
| --- | --- | --- | --- | --- | --- |
| user drive 1 | main | 11.97 | 35.82 | 4.32 | 13.97 |
| user drive 1 | perf4 | 6.52 | 30.77 | 3.69 | 2.63 |
| user drive 2 | main | 6.69 | 10.68 | 2.89 | 2.70 |
| user drive 2 | perf4 | 6.30 | 16.93 | 3.35 | 2.65 |
| low drive 1 | main | 6.27 | 39.31 | 3.12 | 8.18 |
| low drive 1 | perf4 | 35.46 | 203.27 | 10.38 | 24.38 |
| low drive 2 | main | 14.57 | 25.09 | 3.12 | 6.91 |
| low drive 2 | perf4 | 14.56 | 46.99 | 3.53 | 14.81 |
| user menu 1 | main | 2.20 | 4.86 | 0.72 | 1.12 |
| user menu 1 | perf4 | 2.73 | 16.54 | 1.04 | 7.81 |
| user menu 2 | main | 8.32 | 33.46 | 0.77 | 3.12 |
| user menu 2 | perf4 | 7.83 | 16.75 | 0.33 | 5.27 |

What it says:

- **Nothing conclusive.** The two runs of one binary differ more than the binaries do; "low drive 1" on perf4 ran
  while the other workload took the GPU (24 ms of base render for a frame that takes 7-8 ms on its own).
- **CPU time counts every thread.** Round four moves work onto threads (the cube faces, AI integration on the
  compute pool), which can raise the process's CPU time per frame while it shortens the frame thread's part. On the
  idle rerun, read it next to the frame time and the main thread's busy share.
- **Not measured by these runs:** the menu's still frames and frame cap (off in benchmark runs), and the mirror's post
  scratch rebuild (the mirror shows only with `rear_view_mirror` on, and the rebuild needs FSR 3 or DLSS too).

## Quieter machine, later the same evening

The same binaries and commands, the game idle in its menu. A mirror round adds `NFSMW_REAR_VIEW_MIRROR=on`.

| Run | main p50 / p99 ms | perf4 p50 / p99 ms | main CPU ms/frame | perf4 CPU ms/frame |
| --- | --- | --- | --- | --- |
| user drive 1 | 6.44 / 10.43 | 6.34 / 10.13 | 2.79 | 3.21 |
| user drive 2 | 6.29 / 9.90 | 6.20 / 9.83 | 2.91 | 3.26 |
| low drive 1 | 6.19 / 9.93 | 6.19 / 9.85 | 2.90 | 3.06 |
| low drive 2 | 6.19 / 9.87 | 6.22 / 9.82 | 2.87 | 3.17 |
| user menu 1 | 2.26 / 4.62 | 2.20 / 4.59 | 0.70 | 0.66 |
| user menu 2 | 2.27 / 4.65 | 2.22 / 4.59 | 0.74 | 0.62 |
| user mirror 1 | 6.86 / 10.46 | 6.75 / 9.87 | 3.54 | 3.75 |
| user mirror 2 | 6.83 / 10.07 | 6.79 / 10.27 | 3.62 | 3.71 |

What it says:

- **The display holds the drive back, not the frame's work.** Both builds sit at a p50 of about 6.2 ms (about
  160 Hz) on the user's settings and on the low preset alike. Meanwhile the GPU base render is about 3 ms and the main
  thread is busy 30-50 % of the time, so the frame waits on present. The frame time therefore cannot show the round's
  CPU and GPU savings here: perf4 is 0.0-0.1 ms lower at p50 and alike at p99.
- **The menu is a little faster** (2.20-2.22 against 2.26-2.27 ms) and spends less CPU (0.62-0.66 against 0.70-0.74
  ms). This is without its still-frame reuse, which benchmark runs leave off.
- **The process spends 0.1-0.4 ms more CPU per frame on perf4 while driving.** The work moved to threads (cube faces,
  AI integration on the compute pool) costs wake-ups across cores. What the moves shorten is the frame thread's share.

To do: read the frame thread's time per phase from traces (`--exec "trace start 30"`) of both builds. Do it with the
present unthrottled, and with FSR 3 on for the mirror's scratch fix.

## Image checks

Offscreen screenshots (`--screenshot`, 3 s after ready) of the drive start with the mirror on, and of the main menu,
from both builds: the world differs in 40 of 921,600 pixels. The menu differs only along the edges, the same picture
moved under a sub-pixel camera shift: its intro animation was caught a moment apart.
