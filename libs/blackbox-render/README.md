# blackbox-render

`EffectLayer` supports depth-tested surface overlays, circular particles and additive
streak triangles and additive radial glows. Streak UV x spans the width and UV y runs head to tail; the analytic
mask concentrates energy at the tip and dims the tail. Streaks use source-alpha additive RGB, no depth
writes or bias, and fade toward black in fog. Callers own geometry, lifetimes and
budgets; the renderer reuses vertex buffers. `streak_capacity()` reports allocation.
Glows use a soft radial mask with a brighter core and the same depth and fog rules.

Backend-neutral renderer for EA Black Box game reimplementations: meshes in the games' common 36-byte vertex
format, DXT or RGBA textures, instanced draws, opaque/alpha-test/blend/additive passes, lit or pre-lit
shading, distance fog, a 2D UI layer (textured, clipped, premultiplied-alpha triangles for consoles, overlays and menus), off-screen capture. Runs on wgpu (Vulkan, Direct3D 12, OpenGL; Metal on macOS).

License: MIT OR Apache-2.0.

The renderer-neutral types (meshes, textures, frame parameters, effect and UI layers, post and upscale settings,
the graphics API enum) live in [`blackbox-gfx`](../blackbox-gfx) and are re-exported from this crate.
The reusable wgpu passes (the UI layer, the world effects, soft particles, the fullscreen filter helper and
FSR 1) live in [`blackbox-gpu-passes`](../blackbox-gpu-passes), which this renderer calls from its frame; a
renderer built on another engine can call the same passes from its own render loop.

`EffectLayer` uploads reusable world-space triangle batches for depth-tested surface overlays
and soft alpha billboards. Effects draw after the scene and before UI; they do not write depth.
The caller owns lifetimes, budgets and particle ordering. Surface overlays have reverse-Z
polygon bias; both analytic masks are procedural and require no asset textures.

Detailed particles optionally use per-vertex age and seed for evolving procedural density,
and a world-unit intersection distance to fade against the completed opaque scene depth.
Their separate pass samples depth without attaching it, handles reverse-Z occlusion and
uses the capture target's depth during off-screen renders. These remain generic renderer
inputs; the application chooses presentation quality and emission budgets.

## As a `RenderBackend`

`Renderer` implements [`blackbox_gfx::RenderBackend`](../blackbox-gfx), so a scene can take
`&mut dyn RenderBackend` and run on any renderer. The inherent methods stay (the trait methods forward to
them). The game and `blackbox-scene` only use the trait; `nfsmw`'s `app/render/native.rs` is the one place
that names this crate.

- `capabilities()` is fixed for the renderer: anti-aliasing `{Off, Fxaa}`, upscalers `{Off, Bilinear, Fsr1}`,
  every tone map, bloom, no ray tracing, a render scale of 0.25 to 2.0 and nothing that needs a restart.
  `compressed_bc` and `hdr_targets` come from the adapter.
- `apply_graphics(&GraphicsSettings)` resolves the request against those capabilities and applies the effective
  settings with `set_post_effects`, `set_upscaler`, `set_upscale_sharpness`, `set_render_scale` and
  `set_texture_lod_bias(suggested_texture_lod_bias(scale))`. It returns the `Resolved` (effective settings, every
  downgrade with its reason, every note); `graphics()` reads the effective settings back. A request the renderer
  can run is applied exactly.
- `info()` names the renderer (`"blackbox"`), the API in use, the adapter and the driver.

### Headless and captures

`Renderer::headless((width, height), options)` creates a renderer with no window or surface: frames go into an
`Rgba8Unorm` texture. `render` draws into it (and returns `true`), `read_output()` reads it back, `resize`
recreates it. `RendererOptions::force_fallback_adapter` asks for the software adapter (lavapipe, WARP). Tests and
tools use it; the GPU tests in `gpu/parity/` run on it.

Captures are asynchronous in the trait: `request_capture(size, frame, instances)` returns a `CaptureId` and
`poll_capture(id)` returns the image once. This renderer draws and reads the frame back inside
`request_capture`, so the first poll has it; `capture()` still returns the bytes directly. The UI layer and the
post chain are part of a capture, as on screen.

### Tests

`cargo test -p blackbox-render --lib -- --include-ignored` runs the GPU tests (headless; see
[docs/testing.md](../../docs/testing.md#gpu-tests)). They render the
[`blackbox-gfx-testkit`](../blackbox-gfx-testkit) scenes under five setting combinations and compare 32x18 colour
digests recorded on Intel Iris Xe (Vulkan, Mesa) within +-2 per channel; the digests are Rust constants in
`gpu/parity/digests/`, and `BLACKBOX_UPDATE_DIGESTS=1` prints new ones. Any change to the native image needs an
intended digest update in the same commit.

## Render pipeline

`Renderer::render` and `Renderer::capture` pick the cheapest path that gives the requested look:

- **Direct (the default).** With no post effect, no upscaler and a render scale that gives the surface's own
  size, the scene and the world effects draw straight into the surface image, with a reverse-Z `Depth32Float`
  buffer at the same size. There is no offscreen colour image and no `resolve` copy: against the offscreen
  path that is one full-size `Rgba16Float` image (8 bytes a pixel, 28 MiB at 2560x1440) and one full-screen
  read and write less per frame, and the scene pipelines blend in the surface's 8-bit format. The
  result matches the offscreen path to within rounding (+-1 in a fifth of the pixels, where blended layers
  round once per layer instead of once at the end). `draws_directly()` reports it. A surface that encodes sRGB
  on write always takes the offscreen path, so blending stays linear.
- **Offscreen.** Any post pass, an upscaler or a render scale other than 1.0 sends the scene into an offscreen
  colour image and a depth buffer, both at the internal render size. The colour image is `Rgba16Float` (HDR)
  only when bloom or tone mapping runs and the adapter can render to, blend into and filter that format;
  FXAA, FSR 1 and the bilinear upscale draw into the surface's own 8-bit format instead (half the memory and
  bandwidth). An ordered chain of fullscreen post-process passes then reads the scene image (and may sample
  the depth buffer) and writes the surface; intermediate passes ping-pong through scratch images, which are
  freed when the chain gets shorter. The chain always ends with the built-in `resolve` pass: clamp to the
  output range, alpha 1, an exact texel copy when the render size equals the output size and bilinear
  filtering otherwise. The post effects sit at the front (see below) and upscalers are inserted before
  `resolve` (`gpu/post/`).
- The UI layer draws on the surface last, at surface resolution: it is never post-processed or upscaled.

Scene pipelines, effect pipelines and the glossy resources are built when first needed: the pipelines per
target format (a renderer that stays on the direct path never builds the `Rgba16Float` set), the glossy shader,
rig, environment cube map and bind groups on the first glossy call (`glossy_in_use()`).
Switching between paths recreates the targets; a path used before keeps its pipelines.

The internal render size is the surface size times the render scale, per axis:

- `Renderer::set_render_scale(scale)` clamps `scale` to `MIN_RENDER_SCALE..=MAX_RENDER_SCALE`
  (0.25..=2.0; non-finite values give 1.0) and resizes the offscreen targets. The default is
  `DEFAULT_RENDER_SCALE` (1.0), which with no post pass draws straight into the surface.
- `Renderer::resize` keeps the render scale; `render_scale()`, `render_size()`, `surface_size()`, `is_hdr()`,
  `draws_directly()` and
  `post_passes()` (the pass names in order) report the current state. `scaled_size(surface, scale)` is the pure size computation.
- `Renderer::capture(width, height, ...)` runs the same stages at the capture size and returns the final image.

### Post effects

`Renderer::set_post_effects(PostSettings)` turns on bloom, tone mapping and FXAA (`post_effects()` reads the
clamped settings back). The default `PostSettings` runs none of them, so the chain is just `resolve` and the
frame is unchanged. Enabled effects run at the render size in the fixed order bloom, tone mapping (an ACES fit
with an exposure), FXAA, then `resolve`; passes inserted by other code stay behind them. Each effect is its own
pass built on the fullscreen-filter helper of `blackbox-gpu-passes` with one WGSL module (its
`post_common.wgsl` plus the effect's file here); an effect that is off has no pass. Setting the current value again does nothing; changing it
rebuilds the effect passes. FXAA is an independent implementation of the published algorithm, not a copy of
the reference header.

### Upscaling

When the render scale is below 1.0 the scene is brought back to the output size by an upscaler:

- `Renderer::set_upscaler(Upscaler::Bilinear)` (default) leaves it to the resolve pass's bilinear filter.
- `Upscaler::Fsr1` adds two output-size passes before the resolve pass: AMD FidelityFX Super Resolution 1's EASU
  (edge-adaptive upscale) and RCAS (sharpening), ported to WGSL (`blackbox-gpu-passes`' `shaders/fsr1.wgsl`, MIT, notice kept in the file
  and in the repository's NOTICE). It expects an anti-aliased, display-referred input and clamps it to 0..1;
  anti-aliasing passes belong before it in the chain, which keeps output-size passes last.
  `set_upscale_sharpness(0.0..=1.0)` sets RCAS (0 skips the pass; the default is `DEFAULT_UPSCALE_SHARPNESS`).
- Neither runs at a render scale of 1.0 or more. `fsr1_active()` reports whether the FSR 1 passes are in the chain.
- `set_texture_lod_bias(bias)` adds a mip bias to the world's texture samples; `suggested_texture_lod_bias(scale)` is
  `log2(scale)` below native, 0.0 otherwise. Particles and the UI are not biased.

Not done, for DLSS and other temporal upscalers (docs/upscaling.md): projection jitter, previous transforms and a
motion-vector output. The depth buffer is already reverse-Z `Depth32Float`.
