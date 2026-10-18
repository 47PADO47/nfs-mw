# Native renderer DLSS

The optional `native-dlss` game feature integrates NVIDIA Streamline 2.14.16 with
the BlackBox renderer on Windows/Vulkan. It supports Super Resolution, DLAA,
Reflex and fixed-multiplier DLSS Frame Generation. Ray Reconstruction is not included.

## Build and runtime

Download the official [Streamline SDK 2.14.16 release](https://github.com/NVIDIA-RTX/Streamline/releases/tag/v2.14.16).
Set `STREAMLINE_SDK` to its extracted root when building:

```text
cargo build --release -p nfsmw --features native-dlss
```

The feature compiles a small C++17 bridge against the SDK headers. The DLLs are
runtime dependencies, not files in this repository. Follow NVIDIA's distribution
terms for those binaries. Keep the following release binaries together beside
the executable, or set `STREAMLINE_PLUGIN_DIR` to their absolute directory:

- `sl.interposer.dll`
- `sl.common.dll`
- `sl.dlss.dll`
- `sl.pcl.dll`
- `sl.reflex.dll`
- `sl.dlss_g.dll`
- `NvLowLatencyVk.dll`
- `nvngx_dlss.dll`
- `nvngx_dlssg.dll`

The loader checks the interposer's Authenticode signature and NVIDIA signer.
Streamline validates its plugins. Logs go to the temporary directory's
`blackbox-streamline` subdirectory. The loader and plugins with thread-exit callbacks
remain resident through process exit. DLSS-G unloads in SDK dependency order before
the common compute interface is destroyed, releasing its static resource caches.
A process gets one Streamline device session. Additional or recreated
renderers use the existing upscaler fallback; restart the application to enable
DLSS after a renderer has been destroyed. Window resizing retains the renderer
and supports DLSS normally.

Select the BlackBox renderer and Vulkan, then choose DLSS in graphics options.
Quality modes use the SDK's optimal dimensions. Native selects DLAA. Preset M
selects the DLSS 4.5 second-generation transformer for DLAA, Quality, Balanced
and Performance; Ultra Performance uses preset L. The UI is drawn at output
resolution after DLSS, bloom, tone mapping and other post effects. Captures and
rear-view mirrors use their existing independent, non-temporal paths.
Offscreen-only renderers do not advertise DLSS: Streamline needs presentation
hooks on every frame, so its tests use a real window and swapchain.

## Reflex and Frame Generation

F8 exposes Reflex Off, On and On + Boost, and frame multipliers from Off through
the maximum reported by the SDK (up to 6x). Settings persist with other video
options. CLI equivalents are `--reflex off|on|boost` and
`--frame-generation off|2x|3x|4x|5x|6x`. The NVIDIA SDK checks hardware, driver and
Hardware Accelerated GPU Scheduling support before these controls become available.
Dynamic Multi Frame Generation is not exposed because the SDK limits it to D3D12.

Vulkan generation requires VSync off, including driver overrides.
HDR generation automatically selects a 10-bit BT.2100 PQ/HDR10 swapchain,
because Streamline 2.14.16 does not support FP16/scRGB frame generation.
If the surface lacks HDR10 support, HDR retains scRGB and generation suspends
with an explanatory display status. Turning generation off restores scRGB HDR.
Use DLSS or
FSR 3 for temporal depth/motion inputs; choose Native DLSS quality for DLAA.
Generation runs during focused, unpaused gameplay and stops for menus, captures,
unsupported sizes, resize and runtime errors. The SDK also suppresses interpolation
while the window is unfocused. Reflex On is enforced whenever generation is requested.
The overlay shows measured base/rendered FPS and actual SDK presented FPS,
including generated frames. It does not infer presentation rate from the selected
multiplier. Advanced metrics estimate the likely limit and distinguish caps,
missing GPU timings and presentation waits. GPU render time excludes generation;
CPU work is wall time, not utilization. See [graphics lab](graphics-lab.md).

Reflex sleep runs before time/input sampling, with one frame token shared by
simulation, submission, present, SR and generation. The in-game frame limit is
passed to Reflex instead of running a second software limiter.

The post-processed scene is rendered into a full-resolution HUDless target.
The HUD and debug UI render into a separate transparent color target;
`kBufferTypeHUDLessColor` and a separate `kBufferTypeUIAlpha` mask are tagged through
present with `enableUserInterfaceRecomposition` enabled. Thus the UI is recomposed
over generated scene frames. Coverage is accumulated per UI draw before HDR
conversion, independently of colour blending: additive glyphs over translucent
panels must not inherit only the panel opacity. Additive coverage uses the
maximum premultiplied gamma RGB channel; ordinary UI uses its alpha. It is
independent of HDR UI-white calibration. Premultiplied alpha and additive UI
retain the renderer's real-frame composition. Moving HUD elements
still require visual checks. The implementation follows NVIDIA's
[Frame Generation](https://github.com/NVIDIA-RTX/Streamline/blob/v2.14.16/docs/ProgrammingGuideDLSS_G.md)
and [Reflex](https://github.com/NVIDIA-RTX/Streamline/blob/v2.14.16/docs/ProgrammingGuideReflex.md) guides.

HDR tone mapping and UI blending remain in linear BT.709 scRGB, including scene
white, UI white and peak calibration. A final composition pass converts BT.709
to BT.2020 primaries and encodes absolute nits with SMPTE ST 2084 PQ. It writes
the real frame and HUDless resource as RGB10. UI colour blending uses RGBA16F;
an independent R8 coverage target is tagged for generation alongside the final
and HUDless resources. No scene-dependent PQ colour term is passed as an
independently interpolated UI layer. Signed/out-of-gamut components are clipped
at the HDR10 boundary.
Menus retain HDR10 while generation is requested, avoiding output mode switches
on pause. SDR PNG and linear scRGB captures retain their independent formats.
The output follows the [Microsoft Advanced Color contract](https://learn.microsoft.com/en-us/windows/win32/direct3darticles/high-dynamic-range)
and wgpu's `SurfaceColorSpace::Bt2100Pq` contract. Mastering metadata is not set;
the swapchain signals PQ/BT.2020 and existing calibration controls set luminance.

## Integration

The Vulkan instance and device are created through the interposer. This routes
presentation and swapchain functions through Streamline as required by its
[manual hooking guide](https://github.com/NVIDIA-RTX/Streamline/blob/v2.14.16/docs/ProgrammingGuideManualHooking.md).
The bridge follows NVIDIA's
[DLSS guide](https://github.com/NVIDIA-RTX/Streamline/blob/v2.14.16/docs/ProgrammingGuideDLSS.md).

Inputs reuse the native FSR 3 temporal pipeline: jittered HDR colour, reverse-Z
depth, and unjittered camera/object motion. Explicit wgpu resource transitions
synchronize native DLSS commands with surrounding wgpu passes. Motion vectors
are converted from previous-minus-current NDC to UV using `(0.5, -0.5)`.
wgpu 30 forbids raw and wgpu commands on the same encoder. The integration submits
the scene and transitions, a separate raw DLSS command buffer, then the remaining
post chain, all in order on the same queue. Views stay alive until raw GPU work
completes. This adds two queue submissions per frame; performance requires measurement.
Quality changes, upscaler switches and output resizing drain GPU work before
releasing the old SDK feature, so shutdown never leaves deferred NGX handles.
Streamline matrices use the equivalent row-vector representation of glam's
column-vector transforms. Auto exposure is enabled; there is no exposure texture.

The existing limitations of native temporal inputs still apply: transparent
objects and particles lack their own motion vectors, and transient unkeyed
objects have camera motion only. Visual quality during driving must be checked
separately from successful SDK evaluation. Direct3D 12 and OpenGL retain the
normal fallback chain.

## Validation

With the feature and runtime installed, this explicit test requires an RTX
adapter and fails when the runtime is absent. It counts successful real SDK
evaluations through all modes and DLAA, real presentation, resizing, capture and
runtime-failure fallback. It also checks Reflex marker cadence in all three modes,
actual generated presentation at each supported multiplier in SDR and supported HDR10,
VSync/menu suspension, capture isolation and separate SDR/HDR UI composition pixels.
HDR pixel checks cover absolute luminance, gamut conversion and alpha precision.
Generation tests briefly activate their window:
the SDK intentionally suppresses interpolation in an unfocused window.
A fallback image cannot pass the evaluation counter:

```text
cargo test -p blackbox-render --features streamline --lib dlss_sdk -- --ignored --nocapture --test-threads=1
```

Run ordinary renderer tests with and without `streamline`, and the existing
native GPU regressions. A live-window check is also required to verify
presentation, resize/Alt+Tab and full-resolution HUD composition; a headless
SDK test does not exercise swapchain presentation.
