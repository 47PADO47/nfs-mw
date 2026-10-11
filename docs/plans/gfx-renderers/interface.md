# Swappable renderers: the interface crate and re-homing

Part of the [swappable renderers plan](README.md): sections 1 and 2.

## 1. The interface crate: `libs/blackbox-gfx`

### 1.1 Dispatch: trait objects

**Decision:** `Box<dyn RenderBackend>` in `Host`, and `&mut dyn RenderBackend` everywhere scenes take `&mut Renderer` today.

- **Why:**
  - The set of backends is open (other Black Box projects may add one), and variants are compiled in or out by cargo features. An enum in `libs/` would need cfg-gated variants and a match in every method.
  - Calls are coarse: one `render` per frame, plus uploads per streamed tile. Virtual-call cost doesn't matter.
  - `Scene` is already a trait object (`Box<dyn Scene>`).
- **Rejected:**
  - Generics (`Scene<R>`) can't sit behind `Box<dyn Scene>`, and they would monomorphise the game twice.
  - Enum dispatch couples `libs/` to every backend.
  - Cargo-feature-only selection (exactly one backend per build) doesn't allow a runtime `--renderer` choice.
- **Object safety:** no generic methods. Constructors are not on the trait; each backend has its own factory, because the native one needs window handles and Bevy's needs an `App`.

### 1.2 Module layout (each file under 500 lines)

```
libs/blackbox-gfx/src/
  lib.rs            re-exports
  backend.rs        trait RenderBackend
  api.rs            GraphicsApi {Auto, Vulkan, Dx12, Gl} (today's `Backend`, renamed; `Backend` kept as a type alias for one PR)
  error.rs          RenderError
  handles.rs        TextureHandle, MeshHandle, GlossyMaterialHandle, UiTextureId, CaptureId; raw ctor/accessor "for backends"
  mesh.rs           Vertex (36-byte), MeshDesc, DrawRange, BlendMode, Shading
  texture.rs        PixelFormat {Bc1, Bc2, Bc3, Rgba8}, TextureDesc
  material.rs       GlossyMaterial, LightingRig, DirectionalLight, SkyGradient, Environment<'a> {Sky(SkyGradient), Faces{size, faces}}
  frame.rs          FrameParams, Projection, Fog, Instance, InstanceKey
  effects.rs        EffectLayer, EffectVertex, TexturedEffect (moved unchanged)
  ui.rs             UiLayer, UiMesh, UiVertex, UiTexturePatch (moved unchanged)
  capture.rs        RgbaImage, CaptureRequest
  stats.rs          RenderStats, BackendInfo
  caps/mod.rs       Capabilities, small hand-rolled bitsets (no new dependency)
  caps/resolve.rs   resolve(requested, caps) -> Resolved {effective, downgrades}
  settings/mod.rs   GraphicsSettings
  settings/post.rs  PostSettings, Tonemap, Antialiasing {Off, Fxaa, Smaa, Taa}
  settings/upscale.rs  Upscaler {Off, Bilinear, Fsr1, Fsr3, Fsr4, Dlss}, UpscaleQuality, sharpness, LOD-bias helpers (temporal: log2(s) - 1)
  settings/render_scale.rs  moved from blackbox-render
  settings/ray_tracing.rs   RayTracing {Off, Low, Medium, High}
```

Dependencies: `glam 0.34`, `bytemuck`, `thiserror`. There is no wgpu, no Bevy and no `raw-window-handle`. The Bevy backend converts `glam` 0.34 to Bevy's `glam` 0.33 at the boundary, through `to_cols_array`.

### 1.3 Changes to the data model (all small, all needed by temporal methods or Bevy)

- **`FrameParams`**:
  - **New fields:**
    - `view: Mat4`;
    - `projection: Projection::PerspectiveInfiniteReverse { fov_y, aspect, near } | Identity`, replacing `view_proj`, with a helper `view_proj()`;
    - `fog: Option<Fog { start, end }>`, replacing the `f32::MAX` convention;
    - `camera_cut: bool`: a teleport, a scene switch or a camera toggle. It resets the TAA, DLSS and FSR 3 history.
  - **Unchanged:** `camera_position`, `light_dir`, `clear_color`.
  - **Why:** Bevy needs a view and a projection separately, backends apply sub-pixel jitter themselves, and history must reset on cuts.
  - Callers never jitter. Scene culling and LOD keep using the unjittered matrices.
- **`Instance`** gains **`key: InstanceKey(u64)`**, a stable identity per placed object (`InstanceKey::TRANSIENT` allowed).
  - The world scene uses `(tile, instance index)` and the car rig uses `(car, part)`.
  - The native backend ignores the key. The Bevy backend maps key to entity, so `PreviousGlobalTransform` gives correct motion vectors. Without stable keys, pooled entities would swap identities and smear TAA and DLSS.
- **Capture becomes asynchronous:** `request_capture` and `poll_capture`. The native backend completes at once. Bevy completes one to three frames later through `Screenshot` or GPU readback.

### 1.4 Trait sketch

```rust
pub trait RenderBackend {
    fn info(&self) -> &BackendInfo;                 // renderer name, api, adapter, driver
    fn capabilities(&self) -> &Capabilities;

    // surface
    fn resize(&mut self, size: [u32; 2]);
    fn surface_size(&self) -> [u32; 2];
    fn set_vsync(&mut self, vsync: bool);

    // graphics settings: resolved against capabilities, effective + downgrades returned
    fn apply_graphics(&mut self, requested: &GraphicsSettings) -> Resolved;
    fn graphics(&self) -> &GraphicsSettings;        // effective
    fn render_size(&self) -> [u32; 2];

    // resources (handles allocated synchronously by the backend)
    fn create_texture(&mut self, desc: &TextureDesc<'_>) -> TextureHandle;
    fn destroy_texture(&mut self, h: TextureHandle);
    fn redirect_texture(&mut self, from: TextureHandle, to: Option<TextureHandle>);
    fn create_mesh(&mut self, desc: &MeshDesc<'_>) -> MeshHandle;
    fn destroy_mesh(&mut self, h: MeshHandle);
    fn create_glossy_material(&mut self, m: &GlossyMaterial) -> GlossyMaterialHandle;
    fn destroy_glossy_material(&mut self, h: GlossyMaterialHandle);

    // lighting
    fn set_lighting_rig(&mut self, rig: &LightingRig);
    fn set_environment(&mut self, env: Environment<'_>);

    // layers
    fn set_effects(&mut self, layer: &EffectLayer);
    fn update_ui_texture(&mut self, patch: &UiTexturePatch<'_>);
    fn free_ui_texture(&mut self, id: UiTextureId);
    fn set_ui_layer(&mut self, layer: UiLayer);

    // frame
    fn render(&mut self, frame: &FrameParams, instances: &[Instance]) -> Result<FrameStatus, RenderError>;
    fn request_capture(&mut self, size: [u32; 2], frame: &FrameParams, instances: &[Instance])
        -> Result<CaptureId, RenderError>;
    fn poll_capture(&mut self, id: CaptureId) -> Option<Result<RgbaImage, RenderError>>;

    fn stats(&self) -> RenderStats;  // meshes, textures, effect/streak capacities, adapter summary
}
```

- `supports_bc()` becomes `capabilities().compressed_bc`.
- `effect_capacities()` and `streak_capacity()` move into `stats()`.
- `aspect_ratio()` is computed from `surface_size()`.
- `blackbox-scene` takes `&mut dyn RenderBackend` and **depends on `blackbox-gfx` only**, so solid and TPK upload works for every backend.

### 1.5 Capabilities and graceful degradation

```rust
pub struct Capabilities {
    pub renderer: &'static str,            // "blackbox" | "bevy"
    pub api: GraphicsApi,
    pub compressed_bc: bool,
    pub hdr_targets: bool,
    pub antialiasing: AaSet,               // {Off, Fxaa, Smaa, Taa}
    pub upscalers: UpscalerSet,            // {Off, Bilinear, Fsr1, Fsr3, Fsr4, Dlss}
    pub tonemaps: TonemapSet, pub bloom: bool,
    pub ray_tracing: RtSupport,            // None | Available { denoiser: Option<Denoiser::DlssRr> }
    pub render_scale: (f32, f32),
    pub restart_required: RestartSet,      // which settings need a restart on this backend
}
```

`resolve()` is pure and unit-tested in this crate. Its rules:

- An unavailable upscaler falls back along `fsr4 → fsr3 → fsr1 → bilinear` and `dlss → fsr3 → fsr1`.
- A temporal upscaler (FSR 3, FSR 4, DLSS) **replaces** anti-aliasing: TAA, FXAA and SMAA are turned off, with a note.
- TAA unavailable falls back to FXAA.
- Ray tracing unavailable falls back to off. Ray tracing without a denoiser is allowed with a "noisy" note.
- The render scale is ignored when a temporal upscaler's quality mode decides it.
- Each step adds a `Downgrade { setting, requested, effective, reason }`. The app logs these, prints them in the console and uses them for the menu rows.

### 1.6 Choosing the backend at runtime and at compile time

- **Runtime:** setting `renderer = blackbox|bevy` (`--renderer`, `NFSMW_RENDERER`, config `renderer`, console `set renderer`, Video menu row; marked "applies after restart"). Default `blackbox`.
  - The existing `backend` key (graphics API) stays. The docs must separate *renderer* (who draws) from *backend* (Vulkan, DX12, GL).
  - In `app::run`, before `App::new()`:
    - `renderer=bevy` without the feature compiled: error log, use `blackbox`.
    - `renderer=bevy` with `backend=gl`: not supported; log and use `blackbox`.
    - Otherwise run `blackbox_bevy_render::probe(api)`, a wgpu adapter probe that reports API, vendor, BC support, ray-query and binding-array features, and whether the GPU is NVIDIA. No usable adapter: log and use `blackbox`.
    - Only then add `BlackboxBevyRenderPlugin`, which brings `RenderPlugin` and friends. Bevy panics if it finds no adapter after the app is built, so all fallbacks happen before that.
- **Compile time (cargo features):**
  - `nfsmw`:
    - `renderer-bevy` adds the optional dependency on `blackbox-bevy-render`;
    - `ray-tracing`, `fsr3`, `fsr4` and `dlss` each imply `renderer-bevy`;
    - `dlss-mock` (CI);
    - `default = []`.
  - `blackbox-bevy-render`:
    - `ray-tracing` adds `bevy_solari`;
    - `dlss` adds `bevy_anti_alias/dlss` and `bevy_solari/dlss`;
    - `dlss-mock` adds `bevy_anti_alias/force_disable_dlss`;
    - `fsr3` adds `fsr3-wgpu`;
    - `fsr4` adds `ffx-api-wgpu` (Windows only).
  - Default `cargo build -p nfsmw` stays Bevy-render-free. PR 8 verifies this with `cargo tree -p nfsmw -e normal | grep bevy_render`, which must be empty.
  - The new libs are workspace members, so `cargo clippy --workspace` still lints them. Remove them from `default-members` so a plain `cargo build` at the root stays lean.
  - Compile-time and binary-size deltas are **unverified** and measured in PR 8. Guess: +1.5–3 min cold build and +15–30 MB release binary with `renderer-bevy`.


## 2. Re-homing: what moves, what changes, how every PR stays green

| Today (stack tip) | Goes to | How |
|---|---|---|
| `blackbox-render/src/{api, backend, effects, glossy, ui, post_settings, render_scale, upscale}.rs` (types and pure helpers) | `blackbox-gfx` | `git mv`, then `pub use blackbox_gfx::*` in `blackbox-render/lib.rs`. Downstream crates compile unchanged (PR 3). |
| `Renderer`'s inherent methods | `impl RenderBackend for Renderer` (PR 4). The inherent methods stay as thin wrappers until PR 5 removes callers. | — |
| `gpu/ui.rs`, `gpu/effects.rs`, `gpu/soft_particles.rs`, `gpu/textured_effects.rs`, `gpu/post/{filter, fsr1}.rs` and their WGSL | `libs/blackbox-gpu-passes` (PR 7): plain wgpu building blocks that take `&Device, &Queue, &mut CommandEncoder`, views and formats | The native backend calls them. The Bevy backend calls the same code from `Core3d` systems. The existing GPU tests move with them. |
| `crates/nfsmw`: `app/host.rs` (`Option<Renderer>`), `viewer/mod.rs` (`Scene` takes `&mut Renderer`), about 40 files that import `blackbox_render::` | `Option<Box<dyn RenderBackend>>`, `&mut dyn RenderBackend`, `blackbox_gfx::` imports | PR 5, mechanical, one commit per area (scene trait; world scene; car scene; app and screenshot; settings imports). |
| `app/render.rs` (the render bridge) | Becomes `app/render/{mod, native, bevy}.rs`: `native.rs` holds today's `create_renderer`; `bevy.rs` takes the façade from the plugin once `RenderDevice` exists | PR 5 does the split; PR 8 adds `bevy.rs`. |
| `app/post.rs` and `app/upscale.rs` | One `app/graphics.rs`: `backend.apply_graphics(&settings.graphics())`; it logs downgrades when they change | PR 6 |
| `app/screenshot.rs` | Request the capture, poll every frame, write the PNG and exit when ready | PR 5 |

Rules that keep each PR green:

- Moves always leave re-exports behind. Remove the re-exports only in the PR that migrates the last caller.
- No PR changes the image of the native backend except the ones that say so. PR 4 adds the digest tests that would catch it.
- A new crate lands with its own tests and with no caller wired in yet.
