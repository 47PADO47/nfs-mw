# Render scale and upscaling

The 3D scene can be drawn at a different size than the window, then scaled to the window by an
upscaler. Drawing fewer pixels costs less GPU time; the HUD, menus and console are drawn after the
upscale at the window's own resolution, so they stay sharp. Nothing changes at the default of 100 %.

## Status

- **DLSS** (`dlss`, `dlss-mock`): tested through the mock in the Bevy renderer. The real path is written but has
  not been compiled, because no NVIDIA DLSS SDK is installed where it was written. See [DLSS status](#dlss-status).
- **FSR 3** (`fsr3`): wired into the Bevy renderer, with the world's depth and motion-vector prepass fixed.
  Measured against native in [status.md](plans/gfx-renderers/status.md), section 3e. Quality against AMD's
  output is not measured yet.
- **Solari** (ray tracing, not an upscaler): works on an RTX card with colour stand-ins for the materials, so
  its look is not final. [status.md](plans/gfx-renderers/status.md), section 3d.

## Controls

Settings resolve independently: CLI, environment, per-user TOML, then defaults, like every other
setting.

| Setting | Config key | Environment | CLI | Default |
|---|---|---|---|---|
| Render scale | `render_scale = 67` | `NFSMW_RENDER_SCALE=67` | `--render-scale 67` | `100` |
| Upscaler | `upscaler = "fsr1"` | `NFSMW_UPSCALER=fsr1` | `--upscaler fsr1` | `fsr1` |
| Upscale sharpness | `upscale_sharpness = 80` | `NFSMW_UPSCALE_SHARPNESS=80` | `--upscale-sharpness 80` | `80` |
| Upscale quality | `upscale_quality = "quality"` | `NFSMW_UPSCALE_QUALITY=quality` | `--upscale-quality quality` | `quality` |

- **Render scale** is the scene's width and height as a percent of the window's, 50 to 200. `67`,
  `67%` and `0.67` all mean the same.
- **Upscaler** is `fsr1`, `bilinear` or `off` on the native renderer. `fsr3` and `dlss` are temporal upscalers that
  only the Bevy renderer offers, and only in builds with the `fsr3` feature (FSR 3) or the `dlss` or `dlss-mock`
  feature (DLSS). `fsr4` is not built yet. The native renderer runs the temporal ones as `fsr1` at the render size
  of their quality mode, and logs the downgrade. See [renderers.md](renderers.md).
- **Upscale quality** is `auto`, `native`, `quality`, `balanced`, `performance` or `ultra_performance`. It only
  matters for the temporal upscalers, whose render size it sets (100, 67, 59, 50 and 33 %; `auto` is `quality`);
  `render_scale` is then ignored. So `dlss` at `quality` on the native renderer is `fsr1` at 67 %.
- **Upscale sharpness** is 0 to 100. It only affects `fsr1`; 0 turns its sharpening pass off.

Video options in the main menu and the pause menu have rows for Render Scale (presets 50, 59, 67,
77, 85, 100, 125, 150 and 200 %), Upscaler and Upscale Sharpness (plus Upscale Quality on a renderer
with temporal upscalers). The Upscaler row cycles only what the running renderer can do. Changes apply at once and are
saved when you leave the settings screen. In the F12 console, `get render_scale`,
`set render_scale 67`, `set upscaler bilinear` and `set upscale_sharpness 50` read or change the
same settings for the run. `--screenshot` captures the final, upscaled image.

## What each choice does

| Render scale | Upscaler | Result |
|---|---|---|
| 100 | any | The scene is drawn at the window size; no upscale pass runs, and with no post effect it goes straight to the window with no copy. |
| below 100 | `fsr1` | The scene is drawn small, then upscaled by FSR 1 (EASU) and sharpened (RCAS). |
| below 100 | `bilinear` | The scene is drawn small and stretched with bilinear filtering: soft, cheapest. |
| above 100 | `fsr1` or `bilinear` | Supersampling: the scene is drawn large and filtered down with bilinear filtering (the upscaler does not apply). |
| any | `off` | The render scale is ignored; the scene is drawn at the window size. |

The 59, 67 and 77 % presets are the per-axis ratios of FSR 1's Balanced, Quality and Ultra Quality
modes; 50 % is Performance. Lower scales save more GPU time and look softer. While upscaling, the
world's textures also get a negative mip bias of `log2(scale)` (one mip sharper at 50 %), so texture
detail matches the window size and does not blur with the lower resolution. Particles and the HUD
are not biased.

## FSR 1

[FidelityFX Super Resolution 1](https://gpuopen.com/fidelityfx-superresolution/) is a spatial
upscaler from AMD: no motion vectors, no history, one frame in and one frame out. It runs two
passes, EASU (edge-adaptive upscale, 12 taps) and RCAS (contrast-adaptive sharpening). Both are
ported to WGSL in `libs/blackbox-gpu-passes/src/shaders/fsr1.wgsl` from AMD's `ffx_fsr1.h`, which is
MIT-licensed; the copyright and permission notices stay in the shader's header and in
[NOTICE](../NOTICE). It runs on every backend (Vulkan, Direct3D 12 and OpenGL) because the shader only
uses plain texture loads.

FSR 1 expects an input that is already anti-aliased, free of noise and in perceptual (gamma-like)
colour. This renderer's scene image is display-referred already, the same values the final pass
writes to the window, so the pass clamps its input to 0..1 and needs no conversion. It cannot
repair aliasing: a jagged edge in the small image is a jagged edge, only larger. The scene has no
anti-aliasing of its own in this layer; when an anti-aliasing pass joins the post chain it should run
before the upscaler (the chain keeps upscale passes last).

Sharpness maps to RCAS attenuation in stops: 100 is the strongest setting (0 stops), 90 is AMD's
reference value (0.2 stops), and each 50 points halve the strength. The default of 80 is a little
softer than AMD's reference value.

## Limits

- **ReShade and other injected effects.** The final image is the window's size, but the depth buffer
  is the render size. Depth effects (fog, depth of field, ambient occlusion) assume the two match and
  break below 100 %. Keep the render scale at 100 % (or the upscaler `off`) when using them.
  Effects that only read colour work as usual.
- **Screenshots of the window** show the upscaled image. `--screenshot-size` renders at that size
  times the render scale.
- **Alt+Tab, resizing and moving between monitors** keep the setting; the scene images are recreated
  at the new size.

## DLSS status

DLSS Super Resolution is offered by the Bevy renderer only (`libs/blackbox-bevy-render`). It has two builds:

- `dlss-mock` is what has been tested here. A DLSS request runs as TAA plus the FSR 1 pass at the same render
  scale, so the caps, settings and apply path are exercised without an RTX GPU. The picture is not DLSS's.
  `cargo test -p blackbox-bevy-render --features dlss-mock --lib`.
- `dlss` is the real path: Bevy's `Dlss` camera component, written against `bevy_anti_alias` 0.20.0 and
  `dlss_wgpu` 6.0.0. It is **not compiled on the development PC**, because the NVIDIA DLSS SDK is not installed
  there. Treat it as unverified until it builds and runs on an RTX machine. It is offered only when Bevy finds
  DLSS Super Resolution.

On the real path, Bevy sets the render size of the main pass from the SDK, so `post::scale` leaves
`MainPassResolutionOverride` alone while DLSS is the upscaler. Capture cameras (reflection and mirror pictures) do
not use DLSS and draw at their full size. The DLSS quality modes map to the shared render-scale table
(`UpscaleQuality::render_scale`), so Balanced renders at 1/1.7 (about 59 %) where NVIDIA's own Balanced ratio is
0.58.

## For developers

`blackbox-render` exposes `Renderer::set_render_scale`, `set_upscaler(Upscaler::{Bilinear, Fsr1})`,
`set_upscale_sharpness(0.0..=1.0)` and `set_texture_lod_bias`, with `suggested_texture_lod_bias(scale)`
for the matching bias. `crates/nfsmw/src/settings/graphics.rs` builds the request from the settings and `app/graphics.rs`
sends it through `RenderBackend::apply_graphics`. See the
`blackbox-render` README for the post chain.

### Temporal upscaler inputs

DLSS Super Resolution is an optional, default-off Cargo feature (`dlss`, or `dlss-mock` without the SDK), built on
Bevy's DLSS camera component. It is offered on Vulkan and NVIDIA RTX only. A temporal upscaler needs the inputs
below. The native renderer has none of the temporal ones yet; the Bevy renderer has them, which is what FSR 3 and
DLSS run on.

| Input | Native renderer | Bevy renderer |
|---|---|---|
| Colour at the render size | Done: the scene image, at `render_size()`. | Done. |
| Output at the window size | Done: upscale passes write the output size (`Extent::Output`). | Done. |
| Depth | Done: `Depth32Float` at the render size, reverse-Z, bindable as a sampled texture. | Done: the world material writes depth in its prepass (PR 13b). |
| Mip bias | Done: `set_texture_lod_bias`; a temporal upscaler uses `log2(scale) - 1`. | Done: `facade.rs` gives a temporal upscaler the temporal curve. |
| Projection jitter | Missing: `FrameParams` is not jittered. | Done for FSR 3 (Halton(2,3) on the projection). DLSS gets its jitter from Bevy. |
| Motion vectors | Missing: instances carry only the current transform. | Done: the world material writes motion vectors in its prepass, from Bevy's previous transforms. |
| Device creation | Missing: the native renderer would need `dlss_wgpu` to create the device. | Done by Bevy: the `dlss` build adds `DlssInitPlugin` before `RenderPlugin`. |

FSR 2 needs the same inputs and is not planned: FSR 3.1 is built instead. Frame generation (DLSS and FSR 3) and
Streamline are not planned. FSR 4 is not built: it needs DX12 and a Radeon RX 7000 or 9000 to test, and there is
no tester yet ([status.md](plans/gfx-renderers/status.md), PR 14).

### FSR 3.1 temporal upscaler (`libs/fsr3-wgpu`)

[`libs/fsr3-wgpu`](../libs/fsr3-wgpu) is a standalone WGSL port of AMD's FSR 3.1 temporal upscaler (MIT,
from the FidelityFX SDK v1.1.4; frame generation is not included). It depends only on `wgpu`, so a backend can
add it on Vulkan and DirectX 12 (and Metal) without subgroup operations or vendor code.

**Bevy renderer (PR 13b).** The Bevy renderer offers it on a build with the `fsr3` feature (`cargo build -p nfsmw
--features fsr3`, which implies `renderer-bevy`), when the device has the storage limits its passes use. The wiring is in
`libs/blackbox-bevy-render/src/fsr3/`: the screen camera gets the depth and motion-vector prepasses, the jitter, `Hdr`
and the temporal mip bias; the render-world node sets the jitter on the projection, runs the upscale in place of TAA,
and blits the display-size result into the view target before bloom and tone mapping. The render size comes from the
shared quality table. The static-scene GPU test passes on an RTX 4070 SUPER over Vulkan
(`cargo test -p blackbox-bevy-render --features fsr3 --lib fsr3 -- --ignored`).

**Prepass (fixed in PR 13b).** The Blackbox material now writes depth and motion vectors in its prepass
(`libs/blackbox-bevy-render/src/material/`). Before that, the upscaler saw an empty depth buffer and no motion for the
world. On the moving-camera test (FSR 3 Quality against native, 160x90, 30 frames, RTX 4070 SUPER, Vulkan) the mean
sample difference fell from 10.84 to 6.53 (p99 89 to 65). The full table and the method are in
[status.md](plans/gfx-renderers/status.md), section 3e. Quality against AMD's output has not been measured; check the
moving camera by eye with `view-world --upscaler fsr3` on a Vulkan or DX12 card.

It needs the inputs of the table above: the jittered colour at the
render size, the reverse-Z depth (the crate takes the reverse infinite convention as a flag), motion
vectors, the Halton(2,3) jitter (`fsr3_wgpu::jitter`), the texture LOD bias `log2(1 / ratio) - 1`
(`QualityMode::mip_bias`) and a reset on camera cuts. The per-axis ratios are AMD's: native 1.0, quality
1.5, balanced 1.7, performance 2.0, ultra performance 3.0. It replaces anti-aliasing (it is temporal
anti-aliasing at 1.0), so FXAA and FSR 1 are left off with it. Its README lists the SDK files, the
differences from AMD's upscaler and what an integration has to provide; the headless GPU tests check
convergence, motion vectors, disocclusion, reset and robustness on Vulkan.

## Checking it

```sh
cargo test -p blackbox-render                  # shader validation (naga), scale and sharpness maths
cargo test -p blackbox-render -- --ignored     # needs a GPU: FSR 1 passes on a real Vulkan device
cargo run --release -p nfsmw -- view-world --render-scale 50 --upscaler fsr1 \
    --screenshot fsr1.png
cargo run --release -p nfsmw -- view-world --render-scale 50 --upscaler bilinear \
    --screenshot bilinear.png
```

Compare the two screenshots: FSR 1 keeps edges (road markings, wires, building outlines) cleaner and
adds contrast; bilinear is uniformly soft.
