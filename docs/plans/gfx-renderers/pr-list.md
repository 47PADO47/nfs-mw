# Swappable renderers: the stacked PR list

Part of the [swappable renderers plan](README.md): section 6.

## 6. Stacked PR list

Base: `feat/m8-low-end-options`, after it is committed and pushed. Each PR targets the previous one (gh-stack). Parallel worktrees branch from their stated dependency and rebase when the predecessor merges.

| # | Branch | Goal | Depends on | Size (approx. lines) | Can run in parallel with | Hardware |
|---|---|---|---|---|---|---|
| 1 | `docs/gfx-adr-0004` | ADR 0004, this plan in `docs/plans/`, the architecture rule change (Bevy-based backend crates allowed in `libs/`) | stack tip | 600 docs | 2 | — |
| 2 | `build/bevy-0.20-final` | Pins `=0.20.0-rc.2` → `=0.20.0`, MSRV check (maybe 1.97.1), lockfile, migration fixes | stack tip | < 150 | 1, 3 | — |
| 3 | `feat/gfx-interface` | `libs/blackbox-gfx`: moved types, trait, capabilities, `GraphicsSettings`, `resolve`; re-exports from `blackbox-render` | 1 | about 1,400 (mostly moved) | 2 | — |
| 4 | `feat/gfx-native-backend` | `impl RenderBackend for Renderer`, `Renderer::headless`, async capture, real capabilities, `libs/blackbox-gfx-testkit` (synthetic scenes, image metrics, digests), native digest tests | 3 | about 1,200 | 6, 7, 13a | lavapipe and Iris Xe here; DX12 by owner |
| 5 | `refactor/gfx-callers` | `nfsmw` and `blackbox-scene` on `dyn RenderBackend`; `FrameParams` view/projection/`camera_cut`; `InstanceKey`; screenshot polling; render bridge split | 4 | about 900 changed across 40 files | 6, 7, 13a | here (screenshots byte-identical) |
| 6 | `feat/gfx-settings` | `renderer`, extended `post_aa` and `upscaler`, `upscale_quality`, `ray_tracing`, resolve against capabilities, `gfx` console command, menu rows by capability, preset mapping | 5 (types from 3) | about 1,000 (+ tests) | 7, 8, 13a | here |
| 7 | `refactor/gfx-shared-passes` | Extract UI, effects, soft-particle, textured-effect, filter and FSR 1 passes into `libs/blackbox-gpu-passes`; native image unchanged (digests) | 4 | about 1,500 moved | 6, 8, 13a | here |
| 8 | `feat/gfx-bevy-spike` **(GO/NO-GO)** | `libs/blackbox-bevy-render` skeleton: probe, plugin, façade and op queue, textures, meshes, `BlackboxMaterial` (prelit, lit, sky, fog, alpha modes), instance pool, camera, headless capture; `nfsmw` feature `renderer-bevy`, `--renderer bevy`, `--bench-seconds`; spike report | 2, 5 (6 optional) | about 1,800 | 13a | **here** (criteria in §8); owner for DX12 |
| 9 | `feat/gfx-bevy-scene` | Glossy material, rig, environment, texture redirects, blend-order parity, effect layer and UI layer via shared passes, stats and metrics, vsync, resize | 7, 8 | about 1,500 | 10 (partly), 12, 13a | here |
| 10 | `feat/gfx-bevy-post` | Bloom, tonemap, FXAA, SMAA, TAA (jitter, motion vectors from keys, `camera_cut`), render scale, bilinear, FSR 1 pass, mip bias | 9 | about 900 | 12, 13a | here; owner looks on RTX |
| 11 | `feat/gfx-bevy-dlss` | `dlss` and `dlss-mock` features, `DlssInitPlugin` and `DlssProjectId` (UUID passed by `nfsmw`), quality modes, capability probe (NVIDIA on Vulkan), CI compile with the mock | 10 | about 500 | 12, 13 | **implemented + unit-tested here (mock), hardware-verified by owner** |
| 12 | `feat/gfx-bevy-solari` | `ray-tracing` feature, startup gating, ray-tracing mesh mirror, StandardMaterial mapping, sun and sky lights, quality levels, DLSS-RR when present | 9 (10 for TAA interplay) | about 1,100 | 11, 13 | **implemented + unit-tested here; hardware-verified by owner** (lavapipe smoke test if it works) |
| 13 | `feat/gfx-fsr3` (13a: `libs/fsr3-wgpu` standalone; 13b: Bevy integration) | WGSL port of the FSR 3.1 upscaler, headless tests; integrated as an `EarlyPostProcess` system | 13a: 3 only; 13b: 10 | 13a about 3,500 (shaders split under 500 lines each) + about 800 Rust; 13b about 400 | 13a from day one | here (Vulkan); owner checks DX12 and quality |
| 14 | `feat/gfx-bevy-fsr4` (experimental, **cut-able**) | `libs/ffx-api-wgpu` (Windows): `libloading` of AMD's DX12 FidelityFX API DLLs, wgpu-hal DX12 interop (raw device, command list, resources, state transitions), capability probe (DX12, RDNA3/4, DLL found) | 13b | about 900 | — | **cannot be verified by the owner (RTX).** Merge only with an RDNA3/4 tester; otherwise park on a branch. |

User and developer docs land with each PR (§10). A final docs-only sweep can be folded into PR 13b.

### Per-PR detail

**PR 1: `docs/gfx-adr-0004`**
- Files: `docs/decisions/0004-swappable-renderers.md`, `docs/plans/gfx-renderers/` (this plan, split over several files), `docs/decisions/0003-…` (status: partly superseded by 0004), `docs/architecture.md` (renderer rule, libs table placeholders), `AGENTS.md` and `CONTRIBUTING.md` (one line: Bevy-based backend crates are allowed in `libs/` as optional leaves that nothing else depends on).
- Acceptance: the owner accepts the ADR. `cargo xtask check` passes (docs under 500 lines; split the plan if needed).
- Agent can do it all. Owner sign-off needed.

**PR 2: `build/bevy-0.20-final`**
- Files: `Cargo.toml` (8 pins), `Cargo.lock`, migration fixes in `crates/nfsmw` if the rc.2 → final API changed, `docs/rust-stack.md`, ADR 0001 status line.
- MSRV: run `cargo +1.95.0 check -p nfsmw` (install toolchain 1.95.0). If it fails, raise `rust-version` to the lowest that passes (expect 1.97.1). `cargo deny check licenses sources bans` passes.
- Tests: the full guard rails, plus `nfsmw play --skip-boot --screenshot` still works.
- Agent can do it all.

**PR 3: `feat/gfx-interface`**
- Files: new `libs/blackbox-gfx/**` (layout in §1.2), `libs/blackbox-gfx/README.md`, `blackbox-render/src/lib.rs` (re-exports; moved modules deleted), workspace `Cargo.toml`.
- Not yet: `FrameParams` and `Instance` keep their old shape here behind re-exports; the new fields arrive in PR 5. Alternatively add them now with defaults; prefer the PR 5 route for a reviewable diff.
- Tests (CPU): `resolve` (fallback chains, temporal-supersedes-AA, ray tracing without a denoiser, render-scale clamping), handle round-trips, settings sanitising (moved tests), bitsets.
- Agent can do it all.

**PR 4: `feat/gfx-native-backend`**
- Files: `blackbox-render/src/backend_impl.rs` (trait impl), `gpu/init.rs` (`headless(size, options)`: no surface, output texture as target), `gpu/capture.rs` (request/poll), `caps.rs`, new `libs/blackbox-gfx-testkit/**`, `blackbox-render/src/gpu/parity_tests.rs` and `digests.rs`.
- Testkit: procedural meshes and textures only (no game data). Scenes:
  - opaque grid (BC1 and RGBA, vertex colours, fog ramp);
  - alpha-test cards;
  - blend and additive stack;
  - sky dome at 9.7 km;
  - reverse-Z depth probe;
  - glossy sphere;
  - effect layer;
  - UI layer.
  Plus metrics (max, mean, p99 absolute difference, masked by region) and `Digest` (a 32×18 grid of mean RGB per cell, stored as a Rust `const`). Regenerate with `BLACKBOX_UPDATE_DIGESTS=1` printing the const to stdout. Tests never write repo files.
- Tests: `#[ignore = "needs a GPU"]` parity tests on Vulkan (lavapipe allowed through a fallback-adapter env switch), DX12 on Windows, GL. Capabilities report exactly `{Off, Bilinear, Fsr1}` and `{Off, Fxaa}`, with no ray tracing.
- CI: an optional job that installs `mesa-vulkan-drivers` and runs `cargo test -p blackbox-render -- --ignored parity --test-threads=1`.
- Agent can do it all on lavapipe and Iris Xe. Owner runs DX12 once.

**PR 5: `refactor/gfx-callers`**
- Files: `crates/nfsmw/src/{app/host.rs, app/render/*, app/screenshot.rs, app/pacing.rs, viewer/mod.rs, viewer/camera/*, scenes/**, frontend/scene.rs, movie/mod.rs, devtools/{metrics.rs, console/exec.rs}, gui/*, ui/present/blackbox.rs, settings/*}`, `libs/blackbox-scene/**` (depends on `blackbox-gfx` only).
- Changes: `InstanceKey` from `scenes/world/visibility.rs` (tile and instance index) and `drive/rig.rs` (car and part); `camera_cut` on teleport, freecam toggle and scene switch.
- Acceptance:
  - `cargo tree -p blackbox-scene` doesn't contain `blackbox-render`;
  - `rg "blackbox_render::" crates/` only in `app/render/native.rs`;
  - **`view-car` and `view-world --at X,Y --screenshot` byte-identical before and after**, on Vulkan and GL here (real install; the agent compares hashes, nothing committed).
- Agent can do it all.

**PR 6: `feat/gfx-settings`**
- Files:
  - new: `settings/{renderer.rs, ray_tracing.rs, upscale_quality.rs}`, `settings/graphics.rs` (`Settings` → `GraphicsSettings`), `app/graphics.rs` (replaces `app/post.rs` and `app/upscale.rs`), `devtools/console/gfx_cmd.rs`;
  - changed: `settings/{post.rs, upscale.rs, env.rs, file.rs, partial.rs, write.rs, mod.rs}`, `cli.rs`, `frontend/{options.rs, post_options.rs}`.
- Also: the preset mapping in `settings/preset.rs`, adapted to whatever the low-end layer lands.
- Tests: parsing and round-trips per key, layering, the console error text for unavailable values, menu rows for blackbox capabilities vs a fake Bevy capabilities struct, preset to effective per backend.
- Agent can do it all.

**PR 7: `refactor/gfx-shared-passes`**
- Files: new `libs/blackbox-gpu-passes/**` (`ui.rs`, `effects/*`, `soft_particles.rs`, `textured_effects.rs`, `filter.rs`, `fsr1.rs`, shaders); `blackbox-render/src/gpu/*` calls them. The GPU tests move along.
- Acceptance: native digests unchanged, the existing FSR 1, streak and soft-particle tests pass, and the API takes only wgpu types and `blackbox-gfx` types.
- Agent can do it all.

**PR 8: `feat/gfx-bevy-spike` (GO/NO-GO)**
- Files:
  - `libs/blackbox-bevy-render/{Cargo.toml, README.md, src/{lib.rs, probe.rs, plugin.rs, facade.rs, ops.rs, apply/{textures.rs, meshes.rs, instances.rs, camera.rs}, material/{mod.rs, prelit.wesl}, capture.rs, axes.rs}}`;
  - `.gitignore` (`!/libs/*/src/**/*.wesl`), `xtask` size and leak extension lists;
  - `crates/nfsmw` feature `renderer-bevy`, `app/render/bevy.rs`, `--bench-seconds N` (p50, p95 and p99 frame time, and worst, at exit).
- Docs: `docs/bevy-backend.md` with the measured results; the ADR 0004 go/no-go section filled in.
- Tests:
  - testkit strict scenes pass against native (§7);
  - headless Bevy capture works on lavapipe;
  - default `cargo tree -p nfsmw` has no `bevy_render`.
- Agent can do it all here (Iris Xe numbers). The owner records the same on the RTX PC (DX12 and Vulkan) and makes the call.

**PR 9: `feat/gfx-bevy-scene`**
- Files: `material/glossy.{rs,wesl}`, `apply/{lighting.rs, environment.rs, redirects.rs}`, `systems/{effects.rs, ui.rs}`, `stats.rs`.
- Acceptance: testkit glossy, effects and UI scenes within tolerance. A full `nfsmw play --renderer bevy` session works (menus, HUD, console, movie, driving, screenshots).
- Agent can do it all.

**PR 10: `feat/gfx-bevy-post`**
- Files: `post/{mod.rs, taa.rs, fsr1.rs, scale.rs}`, `apply/camera.rs` (component insertion by effective settings).
- Tests:
  - TAA converges on a static scene (frame N vs N+16 difference below ε);
  - `camera_cut` resets history (no ghost after a teleport);
  - no prepass components exist when TAA is off (query test);
  - FSR 1 output matches native within tolerance at 67 %.
- Agent can do it all.

**PR 11: `feat/gfx-bevy-dlss`**
- Files: `dlss.rs` (init plugin ordering, project id passed in, perf-mode mapping, capability probe), `nfsmw` `dlss` and `dlss-mock` features, CI clippy with `dlss-mock`.
- Docs: `docs/licensing.md` (RTX SDK obligations, SDK never in the repo), `docs/upscaling.md`.
- Unverified: whether `dlss` + `force_disable_dlss` compiles without `DLSS_SDK`.
- Owner: build with the SDK, run on RTX, check every mode and the Ultra Performance/Performance models.

**PR 12: `feat/gfx-bevy-solari`**
- Files: `rt/{mod.rs, mirror.rs, materials.rs, lights.rs, quality.rs}`.
- Tests:
  - startup gating (no plugin when off or unsupported);
  - mesh conversion (attributes, U32 indices, tangents) as CPU tests;
  - exclusion of alpha-tested and blended geometry;
  - capability downgrade on Iris Xe.
- Owner: look and performance on RTX, texture and slab limits on the full city, with and without DLSS-RR.

**PR 13: `feat/gfx-fsr3`**
- 13a `libs/fsr3-wgpu`: a port of the FSR 3.1.x upscaler passes from the MIT FidelityFX source (pin the exact SDK tag and file list in the README and NOTICE). One WGSL file per pass, each under 500 lines; no subgroup operations (a portable path); fp32. Rust: context, resources, dispatch, Halton jitter helper, quality ratios.
- 13a headless tests:
  - a static scene converges towards a supersampled reference;
  - a translating checkerboard keeps PSNR above a threshold (motion-vector reprojection);
  - reverse-infinite-depth handling;
  - reset;
  - no NaN.
- 13b: Bevy system in `EarlyPostProcess`, `MainPassResolutionOverride`, jitter and mip bias, reactive mask from the effect pass.
- Agent can do it all here. Owner checks DX12 and image quality.

**PR 14: `feat/gfx-bevy-fsr4` (experimental)**
- Files: `libs/ffx-api-wgpu/{Cargo.toml (cfg windows), src/{loader.rs, ffi.rs, dx12.rs, upscale.rs}}`, Bevy integration behind `fsr4`.
- Never ships the DLL; the user supplies it or a release decision bundles it.
- **Unverified:** DLL names and API entry points in SDK 2.3, and how wgpu-hal DX12 resource states interact with FidelityFX's expected states.
- Acceptance: compiles on Windows CI, falls back to FSR 3 with a note when the DLL or hardware is missing. **Merge only after an RDNA3/4 test.**
