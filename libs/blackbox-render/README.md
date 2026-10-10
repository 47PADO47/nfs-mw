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

`EffectLayer` uploads reusable world-space triangle batches for depth-tested surface overlays
and soft alpha billboards. Effects draw after the scene and before UI; they do not write depth.
The caller owns lifetimes, budgets and particle ordering. Surface overlays have reverse-Z
polygon bias; both analytic masks are procedural and require no asset textures.

Detailed particles optionally use per-vertex age and seed for evolving procedural density,
and a world-unit intersection distance to fade against the completed opaque scene depth.
Their separate pass samples depth without attaching it, handles reverse-Z occlusion and
uses the capture target's depth during off-screen renders. These remain generic renderer
inputs; the application chooses presentation quality and emission budgets.

## Render pipeline

`Renderer::render` and `Renderer::capture` draw in three stages:

1. The scene and the world effects (including soft particles) draw into an offscreen colour image and a
   reverse-Z `Depth32Float` buffer, both at the internal render size. The colour image is `Rgba16Float`
   (HDR) when the adapter can render to, blend into and filter that format, otherwise the surface's own
   format, so the frame then looks as it did before the offscreen image existed.
2. An ordered chain of fullscreen post-process passes reads the scene image (and may sample the depth
   buffer) and writes the surface; intermediate passes ping-pong through scratch images. The chain always
   ends with the built-in `resolve` pass: clamp to the output range, alpha 1, an exact texel copy when the
   render size equals the output size and bilinear filtering otherwise. The post effects sit at the front
   (see below) and upscalers are inserted before `resolve` (`gpu/post/`).
3. The UI layer draws on the surface after the chain, at surface resolution: it is never post-processed or
   upscaled.

The internal render size is the surface size times the render scale, per axis:

- `Renderer::set_render_scale(scale)` clamps `scale` to `MIN_RENDER_SCALE..=MAX_RENDER_SCALE`
  (0.25..=2.0; non-finite values give 1.0) and resizes the offscreen targets. The default is
  `DEFAULT_RENDER_SCALE` (1.0), which is pixel-identical to drawing straight into the surface.
- `Renderer::resize` keeps the render scale; `render_scale()`, `render_size()`, `surface_size()` and `is_hdr()`
  report the current state. `scaled_size(surface, scale)` is the pure size computation.
- `Renderer::capture(width, height, ...)` runs the same stages at the capture size and returns the final image.

### Post effects

`Renderer::set_post_effects(PostSettings)` turns on bloom, tone mapping and FXAA (`post_effects()` reads the
clamped settings back). The default `PostSettings` runs none of them, so the chain is just `resolve` and the
frame is unchanged. Enabled effects run at the render size in the fixed order bloom, tone mapping (an ACES fit
with an exposure), FXAA, then `resolve`; passes inserted by other code stay behind them. Each effect is its own
pass built on a small fullscreen-filter helper with one WGSL module (`shaders/post_common.wgsl` plus the
effect's file); an effect that is off has no pass. Setting the current value again does nothing; changing it
rebuilds the effect passes. FXAA is an independent implementation of the published algorithm, not a copy of
the reference header.
