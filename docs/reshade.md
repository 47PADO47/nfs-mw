# ReShade and other injected effects

Nothing in this project links or ships ReShade. It stays *injectable*: this page says what a ReShade
(or vkBasalt) setup should expect. Reasoning and sources: [ADR 0003](decisions/0003-renderer-after-milestone-8.md).

**Status:** the depth contract below is tested on the CPU side (`viewer/camera/depth_tests.rs`). Running
ReShade against this renderer has **not** been tested on any backend; the far-plane advice is derived
from `ReShade.fxh`, not tried in game.

## Where it works

| Platform | Setup |
|---|---|
| Windows, `--backend vulkan` or `--backend dx12` | Install ReShade with its setup tool for `nfsmw.exe` (for Vulkan the setup registers the layer for that executable). |
| Windows, `--backend gl` | ReShade supports OpenGL in general; not verified with wgpu's GL backend. |
| Linux | ReShade has no native build. Use [vkBasalt](https://github.com/DadSchoorse/vkBasalt) on the Vulkan backend. Wine is untested. |

wgpu has no Direct3D 11 backend, the most common ReShade path, so choose Vulkan or DX12.

ReShade sees the final image, post-processing chain and HUD included, as for any game. Tone mapping, bloom and
FXAA from [post-processing.md](post-processing.md) stack with it; turn them off if ReShade does the same job.

## Depth buffer

The depth image is `Depth32Float`, **reverse Z**, with **no far plane**: 1.0 at the near plane and
`near / distance` beyond it, tending to 0 at infinity. Camera near planes: 0.3 m (chase), 0.5 m (free
camera), the orbit camera's own value in the car viewer.

In ReShade's global preprocessor definitions:

| Definition | Value |
|---|---|
| `RESHADE_DEPTH_INPUT_IS_REVERSED` | `1` (the default) |
| `RESHADE_DEPTH_INPUT_IS_UPSIDE_DOWN` | `0` |
| `RESHADE_DEPTH_INPUT_IS_LOGARITHMIC` | `0` |
| `RESHADE_DEPTH_LINEARIZATION_FAR_PLANE` | raise it, see below |

With the reversed setting ReShade linearises to `(z - near) / (z + near * (F - 1))`, where `F` is the far-plane
definition (default 1000) and `z` the distance in metres. That reaches 0.5 at `near * (F + 1)` metres, about
300 m for the chase camera with the default, and bends strongly before that. Effects that assume
a linear depth (fog, depth of field, ambient occlusion) look better with `F` raised until the curve is flat
over the distances they use. Pick the value by eye in the depth debug view of ReShade's `DisplayDepth.fx`.

## Render scale

Below 100 % [render scale](upscaling.md) the depth image is smaller than the window, and ReShade's depth
effects assume the two match. Keep render scale at 100 % (or the upscaler off) when using them. Effects that
only read colour work at any scale.

## Keeping the contract

Changing the projection, the depth format or the near planes changes what injected effects see. The tests in
`crates/nfsmw/src/viewer/camera/depth_tests.rs` pin reverse Z and `near / distance`; if one has to change,
update this page and the ADR in the same PR.
