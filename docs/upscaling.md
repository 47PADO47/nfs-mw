# Render scale and upscaling

The 3D scene can be drawn at a different size than the window, then scaled to the window by an
upscaler. Drawing fewer pixels costs less GPU time; the HUD, menus and console are drawn after the
upscale at the window's own resolution, so they stay sharp. Nothing changes at the default of 100 %.

## Controls

Settings resolve independently: CLI, environment, per-user TOML, then defaults, like every other
setting.

| Setting | Config key | Environment | CLI | Default |
|---|---|---|---|---|
| Render scale | `render_scale = 67` | `NFSMW_RENDER_SCALE=67` | `--render-scale 67` | `100` |
| Upscaler | `upscaler = "fsr1"` | `NFSMW_UPSCALER=fsr1` | `--upscaler fsr1` | `fsr1` |
| Upscale sharpness | `upscale_sharpness = 80` | `NFSMW_UPSCALE_SHARPNESS=80` | `--upscale-sharpness 80` | `80` |

- **Render scale** is the scene's width and height as a percent of the window's, 50 to 200. `67`,
  `67%` and `0.67` all mean the same.
- **Upscaler** is `fsr1`, `bilinear` or `off`.
- **Upscale sharpness** is 0 to 100. It only affects `fsr1`; 0 turns its sharpening pass off.

Video options in the main menu and the pause menu have rows for Render Scale (presets 50, 59, 67,
77, 85, 100, 125, 150 and 200 %), Upscaler and Upscale Sharpness. Changes apply at once and are
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

## For developers

`blackbox-render` exposes `Renderer::set_render_scale`, `set_upscaler(Upscaler::{Bilinear, Fsr1})`,
`set_upscale_sharpness(0.0..=1.0)` and `set_texture_lod_bias`, with `suggested_texture_lod_bias(scale)`
for the matching bias. `crates/nfsmw/src/app/upscale.rs` maps the settings onto them. See the
`blackbox-render` README for the post chain.

### Seams for DLSS and other temporal upscalers (not implemented)

The architecture decision for milestone 8 (ADR 0003, "the renderer after milestone 8") keeps NVIDIA
DLSS Super Resolution for after the milestone, as an optional, default-off Cargo feature that wraps
`dlss_wgpu` (Vulkan only, NVIDIA RTX). What a temporal upscaler needs, and where this code stands:

| Input | Status |
|---|---|
| Colour at the render size | Done: the scene image, at `render_size()`. |
| Output at the window size | Done: upscale passes write the output size (`Extent::Output`). |
| Depth | Done: `Depth32Float` at the render size, reverse-Z (1 at the near plane, 0 far; no far plane), already bindable as a sampled texture. A reverse-depth flag is all the SDK needs. |
| Mip bias | Done: `set_texture_lod_bias`; temporal upscalers use `log2(scale) - 1`. |
| Projection jitter | Missing: `FrameParams` (`view`, `projection`) is not jittered; needs a per-frame sub-pixel offset (a Halton sequence) in the projection and its value passed to the upscaler. |
| Motion vectors | Missing: instances carry only the current transform. It needs a per-instance previous transform, a previous view-projection, and a second colour output for the vectors. Skinned or animated parts (wheels, the sky) need care. |
| Hook into device creation | Missing: `dlss_wgpu` creates the instance and device itself, replacing the calls in `gpu/init.rs` when the feature is on. |

FSR 2 and later temporal upscalers need the same inputs and are future work. DLSS frame generation,
FSR 3 frame generation, FSR 4 and Streamline are not reachable through wgpu and are not planned.

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
