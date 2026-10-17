# Benchmark: performance sweep, 2026-10-17

CPU and GPU cost per frame before and after the performance sweep, and the load times of the install data. The sweep
ran in two rounds. The findings and fixes are in [performance-audit.md](../performance-audit.md). How to profile is in
[profiling.md](../profiling.md).

## Setup

- Date: 2026-10-17.
- Backend: headless Vulkan. Resolution 1920x1080.
- Profile: `release`.
- Before: the sweep's starting point, with only the two benchmark tests added. After: the sweep with the system
  allocator, and the same code built with the `mimalloc` feature, which the sweep then made the default.
- The numbers were taken on a local `main` with a thin-LTO release profile. Rerun on the branch as merged, the
  "after" numbers matched within run-to-run noise (5 cars 43.8 ms, city with mirror 0.932 ms).
- Machine: AMD Ryzen 7 9800X3D, NVIDIA GeForce RTX 4070 SUPER (Vulkan, headless), Windows 10.
- Frame cost: `cargo test --release -p nfsmw --bin nfsmw world_frame_costs -- --ignored --nocapture`.
- Load timing: `cargo test --release -p nfsmw-data --test real_install load_timing -- --ignored --nocapture`, with
  `NFSMW_GAME_DIR` set to the install.
- Round two used the same machine, backend, resolution, profile and commands. See [Round two](#round-two).

## Round one: frame cost (CPU per frame)

The frame cost includes the world update, the mirror and the render submit. Values are milliseconds, p50 and p95.
Where a row gives a range, the low and high value of two runs are shown.

| Scene and mirror | Before | After | After, with mimalloc |
| --- | --- | --- | --- |
| hill, mirror off | 0.465 / 0.691 | 0.322-0.467 / 0.471-0.681 | 0.312 / 0.515-0.543 |
| hill, mirror on | 1.500 / 1.657 | 0.706-0.711 / 0.767-0.829 | 0.684-0.687 / 0.726-0.755 |
| city, mirror off | 0.493 / 1.732 | 0.401-0.402 / 0.533-0.548 | 0.381-0.386 / 0.476-0.509 |
| city, mirror on | 1.804 / 1.992 | 0.929-0.942 / 1.045-1.097 | 0.887-0.894 / 0.937-0.999 |

The GPU p50 with the mirror off did not change: 0.16 ms on hill, 0.12 ms on city.

Before this work, the GPU numbers with the mirror on were not valid. The mirror captures overwrote the GPU timing
slots. They now match the mirror-off numbers.

## Round one: load timing

Release build, real install. Values in milliseconds unless the row says otherwise.

| Step | Before | After |
| --- | --- | --- |
| GLOBALB unwrap | 5.5 | 2.5 |
| CarTables | 43.2, then 10-11 | 7.4, then 0.1 |
| 5 cars, total | 849.8 | 45.1 |
| 5 cars, per car | 155-191 | 3.9-5.1 after the first; the first is 26.9 |
| PhysicsData | 20.6, then 13.8 | 0.2, then 0.1 |
| Gameplay data | 32-49 | 20-21 |
| Global textures | 13.9 | 3.9 |

## Round two

Round two runs on the same machine and setup as round one. Its A/B runs were made in one session, against the round
one tip. Ranges are the low and high of three runs. The values are milliseconds.

### Same-session A/B, mirror on

| Scene | Round one tip, CPU p50 / GPU p50 | Round two, CPU p50 / GPU p50 |
| --- | --- | --- |
| hill | 0.739-0.885 / 0.156-0.159 | 0.707-0.713 / 0.143-0.144 |
| city | 1.003-1.126 / 0.115 | 0.963-1.089 / 0.099-0.101 |

The machine was noisier than during round one. Compare the same-session A/B above. Do not compare these rows with the
round one tables across sessions.

### Mirror off, round two

| Scene | CPU p50 | GPU p50 |
| --- | --- | --- |
| hill | 0.332-0.347 | 0.140-0.141 |
| city | 0.406-0.468 | 0.098-0.101 |

### Load timing, round two

Load timing is unchanged. The 5 cars step took 46.5 ms in this session (45.1 ms in round one).

### In-game check

Offscreen play-mode screenshots with the mirror on are identical to before the round two changes.

An earlier session's in-game overlay read 17 fps before the sweep and 51 fps after it, with the user's settings (FSR 3,
mirror on). That is a single overlay reading, not a benchmark.

## Limits

- One sweep per round, one headless setup. The round one ranges are the spread of two runs. The round two ranges are
  three runs. Neither is a statistical result.
- The mimalloc column adds the allocator change on top of the "after" column.
- The mirror-on rows measure the reduced view capture, not a full second frame.
