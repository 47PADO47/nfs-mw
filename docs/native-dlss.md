# Native renderer DLSS

The optional `native-dlss` game feature integrates NVIDIA Streamline 2.14.16 with
the BlackBox renderer on Windows/Vulkan. It requests Super Resolution only; it
does not request Frame Generation, Reflex or Ray Reconstruction.

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
- `NvLowLatencyVk.dll`
- `nvngx_dlss.dll`

The loader checks the interposer's Authenticode signature and NVIDIA signer.
Streamline validates its plugins. Logs go to the temporary directory's
`blackbox-streamline` subdirectory. The loader and plugin code remain resident
through thread/process exit, including after Streamline shutdown and Vulkan
destruction. A process gets one Streamline device session. Additional or recreated
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
hooks on every frame, so its tests use a hidden window and real swapchain.

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
runtime-failure fallback. A fallback image cannot pass the evaluation counter:

```text
cargo test -p blackbox-render --features streamline --lib dlss_sdk -- --ignored --nocapture --test-threads=1
```

Run ordinary renderer tests with and without `streamline`, and the existing
native GPU regressions. A live-window check is also required to verify
presentation, resize/Alt+Tab and full-resolution HUD composition; a headless
SDK test does not exercise swapchain presentation.
