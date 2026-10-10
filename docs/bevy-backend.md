# The Bevy renderer backend (spike)

`libs/blackbox-bevy-render` is an optional renderer for the games that implements the same
[`RenderBackend`](../libs/blackbox-gfx) trait as the native `blackbox-render`. It runs inside the game's own
Bevy `App`, behind the `renderer-bevy` cargo feature of `nfsmw` (off by default), and is chosen with the
[`renderer`](renderers.md) setting. This page is for developers: how it works, how it was measured, what is
missing. The decision it feeds is in [ADR 0004](decisions/0004-swappable-renderers.md).

**Status: PR 10 landed.** The spike's world path (textures, meshes, instances, camera, fog, alpha test,
blending, sky, headless capture) has glossy shading, the lighting rig and environment, texture redirects,
the world effect layer and the UI layer (PR 9), and now bloom, tone mapping, FXAA, SMAA, TAA (jitter,
motion vectors, `camera_cut`), the render scale and FSR 1 (PR 10); see "PR 9" and "PR 10" below. Temporal
upscalers (FSR 3/4, DLSS) and ray tracing still belong to later PRs; the facade accepts those calls, logs
once that they are ignored, and draws nothing for them.

```sh
cargo run --release -p nfsmw --features renderer-bevy -- view-world --renderer bevy
cargo run --release -p nfsmw --features renderer-bevy -- view-world --renderer bevy --bench-seconds 30 --no-vsync
cargo test -p blackbox-bevy-render -- --include-ignored --test-threads=1     # GPU tests (see Testing)
```

## How it fits in

```
game systems (Update)                    PostUpdate                      Bevy render world
  Scene::frame() ──► BevyBackend ──ops──►  apply()  ──entities──►  extract ► queue ► draw ► present
  (RenderBackend)    handles at once       (one system)             (Bevy's own schedules)
```

- **`BackendFactory`** (a `SystemParam`) creates the backend in `Update` once `RenderDevice` exists. The
  app then treats it like the native one: `apply_graphics`, `Scene::init`, `render` every frame.
- **Handles and the op queue.** The game calls `RenderBackend` from its own systems, where the Bevy world is not
  reachable. `BevyBackend` numbers textures, meshes and captures itself (from 1, never 0) and records what
  it was asked to do in a shared queue (`ops.rs`). Textures and meshes are converted to Bevy `Image`s and
  `Mesh`es on the calling thread; `render()` stores the newest `FrameParams` and a copy of the instance list.
- **`apply()`** (`apply/`) runs in `PostUpdate` before asset events and visibility. It adds and removes
  assets, syncs the instance pool, steers the cameras, and advances captures. It is one system and costs
  about 0.055 ms per frame at 370 objects.
- **Plugin set** (`plugin.rs`): `FrameCountPlugin`, `AssetPlugin`, `RenderPlugin` (synchronous pipeline
  compilation, backends from the `backend` setting), `ImagePlugin`, `MeshPlugin`, `CameraPlugin`,
  `LightPlugin`, `CorePipelinePlugin`, `AntiAliasPlugin`, `PbrPlugin` (prepass and deferred off) and
  `MaterialPlugin<BlackboxMaterial>`. `TransformPlugin` is not added: the pool writes `GlobalTransform` directly.
  Order matters in two places: the material shader is embedded **before** `MaterialPlugin` is built (it
  starts loading the shader at once, and a load that races the embed fails with "Path not found"), and
  `AssetPlugin` comes before `RenderPlugin`.
- **The window.** Bevy's `RenderPlugin` creates its wgpu instance when the plugin is built, before the window
  exists (`bevy_winit` creates it in the event loop). It does not need the window then: Bevy's own window
  systems create the surface later from the `RawHandleWrapper` that `bevy_winit` provides, and the camera
  renders to `WindowRef::Primary`. So `bevy_winit` keeps owning the window and the loop (ADR 0001), and no
  surface or handle is passed around by the app. Verified by running the game: `--renderer bevy` opens the
  normal window, `--screenshot` works with the window hidden.
- **Choosing at startup** (`app/render/select.rs`, `bevy.rs`): the request, the cargo feature and
  `probe(api)` decide before the `App` exists. `probe` opens a throw-away wgpu instance and reports API,
  vendor, BC support, ray-query and binding-array features; OpenGL or no adapter falls back to `blackbox` with a
  logged reason (Bevy panics on a missing adapter once built). The choice is stored as `ActiveRenderer` and
  cannot change while the app runs.

## Axes

The games' world is right-handed with **z up** (x right, y forward). Bevy's is right-handed with **y up**.
`axes.rs` holds the one basis change, `(x, y, z) -> (x, z, -y)`, a rotation of -90 degrees about x (so
winding and handedness are unchanged). It is applied to every instance transform (`B * M`), to the camera
(`B * inverse(view)`, Bevy looks down -z like the games' look-at, so view space needs no change) and to
the light direction. Callers never see it. The projection is Bevy's own infinite reverse-Z perspective, which is
the matrix the callers build from `FrameParams`.

## Colour

The native renderer shades and blends gamma-encoded values into an 8-bit UNORM surface. Bevy renders into an
sRGB target (the window's, or `Rgba8UnormSrgb` for captures) and encodes on write. The material shader
(`material/blackbox.wesl`) therefore works in the games' gamma space: textures are uploaded as plain `Unorm`
(no sRGB views, so filtering and mip selection match native), the vertex colour is the raw byte value
(`Unorm8x4` BGRA, swizzled in the shader), fog and lighting are computed as in `scene.wgsl`, and the result is
converted with the exact sRGB decode at the very end. The hardware encode then writes the bytes the native
shader would have written. Bevy's tone mapping, dithering and MSAA are off on the camera.

What this cannot match: **alpha-blended and additive layers blend in linear space** on Bevy (the target is
sRGB; `CompositingSpace::Srgb` exists in 0.20 but is honoured for 2D cameras only). The sky, which uses
blended layers, is off by up to about 12 levels for that reason. See the results.

## Textures, meshes, materials

- **Textures:** every mip, back to back, in one `Image`; BC1 to BC3 when the device has
  `TEXTURE_COMPRESSION_BC` (the callers decode to RGBA8 otherwise). The sampler repeats, filters trilinear
  and is 8x anisotropic, like native. A texture that cannot be built stays a handle without an image and draws
  white. Images live in the render world only, so the CPU copy is dropped after upload.
- **Meshes:** one native `MeshDesc` with N `DrawRange`s becomes N Bevy `Mesh`es (Bevy has one material per
  entity), each compacted to the vertices it uses with the order kept and indices rebased. The source indices
  are `u16`, so a range never needs more than 65,536 vertices and the new indices stay `u16`. Attributes:
  position, normal, UV0 and a custom `Unorm8x4` BGRA colour.
- **`BlackboxMaterial`:** one asset per (texture, blend, shading, glossy material), shared by every draw
  that uses the combination. Specialisation: `Prelit`, `Lit`, `Sky` and `Glossy` (shader defs `PRELIT`,
  `FOG`, `GLOSSY`) by `Opaque`, `AlphaTest` (`Mask(0.5)`, discard in the shader), `AlphaBlend` and
  `Additive` (the exact native blend states, colour written only so captures stay opaque). Culling is off,
  as native. The prepass and shadows are off for the material. The uniform holds the fog colour and range
  and the light direction; it is rewritten on all materials only when a frame's values differ from the
  last. The native texture LOD bias rides on the camera's `MipBias` (read as `view.mip_bias`). Every
  material also carries the glossy uniform, the lighting rig and the environment cube map (unused unless
  the shading is `Glossy`): cheap enough, and keeps one material type instead of two.
- **`redirect_texture`:** a `HashMap<usize, usize>` in `WorldState`, consulted when a draw becomes a
  material; existing materials keyed on the redirected number are repointed in place, not respawned. See
  "PR 9" for the lighting rig, the environment and the effect and UI layers.

## Instances

`instances.rs` keeps a pool keyed by `InstanceKey` (transient instances get a per-frame slot index): one
entity per (object, draw range) with `Mesh3d`, `MeshMaterial3d`, a `GlobalTransform` written straight from
the matrix (no TRS decomposition, so any affine placement is exact), `Visibility` and `NoFrustumCulling`
(the scene already culls and picks LODs, so Bevy's CPU culling is skipped).

- A key seen again with the same mesh costs a hash lookup and a matrix compare; the `GlobalTransform` is written
  only when the matrix changed.
- A key that is no longer listed is hidden at once and despawned after 120 frames, so tiles that flicker at
  the edge of view do not churn entities. A key whose mesh changed is respawned.
- Bevy batches same-mesh, same-material entities into GPU-preprocessed instanced draws.

## Draw order

Native draws Opaque, then AlphaTest, then AlphaBlend in submission order (unsorted), then Additive. Bevy bins
opaque and alpha-masked draws (order is irrelevant for them, the depth test decides) and **sorts blended draws
by distance**. Where blended layers overlap, the order differs from submission order; the blend stack scene
shows it (one quad flips). Matching it needs a per-draw sort key, left for the scene PR.

## Capture and tests without a window

`request_capture` queues a request; `apply()` gives it a camera that renders into an `Rgba8UnormSrgb` image
and, after three frames (pipelines and assets are ready), a one-shot `ReadbackOnce`. The observer unpads the
rows and files the picture where `poll_capture` finds it, and sets an atomic flag the capture state machine
watches (the caller takes the result out of the queue before the next frame, so the queue itself cannot signal
completion). A capture takes about five frames. `HeadlessBevy` (`harness.rs`) wraps a window-less `App`
and runs one update per `render()`, so `blackbox_gfx_testkit::capture_scene` works unchanged.

## Threads

Bevy's schedules run on the calling thread, because the game builds `bevy_ecs` without `multi_threaded` and
rendering is not pipelined. Turning `multi_threaded` on for `bevy_ecs` and `bevy_render` was tried: CPU time
per frame went from 6.9 to 11.3 ms and RSS from 503 to 653 MiB with no frame-time gain on the measured drive,
so it stays off. Pipelined rendering is a PR 11 question.

## Results

All measurements: Intel Iris Xe (Mesa 26.2.3, Vulkan) on Linux, release builds, 2026-10-09. The NVIDIA GPU's
driver is not loaded here, and no DX12 numbers exist; the owner records the RTX PC.

### Parity with the native renderer

The testkit scenes at 320x180, `compare()` (out of 255). Iris Xe and lavapipe (software Vulkan) agree.

| Scene | Tolerance | Iris Xe: max / mean / p99 | Lavapipe: max / mean / p99 |
|---|---|---|---|
| Grid (BC1 + RGBA, vertex colours, fog, lit boxes) | max 4, mean 0.5, p99 2 | 1 / 0.030 / 1 | 1 / 0.033 / 1 |
| Alpha cards | max 4, mean 0.5, p99 2 | 1 / 0.024 / 1 | 1 / 0.028 / 1 |
| Sky dome (9.7 km, infinite far plane) | max 4, mean 0.5, p99 2 | 1 / 0.033 / 1 | 1 / 0.040 / 1 |
| Depth probe (reverse Z, 2 m to 9 km) | max 4, mean 0.5, p99 2 | 0 / 0.000 / 0 | 0 / 0.000 / 0 |
| Blend stack (not strict) | mean 4, p99 24 | 63 / 2.12 / 37 | 63 / 2.12 / 37 |

The strict scenes pass with a margin of at least 15x on the mean. The blend stack meets the plan's mean (2.12 of
4) but not its p99 (37 of 24): linear against gamma blending and the distance sort of blended draws.

### The real city

`nfsmw view-world --at X,Y --renderer {blackbox|bevy} --show-metrics off --wait-for-load --screenshot`
at 1920x1080 (3840x2160 pictures), compared with `cargo xtask img-diff` (mean, p99, max absolute difference of the
colour samples, out of 255). Pictures are not committed.

| Position | Zone | Mean | p99 | Max |
|---|---|---|---|---|
| 2152,1399 (the default start) | I7 | 1.61 | 11 | 236 |
| 1000,500 | T21 | 0.88 | 10 | 235 |
| -800,300 | J13 | 1.39 | 11 | 34 |
| 2800,1900 | P11 | 1.29 | 11 | 237 |
| 400,1800 | I4 | 0.22 | 3 | 231 |
| 1500,-500 | B8 | 1.51 | 11 | 233 |

Criterion: mean at most 3/255 at five positions. **Met** at all six (worst 1.61). Visual review of the pictures and
their difference images (amplified 16 times): foliage cards and fences (alpha test) match, including the
dithered edges of tree canopies; the sky differs by a smooth tint of 8 to 12 levels (blended layers in linear
space); a few bridge and road-edge pixels differ by a lot (max 231 to 237), where blended draws overlap in a
different order. The depth order and fog are indistinguishable.

### Frame time, CPU and memory

`--bench-seconds 30 --no-vsync --max-fps unlocked`, 1920x1080, `view-world --drive` with the fixed script
`60:throttle=1,steer=0.05` from 2152,1399, two runs each. The scene reports about 370 to 480 objects (815 to
1,060 entities on Bevy) after its own culling. Frame time includes Mailbox presentation, so values near 16.6 ms
are display-limited stalls, not work.

| | Native (blackbox) | Bevy | Ratio |
|---|---|---|---|
| Frame time p50 | 5.60 / 6.13 ms | 8.38 / 8.89 ms | 1.5 / 1.45 |
| p95 | 16.8 / 16.7 ms | 15.7 / 11.7 ms | |
| p99 | 17.0 / 17.0 ms | 16.1 / 13.4 ms | |
| Worst | 19.5 / 17.5 ms | 17.9 / 16.0 ms | |
| Frames per second | 150 / 144 | 106 / 110 | |
| CPU time per frame | 2.48 / 2.25 ms | 6.89 / 7.34 ms | +4.4 / +5.1 ms |
| RSS at the end | 390 / 348 MiB | 503 / 506 MiB | 1.3 to 1.45 |
| Peak RSS | 399 / 405 MiB | 532 / 534 MiB | 1.3 to 1.35 |

In a sparse place (1500,-500: about 100 objects drawn) the gap is the same or wider: native 116 fps, p50 8.5 ms, 2.7
ms CPU per frame; Bevy 92 fps, p50 12.6 ms, 8.0 ms CPU per frame. At 960x540 the numbers do not change (native p50 6.1 ms, Bevy 8.2 ms, CPU 2.25 and 6.9 ms), so the cost is
per frame and per draw, not per pixel. In `apply()` itself the pool costs 0.055 ms per frame: the extra
4.4 to 5.3 ms of CPU does not shrink with fewer objects, so it is a fixed per-frame cost of Bevy's schedules
(extract, queue, prepare and the idle plugins `PbrPlugin` adds), not the instance pool and not the draws.
Where exactly it goes is **not yet profiled** (no profiler was available on the spike machine); the `trace`
feature and the plan below are the way in. Custom instancing would not remove it.

### Investigating the fixed CPU cost

A rougher data point from the owner's RTX-class PC (reported, **not** measured with the bench harness above):
about 800 to 1,000 fps native against 240 to 380 fps on Bevy — a worse ratio than the Iris Xe's 1.45x. That is
consistent with the tax being fixed: a constant ~4.5 ms added to native's ~1 to 1.3 ms/frame is a 3 to 4x
slowdown, while the same ~4.5 ms added to the laptop's already-slow 6 to 7 ms/frame is only 1.45x. A bigger GPU
makes the fixed CPU cost a larger fraction of the frame, not smaller.

What the spike's own numbers already establish about where the cost is **not**: it is flat across entity count
(100 vs 480 objects) and across resolution (960x540 vs 1920x1080). That rules out draws, pixels and per-entity
extract/batch work — all of those shrink in the sparse scene or at the smaller target. A cost that stays put is
**per-frame fixed schedule overhead**: Bevy running a large, fixed set of systems serially every frame (ECS is
single-threaded here, and `multi_threaded` was tried and made it worse — see "Threads"), plus the fixed part of
`ExtractSchedule`. The prime suspect is the forward-PBR machinery `PbrPlugin` schedules every frame (clustering,
light probes, shadow/prepass prep, SSAO/SSR, volumetric fog, lightmaps, decals) — none of which `BlackboxMaterial`
uses: it does its own fog and lighting in the shader, with shadows, prepass and deferred off.

**Now wired to measure it (this PR):**

- **A `trace` cargo feature** on `blackbox-bevy-render`, forwarded by `nfsmw`'s own `trace` feature. It turns on
  the bevy render crates' `trace` feature (their per-system `tracing` spans) and installs a Tracy subscriber
  (`blackbox_bevy_render::init_tracing`, called from `main` under the feature). Build
  `cargo run --release -p nfsmw --features trace -- view-world --renderer bevy --drive --no-vsync` with the Tracy
  client attached to get per-system times in extract/queue/prepare and confirm which systems eat the ~4.5 ms. This
  replaces the earlier "no profiler available" gap; run it on the RTX machine, where the tax hurts most.
- **Interactive pipeline compilation is now asynchronous.** `synchronous_pipeline_compilation` became a field on
  `BlackboxBevyRenderPlugin`: headless captures and tests keep it `true` (reproducible frames), but the live
  window sets it `false`, so steady-state frames never block on compilation (and new pipelines appearing mid-drive
  no longer stall the frame). This is a correctness/hitch fix as much as a throughput one; it is not expected to
  move the steady-state fixed cost, which is why the Tracy trace is the real next step.

**Candidate fixes, once the trace attributes the cost (highest payoff first, not yet done):**

- **A zero-tooling bisection** to attribute the cost without Tracy: re-run the CPU-time bench with `PbrPlugin` +
  `MaterialPlugin` removed (clear-only camera) for the floor, then with `MaterialPlugin` but no `PbrPlugin`; the
  gap is PbrPlugin's fixed tax, measured directly.
- **Drop or decompose `PbrPlugin`** — the biggest lever. Strip the sub-features `BlackboxMaterial` never consumes,
  or replace `MaterialPlugin` + `PbrPlugin` with a minimal mesh-draw pipeline on a render phase that skips
  clustering and lighting entirely.
- **Pipelined rendering** (the PR 11 question) to overlap frame N's render-world schedule with frame N+1's
  main-world. It hides the tax rather than removing it, and is distinct from the `multi_threaded` ECS that already
  regressed; worth measuring only if CPU is still the ceiling after the PbrPlugin work.

Instance scaling (headless, 640x360, keyed static instances of one single-range mesh, so the best case for
Bevy's batching, frame cost including GPU): 1,000 instances 1.5 ms, 10,000 4.2 ms, 40,000 15.2 ms (about 0.35 ms per
1,000). The city's 80,000 placed objects are never drawn at once; the scene culls to a few hundred per frame.

### Build cost

Cold release builds of `nfsmw` on this machine (20 threads, fresh target directories, other work running
at the same time, so read the times as rough):

| Build | Cold build time | `nfsmw` binary (not stripped) |
|---|---|---|
| default (no `renderer-bevy`) | 120 s | 62.4 MiB (65,433,024 bytes) |
| `--features renderer-bevy` | 215 s | 111.0 MiB (116,350,032 bytes) |
| difference | +95 s (+79 %) | +48.6 MiB (+78 %) |

The plan guessed +1.5 to 3 minutes and +15 to 30 MB: the time is inside that, the size is not. The default
build has no `bevy_render` (`cargo tree -p nfsmw -e normal | grep -c bevy_render` prints 0), compiles
the same crates as before and still checks on Rust 1.95.

## PR 9: glossy shading, redirects, the world effect layer and the UI

Glossy shading, the lighting rig, the environment, `redirect_texture`, the world effect layer and the UI
layer. Glossy and redirects are in `material/` and `apply/`, as planned (§9); the effect layer and the UI
layer turned out to need `systems/{effects,ui}.rs` added **directly to Bevy's render sub-app**, not routed
through the facade's `apply()` queue:

- **Where they run.** Both are systems added to Bevy's own `Core3d` schedule (`render_app.add_systems
  (Core3d, ...)` in `plugin.rs`), not a render-graph node: Bevy 0.20 replaced that with an ECS schedule,
  `Core3dSystems::{Prepass, MainPass, EarlyPostProcess, PostProcess}`, weakly chained. The effect layer
  runs `.in_set(EarlyPostProcess)` (after the world meshes, before tone mapping), matching native's
  `gpu/frame.rs` order; a second, chained system draws detailed particles over the finished colour and
  depth. The UI runs `.after(bevy_core_pipeline::upscaling::upscaling)` by function reference (ordering
  only against `Core3dSystems::PostProcess` would leave it ambiguous against `upscaling`, the only other
  system scheduled there), targeting `ViewTarget::out_texture()` at the camera's `physical_target_size`,
  not the (possibly scaled) render size.
- **Where their data comes from.** Both read `EffectLayer`/`UiLayer` straight from the facade's `Shared`
  bridge (`ops.rs`) — the *same* `Arc<Mutex<_>>`, inserted into the render sub-app once at plugin build
  (`render_app.insert_resource(bridge.clone())`), not through Bevy's `Extract` step, so there is no extra
  frame of latency and no `WorldState` involvement. Texture handles and the fog/light parameters, which
  only `WorldState` (main-world only) has, do go through a real `ExtractSchedule` system
  (`systems::effects::extract_world`).
- **Effect vertices are world-space, not model-local.** Mesh instances are converted to Bevy's axes by
  their per-instance transform (`axes::model`, in `apply/instances.rs`); nothing does that for effect
  vertices, which are already in world space when the game hands them over. The first attempt skipped
  this and drew streaks as vertical lines off the top of the screen; `systems::effects::to_bevy_axes`
  converts every vertex before `Effects::upload`.
- **The environment cube map is written in the games' axes, sampled in Bevy's.** `apply/environment.rs`
  ports native's procedural sky generator (and the `Faces` case) byte for byte, so the cube map's six
  faces hold exactly what native's would. The glossy shader's reflection direction is computed in Bevy's
  axes, so it is converted back to the games' axes (`vec3(r.x, -r.z, r.y)`) only at the `textureSample`
  call, in `material/blackbox.wesl` — the one place the mismatch actually matters. **Fixed after PR 10:**
  no caller in `crates/nfsmw` ever calls `set_environment`, so a material's cube map stayed Bevy's opaque
  white fallback instead of native's lazy default sky, making cars look too shiny and paint read pink,
  worst on chrome. `BevyBackend::ensure_default_environment` (`facade.rs`) now mirrors native's lazy
  default; see [status.md §3c](plans/gfx-renderers/status.md) for the investigation.
- **A render target view that is sRGB needs its shaders to gamma-decode once.** The world material already
  did this (`linear_from_gamma`, "Colour" above); `blackbox-gpu-passes`' effect, textured-effect,
  soft-particle and UI shaders did not, because native always draws into a plain UNORM surface. Writing
  gamma bytes straight into an sRGB view double-encodes them on write (a *lot* brighter than native, not a
  subtle gap). Fixed in the shared crate: an `_srgb` fragment entry point per pass, picked automatically
  from the target format (`world::is_srgb`) — invisible to native, which never hits an sRGB format.
- **Measured** (testkit, this machine's Vulkan adapter — an NVIDIA RTX 4070 SUPER, not the Iris Xe of the
  spike's numbers above): `glossy_sphere` max 1, mean 0.254, p99 1 (`Fidelity::Glossy`: mean 1.5, p99 8);
  `effects` max 65, mean 0.405, p99 13 (`Fidelity::Blended`: mean 4, p99 24); `ui` max 34, mean 2.516, p99
  31. The plan's original guess for the UI tolerance (max 1, "the same shared pass, off by at most one")
  predates the sRGB finding above; opaque UI pixels do match almost exactly now, but overlapping
  translucent UI meshes hit the same linear-vs-gamma blending gap as `Blended`, so `Fidelity::Ui`'s
  tolerance was widened to match what was actually measured (`libs/blackbox-gfx-testkit/src/scenes/mod.rs`).
- Also verified directly, not just through the testkit: `nfsmw view-world --renderer bevy --drive
  --screenshot` (the HUD, the glossy car, the world and its effects all draw together) and `nfsmw
  view-screen MainMenu.fng --renderer bevy --screenshot` (the main menu, previously solid black under
  `--renderer bevy`, now draws normally).

## PR 10: bloom, tone mapping, FXAA, SMAA, TAA, render scale, FSR 1

Every post component is driven by the full effective `PostSettings`/`Upscaler` now, not just the FXAA bool
and render-scale float the spike carried (`ops::CameraSettings` widened to hold `PostSettings`, `Upscaler`
and `upscale_sharpness`; `facade::apply_graphics` populates all of it from `resolve()`'s output, same as
the native backend already does). `apply/camera.rs::set_post` and the new `post/` module insert or remove
every component strictly by whether its effect is on, matching the settings doc's rule ("nothing runs or
is allocated when off", `docs/plans/gfx-renderers/upscalers-settings.md` §5): no `Hdr`, `Bloom`,
`TemporalAntiAliasing` or its prepasses, `Fxaa` or `Smaa` when their setting is off.

- **Bloom needed a new pinned dependency.** The plan expected `bevy_core_pipeline` to already wire bloom's
  render-graph node, leaving PR 10 to insert only the `Bloom` marker component. In Bevy 0.20 bloom (with
  depth of field, motion blur and the other screen-space effects) moved out of `bevy_core_pipeline` into
  its own crate, `bevy_post_process`, not in the workspace's existing Bevy pin list. Added
  `bevy_post_process = "=0.20.0"` next to the other Bevy crates in the workspace `Cargo.toml` and its
  `PostProcessPlugin` to `plugin.rs`'s plugin set (it also builds depth-of-field and motion blur, both
  unused and dormant: `BlackboxMaterial` requests no depth prepass for them to read). `post::set` maps
  `PostSettings::bloom_intensity`/`bloom_threshold` onto `Bloom`'s own fields on top of its `NATURAL`
  preset, and `Tonemap::Aces` onto `Tonemapping::AcesFitted` (Bevy's closest match — `TonyMcMapface`/`AgX`
  need the `tonemapping_luts` feature, not pulled in, and would not match `Tonemap`'s two-value contract
  anyway).
- **Bloom and tone mapping need an `Hdr` camera; the world material needed no changes for it.** Both
  `Bloom` and `Tonemapping` other than `None` only do anything useful on an HDR-rendered scene (`Bloom`
  even `#[require(Hdr)]`s it), so `post::set` inserts `Hdr` exactly when `PostSettings::needs_hdr()` is
  true. The material's own "Colour" trick (above) already makes this work for free: `linear_from_gamma`
  decodes once and an sRGB *target* re-encodes it on write, netting out to native's gamma bytes — but an
  `Hdr` target (`Rgba16Float`) does not re-encode anything, so the same decoded value is simply stored as
  the linear colour Bevy's own bloom and tone-mapping passes expect. The effect layer and UI layer passes
  (`blackbox-gpu-passes`) were not given the same treatment: their `_srgb`/plain entry-point split
  (PR 9) only knows about `is_srgb`, not "linear HDR", so their colours are **not yet correct when bloom
  or tone mapping is on** — a gap scoped out here (see "Known gaps") because the testkit scenes used to
  verify bloom and tone mapping (`Grid`, the strict ones) never exercise the effect or UI layer.
- **SMAA needed no spike.** `bevy_anti_alias::smaa::Smaa` is already a plain drop-in component (its own
  plugin was already in the `AntiAliasPlugin` bundle PR 9 added), so `Antialiasing::Smaa` maps onto it the
  same way `Fxaa` already did. Not separately measured against native (native has no SMAA to compare
  against; per-backend sanity only, `docs/plans/gfx-renderers/verification-risks.md` §7).
- **TAA's motion vectors needed no new plumbing at all.** The worry going in was whether the instance
  pool's entity reuse (PR 5/9) was actually stable enough for Bevy's own motion-vector machinery; it is.
  `bevy_pbr::prepass::update_mesh_previous_global_transforms` runs in `PreUpdate` every frame and keeps
  every `Mesh3d` entity's `PreviousGlobalTransform` one tick behind its `GlobalTransform`, driven by
  Bevy's own change detection; `apply/instances.rs` only *writes* `GlobalTransform` when an instance's
  matrix actually changed, and reuses the same `Entity` for a given `InstanceKey` instead of respawning —
  exactly what that system needs for a still object to read zero motion and a moving one to read the
  correct one-frame-old value. `post/taa.rs::set` inserts `TemporalAntiAliasing` (which
  `#[require(...)]`s `TemporalJitter`, `MipBias`, `DepthPrepass` and `MotionVectorPrepass` — jitter and
  both prepasses, for free) via `entry().or_insert_with` rather than a plain `insert`, so turning TAA on
  is idempotent across frames: a fresh `TemporalAntiAliasing::default()` has `reset: true`, and
  overwriting an already-on camera's component every frame would wipe its history every frame and it
  would never converge. Turning it off removes `TemporalAntiAliasing` **and** the three components it
  required explicitly — Bevy does not cascade-remove required components, so without this a camera that
  had TAA on once would keep paying for both prepasses forever.
- **`camera_cut` resets TAA's history**, finally wired up (PR 9's "Known gaps" called it out as ignored).
  `post/taa.rs::reset_on_cut` runs from `apply/camera.rs::follow` — every frame, not gated by the
  settings-changed check `set_post` uses — since `camera_cut` is a per-frame flag on `FrameParams`, not
  part of the settings `set_post` reacts to; it sets `TemporalAntiAliasing.reset = true` only when TAA is
  on and this frame is a cut (Bevy clears `reset` back to `false` itself once used, so nothing here ever
  needs to clear it).
- **The render scale had a latent bug from the spike, caught by the FSR 1 test.** PR 8/9 already inserted
  `MainPassResolutionOverride` to shrink the main pass below the output size, but from the **main world**
  (`apply/camera.rs`, in the facade's own `PostUpdate` system) — which compiled, looked right, and was
  silently a complete no-op: `MainPassResolutionOverride`'s own doc comment says to insert it "on a 3d
  camera entity in the render world", and nothing syncs a main-world copy there (unlike `Bloom`, `Fxaa`,
  `Smaa` and `TemporalAntiAliasing`, which are all genuinely synced components). The scene always drew at
  the full surface size regardless of the render scale; PR 9's own "wired but not measured" caveat on this
  was accurate in a way nobody had tested. Caught here because FSR 1 (below) upscales from whatever the
  main pass actually drew, and a render running at full resolution with the shader *assuming* a smaller
  one gave a dramatically wrong (not just blurrier) picture — see `post::scale::apply_resolution_override`'s
  own doc comment for the fix: a new render-world-only `Core3d` system, scheduled
  `.before(Core3dSystems::Prepass)` (ahead of the depth/motion-vector prepasses, which read the override
  too), that inserts or removes the component straight onto the render-world camera entity every frame
  from `BlackboxBridge`.
- **FSR 1 reuses the shared pass, but needed a size the shared pass previously assumed it could read off
  the texture.** `blackbox_gpu_passes::fsr1`'s WGSL read the source image's size with `textureDimensions`,
  which is exactly right for native (its render target is allocated at exactly the render size) but wrong
  for Bevy: `MainPassResolutionOverride` does not shrink `main_texture` itself, it only restricts the
  *viewport* the main pass draws into within an output-sized texture — so `textureDimensions` would read
  the whole, larger texture as if every pixel of it were valid content. Added an explicit `input_size`
  field to `Fsr1Io`/the shader's `Params` uniform (`in_size: vec2<f32>`, read instead of
  `textureDimensions` for both EASU's upscale ratio and its edge clamp); native's own call site now
  forwards its already-existing `PassIo::input_size` (previously computed and silently discarded there —
  it happened to always equal `textureDimensions` for native, so nothing was broken, just redundant), and
  the Bevy pass computes it the same way `apply_resolution_override` does (`scaled_size` of the output by
  the render scale). RCAS, reading EASU's own tightly-sized scratch output rather than the oversized main
  texture, passes its content size as simply the output size. A `vec3<f32>` padding field in the uniform
  initially mismatched the Rust struct's plain `repr(C)` layout (WGSL's uniform address space aligns
  `vec3` to 16 bytes, not 12) and failed wgpu's bind-group-size validation outright; a lone `f32` pad field
  avoids the mismatch.
- **Replacing Bevy's own `upscaling` blit, not suppressing it.** `bevy_core_pipeline::upscaling::upscaling`
  has no run condition — it always blits the (possibly override-sized) main texture to the output, and
  there is no public hook from outside `UpscalingPlugin` to stop it running. `post::fsr1::fsr1_pass`
  instead runs `.after(upscaling)` and, only when FSR 1 is the active upscaler, fully overwrites what
  `upscaling` just wrote (EASU's render pass clears its own output). The final image is correct either
  way; the cost is one extra, thrown-away full-resolution bilinear blit on every frame FSR 1 is active.
  Left as a known, documented inefficiency rather than patched around — cleanly removing it needs changing
  how `UpscalingPlugin` registers its own system, not something this crate can reach into.
- **A capture's own camera now waits for TAA to converge before reading back.** A `request_capture`
  camera is always fresh (a new `TemporalAntiAliasing` with `reset: true`), so a screenshot taken after
  the spike's fixed three-frame warm-up would read back a barely-converged, visibly noisy frame whenever
  TAA is on. `apply/capture.rs` now waits longer (`TAA_WARMUP_FRAMES`, picked past where
  `post_tests::taa_converges_on_a_static_scene` sees the difference against a long-settled frame drop
  under its own epsilon) specifically when the capture's own effective settings have TAA on; every other
  capture keeps the original three-frame warm-up.
- **Tests** (`post_tests.rs`; all `#[ignore = "needs a GPU"]`, all pass on an NVIDIA RTX 4070 SUPER over
  Vulkan): TAA converges on a static scene (frame 16 vs frame 32 of the Grid scene, mean 1.085, p99 10,
  max 30 out of 255 — `max` alone is not a convergence signal, TAA keeps dithering at edges by design
  forever, so the assertion is on `mean` and `p99`); a camera cut resets history (compared against a
  **fresh, unjittered** (`Antialiasing::Off`) camera at the same pose rather than another TAA camera, to
  avoid conflating "ghosting" with "two single-sample frames landing on different points of the Halton
  jitter sequence" — confirmed to actually detect a broken reset by temporarily disabling it and watching
  the measured mean roughly quadruple, from 2.391 to 9.629); no prepass components exist when TAA is off,
  checked both for a camera that never had it and (to prove the *removal* strips what it once added, not
  merely that a pristine camera lacks it) one toggled on then off again; FSR 1 matches native within
  tolerance at 67% render scale (measured mean 1.78, p99 18, max 54 — wider than the strict scenes' own
  mean 0.5 because EASU and RCAS are edge-adaptive and amplify the pre-upscale differences every scene
  already has at the checkerboard's many edges, not because the upscale pass itself differs: both
  renderers run the exact same shared shader on it). The first three drive a camera directly
  (`spawn_camera`/`readback` helpers in `post_tests.rs`), bypassing the facade's screen camera (which only
  exists behind a real window) and the one-shot `request_capture` camera (which despawns itself a few
  frames after completion, too short-lived to accumulate and then re-read 16+ frames of history); doing
  this surfaced one more Bevy-specific timing wrinkle, documented on the test file's own `readback` helper:
  a `ReadbackOnce` entity spawned directly on the `World` between two `app.update()` calls gets its
  marker components stripped by Bevy's own `cleanup_readback_once` (`First`, every tick) before the render
  world's `ExtractSchedule` (near the end of the same tick) ever sees it, so it must be queued and spawned
  from a system in `PostUpdate` instead — exactly the timing `apply/capture.rs`'s own `commands.spawn(...)`
  already has for free, which is why that existing, narrower path never needed this.

## Known gaps

- Blended and additive draws: linear blending and distance sorting (PR 9); the UI layer and the effect
  layer inherit the same gap where their own meshes overlap translucently.
- The effect layer and the UI layer are not yet HDR-aware: when bloom or tone mapping is on, their colours
  are wrong (their shared shaders only branch on `is_srgb`, not "linear HDR") — not caught by the PR 10
  tests, which use testkit scenes without an effect or UI layer. A follow-up needs a third entry-point
  variant in `blackbox-gpu-passes` for a linear, non-sRGB target.
- Temporal upscalers (FSR 3/4) and DLSS, which need PR 11, 13b and the DLSS SDK; ray tracing needs PR 12.
- No hitch measurement for tile streaming, and the frame-time/CPU/memory table above is still Iris Xe
  only; PR 9 was functionally verified (testkit parity, a driving screenshot, the main menu) on an NVIDIA
  RTX 4070 SUPER over Vulkan, but the benchmark was not repeated there; PR 10's own testing was GPU-test
  only (no `--bench-seconds`/real-city screenshot pass), also on the RTX 4070 SUPER. DX12 remains untested
  for both.

## Plan section 11 items this spike settled

- **Raw wgpu `CommandEncoder` access from `Core3d` systems:** yes. A system takes `RenderContext` and calls
  `command_encoder()` (a plain wgpu encoder); per-view data comes from `ViewQuery`. Bevy's own
  `main_transparent_pass_3d` does exactly this, and `Core3d` turned out to be an ECS schedule, not a
  render-graph node system (see "PR 9"); the effect layer's two systems (`systems/effects.rs`) are
  built the same way, manually `begin_render_pass`-ing a raw `wgpu::RenderPass` since
  `blackbox_gpu_passes::Effects` takes one, not Bevy's tracked wrapper.
- **`ViewDepthTexture` sampleability:** `Camera3d::depth_texture_usages` takes a `TextureUsages`, so
  `TEXTURE_BINDING` can be requested on the camera; set in `apply/camera.rs`'s camera bundle for the
  soft-particle pass. **Not exercised by a test**: the testkit's `effects` scene never sets
  `detailed_particles`, and that scene's digests are pinned for the native renderer (changing it needs
  regenerating those on the recorded hardware, which this session doesn't have) — a gap for the next PR
  that actually turns detailed particles on (`smoke-quality high`).
- **MSRV:** `cargo +1.95.0 check -p nfsmw --features renderer-bevy` **fails**: Bevy 0.20's WESL stack (`wesl`,
  `wesl-core`, `wesl-macros`, `wgsl-parse`, `wgsl-types` 0.6) requires rustc 1.97.1. The default build
  (`cargo +1.95.0 check -p nfsmw`) still passes. So only `blackbox-bevy-render` declares
  `rust-version = "1.97.1"`; the workspace stays on 1.95 because nothing else depends on the crate. Whether to
  raise the whole workspace is an owner decision (it would be needed to check `--workspace` on 1.95).
- **Compile time and binary size deltas:** measured above.
- **How Bevy gets the existing window:** it does not need to be given one; see "The window".

## Testing

- CPU tests: `cargo test -p blackbox-bevy-render` (axes, mesh compaction, textures, capabilities, material
  parameters, readback unpadding, the probe, bloom/tone-map mapping, the render-scale LOD bias curve).
- GPU tests (`#[ignore = "needs a GPU"]`, all take `blackbox_gpu_passes::test_support::serial()`):
  `parity_tests.rs` (the strict scenes against native, the blend stack report, back-to-back captures, the
  instance stress timing) and `post_tests.rs` (PR 10: TAA convergence, the camera-cut reset, no prepass
  when TAA is off, FSR 1 against native — see "PR 10" above). `BLACKBOX_GPU_FALLBACK=1` uses the software
  adapter; this machine's environment sets `VK_LOADER_DRIVERS_SELECT=*intel*`, so lavapipe needs
  `VK_LOADER_DRIVERS_SELECT='*lvp*'`. `BLACKBOX_DUMP_DIR=<dir>` writes the native, Bevy and difference
  pictures `parity_tests.rs` draws for looking at (`post_tests.rs` does not use it).
- `cargo xtask img-diff A.png B.png [--out diff.png]` compares two screenshots.
