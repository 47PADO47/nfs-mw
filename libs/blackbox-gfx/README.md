# blackbox-gfx

The renderer-neutral graphics interface for EA Black Box game reimplementations. It has no GPU API, no
engine and no window-system dependency (only `glam`, `bytemuck` and `thiserror`), so a game and its scene
code can talk to any renderer through it, and a renderer can be swapped without touching the game.

License: MIT OR Apache-2.0.

## What is in it

| Module | Contents |
|---|---|
| `api` | `GraphicsApi { Auto, Vulkan, Dx12, Gl }`: which GPU API to draw with (the old `Backend`, still available as an alias) |
| `backend` | the `RenderBackend` trait every renderer implements |
| `caps` | `Capabilities`, small enum bitsets (`AaSet`, `UpscalerSet`, `TonemapSet`, `RestartSet`), and the pure `resolve(requested, caps)` |
| `settings` | `GraphicsSettings`, `PostSettings`, `Antialiasing`, `Upscaler`, `UpscaleQuality`, `RayTracing`, render-scale and upscale helpers |
| `mesh`, `texture`, `material`, `frame` | the 36-byte `Vertex`, `MeshDesc`, `DrawRange`, `BlendMode`, `Shading`, `TextureDesc`, glossy materials and the lighting rig, `FrameParams`, `Instance`, `FrameStatus` |
| `effects`, `ui` | the world `EffectLayer` and the 2D `UiLayer` (textured, clipped, premultiplied-alpha triangles) |
| `handles`, `error`, `capture`, `stats` | opaque resource handles, `RenderError`, `RgbaImage`, `BackendInfo` and `RenderStats` |

## The trait

`RenderBackend` is object safe: callers hold a `Box<dyn RenderBackend>` and scenes borrow
`&mut dyn RenderBackend`. There are no generic methods and no constructors, because creating a backend
needs things only that backend knows (window handles, an app). A backend allocates handles
synchronously, so a handle can be used right after the `create_*` call that returned it.

Captures are asynchronous: `request_capture` returns a `CaptureId`, `poll_capture` returns the image once
it is ready (a backend may take a few frames; each capture is returned once).

## Capabilities and `resolve`

A backend reports what it can run once, as `Capabilities`. `apply_graphics(&GraphicsSettings)` maps the
user's request onto it with `resolve`, and returns the effective settings plus every `Downgrade` (a setting
that was changed, what it became and why) and every `Note` (a caveat on a setting that was kept). `resolve`
is pure, so it is fully unit-tested without a GPU. The rules, in the order they run:

1. The request is sanitised (clamped) first, silently.
2. A tone-mapping curve or bloom the renderer lacks is turned off.
3. An unavailable upscaler falls back along `fsr4 > fsr3 > fsr1 > bilinear > off` and
   `dlss > fsr3 > fsr1 > bilinear > off`. `Off` always works.
4. A temporal upscaler (`Fsr3`, `Fsr4`, `Dlss`) does its own anti-aliasing and replaces TAA, FXAA and SMAA,
   which are turned off with a downgrade that says so. Otherwise an unavailable `Taa` or `Smaa` falls back
   to `Fxaa`, and then to `Off`.
5. Render scale: when the *requested* upscaler is temporal, its `UpscaleQuality` decides the scale
   (the `render_scale` setting is ignored), even if the upscaler itself fell back to a spatial one.
   `Upscaler::Off` draws at the output size. The result is clamped to the renderer's range.
6. Ray tracing the renderer lacks is turned off. Ray tracing without a denoiser is allowed, with a note that
   it will be noisy.

`resolve` is idempotent: resolving an effective result again changes nothing.

## What is not here yet

This crate is the first step of the swappable-renderers plan. Two things are deliberately left for the PR
that migrates the callers, so that this one stays a move plus the new interface:

- `FrameParams` and `Instance` keep their current shapes (`view_proj`, `fog_start`/`fog_end`). The plan
  adds `view`, `projection`, `fog`, `camera_cut` and a stable `InstanceKey` there.
- Nothing implements `RenderBackend` yet. `blackbox-render` re-exports the moved types, so existing callers
  compile unchanged; it gains its `impl RenderBackend for Renderer` next.

## Rules

- No wgpu, Bevy or `raw-window-handle` dependency. Backends convert at their own boundary.
- Engine-generic: nothing here may name a specific game.
- Handles carry a number only their backend understands (`from_raw` / `raw` are for backends).
