# fsr3-wgpu

AMD FidelityFX Super Resolution 3.1 **temporal upscaler** for [wgpu](https://wgpu.rs), ported to WGSL. It
renders nothing itself: the application renders a jittered frame at a lower resolution with depth and
motion vectors, and the upscaler accumulates those frames into an image at the output resolution. Only
the upscaler is ported, not frame generation.

It runs on every wgpu backend with compute shaders (tested on Vulkan, on an Intel Iris Xe and on lavapipe;
DX12 and Metal are expected to work and are not tested yet), in 32-bit floats, **without subgroup operations**, within wgpu's default limits
(4 storage textures per stage, 8 storage buffers). It depends only on `wgpu`, `glam`, `bytemuck` and
`thiserror`: no Bevy, no renderer of this repository, nothing game-specific.

License: MIT OR Apache-2.0 for this crate; the shaders are derived from AMD's MIT-licensed FidelityFX SDK
and keep its notice (below).

## Using it

```rust
use fsr3_wgpu::{Fsr3Config, Fsr3Context, Fsr3Inputs, Fsr3Outputs, QualityMode, jitter, mip_bias,
                motion_vector_scale_ndc};

// Once.
let mode = QualityMode::Quality;                       // 1.5x per axis
let render_size = mode.render_size(output_size);       // what to render at
let mut upscaler = Fsr3Context::new(&device, Fsr3Config::new())?;   // HDR, reverse infinite depth

// Every frame.
let jitter = jitter::offset_for_sizes(frame_index, render_size, output_size);   // pixels, Halton(2,3)
let ndc = jitter::projection_offset_ndc(jitter, render_size);   // translate the projection by this
// ... render the scene at render_size with that projection, texture LOD bias mode.mip_bias(),
// writing colour (linear HDR, not tone mapped), depth and motion vectors ...
upscaler.dispatch(&device, &queue, &mut encoder, &Fsr3Inputs {
    color: &color_view, depth: &depth_view, motion_vectors: &motion_view,
    exposure: None, reactive: None, transparency_and_composition: None,
    render_size, jitter, motion_vector_scale: Vec2::ONE,   // vectors in render pixels
    delta_time: dt_seconds, pre_exposure: 0.0, sharpness: Some(0.8),
    camera_near: 0.1, camera_far: f32::INFINITY, camera_fov_y: fov_y_radians,
    view_space_to_meters: 1.0, reset: camera_cut,
}, &Fsr3Outputs { output: &output_view, size: output_size })?;
// ... tone map and present `output` (a storage texture) ...
```

- **Inputs.** `color`: the jittered scene at render resolution, linear, ideally HDR and before tone
  mapping, bloom, film grain and other jitter-sensitive effects. `depth`: a `Depth32Float` (or a 32-bit
  float colour) texture at render resolution. `motion_vectors`: two float channels, **previous position
  minus current position**, in screen units that `motion_vector_scale` turns into pixels (`Vec2::ONE`
  for pixels, [`motion_vector_scale_ndc`] for normalised device coordinates). Textures are read with
  integer loads, so unfilterable formats such as `R32Float` and depth are fine.
- **Outputs.** `output` is a storage texture of `Fsr3Config::output_format` (`Rgba16Float`,
  `Rgba32Float` or `Rgba8Unorm`) at the output resolution.
- **Config** (fixed per context): HDR or LDR input, the depth convention (`DepthConvention::
  REVERSE_INFINITE`, `REVERSE`, `STANDARD` or standard-infinite), auto exposure or an exposure texture,
  the layout of the motion vectors (render or output resolution, jittered or not), the output format and
  AMD's tuning constants.
- **Sizes.** The first dispatch allocates the internal images; a changed render or output size
  reallocates them and drops the history. The render size must not exceed the output size. A context
  holds the history of one camera.
- **Reset.** Pass `reset: true` on a cut, a teleport or a scene change, or call `Fsr3Context::reset`.
- **Optional masks.** `reactive` (render size, 0 trusts the history, 1 rejects it) for particles and
  reflections of moving objects; `transparency_and_composition` for glass and composited objects.
- **Quality modes and helpers.** `QualityMode::{NativeAa, Quality, Balanced, Performance,
  UltraPerformance}` give the per-axis ratios 1.0, 1.5, 1.7, 2.0, 3.0 and `render_size`; `mip_bias` is
  `log2(1 / ratio) - 1`, the LOD bias to sample world textures with. `jitter::halton`, `jitter::offset`,
  `jitter::phase_count` (8 at 1x, 18 at 1.5x, 32 at 2x, 72 at 3x) and `jitter::projection_offset_ndc`
  produce and apply the jitter.
- **Debugging.** `Fsr3Context::debug_texture(DebugTexture)` returns the internal images (masks, dilated
  depth and vectors, accumulation, history) for overlays; `gpu_memory_bytes` reports the memory
  (about 100 MiB for 1920x1080 at Quality).
- **One dispatch per context per submit,** in order: each dispatch continues the history of the last.

### What a renderer integration has to supply

1. A **jittered projection** each frame, with the jitter from `jitter::offset_for_sizes` (the same
   value goes into `Fsr3Inputs::jitter`), and a **texture LOD bias** of `QualityMode::mip_bias`.
2. **Depth** with the convention declared in the config. Reverse infinite depth (`near / distance`)
   needs only the near plane; other conventions need `camera_far` too.
3. **Motion vectors** for every pixel, including the camera's own motion, from previous and current
   transforms with the *unjittered* matrices (or jittered ones with `MotionVectorLayout::jittered`).
   Pooled objects need stable identities so that vectors do not jump between objects.
4. **Colour** before tone mapping in linear space, and, if the application exposes the image itself,
   the pre-exposure in `Fsr3Inputs::pre_exposure` or an exposure texture.
5. A **storage-capable output texture** (not a swap-chain image) and the `reset` flag on camera cuts.
6. The reactive and transparency masks if there are particles or glass (optional).
7. The render target size must follow the quality mode; changing it reallocates the history.

## SDK, files and licence

The port follows **FidelityFX-SDK `v1.1.4`**
([`GPUOpen-LibrariesAndSDKs/FidelityFX-SDK`](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK),
commit `c6efa6bf7f2027b3ec94f28578bb5965eabb9e55`), whose FSR 3 upscaler is version **3.1.4**
(`FFX_FSR3UPSCALER_VERSION_*`). The SDK's `LICENSE.txt` is the MIT licence, "Copyright (C) 2024 Advanced
Micro Devices, Inc."; that notice stands at the top of every shader here and in the repository's
[NOTICE](../../NOTICE) and [docs/licensing.md](../../docs/licensing.md). **No upstream file is vendored;**
the SDK was read from a scratch directory and only the WGSL and Rust written for this crate are committed.

| This crate | AMD source files read (paths under `sdk/`) |
|---|---|
| `shaders/common.wgsl` | `include/FidelityFX/gpu/fsr3upscaler/ffx_fsr3upscaler_common.h`, `…_callbacks_hlsl.h`, `…_resources.h` |
| `shaders/prepare_inputs.wgsl` | `…/ffx_fsr3upscaler_prepare_inputs.h` |
| `shaders/luma_pyramid.wgsl` | `…/ffx_fsr3upscaler_luma_pyramid.h` (and `ComputeAutoExposureFromLavg` of `…_common.h`) |
| `shaders/shading_change_pyramid.wgsl` | `…/ffx_fsr3upscaler_shading_change_pyramid.h` |
| `shaders/shading_change.wgsl` | `…/ffx_fsr3upscaler_shading_change.h` |
| `shaders/prepare_reactivity.wgsl` | `…/ffx_fsr3upscaler_prepare_reactivity.h` |
| `shaders/luma_instability.wgsl` | `…/ffx_fsr3upscaler_luma_instability.h` |
| `shaders/accumulate.wgsl` | `…/ffx_fsr3upscaler_accumulate.h`, `…_upsample.h`, `…_reproject.h`, `…_sample.h` |
| `shaders/rcas.wgsl` | `…/ffx_fsr3upscaler_rcas.h`, the RCAS of `include/FidelityFX/gpu/fsr1/ffx_fsr1.h` |
| `src/constants.rs`, `jitter.rs`, `quality.rs`, `context.rs` | `src/components/fsr3upscaler/ffx_fsr3upscaler.cpp`, `…_private.h`, `include/FidelityFX/host/ffx_fsr3upscaler.h`; the DX12 pass wrappers in `src/backends/dx12/shaders/fsr3upscaler/` for the bindings |

The passes run in AMD's order: prepare inputs, luma pyramid (auto exposure), shading change pyramid,
shading change, prepare reactivity, luma instability, accumulate, RCAS. The "reconstruct previous depth"
and the depth and motion vector dilation are part of prepare inputs, and the "depth clip" is the
disocclusion computed in prepare reactivity, as in the SDK.

## Differences from AMD FSR 3.1

This is a port, not a binary-compatible copy. Results are close in character to AMD's but **not
bit-exact**; it was checked against analytic scenes (see Tests), not against AMD's output. Known
differences, all deliberate unless marked as a gap:

- **No single-pass downsampler.** AMD's luma pyramid and shading change pyramid use SPD, which needs a
  global atomic counter and globally coherent storage that WGSL cannot promise. Here the luma
  reduction takes two dispatches (a per-workgroup reduction, then one workgroup that sums the partial
  results) and the shading change pyramid takes one dispatch per level, with three levels instead of
  six (only three are read). The average luma is the exact mean over the frame; SPD repeats the edge
  pixels to fill its 64x64 tiles and so over-weights the right and bottom edges.
- **Exposure.** The exposure (computed or from the input texture) is stored once and read by all passes.
  On a reset the auto exposure snaps to the first frame's average instead of easing from AMD's cleared
  value of 1 (`resetAutoExposureAverageSmoothing` is defined but unused in AMD's clear).
- **History taps at the screen edge.** AMD clamps the 4x4 Lanczos taps of the history reprojection to
  `1..size-2`, which makes the outermost texel read its neighbour and shifts the picture by one pixel
  along the screen edges (visible as hooks on thin lines). The taps here repeat the edge texel.
- **Jitter cancellation.** The jitter difference of jittered motion vectors is divided by the render
  size (it is in render pixels); AMD divides by the size of the motion vectors, which differs only for
  output-resolution vectors.
- **Storage formats are 32-bit floats** where AMD uses 8 or 16 bits (accumulation, luma, farthest depth,
  shading change, luma instability, locks) and 16-bit floats for the dilated reactive masks (AMD: 8-bit
  unorm); the colour history and the luma history stay 16-bit float, as in AMD. The reconstructed
  previous depth is a buffer of atomics (`atomicMax` of the depth's bits, or of their complement for
  standard depth), not an `R32_UINT` texture with `InterlockedMin/Max`.
- **Robustness.** The colour input is clamped to `[0, 1e7]`; output and history are clamped to the 16-bit
  float range; a flat neighbourhood returns no thin-feature lock instead of dividing by zero; the motion
  divergence and RCAS guard their divisions; RCAS uses exact reciprocals; textures read outside their
  size give zero like DirectX does. The accumulation is zero on the first frame after a reset
  (AMD relies on clearing a texture).
- **Variants not ported:** the 16-bit (FP16) paths, the wave32 and wave64 permutations, the paired
  16-bit Xbox path, and the Lanczos lookup-table variant of the history reprojection (AMD picks it only
  on some wave32/wave64 GPUs). The upsample uses AMD's default (the FSR 1 approximation) and the history
  reprojection the sin-based Lanczos, as the SDK does on most hardware.
- **Not ported (gaps):** the automatic reactive mask generation pass
  (`ffxFsr3UpscalerContextGenerateReactiveMask`), the debug view pass, dynamic resolution (a different
  render size reallocates and resets; AMD allocates for a maximum size), the shared resources for frame
  generation (dilated depth and vectors, reconstructed depth stay internal) and the SDK's debug checks.
- **API.** `delta_time` is in seconds (AMD: milliseconds); the sharpness maps to the RCAS lobe as in
  AMD (`exp2(-(2 - 2 * sharpness))`); the AMD tuning constants are `Fsr3Config::tuning`. The accumulation
  pass skips the output write when sharpening follows, through a pipeline override, as AMD does through a
  permutation.

## Tests

```sh
cargo test -p fsr3-wgpu                    # CPU: Halton, ratios, mip bias, depth factors, uniform layout, naga
cargo test -p fsr3-wgpu -- --ignored --test-threads=1    # needs a Vulkan GPU
FSR3_TEST_DUMP=/tmp/fsr3 cargo test -p fsr3-wgpu -- --ignored   # also writes PPM images
```

The CPU tests check the Halton sequence and phase counts, the quality ratios and render sizes, the
mip bias, the uniform block's size and offsets against the WGSL struct (with naga), the view-depth
factors of every depth convention, and that every pass parses and validates with naga for every output
format. The GPU tests render an analytic scene at the render resolution (jittered, with
`Depth32Float` depth and motion vectors), supersample it 8x8 at the output resolution as the
reference and compare in PSNR: a static scene converges (35.4 dB after 64 frames at Quality against
31.7 dB for a bilinear upscale of a plain render), 1x anti-aliasing, a translating checkerboard
(correct vectors 21.0 dB, zero vectors 11.3 dB, bilinear 18.6 dB), disocclusions in all four depth
conventions (and fewer with a wrong convention), vectors at output resolution and jittered vectors,
reset (a reset frame equals a fresh context), resizing, masks, auto exposure, an exposure texture and a
pre-exposure change, the three output formats, extreme inputs without NaN or infinity, odd sizes, and
full HD. Other tests feed plain numbers and compare single passes with a CPU model of AMD's formulas
(the dilated depth and vectors, the farthest depth, the luma, the auto exposure and its smoothing, the
shading change of a brightness change, the accumulation ramp). They are `#[ignore]` and run within
wgpu's default limits. On an Intel Iris Xe (Vulkan),
1920x1080 at Quality takes about 6.9 ms per frame including a trivial scene. Lavapipe gives the same
numbers to within 0.1 dB (`FSR3_TEST_FALLBACK=1`, and `VK_LOADER_DRIVERS_SELECT='*lvp*'` if the loader
filters drivers).
