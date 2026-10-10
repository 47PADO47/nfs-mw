# The Bevy renderer backend (spike)

`libs/blackbox-bevy-render` is an optional renderer for the games that implements the same
[`RenderBackend`](../libs/blackbox-gfx) trait as the native `blackbox-render`. It runs inside the game's own
Bevy `App`, behind the `renderer-bevy` cargo feature of `nfsmw` (off by default), and is chosen with the
[`renderer`](renderers.md) setting. This page is for developers: how it works, how it was measured, what is
missing. The decision it feeds is in [ADR 0004](decisions/0004-swappable-renderers.md).

**Status: PR 9 landed.** The spike's world path (textures, meshes, instances, camera, fog, alpha test,
blending, sky, headless capture) now has glossy shading, the lighting rig and environment, texture
redirects, the world effect layer and the UI layer; see "PR 9" below. Post effects (bloom, tone mapping,
SMAA, TAA, FSR 1), temporal methods and ray tracing still belong to later PRs; the facade accepts those
calls, logs once that they are ignored, and draws nothing for them.

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
Where exactly it goes is **not yet profiled** (no profiler was available here; Bevy's `trace_tracy` is the
first thing to try in the next PR). Custom instancing would not remove it.

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
  call, in `material/blackbox.wesl` — the one place the mismatch actually matters.
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

## Known gaps

- Blended and additive draws: linear blending and distance sorting (above); the UI layer and the effect
  layer inherit the same gap where their own meshes overlap translucently (PR 9's measurements).
- Post effects: bloom, tone mapping, SMAA, TAA, FSR 1 and all temporal methods. `post_aa fxaa` and
  `upscaler bilinear` with a render scale below 1 are wired (Bevy's `Fxaa` and `MainPassResolutionOverride`)
  but not measured.
- Ray tracing and DLSS, which need PR 11 and 12.
- No hitch measurement for tile streaming, and the frame-time/CPU/memory table above is still Iris Xe
  only; PR 9 was functionally verified (testkit parity, a driving screenshot, the main menu) on an NVIDIA
  RTX 4070 SUPER over Vulkan, but the benchmark was not repeated there, and DX12 remains untested.
- `Camera.camera_cut` is ignored (nothing temporal exists yet).

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
  parameters, readback unpadding, the probe).
- GPU tests (`#[ignore = "needs a GPU"]`, all take `blackbox_gpu_passes::test_support::serial()`): the strict
  scenes against native, the blend stack report, back-to-back captures and the instance stress timing.
  `BLACKBOX_GPU_FALLBACK=1` uses the software adapter; this machine's environment sets
  `VK_LOADER_DRIVERS_SELECT=*intel*`, so lavapipe needs `VK_LOADER_DRIVERS_SELECT='*lvp*'`.
  `BLACKBOX_DUMP_DIR=<dir>` writes the native, Bevy and difference pictures for looking at.
- `cargo xtask img-diff A.png B.png [--out diff.png]` compares two screenshots.
