# Status and handoff (2026-10-09)

Where the swappable-renderers plan stands, what changed from the plan while building it, and what is left for a
machine with the right hardware (an NVIDIA RTX PC). The rest of the plan is unchanged; section numbers (`§N`)
refer to it. Everything here was built and tested on an Intel Iris Xe laptop (Vulkan, Mesa); no NVIDIA, AMD or
DX12 result exists yet.

## 1. PR status

All branches are in one GitHub stack (stack #23, draft PRs, each based on the previous one). `main` is the base.

| Plan PR | Branch | GitHub | State |
|---|---|---|---|
| (milestone 8) render pipeline, car shader, post-processing, FSR 1, ADR 0003, ReShade docs, low-end options | `feat/m8-render-pipeline` ... `feat/m8-low-end-options` | #17 to #22, #30 | done, draft |
| 1 | `docs/gfx-adr-0004` | #32 | done: ADR 0004 (proposed) and this plan |
| 2 | `build/bevy-0.20-final` | #33 | done: pins `=0.20.0`, workspace MSRV stays 1.95 |
| 3 | `feat/gfx-interface` | #34 | done: `libs/blackbox-gfx` |
| 4 | `feat/gfx-native-backend` | #35 | done: `impl RenderBackend for Renderer`, headless, testkit, digests |
| 5 | `refactor/gfx-callers` | #36 | done: callers on `dyn RenderBackend`, `FrameParams`/`Instance` fields |
| 7 | `refactor/gfx-shared-passes` | #37 | done: `libs/blackbox-gpu-passes` |
| 6 | `feat/gfx-settings` | #39 | done (built after 7, so it sits above it in the stack) |
| 8 | `feat/gfx-bevy-spike` | #40 | done: conditional GO, see §3 |
| 9 | `feat/gfx-bevy-scene` | | **not started** |
| 10 | `feat/gfx-bevy-post` | | **not started** |
| 11 | `feat/gfx-bevy-dlss` | | **not started**, needs the NVIDIA PC |
| 12 | `feat/gfx-bevy-solari` | | **not started**, needs the NVIDIA PC |
| 13a | `feat/gfx-fsr3-core` | #41 | done: `libs/fsr3-wgpu`, not used by a renderer yet |
| 13b | `feat/gfx-fsr3` | | **not started** |
| 14 | `feat/gfx-bevy-fsr4` | | **not started**, experimental, needs an RX 7000/9000 tester |

The status PR itself is the top of the stack (see the PR list on GitHub). Not part of the stack: exhaust flames (#31, against `main`).

## 2. Changes from the plan while building it

- **Order:** PR 7 (shared passes) was merged below PR 6 (settings) in the stack, because they touch different files.
- **PR 5 acceptance:** `rg "blackbox_render" crates/` only finds `app/render/native.rs` (plus `Cargo.toml`); the plain
  `rg` misses it because `.gitignore` is a whitelist, so use `rg --no-ignore`.
- **Presets** (PR 6): `high` stays FXAA with tone mapping off (the plan said TAA and ACES), to keep the native image
  and avoid downgrade warnings. `ultra` is `high` plus TAA and `ray_tracing = medium`, with no temporal upscaler
  (the choice depends on the GPU). On the native renderer `ultra` resolves to `high` with two logged downgrades.
- **MSRV:** the workspace stays at 1.95. Only `blackbox-bevy-render` declares `rust-version = 1.97.1`, because
  Bevy's WESL stack needs it; builds with `--features renderer-bevy` need that toolchain.
- **Capture** is asynchronous (`request_capture`/`poll_capture`); the Bevy backend finishes a capture the caller
  already took with an atomic flag, not by watching the queue.
- **Overlay text:** the adapter name now reads `(vulkan)` instead of `(Vulkan)`.
- **ADR 0003:** its "6,000 line" revisit trigger is already exceeded by this stack, so the ADR needs its trigger
  reworded or the Bevy question re-opened (owner decision, still open).

## 3. PR 8 verdict: conditional GO

Details and the full table: [bevy-backend.md](../../bevy-backend.md) and ADR 0004's go/no-go section. On Iris Xe:

- Strict scenes match the native renderer (max 1, mean <= 0.04). The real city differs by a mean of 0.22 to 1.61 out
  of 255 at six positions. The blend stack misses on p99 (37 vs 24): linear blending and distance sorting.
- **Two misses:** CPU per frame is native + 4.4 to 5.1 ms (limit + 4 ms) and frame time is 1.45 to 1.5x native
  (limit 1.3x). It is a fixed per-frame Bevy cost (schedules and idle PBR plugins), not instancing. Not profiled.
- Release build +95 s, binary +48.6 MiB with the feature. Default builds have no `bevy_render`.
- **Gate before PR 9:** profile with `trace_tracy`, trim the plugin set until both criteria hold. If they cannot be
  met, take no-go path (a) (§8): Bevy for the car viewer, showroom and menus, the city stays on the native renderer.
- **Not drawn under `--renderer bevy` yet:** glossy shading (falls back to lit), texture redirects, effect layer,
  **UI layer (menus, HUD, console)**, post effects, camera cut. PR 9 and 10 add them.

## 4. Handoff: building the remaining PRs

Start from the stack tip (`feat/gfx-bevy-spike`, or this docs branch which sits on it). Every commit must pass
`cargo check --workspace` and `cargo xtask size-check`; run the guard rails from AGENTS.md at the end of each PR
(`clippy --all-targets` has 5 failures on `main` in files this series does not touch).

| PR | Needs | Hardware you need |
|---|---|---|
| 9 Bevy scene | the gate in §3; calls the passes of `blackbox-gpu-passes` (its README has the foreign-loop contract: group-0 `Globals`, `Depth32Float` reverse-Z, UI last); `RenderContext::command_encoder()` gives the raw encoder (read from source, not exercised); depth sampling for soft particles needs `TEXTURE_BINDING` on the camera depth (read from source, not exercised) | any Vulkan GPU |
| 10 Bevy post | bloom, tone mapping, FXAA, SMAA, TAA (jitter, motion vectors from `InstanceKey`, `camera_cut` reset), render scale, FSR 1 pass, mip bias; components only inserted when enabled | any |
| 11 DLSS | `dlss` / `dlss-mock` features, `DlssInitPlugin` before `RenderPlugin`, `DlssProjectId`, build needs the DLSS SDK (`DLSS_SDK`, `VULKAN_SDK`, clang); CI compile-checks with the mock; `docs/licensing.md` must record the NVIDIA terms | **NVIDIA RTX, Vulkan** |
| 12 Solari | startup-gated (`ray_tracing != off`), needs the wgpu ray-query features, mirrors world meshes into `RaytracingMesh3d` + `StandardMaterial` (alpha-tested and blended geometry excluded), DLSS-RR as denoiser when built | **RTX, Vulkan** |
| 13a/13b FSR 3 | 13a is merged into the stack (#41: `libs/fsr3-wgpu`, FSR 3.1.4 port, 32 CPU + 28 GPU tests); 13b adds the `EarlyPostProcess` system (README of the crate lists what Bevy must supply: jitter, depth, motion vectors, a storage-capable output texture, reset on `camera_cut`) | Vulkan here; DX12 image quality on your PC |
| 14 FSR 4 | DX12 only, AMD's signed DLL loaded at run time, RX 7000/9000; cannot be verified without that hardware, park the branch if nobody can test | **AMD RDNA 3/4, DX12** |

Notes that save time:

- **GPU tests:** every GPU test must take `blackbox_gpu_passes::test_support::serial()` (device creation on parallel
  threads crashed in 15 to 35% of runs on Mesa). The native digests in `libs/blackbox-render/src/gpu/parity/` exist
  only for Intel Xe + Vulkan + Mesa; for another GPU run the generator (`docs/testing.md`) and add a `Family` variant.
- **Lavapipe** is optional: tests need `VK_LOADER_DRIVERS_SELECT='*lvp*'` and `BLACKBOX_GPU_FALLBACK=1` here because
  the loader is pinned to `*intel*`.
- **Present mode:** use `Mailbox` for no-vsync (Bevy's `AutoNoVsync` ran at the display rate). Benchmarks:
  `--bench-seconds 30 --no-vsync --max-fps unlocked`; do not enable Bevy's `multi_threaded` (it made CPU worse).
- **Plugin order:** embed the material shader before `MaterialPlugin` is built, or the load races and fails.
- **OpenGL:** `--backend gl` already panics at renderer creation on `main` (`textureLoad` from a depth texture in
  GLSL, in the soft-particle pipeline); the native GL path needs that fixed before any GL check can pass.
- **Your PC:** record the same `--bench-seconds` and real-city screenshot comparisons with `renderer = bevy` on DX12
  and Vulkan, and add them to the tables in `docs/bevy-backend.md`. Test DLSS in every quality mode; the "4.5"
  models are only reached through the SDK's per-mode defaults (not verified).

## 5. Open owner decisions

Answered: Solari is out of milestone 8 (a gated later spike), DLSS bundling is deferred, DLSS 5 is not reachable.
Still open: accept ADR 0004 and the `libs/` rule change; reword the ADR 0003 trigger; the preset deviations in §2;
raise the workspace MSRV or keep the Bevy crate on 1.97.1 only; whether releases include `renderer-bevy`; bundling
the DLSS and FSR 4 DLLs; an FSR 4 tester or cutting PR 14; the ray-tracing look (pre-lit vertex colours as albedo
darkening, whole city or only nearby tiles); `exhaust_flames` default (on in #31).
