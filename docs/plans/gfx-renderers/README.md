# Plan: swappable renderers (native Black Box and Bevy) behind one interface

*Planning document for the nfs-mw workspace. Retrieved and checked 2026-10-09. Base: the milestone-8 stack tip `docs/m8-reshade` (cb74ba4) plus the low-end layer `feat/m8-low-end-options` (local branch, 111db46, with uncommitted `car_shading` work in its worktree). This plan replaces the part of ADR 0003 that says "keep only `blackbox-render`". ADR 0004 records that change (outline in §9).*

The decision this plan supports is [ADR 0004](../../decisions/0004-swappable-renderers.md) (proposed). The plan is
split over several files to respect the 500-line limit; section numbers (`§N`) are the plan's own and keep
their meaning across files.

| Sections | File | Covers |
|---|---|---|
| 0 | this file | Summary of the recommendation and the facts that shape the plan |
| 1, 2 | [interface.md](interface.md) | The `blackbox-gfx` interface crate (trait, capabilities, data-model changes, runtime and compile-time selection) and what moves where |
| 3 | [bevy-backend.md](bevy-backend.md) | The `blackbox-bevy-render` backend: façade and op queue, rendering Black Box assets, ray tracing |
| 4, 5 | [upscalers-settings.md](upscalers-settings.md) | Upscaler and anti-aliasing capability matrix, who can test what, settings, presets and low-end |
| 6 | [pr-list.md](pr-list.md) | The 14-PR stack with per-PR detail |
| 7 to 11 | [verification-risks.md](verification-risks.md) | Parity and verification, the go/no-go spike criteria, risks, owner decisions, the ADR 0004 outline, docs to write, what could not be verified, sources |

---

---

## 0. Summary of the recommendation

1. **Add a renderer-neutral interface crate, `libs/blackbox-gfx`.** It has no wgpu and no Bevy dependency. It holds:
   - every type callers use today: vertex, texture, mesh, draw range, blend, shading, glossy, lighting rig, effect layer, UI layer, frame parameters and instances;
   - a `RenderBackend` **trait object**;
   - a `Capabilities` struct;
   - typed `GraphicsSettings` with a pure `resolve(requested, caps)`, so a setting the backend can't run falls back gracefully.
2. **`libs/blackbox-render` stays the native backend.** It keeps its name, wgpu and the Vulkan/DX12/GL support, and implements the trait. Its feature set is frozen at FXAA, bloom, ACES tone mapping, render scale with bilinear upscaling, and FSR 1. It is the default renderer and the only one for OpenGL and low-end PCs.
3. **New optional `libs/blackbox-bevy-render`.** It is a Bevy 0.20 renderer backend added as plugins to the *same* Bevy `App` the game already runs (ADR 0001 shell). It sits behind the `renderer-bevy` cargo feature of `nfsmw`, which is **off by default**, so default builds never compile `bevy_render`. It offers the native set plus:
   - TAA;
   - Bevy Solari ray tracing;
   - FSR 3 on DX12 and Vulkan, through a backend-agnostic WGSL port in `libs/fsr3-wgpu`;
   - FSR 4 on DX12 only, through AMD's FidelityFX API DLL. This is experimental and the PR can be cut;
   - DLSS through Bevy's `dlss` feature, which wraps `dlss_wgpu`. Vulkan and NVIDIA only.
4. **Choosing the renderer:** `renderer = blackbox|bevy` is chosen at startup (CLI, env, TOML, console, menu; it needs a restart). A wgpu adapter probe runs before the `App` is built. A Bevy request that the build or the hardware can't satisfy falls back to `blackbox` with a log line.
5. **A go/no-go spike PR comes early (PR 8)** with numeric criteria (§8). If it fails, the fallback still keeps the interface, the settings work and the FSR 3 crate, and moves TAA/DLSS/FSR 3 into the native backend (the ADR 0003 path).
6. **Stack:** 14 PRs (§6). Several can run in parallel (FSR 3 core, shared passes, settings, parity harness).

### Facts that shape the plan (checked 2026-10-09)

| Fact | Consequence |
|---|---|
| **Bevy 0.20 final is out** (2026-10-08). The workspace pins `=0.20.0-rc.2`. The `bevy` umbrella crate declares `rust-version = 1.97.1` (per ADR 0003). The crates.io API shows `rust_version: null` for `bevy_render`, `bevy_anti_alias` and `bevy_solari` 0.20.0. | Moving the pin is its own PR. The MSRV may rise from 1.95 to 1.97.1; check it with `cargo +1.95 check`. The local toolchain is 1.99.0. |
| **Bevy 0.20 replaced render-graph nodes with systems in the `Core3d` schedule.** Ordering uses `Core3dSystems::{Prepass, MainPass, PostProcess}`, and `upscaling` runs after `PostProcess`. DLSS runs as systems in `Core3dSystems::EarlyPostProcess`. | Our custom passes (UI layer, effects, FSR 1, FSR 3) become render-world systems ordered against these sets. |
| **Bevy 0.20 shaders are WESL.** Plain WGSL without directives still works. | Materials that import Bevy's mesh and view bindings must be `.wesl`. `.gitignore` (a whitelist) and the size check must learn `*.wesl`. |
| **Solari 0.20 is experimental.** Raytraced entities need `RaytracingMesh3d` with exactly `{POSITION, NORMAL, UV_0, TANGENT}`, a TriangleList, `Indices::U32`, `Mesh::enable_raytracing`, and **`MeshMaterial3d<StandardMaterial>` only**. It needs the wgpu features `EXPERIMENTAL_RAY_QUERY` plus binding arrays. Lights: directional, emissive, Atmosphere, EnvironmentMapLight. No point, spot or rect lights. Alpha-mask support is still not documented. | Ray tracing re-lights a PBR stand-in of the world. It cannot keep the pre-lit look. Alpha-tested props are excluded from ray tracing. |
| **Solari's only realtime denoiser is DLSS Ray Reconstruction.** The 0.20 notes say `dlss_wgpu` now supports DLSS-RR 4.5. There is no denoiser on Metal. | Usable ray tracing means NVIDIA RTX on Vulkan. Elsewhere it is noisy. |
| **`dlss_wgpu` 6.0.0** (2026-09-16): wgpu 30, DLSS SDK **310.9.1**, **Vulkan only**, Windows and Linux x86_64. Build needs `DLSS_SDK`, `VULKAN_SDK` and clang. It has a `mock` feature (also `debug_overlay`). MIT OR Apache-2.0. | DLSS builds only where the SDK is installed. CI compile-checks it with the mock. |
| **Bevy's DLSS:** cargo feature `dlss` on `bevy_anti_alias`, which turns on `bevy_render/raw_vulkan_init`. `force_disable_dlss` maps to `dlss_wgpu/mock`. You insert `DlssProjectId(Uuid)`, add `DlssInitPlugin` before `RenderPlugin`, then `DlssPlugin`. The camera gets a `Dlss<F>` component, which pulls in `TemporalJitter`, `MipBias`, `DepthPrepass`, `MotionVectorPrepass` and `Hdr`. Quality is set with `DlssPerfQualityMode {Auto, Dlaa, Quality, Balanced, Performance, UltraPerformance}`. **There is no preset selection.** | See the DLSS versions row below. |
| **"DLSS 4.5"** is the second-generation transformer Super Resolution models (presets L and M), added in SDK 310.5.0. Per the release notes as quoted second-hand: M is the default for Performance, L for Ultra Performance, K (DLSS 4) for DLAA, Quality and Balanced. | SDK 310.9.1 (`dlss_wgpu` 6) contains them. **Unverified:** the preset defaults per mode; check NVIDIA's programming guide. Picking L or M at Quality needs an upstream `dlss_wgpu` change or the NVIDIA app's override. |
| **"DLSS 5"** is real. It was announced at GTC on 2026-03-16 as *neural rendering*, a final-stage model that adds lighting and materials. It is not an upscaler. Reports say it launched for RTX 50 on 2026-09-03 (40-series announced, no date), at about 50–60 % performance cost. Integration is through **NVIDIA Streamline**. | It is not in the NGX Super Resolution SDK `dlss_wgpu` wraps. **Not reachable from wgpu or Bevy today. Not planned.** It becomes a revisit trigger in ADR 0004. |
| **AMD FSR SDK 2.3.0 "Redstone"** ships FSR 4.1.1 (ML) as a signed DX12 binary. Its Known Issues list says "Vulkan is currently not supported in SDK". FSR 3.1 got Vulkan in SDK 1.1.x (2024). FSR 4 needs RX 7000 or 9000 (per ADR 0003's source). No Rust FSR 3 or FSR 4 crate exists for wgpu or Bevy; the `fsr` crate is FSR 2 over raw `ash`/`windows` handles. | FSR 3 on both APIs means a WGSL port of the MIT FSR 3.1 upscaler. FSR 4 means calling AMD's DX12 DLL through wgpu-hal. |
| **This laptop** has an Intel Iris Xe (Alder Lake-P GT2) **and an NVIDIA RTX 3050 Ti Mobile (GA107)**. The NVIDIA kernel driver is not loaded right now (`nvidia-smi` fails), and the Mesa ICDs include **lavapipe**. Lavapipe has implemented `VK_KHR_ray_query` since Mesa 24.1. | With the NVIDIA driver enabled (owner decision, not done here), DLSS and Solari could be tested locally. Lavapipe allows headless GPU tests in CI, and maybe a tiny Solari smoke test; not verified that wgpu enables all five Solari features on lavapipe. |
| `.png` is a forbidden extension in `xtask/src/leak.rs`. | Golden images can't be committed. Goldens become Rust-const digests (§7). |
