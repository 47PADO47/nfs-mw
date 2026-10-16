# Status and handoff (2026-10-09)

**Update 2026-10-16:** [ADR 0004](../../decisions/0004-swappable-renderers.md) is accepted by the project owner.
It was proposed on 2026-10-09. The ADR now also states the DLSS target (the newest DLSS 4.x Super Resolution and Ray
Reconstruction transformer preset in the SDK bundled with `dlss_wgpu` 6.0.0; the exact preset is unconfirmed).
DLSS 5 and frame generation stay out of scope and are tracked as a follow-up.

Where the swappable-renderers plan stands, what changed from the plan while building it, and what is left for a
machine with the right hardware (an NVIDIA RTX PC). The rest of the plan is unchanged; section numbers (`§N`)
refer to it. Everything here was built and tested on an Intel Iris Xe laptop (Vulkan, Mesa); no NVIDIA, AMD or
DX12 result exists yet.

## 1. PR status

All branches are in one GitHub stack (stack #23, draft PRs, each based on the previous one). `main` is the base.

| Plan PR | Branch | GitHub | State |
|---|---|---|---|
| (milestone 8) render pipeline, car shader, post-processing, FSR 1, ADR 0003, ReShade docs, low-end options | `feat/m8-render-pipeline` ... `feat/m8-low-end-options` | #17 to #22, #30 | done, draft |
| 1 | `docs/gfx-adr-0004` | #32 | done: ADR 0004 (accepted 2026-10-16) and this plan |
| 2 | `build/bevy-0.20-final` | #33 | done: pins `=0.20.0`, workspace MSRV stays 1.95 |
| 3 | `feat/gfx-interface` | #34 | done: `libs/blackbox-gfx` |
| 4 | `feat/gfx-native-backend` | #35 | done: `impl RenderBackend for Renderer`, headless, testkit, digests |
| 5 | `refactor/gfx-callers` | #36 | done: callers on `dyn RenderBackend`, `FrameParams`/`Instance` fields |
| 7 | `refactor/gfx-shared-passes` | #37 | done: `libs/blackbox-gpu-passes` |
| 6 | `feat/gfx-settings` | #39 | done (built after 7, so it sits above it in the stack) |
| 8 | `feat/gfx-bevy-spike` | #40 | done: conditional GO, see §3 |
| 9 | `feat/gfx-bevy-scene` | | done: glossy shading, the lighting rig and environment, texture redirects, the effect layer, the UI layer; see §3a |
| 10 | `feat/gfx-bevy-post` | | done: bloom, tone mapping, FXAA, SMAA, TAA, render scale, FSR 1; see §3b |
| 11 | `feat/gfx-bevy-dlss` | | **not started**, needs the NVIDIA PC |
| 12 | `feat/gfx-bevy-solari` | | **done on the RTX card**: startup gate, caps, stand-ins, cameras, sun, camera-cut reset, game wiring; the stand-in look is white (open, §3d) |
| 13a | `feat/gfx-fsr3-core` | #41 | done: `libs/fsr3-wgpu`, not used by a renderer yet |
| 13b | `feat/gfx-fsr3` | | **wired, partly verified**: `fsr3` feature in `blackbox-bevy-render` (`src/fsr3/`), caps offer FSR 3 when the device can run it, static-scene GPU test passes on the RTX card; **open**: `BlackboxMaterial` has no prepass (`enable_prepass` false), so world depth and motion vectors are empty and moving-camera reprojection is wrong until the material writes them (outside PR 13b's files); quality vs AMD not measured |
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
  **Not re-run**: PR 9 (§3a) was built directly on top of the spike at the owner's instruction, without
  repeating this profiling pass on Iris Xe. It was, however, verified functionally on different hardware
  (an NVIDIA RTX 4070 SUPER) where the spike's CPU/frame-time numbers were never measured in the first
  place, so whether the two criteria hold on Iris Xe is still open.
- **Not drawn under `--renderer bevy` yet (as of PR 8):** glossy shading (falls back to lit), texture
  redirects, effect layer, **UI layer (menus, HUD, console)**, post effects, camera cut. PR 9 (§3a) added
  everything except post effects and camera cut, which stay for PR 10.

## 3a. PR 9: glossy shading, redirects, the effect layer and the UI

Built on an NVIDIA RTX 4070 SUPER over Vulkan (this machine, not the Iris Xe laptop §3's numbers come
from). Full write-up, the axes and sRGB gotchas, and the measured testkit numbers:
[bevy-backend.md](../../bevy-backend.md), "PR 9". Also verified directly on the real install: driving
screenshot (`nfsmw view-world --renderer bevy --drive --screenshot`, world, glossy car and HUD all drawing
together) and the main menu (`nfsmw view-screen MainMenu.fng --renderer bevy --screenshot`), previously
solid black under `--renderer bevy` and now drawing normally.

Left for PR 9's own follow-up or PR 10: the soft/detailed-particle path (`smoke-quality high`) compiles and
sets the camera's depth usages correctly, but has no test coverage — the testkit's `effects` scene never
turns `detailed_particles` on, and that scene's pixels are pinned by the native renderer's own digest tests
(recorded on Iris Xe, which this session could not regenerate).

## 3b. PR 10: bloom, tone mapping, FXAA, SMAA, TAA, render scale, FSR 1

Also built on the NVIDIA RTX 4070 SUPER over Vulkan. Full write-up, every deviation and what was measured:
[bevy-backend.md](../../bevy-backend.md), "PR 10". The one worth repeating here: the render scale
(`MainPassResolutionOverride`) had been a silent no-op since PR 8/9 — inserted from the main world, where
Bevy never looks for it — so Bevy always rendered at full resolution regardless of the setting. PR 10's own
FSR 1 test caught it (a render that was never actually downscaled, upscaled anyway, framed wrongly rather
than just blurrier) and fixed it with a new render-world-only `Core3d` system. `libs/blackbox-gpu-passes`'s
shared FSR 1 pass gained an explicit input-size parameter as part of the same fix (native's call site
forwards a value it already had but previously discarded; no behaviour change there).

Four new GPU tests in `libs/blackbox-bevy-render/src/post_tests.rs`, all passing: TAA converges on a
static scene, a camera cut resets TAA's history (verified to actually catch a broken reset, not just
measure one that works, by temporarily disabling it and watching the metric roughly quadruple), no
prepass components exist when TAA is off, and FSR 1 matches native within tolerance at 67% render scale.

Known gap carried forward, not caught by this PR's own tests: the world effect layer and the UI layer are
not HDR-aware, so their colours are wrong whenever bloom or tone mapping is on (their shared shaders only
branch on sRGB-vs-not, not "linear HDR"); needs a third shader variant in `blackbox-gpu-passes`.

## 3c. Fix: glossy materials had no default environment, so they reflected white

Reported after PR 10 landed: running the real game with `--renderer bevy`, cars looked too shiny, a red
car's paint read pink, and a BMW's chrome exhaust tips read white instead of grey. Two already-documented
gaps looked like plausible causes; both were ruled out by reproducing the bug first, not by reasoning about
it:

- **Not PR 10's HDR/bloom gap.** The bug reproduces identically with `post_bloom`/`post_tonemap` both off
  (confirmed against the user's own saved config, `%APPDATA%\nfsmw\config\config.toml`), so
  `PostSettings::needs_hdr()` is false and no `Hdr` component is ever inserted; that gap cannot apply.
- **Not "Colour"'s linear-vs-gamma blend gap.** Car paint, chrome and exhaust all draw
  `BlendKind::Opaque` (`blend_mode()` in `libs/blackbox-scene/src/textures.rs`: a copy-mode texture with a
  modulated alpha, which is what a specular/reflection mask is, draws opaque; confirmed by the shader's
  own `OPAQUE_ALPHA` branch in `material/blackbox.wesl` writing alpha 1 unconditionally). That blend gap
  only matters when two draws composite in hardware, so it cannot apply to a single opaque draw.
- **What it actually was.** Sampling identical pixels directly (not just the aggregate `img-diff` stats)
  on `nfsmw view-car FXXEVO --renderer {blackbox|bevy}` showed every channel brighter on Bevy, including
  points with no specular highlight or fog involved — e.g. a shaded roof/glass pixel read `(46, 48, 53)`
  natively and `(109, 109, 107)` on Bevy: brighter *and* pulled toward neutral on every channel, the
  signature of a flat colour added everywhere, not a clipped highlight or a blend-order swap.

**Root cause:** no caller in `crates/nfsmw` ever calls `RenderBackend::set_environment` — not `CarScene`,
not `WorldScene`. Native papers over this: `Glossy::new()` (`libs/blackbox-render/src/gpu/glossy/mod.rs`)
builds a procedural sky cube map from `SkyGradient::default()` the moment any glossy resource is first
touched (`set_lighting_rig`, `create_glossy_material`, or an explicit `set_environment_*`), via its
`ensure()` helper, so a caller that never sets one still gets a dim sky reflection for free.
`BevyBackend::create_glossy_material` and `set_lighting_rig` had no equivalent, so
`BlackboxMaterial::environment` stayed `None` forever in every real scene. Bevy's `AsBindGroup`
substitutes its own fallback for a `None` texture binding; for a plain `Option<Handle<Image>>` field that
fallback is `Image::default()` — an opaque *white* 1x1 cube, not native's dim sky. Every glossy surface's
`env * (occlusion * env_strength * diff.a * 0.5)` term (`shade_glossy`, `material/blackbox.wesl`) added a
flat white tint instead, since the glossy spike (PR 9) landed, not something PR 10 introduced. That
explains all three symptoms: "too shiny" is a brightness floor under every highlight, the paint reading
pink is white desaturating red, and the exhaust reading white is chrome/metal light materials having the
highest `envmap_min`/`envmap_range` of any part.

**The fix** (`libs/blackbox-bevy-render/src/facade.rs`): `BevyBackend::ensure_default_environment` mirrors
native's `ensure()`, called from `create_glossy_material` and `set_lighting_rig`. The first call queues one
`Op::SetEnvironment` from `Environment::Sky(SkyGradient::default())` and never a second; a caller's own,
later `set_environment` is never clobbered, in either order, because `set_environment` marks the default
"requested" itself. This is also why `glossy_spheres_match_native` never caught the bug: the testkit's own
`glossy` scene (`libs/blackbox-gfx-testkit/src/scenes/glossy.rs`) calls `set_environment` itself before
drawing, unlike any real scene. Four new facade tests (`facade.rs`, `mod tests`) pin the lazy default and
its ordering against an explicit call; they need no GPU, since the facade only ever queues ops.

**Verified**, same car and position, before/after, `cargo xtask img-diff` (1280x720, the user's own saved
settings: glossy shading, bloom and tone mapping off):

| Comparison | Before: mean / p99 / max | After: mean / p99 / max |
|---|---|---|
| `view-car FXXEVO`, native vs. bevy | 7.145 / 73 / 241 | 3.825 / 13 / 241 |
| `view-car BMWM3GTR`, native vs. bevy | 6.412 / 58 / 241 | 3.914 / 14 / 242 |
| Driving past FXXEVO at 2152,1399 (I7) | 8.257 / 109 / 255 | 4.248 / 61 / 255 |

The sampled roof pixel above now reads `(53, 56, 62)` against native's `(46, 48, 53)` — a handful of
levels, not a 60-level flat offset. The diff pictures afterwards show the car's own body panels as
near-black (no difference); what remains is anti-aliased silhouette edges and, for the driving shot, the
sky's own already-documented linear-vs-gamma tint and the windshield's blended glass (a separate sorting
gap, see "Colour" and "Draw order" in `bevy-backend.md`) — not the car's paint or chrome.

Every tolerance already recorded in `bevy-backend.md`'s "Results" and "PR 9" sections is unchanged by this
fix, because none of those measurements could have exercised the bug: the testkit's `glossy` scene sets its
own environment (`glossy_sphere` is still `max 1, mean 0.254, p99 1`, identical to the recorded PR 9
numbers, and all 63 of `blackbox-bevy-render`'s tests — including every ignored GPU one — pass unchanged),
and the "real city" flyover table never drives a car (no `--drive`) while the city's own traffic and
scenery never request `Shading::Glossy` (`crates/nfsmw/src/scenes/world/resident.rs` only ever asks for
`Prelit`/`Sky`), so neither table ever touched a glossy material in the first place.

## 3d. PR 12: Solari ray tracing (works on the RTX card; the look is a stand-in)

Code in `libs/blackbox-bevy-render/src/solari.rs` (tests in `solari/tests.rs`), behind `ray-tracing`, with hooks in
`plugin.rs`, `apply/instances.rs`, `apply/mod.rs` and the caps in `caps.rs`. The game turns it on through
`crates/nfsmw/src/app/render/bevy.rs` (`add_plugin` reads the resolved `ray_tracing` setting).

What it does:

- **Startup gate.** `BlackboxBevyRenderPlugin::ray_tracing` (default off). `solari::install` adds `SolariPlugins` only
  when that is set and the adapter reports the five features Solari needs. Otherwise it logs a warning and the
  lighting stays rasterised. Deliberate deviation from the brief: the plugins are added only when requested, not
  whenever the feature is compiled. Solari's build turns `DefaultOpaqueRendererMethod` to deferred, so the Blackbox
  materials would move to the deferred path in every build. `install` sets it back to forward. The setting is
  restart-required.
- **Caps.** `RtSupport::Available` when the device has the Solari features (`solari::support`), with
  `denoiser: Some(DlssRr)` only in the `dlss` build when Bevy's `DlssRayReconstructionSupported` resource exists.
  Otherwise `None`.
- **Stand-ins.** Each opaque, non-sky draw becomes an entity with `Mesh3d` and `RaytracingMesh3d` on the same mesh,
  and a `MeshMaterial3d<StandardMaterial>` that carries the base colour texture only, one per texture number. The
  stand-in uses `OpaqueRendererMethod::Deferred`. Alpha-tested, blended and additive draws keep the Blackbox material
  and are not traced (Solari 0.20 has no alpha masks). Sky stays on the Blackbox material. Emissive is not carried
  over (ADR 0004, open decision 5).
- **Meshes.** Only the ranges a stand-in draws are prepared, as they are added: `Indices::U32`, tangents, the
  ray-tracing flag, and the vertex colour removed. Solari builds a BLAS only for a mesh whose attributes are exactly
  POSITION, NORMAL, UV_0 and TANGENT (`is_mesh_raytracing_compatible`), and a Blackbox range always carries the colour.
  This has to happen before the add, because Bevy drops a `RENDER_WORLD` mesh from the main world once it is extracted.
- **Light.** A shadowless sun (`lux::AMBIENT_DAYLIGHT`, a guess to tune) follows the frame's light direction. There is
  no sky or environment light yet, so unlit parts are dark.
- **Cameras.** Every `Camera3d` gets `SolariLighting`, the deferred, depth and motion prepasses and `Hdr`. Its main
  texture gets `STORAGE_BINDING` (Solari writes it from a compute shader). `Camera` already requires a default
  `CameraMainTextureUsages` without that flag, so the flag is set when missing, not inserted only when absent.
- **Camera cuts.** `note_camera_cut` reads the frame's `camera_cut` before `apply` takes the frame away, and
  `solari::reset_on_cut` sets `SolariLighting::reset` on a cut, the way `post/taa.rs` resets TAA. Bevy clears `reset`
  itself at extraction.
- **Captures** are lit by Solari too. `apply/capture.rs` removes the temporal parts only in the `dlss` build, and
  `sync_cameras` puts back the prepasses Solari needs after `apply`, so a capture keeps them.
- **DLSS-RR.** `sync_denoiser` (the `dlss` build only) inserts `Dlss<DlssRayReconstructionFeature>` on every camera
  once Bevy reports Ray Reconstruction. Not compiled here: the DLSS SDK is not installed. It is not yet reconciled
  with the DLSS super resolution component `dlss/sdk.rs` adds on the same camera, and it also reaches capture cameras,
  which `apply/capture.rs` says never run DLSS.

**Root cause of the clear-colour picture (fixed).** Solari's lighting node needs the scene bind group, which needs a
TLAS, which needs a BLAS per instance. No BLAS was built for any Blackbox range, because each carried
`ATTRIBUTE_COLOR_BGRA` and Solari accepts only the four attributes above. So `live_count` stayed 0, the scene bind
group was `None`, and `solari_lighting` returned early. Solari's extract also inserts `SkipDeferredLighting` on the
camera, so Bevy's own deferred lighting did not draw either. Only the clear colour was left. The forward stand-in drew
because raster needs no BLAS. The fix is the mesh preparation above.

**Second bug, found on the way (fixed).** `sync_cameras` used `insert_if_new` for `CameraMainTextureUsages`, but
`Camera` already requires the default, so the storage flag was never added. Solari then failed bind group creation
with a validation error on the capture target.

Game wiring: `add_plugin` passes `ray_tracing` from `Settings::ray_tracing` (anything but `off`). The setting
is read when the App is built, so a change takes effect on the next start.

Look (open, a decision for ADR 0004): the cuboid is lit but renders white, not the authored colour. A traced mesh
has exactly four attributes, so Solari reads no per-vertex colour. A stand-in with no texture falls back to the
default white base colour. Carrying the colour needs a per-colour stand-in material (the cache is keyed by texture
number), or a texture, and is not done.

Verified (RTX 4070 SUPER, Vulkan):

- `cargo test -p blackbox-bevy-render --features ray-tracing --lib`: 62 pass, 24 ignored. Without the feature: 52
  pass, 23 ignored. New CPU tests cover the four-attribute set, that only stand-in ranges lose the colour, and the
  camera-cut reset.
- `cargo test -p blackbox-bevy-render --features ray-tracing --lib solari -- --include-ignored --nocapture`: the GPU
  test passes with no validation errors. Its PNG (`solari_cuboid.png` under the temp dir) shows the lit cuboid on the
  clear colour. The test now asserts more than two distinct colours.
- `cargo clippy -p blackbox-bevy-render -- -D warnings` and with `--features ray-tracing` (lib and tests): clean.
  rustfmt is clean for `solari.rs`, `solari/tests.rs` and `nfsmw` `app/render/bevy.rs`.
- `cargo check -p nfsmw --features ray-tracing` fails in files this change does not touch (`saves.rs`: a missing
  `mods` field; `scenes/world/effects/world.rs`: a missing `cop_health_bars` field). No error is reported in `bevy.rs`.

Not verified: DLSS-RR (no SDK, and the capture-camera overlap above); DX12; the look (white, see above); the sun's
intensity; whether hidden stand-ins are still traced; the `nfsmw` build (blocked by the errors above).

Next leads: a per-colour stand-in for untextured prelit geometry; keep `sync_denoiser` off capture cameras; check
hidden stand-ins; tune the sun.

## 4. Handoff: building the remaining PRs

Start from the stack tip (`feat/gfx-bevy-spike`, or this docs branch which sits on it). Every commit must pass
`cargo check --workspace` and `cargo xtask size-check`; run the guard rails from AGENTS.md at the end of each PR
(`clippy --all-targets` has 5 failures on `main` in files this series does not touch).

| PR | Needs | Hardware you need |
|---|---|---|
| 9 Bevy scene | **done**, see §3a | any Vulkan GPU |
| 10 Bevy post | **done**, see §3b | any Vulkan GPU |
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
ADR 0004 and the `libs/` rule change were accepted on 2026-10-16.
Still open: reword the ADR 0003 trigger; the preset deviations in §2;
raise the workspace MSRV or keep the Bevy crate on 1.97.1 only; whether releases include `renderer-bevy`; bundling
the DLSS and FSR 4 DLLs; an FSR 4 tester or cutting PR 14; the ray-tracing look (pre-lit vertex colours as albedo
darkening, whole city or only nearby tiles); `exhaust_flames` default (on in #31).
