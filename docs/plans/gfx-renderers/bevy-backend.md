# Swappable renderers: the Bevy backend

Part of the [swappable renderers plan](README.md): section 3.

## 3. The Bevy backend: `libs/blackbox-bevy-render`

### 3.1 Same App, render sub-app as Bevy designs it

- **Placement:** the backend is a set of plugins added to the game's existing `App` (ADR 0001: `bevy_winit` owns the window). Bevy's `RenderPlugin` creates its own wgpu instance and surface for the primary window. The native backend's `create_renderer` system is simply not registered when Bevy draws.
- **Pipelined rendering:** off at first, so screenshots are deterministic and there is less latency to reason about. Measure it in PR 11.
- **Bevy crates used:**
  - by individual crate, not the umbrella `bevy`: `bevy_asset`, `bevy_render`, `bevy_core_pipeline`, `bevy_pbr`, `bevy_post_process`, `bevy_anti_alias`, `bevy_image`, `bevy_mesh`, `bevy_camera`, `bevy_light`, `bevy_shader`, and optional `bevy_solari`;
  - `tonemapping_luts` only if a tonemapper that needs it is offered;
  - all pinned `=0.20.0`.
- **The façade and command queue:**
  - `BevyBackend` implements `RenderBackend`. It allocates handles synchronously from counters and records operations into a shared queue (`Arc<Mutex<Ops>>`): create or destroy texture, mesh and material, redirect, rig, environment, effects, UI patches and layer, frame snapshot, capture request.
  - The `BlackboxBridge` resource holds the other end. A main-world system in `PostUpdate` applies the operations to `Assets<Image>`, `Assets<Mesh>`, materials and entities before Bevy extracts.
  - This keeps the game's immediate-mode API unchanged while Bevy stays retained-mode.
- **Startup:**
  - The façade is created in `FrameSet::Prepare` once `RenderDevice` and `RenderAdapterInfo` exist in the main world, so capabilities (BC support, ray tracing) are real when `Scene::init` uploads.
  - `WgpuSettings` gets `backends` from the `backend` setting (Vulkan, DX12, or both for auto).
  - `features` are set explicitly: `TEXTURE_COMPRESSION_BC` when the probe saw it, plus Solari's features only when ray tracing is on at startup.
- **Axes:** the game's world is z-up. Bevy features such as `Atmosphere` and Solari's sky assume y-up. The backend applies one fixed basis change (z-up to y-up) to every transform, the view and the light directions. Callers never see it.

### 3.2 Rendering Black Box assets

- **Textures:**
  - BC1, BC2 and BC3 upload as `Image`s with all mips (`Bc1RgbaUnormSrgb` and so on) when BC is supported; otherwise as RGBA8, decoded by `blackbox-scene` as today.
  - The *sRGB* view is chosen on purpose (see colour).
- **Colour (the main parity issue):** the native renderer works in "gamma space": UNORM surface, UNORM textures, blending on 8-bit gamma values. Bevy is linear.
  - The custom material shader works in gamma space. It samples through an sRGB view and re-encodes, or samples a UNORM view (decide in the spike).
  - It does texture × vertex colour × 2, fog and the glossy maths exactly as `scene.wgsl` and `glossy.wgsl` do, then outputs `srgb_to_linear(result)`. Bevy's sRGB output encode then reproduces the native bytes.
  - Opaque, alpha-test, sky and fog pixels match within rounding. **Alpha-blend and additive layers blend in linear space and will differ**, so the parity harness uses looser tolerances there.
- **Meshes:**
  - A native `MeshDesc` with N `DrawRange`s becomes N Bevy `Mesh`es (Bevy has one material per entity). Each one is compacted to the vertices it uses; indices are rebased and kept `u16` when they fit, `u32` otherwise.
  - Vertex attributes: `POSITION`, `NORMAL`, `UV_0`, and a custom `MeshVertexAttribute` for the `Unorm8x4` BGRA colour.
- **Material:** `BlackboxMaterial: Material`, `#[bindless]` where supported.
  - Specialisation keys:
    - `Shading`: Prelit, Lit, Sky (no fog), Glossy;
    - `BlendMode`: `AlphaMode::Opaque`, `Mask(0.5)`, `Blend`, `Add`.
  - Uniforms: fog and globals from `FrameParams`, plus the texture LOD bias.
  - Glossy: a `GlossyMaterial` uniform, a rig uniform and an environment cube map. Port `glossy.wgsl` to WESL and share the pure shading functions with the native WGSL (one shared `shading.wgsl` include).
- **Instances and the 80,000-object city:** one entity per `(InstanceKey, draw range)`.
  - Keys seen this frame are spawned or updated; `Transform` is written only when it changes.
  - Keys missing this frame are set to `Visibility::Hidden` at once and despawned after a grace period of about 2 s, so streaming doesn't churn entities.
  - Bevy batches same-mesh and same-material entities into instanced, GPU-preprocessed draws. Bevy's own frustum culling stays on or is turned off (`NoFrustumCulling`); the spike measures both, since the scene already culls and picks LODs.
  - **Fallback if the spike's CPU criterion fails:** a custom instanced draw (Bevy's "custom shader instancing" pattern) with one entity per mesh and an instance buffer. It is cheaper on CPU but loses Bevy's prepass, motion vectors and Solari integration for the city, so it is the cut-down mode in §8.
- **Draw order:** native draws Opaque, then AlphaTest, then AlphaBlend in *submission order* (unsorted), then Additive. Bevy sorts transparent items by distance. Use a per-material `depth_bias` or sort-key tweak to mirror the order; any remaining difference is a known parity gap the spike reports.
- **Reverse-Z:** Bevy is already infinite reverse-Z. The camera gets a custom `Projection` (`CameraProjection` impl) built from `FrameParams::projection` and a `GlobalTransform` that is the inverse of `view`. The clear colour comes from `FrameParams`.
- **Sky:** the same material with Sky shading. It needs no far plane.
- **Fog:** in the material shader, for parity. Bevy's `DistanceFog` is not used in raster mode.
- **Effect layer:** render-world systems in `Core3d` after `main_transparent_pass_3d`, calling `blackbox-gpu-passes`:
  - surfaces, glows and streaks with depth testing;
  - soft particles sampling the view depth (needs `DepthPrepass` or a depth texture with `TEXTURE_BINDING`; **unverified** in Bevy 0.20, checked in PR 9);
  - textured effects.
- **UI layer:** a render-world system ordered after `upscaling`. It draws the shared UI pass with `LoadOp::Load` into the window's output texture at surface resolution, so it is never post-processed (the same contract as native). Texture patches are applied through the shared pass's texture table. egui, the FEng presenter and movies keep handing `UiLayer` and `UiTexturePatch` over, unchanged (ADR 0002 holds).
- **Capture:**
  - windowed: Bevy's `Screenshot` with an observer that fills the capture slot;
  - `--screenshot` and tests: a headless camera targeting an `Image` plus GPU readback. Neither needs a visible window.
- **What Bevy gives for free:** `Bloom`, `Tonemapping` (map `Aces` to `AcesFitted`; Bevy's ACES fit is not Narkowicz's, a documented difference; `Off` maps to `None`), `Fxaa`, `Smaa`, `TemporalAntiAliasing`, `ContrastAdaptiveSharpening`, `MainPassResolutionOverride` (render scale), `MipBias`, `TemporalJitter`, DLSS, Solari.
- **What we add:**
  - FSR 1 (shared pass, a system after tonemapping and FXAA, before `upscaling`);
  - FSR 3 (system in `EarlyPostProcess`, like DLSS);
  - FSR 4 (same slot, DX12 only);
  - bilinear (Bevy's own `upscaling` at the reduced main-pass size).

### 3.3 Ray tracing (Solari)

- **On at startup only** (`ray_tracing != off`, `ray-tracing` feature, probe reports the features). Then:
  - `SolariPlugins` is added;
  - `SolariLighting` goes on the camera (Low, Medium, High map to bounces, samples and ReSTIR);
  - each world instance gets a parallel `RaytracingMesh3d` with `{POSITION, NORMAL, UV_0, TANGENT}` (generated tangents), `U32` indices and a `StandardMaterial`:
    - `base_color_texture` = diffuse texture;
    - additive and glow textures become emissive;
    - alpha-tested and blended geometry is excluded from the BLAS;
    - pre-lit vertex colours are dropped. Optionally they become an albedo darkening factor, an owner decision because it double-counts baked occlusion.
  - Lights: the rig's sun becomes a `DirectionalLight`, the sky becomes `Atmosphere` or `EnvironmentMapLight`.
  - The denoiser is DLSS-RR when the `dlss` feature is built and the GPU is an NVIDIA RTX. Otherwise there is only temporal accumulation (noisy; the menu says so).
- **Off:** no plugin, no ray-tracing meshes, no extra device features. Nothing is allocated.
- **Limits to measure:** 5,000 bound textures and 500 mesh slabs (ADR 0003's reading). The city has 2,111 shared plus tile textures. "Low" may cover only nearby tiles and the cars.
