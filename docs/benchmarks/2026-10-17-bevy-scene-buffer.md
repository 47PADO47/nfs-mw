# Benchmark: shared scene buffer in the Bevy renderer, 2026-10-17

Before and after moving the per-scene data (fog, light, tone-map flag, glossy lighting rig) out of every
`BlackboxMaterial` and into one shared storage buffer. Design: [bevy-backend.md](../bevy-backend.md), "Textures,
meshes, materials". The earlier comparison with the native renderer is in
[2026-10-10-bevy-vs-native.md](2026-10-10-bevy-vs-native.md).

## Why

The day-night clock (`update_day`, `freeze_sun` off by default) changes fog and light every frame. `sync_params`
copied them into every material with `Assets::get_mut`. In Bevy 0.20 a modified non-bindless material gets new
uniform buffers and a new bind group, and every entity that uses it is re-extracted, re-specialised per view and
re-binned. The values now live in one `ShaderBuffer` that Bevy rewrites in place, so no material changes in steady
state (test: `apply/assets/tests.rs`).

## Setup

- Machine: RTX 4070 SUPER, Windows 10, Vulkan, release build with `--features renderer-bevy`.
- Command: `nfsmw view-world --renderer bevy --drive --no-vsync --window-mode borderless --no-sound
  --bench-seconds 20`, with `NFSMW_RENDER_SCALE=100 NFSMW_UPSCALER=off NFSMW_RAY_TRACING=off`. The author's
  config has render scale 200 with FSR 3, which would hide the difference. Other config values (shadows medium,
  traffic, HUD) stay as they were.
- Sun running is the default. Sun frozen is `NFSMW_LAB_FROZEN_SUN=1`.
- Two runs of each binary, interleaved (base, new, base, new), because other builds were running on the machine.

## Results

| Binary | Sun | Run | fps | p50 | p95 | p99 |
| --- | --- | --- | --- | --- | --- | --- |
| before | running | 1 | 39.9 | 20.04 ms | 50.36 ms | 74.93 ms |
| before | running | 2 | 47.8 | 20.27 ms | 30.44 ms | 47.52 ms |
| before | frozen | 1 | 52.5 | 17.12 ms | 29.42 ms | 44.50 ms |
| before | frozen | 2 | 55.9 | 16.82 ms | 22.68 ms | 34.58 ms |
| after | running | 1 | 110.3 | 8.02 ms | 13.02 ms | 17.48 ms |
| after | running | 2 | 123.4 | 7.54 ms | 10.60 ms | 15.53 ms |
| after | frozen | 1 | 116.9 | 7.96 ms | 11.73 ms | 23.19 ms |
| after | frozen | 2 | 119.1 | 7.83 ms | 11.13 ms | 14.39 ms |

With the sun running, p50 goes from about 20 ms to about 7.8 ms (2.5x). After the change a running and a frozen
sun cost the same, which is the point: the clock no longer costs anything per frame. Before it, a frozen sun
was already 3 ms faster at p50.

## Limits

- The absolute fps is far below the 311 fps of the 2026-10-10 run: the scene, settings (shadows, traffic, HUD,
  car shading) and the code have all changed since, and the machine was not idle. Compare the before and after
  rows of this table only.
- Two runs per cell, one machine, one scene.
