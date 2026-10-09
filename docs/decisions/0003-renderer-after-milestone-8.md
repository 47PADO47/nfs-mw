# 0003 — The renderer after milestone 8: keep ours or adopt Bevy's

- **Status:** proposed (2026-10-09), for the project owner to accept. Answers the question
  [0001](0001-bevy.md) left open ("decide on B in milestone 8").
- **Milestone:** 8 (graphics).

## Question

[0001](0001-bevy.md) chose Bevy as the application shell and kept `blackbox-render` (our wgpu renderer) as
the renderer, and said to decide in milestone 8 whether to move to Bevy's renderer (option B), "when the car
shader and post-processing show whether Bevy's renderer can host them". Milestone 8 also lists four
features that depend on that choice: upscaling (FSR, DLSS), ReShade compatibility and Bevy Solari
(ray-traced lighting). This ADR decides the renderer and says where each of those four lands.

## Facts checked on 2026-10-09

Numbers in brackets are the sources at the end. "Read" means I read the source or manifest at the tag named;
nothing below was built or run, and no prototype was made.

### Bevy and wgpu

| Fact | Consequence |
|---|---|
| Bevy **0.20.0 final** was published on crates.io on **2026-10-08** (rc.1 2026-09-15, rc.2 2026-09-28; 0.19.0 2026-06-19; 0.18.0 2026-01-13) [1]. Our workspace still pins `=0.20.0-rc.2`. | The pin can move to the final release. Releases come about 3.5 to 5 months apart (0.18 to 0.19: 5.2, 0.19 to 0.20: 3.6), each with breaking changes. |
| `bevy_render` 0.20.0 depends on wgpu **^30**, `bevy_math` on glam **^0.33.2**; the `bevy` crate declares `rust-version = "1.97.1"` (rc.2: 1.96.0), `bevy_ecs` 1.95.0 [1][5]. We use wgpu 30, glam 0.34, `rust-version = "1.95"`. | One wgpu version across both stacks, so no type clash if a Bevy renderer ever replaces ours. Bumping to the final release may raise our MSRV (not checked per adopted crate). |
| Bevy 0.20 moved shaders from its own WGSL dialect to **WESL** and rebuilt the sprite renderer on the 3D infrastructure [2]. | Evidence for the churn cost of option B: custom shaders are migrated by hand on every release. |
| Bevy's `AntiAliasPlugin` adds FXAA, SMAA, TAA, contrast-adaptive sharpening and optional DLSS. **No FSR** [6]. | FSR is ours to write under either option. |
| Bevy uses an infinite reverse-Z projection (`perspective_infinite_reverse`) [5]. | Same depth convention as `blackbox-render`. |

### Bevy Solari

| Fact | Consequence |
|---|---|
| `bevy_solari` 0.20.0 is published (2026-10-08), MIT OR Apache-2.0, and documents `SolariPlugins` as "experimental" [5]. docs.rs failed to build 0.20.0, so the docs page shows 0.18.1 [4]. The 0.17 release notes called it "not yet production ready" [3]. | Maturity: experimental for three releases. I found no later statement that it is production ready. |
| It needs the wgpu features `EXPERIMENTAL_RAY_QUERY`, `BUFFER_BINDING_ARRAY`, `TEXTURE_BINDING_ARRAY`, `SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING`, `PARTIALLY_BOUND_BINDING_ARRAY` [5]. | Hardware floor is whatever exposes ray queries (next table). |
| It sets `DefaultOpaqueRendererMethod::deferred()`, takes only `StandardMaterial` meshes with exactly `{POSITION, NORMAL, UV_0, TANGENT}`, triangle lists and `u32` indices, and binds at most 5,000 textures and 500 mesh slabs [5]. It is written against `bevy_render` internals (`MeshAllocator`, the extract and render schedules). | **It cannot be used without Bevy's renderer.** Our pre-lit world, its alpha-test, additive and blend passes and the custom car shader are not `StandardMaterial`. |
| Lights supported in 0.20: directional, emissive meshes, `Atmosphere`, `EnvironmentMapLight`. Point, spot and rect lights: "future" [2]. Realtime output needs a denoiser; the only one is **DLSS Ray Reconstruction** (NVIDIA, Vulkan, Windows/Linux); macOS (Metal, new in 0.20) has none yet [2][3]. FSR Ray Regeneration exists but is a DX12 DLL [4][16]. | Without an NVIDIA RTX GPU the realtime mode is noisy. The author wrote in December 2025 that Solari "is still NVIDIA only in practice" [4]. I found nothing newer that changes this. |
| Per the 0.18 write-up: no skinned or morphed meshes, no alpha masks, no transparent materials; 0.18 cost 7 to 14 ms per frame on an RTX 3080 at 1600x900 upscaled to 3200x1800, about 6 ms of it DLSS-RR [4]. In the 0.20 Solari sources I read I found no alpha-mask handling (`extract.rs`, `assets.rs`, `bindings.wesl`). | MW's trees, fences and wires are alpha-tested, so the city would be mis-lit or missing. Not tested; 0.20 notes do not list alpha masks as fixed. |

### Ray tracing in wgpu itself

| Fact | Consequence |
|---|---|
| wgpu 30.0.0 was released 2026-07-01 [7]. Ray tracing is behind `Features::EXPERIMENTAL_RAY_QUERY` and documented as "experimental" and "subject to change"; only ray queries exist in naga, no ray-tracing pipelines; partial BLAS updates are unsupported [9]. | We can build ray-query effects in our renderer, but the API will break. |
| The wgpu-hal sources at v30.0.0 enable it on **Vulkan** when `VK_KHR_acceleration_structure`, `deferred_host_operations`, `buffer_device_address` and `VK_KHR_ray_query` are present, on **DX12** with ray tracing tier 1.1 and shader model 6.5, and Metal has acceleration structures since wgpu 29 [7][8]. | Vulkan and DX12 can both do it; **OpenGL cannot**. Which GPUs expose the extensions I did not verify; in general it is NVIDIA RTX, AMD RDNA2 and newer, Intel Arc. |

### DLSS

| Fact | Consequence |
|---|---|
| `dlss_wgpu` **6.0.0** (2026-09-16), MIT OR Apache-2.0, is a standalone crate on **wgpu 30**, not tied to Bevy [10]. It wraps NVIDIA's NGX DLSS Super Resolution and Ray Reconstruction. **Vulkan only**, Windows and Linux x86_64. It needs the SDK (v310.9.1) at build time (`DLSS_SDK`, the Vulkan SDK, clang) and ships `nvngx_dlss.dll` or `libnvidia-ngx-dlss.so.310.9.1` with the app [10]. | Feasible from our renderer, no Bevy needed. A `mock` feature builds without the SDK. |
| Integration goes through wgpu-hal: `dlss_wgpu::create_instance` and `request_device` call `Instance::init_with_callback`, `Adapter::open_with_callback`, `Instance::from_hal` and `create_device_from_hal` to add the Vulkan extensions NGX asks for, and return normal wgpu objects [10]. | Our one change is in `gpu/init.rs`: when the `dlss` feature is on and the backend is Vulkan, create the instance and device with these functions. |
| Per frame it needs colour at render resolution, depth, motion vectors, the camera jitter, exposure (or auto-exposure) and has a flag for a reverse depth buffer [10]. It has no frame generation [10]. Bevy lists frame interpolation as not planned [3]. | We have **no jitter and no motion vectors** today. That is the real work, and the same inputs any temporal upscaler (FSR 2 or later) needs. |
| The SDK licence is the **NVIDIA RTX SDKs LICENSE** (`NVIDIA/DLSS`, DLSS 310.9.1, commit 2026-09-08) [11]. It allows shipping the runtime library in object form inside an application with other functionality, but: the SDK may not be distributed on its own (no copy in the repo); attribution with NVIDIA marks on credits or the about box; notify NVIDIA before a commercial release; use only on NVIDIA GPUs; the SDK may not be put under an open source licence that requires source disclosure; use of NVIDIA marks beyond that needs written approval [11]. | Not MIT/Apache, so DLSS is an **optional, default-off Cargo feature**. The crate passes `cargo deny check licenses`; the SDK never enters `Cargo.lock`. |
| NVIDIA Streamline (cross-vendor wrapper, v2.14.1, Windows-centric build, interposer or manual hooking) [12]. | Streamline works by interposing the app's D3D or Vulkan calls, or by manual hooking with the app's own device; wgpu exposes neither cleanly (not tried). `dlss_wgpu` talks to NGX directly. Skip Streamline. |

### FSR

| Fact | Consequence |
|---|---|
| **FSR 1** (`GPUOpen-Effects/FidelityFX-FSR`, last push 2022-05-22): `ffx_fsr1.h` and `ffx_a.h`, MIT (AMD 2021; `ffx_a.h` also carries a 2014 MIT notice of Michal Drobot) [13]. Passes: EASU (edge-adaptive upscale) and RCAS (sharpening). Written for HLSL and GLSL; I found no Rust or WGSL port [14]. | Portable to WGSL by hand, with the notices kept. Spatial: no motion vectors, no jitter. |
| FSR 1 wants an **already anti-aliased**, noise-free input in a perceptual (sRGB-like) space, placed after anti-aliasing and tone mapping and before film grain and UI [14]. | We have no anti-aliasing today. The M8 chain needs MSAA, FXAA or SMAA ahead of FSR 1. |
| **FSR 2.2.1** repo: MIT-style, DX12 and Vulkan backends, HLSL, needs depth, motion vectors, jitter, reactive masks; supports inverted and infinite depth [15]. The Rust bindings `fsr2-sys` last changed 2023-05-24 and have no licence on the repo. | Option for later: port the HLSL shaders to WGSL. Large, and needs the same temporal inputs as DLSS. |
| **FidelityFX SDK 2.3.0** (2026-06-24): FSR 3.1.5 upscaler source is in its MIT-licensed file list, but "Vulkan is currently not supported" in the SDK; **FSR 4 is a signed binary DLL** (DX12), "no reverse engineering", and "requires an AMD 7000 series or 9000 series GPU or later" [16]. | FSR 3.1 and FSR 4 are not usable through wgpu. Neither is the frame-generation swapchain (a DXGI swapchain replacement). |

### ReShade

| Fact | Consequence |
|---|---|
| ReShade is BSD-3-Clause, Windows only, latest tag v6.8.0 (commit 2026-08-02) [17]. It injects into the app's graphics API: D3D9 to D3D12, OpenGL, and Vulkan through an implicit **Vulkan layer** registered by its installer (since 5.1 only for executables listed in `ReShadeApps.ini`) [17][19]. | Nothing for us to link or ship. We must stay injectable, and document how. |
| No native Linux build; the author said a port is unlikely because the hooking code is Windows specific, and pointed to **vkBasalt** for Vulkan (Zlib, last push 2023-10-04) [19][20]. The Windows ReShade Vulkan layer works under Wine only for Windows-Vulkan apps, not for D3D through DXVK [19]. | Linux players use vkBasalt; Windows build under Wine with `--backend vulkan` might take ReShade. **Untested.** |
| `ReShade.fxh` defaults to `RESHADE_DEPTH_INPUT_IS_REVERSED 1`, `..._IS_UPSIDE_DOWN 0`, `..._IS_LOGARITHMIC 0`, `RESHADE_DEPTH_LINEARIZATION_FAR_PLANE 1000.0` [18]. Its linearisation is `lin = d' / (F - d' * (F - 1))` with `d' = 1 - d` when reversed [18]. | See "ReShade" below: our depth is reverse-Z with no far plane, and the default far-plane setting is wrong for it. |

## Options

### A. Keep `blackbox-render` (wgpu) and add what milestone 8 needs ourselves

- The renderer is about 2,700 lines (Rust and WGSL, tests included) plus 273 in `blackbox-scene`. It already does reverse-Z, pre-lit
  and lit shading, fog, instancing of ~80,000 scenery copies with our culling, BC textures, effects, UI layer
  and off-screen capture, on Vulkan, DX12 **and OpenGL**.
- The car shader in `specs/car-assembly.md` §8 is a closed-form formula (three directional lights, a facing term,
  a cube map). In plain WGSL it is a pipeline plus one shader; no render-graph work.
- FSR 1, a post chain and the DLSS seam are written by us, but small and understood.
- Costs: bloom, tone mapping, SSAO, temporal anti-aliasing and the like are ours to write. **Solari cannot be
  adopted**; ray-traced effects would be our own wgpu ray-query work.

### B. Move to Bevy's renderer now

- Gets FXAA, SMAA, TAA, CAS, bloom, tone mapping, DLSS (`dlss` feature of `bevy_anti_alias`) and Solari.
- The world has to become `StandardMaterial`-compatible for Solari: the city is pre-lit (vertex colour times 2,
  no sun) and Solari replaces lighting with PBR. We would have to invent roughness, metallic and emissive values
  for assets that have none, and the original look would change. Alpha-tested props are an open problem.
- Rewrites everything milestone 3 made work (see 0001), plus the soft particles, effect layer and UI layer from
  milestones 5 and 6, and Solari cannot run on OpenGL (it needs ray queries); I did not evaluate Bevy's own GL path.
- Custom shaders follow Bevy's shader and render-schedule changes every release (WESL in 0.20).
- `blackbox-render`, built to be shared with other Black Box games, would be dropped or reduced to a plugin.

### C. Keep A as the product renderer; keep a Bevy-renderer experiment possible and separate

Same as A for milestone 8. In addition, if a trigger below fires, a time-boxed spike builds a small Bevy
renderer app (for example the car showroom of `view-car`, a few meshes, where `StandardMaterial` is a smaller
stretch than for the city) next to the main one, sharing `libs/` and using Bevy 0.20 or later. It decides
nothing until it has numbers. Nothing in `libs/` changes for it.

| | A / C | B |
|---|---|---|
| Car shader (§8) | one WGSL pipeline | custom Bevy material and shader |
| Pre-lit city, alpha-test, additive passes | as today | rewrite; not PBR |
| OpenGL backend | kept | Solari impossible there; Bevy's GL path not evaluated |
| FSR 1 | port to WGSL | port to WESL (Bevy has none) |
| DLSS Super Resolution | `dlss_wgpu` through wgpu-hal (Vulkan) | `dlss` feature of `bevy_anti_alias` (Vulkan) |
| Solari | not possible | possible, with the material and denoiser limits above |
| ReShade | works the same | works the same |
| Breaking-change load | wgpu bumps (we already follow) | wgpu, Bevy render and WESL every 3.5 to 5 months |

## Decision

**Adopt C: keep `blackbox-render` as the renderer through milestone 8 and until a revisit trigger fires. Do not
move to Bevy's renderer, and do not make Solari part of milestone 8's deliverable.**

Reasons, in order of weight:

1. **The features that justify B do not fit this game's content yet.** Solari needs PBR deferred meshes,
   lacks alpha masks (as far as I could verify), supports few light types and is, in practice, NVIDIA-only for
   a usable image. MW's world is pre-lit; ray-traced lighting on top of baked lighting is a different look,
   not a drop-in.
2. **What Bevy's renderer offers that we can reach without it is already reachable.** DLSS works through
   `dlss_wgpu` on wgpu 30 with no Bevy; FSR 1 is MIT and small; Bevy has neither FSR nor anything we cannot
   write for a fixed-look game. ReShade is independent of the engine.
3. **The car shader is small and the evidence is that a plain WGSL pipeline hosts it.** The condition 0001 set
   ("whether Bevy's renderer can host them") is answered the other way round: nothing in the car shader or the
   post chain needs Bevy.
4. **Cost and risk of B are front-loaded and recurring**; the benefit (Solari) is experimental.

The decision is reversible: 0001's seam rules still hold (one render bridge, no wgpu types in game code,
rendering code stays out of game crates), so a later move replaces one layer.

### What each milestone 8 item becomes

| Item | Decision |
|---|---|
| Car shader, lighting rig, tire smoke and skid marks, exhaust flames | **In M8**, in `blackbox-render` (WGSL). |
| Post-processing | **In M8**: render to an off-screen scene target, then a small chain (anti-aliasing, then the chain's other passes, then UI at native resolution). Keep passes optional so the plain path stays. |
| Anti-aliasing | **In M8**, prerequisite of FSR 1: MSAA, or FXAA/SMAA (SMAA is MIT in Bevy and can be a reference). |
| FSR 1 (EASU + RCAS) | **In M8**: port `ffx_fsr1.h` to WGSL, keep AMD's and Drobot's notices in the file, list it in `NOTICE`. Render scale setting in the video options. |
| DLSS Super Resolution | **Seams in M8, integration after.** M8 adds what any temporal upscaler needs: render scale target, projection jitter, per-instance previous transform and a motion-vector output, mip bias, a reverse-Z flag. The `dlss` feature (dlss_wgpu, Vulkan only, off by default) follows once there is NVIDIA RTX hardware to test on. |
| FSR 2 / 3 style temporal upscaler | **Future.** Vendor-neutral complement to DLSS; ports FSR 2.2.1's MIT HLSL shaders to WGSL on the same inputs. FSR 3.1 frame generation, FSR 4, DLSS frame generation and Streamline: **not planned** (DX12-only or binary-only, not reachable through wgpu). |
| ReShade compatibility | **In M8** as a documented contract and a check (see below), no code beyond depth hygiene. |
| Bevy Solari | **Out of M8.** Future spike under C, gated by the triggers below. Roadmap row 8 should stop promising it. |

## ReShade

What we promise and what we document:

- **Injectable on Windows** with `--backend dx12` or `--backend vulkan`. For Vulkan the user installs ReShade with
  its setup tool, which registers the layer for `nfsmw.exe` (the app list in `ReShadeApps.ini`). OpenGL is
  supported by ReShade in general; I did not verify it with wgpu's GL backend. There is no D3D11 backend in wgpu,
  which is the most trodden ReShade path.
- **Depth.** `blackbox-render` writes `Depth32Float`, reverse-Z (1 at the near plane), no far plane
  ([api.rs](../../libs/blackbox-render/src/api.rs)), with `TEXTURE_BINDING` usage already set. In ReShade:
  `RESHADE_DEPTH_INPUT_IS_REVERSED=1` (the default of the standard shaders), `IS_UPSIDE_DOWN=0`,
  `IS_LOGARITHMIC=0`.
- **Far plane.** The linearisation in `ReShade.fxh` takes `d = near / z`, so `lin = (z - near) / (z + near * (F - 1))`
  with `F = RESHADE_DEPTH_LINEARIZATION_FAR_PLANE` (my derivation from [18], not tested in game). With the
  default `F = 1000` and a near plane of 0.1 m, `lin` reaches 0.5 at about 100 m and bends strongly before that.
  Effects that assume linear depth (fog, depth of field, SSAO) want `F` raised until the curve is flat over
  the distances they use. Document the near plane the viewers use next to this setting.
- **Render scale.** Once FSR or DLSS render below native, the depth buffer is smaller than the back buffer.
  ReShade's depth effects assume they match. State this limitation in the docs; the safe path for ReShade
  users is render scale 100 %. A native-resolution depth copy is possible later if it turns out to matter.
- **ReShade sees the final image** (post chain and HUD included), as it does for every game.
- **Linux:** no native ReShade; use vkBasalt on the native Vulkan build. Do not promise Wine behaviour until
  someone tests it.

A check for this: a headless test that renders a known reverse-Z scene and asserts the depth values
(`near / z`) so the contract cannot change unnoticed.

## Consequences

- **Good:** one more milestone on a renderer that works on three backends, no forced art-direction change, no
  wholesale rewrite; DLSS and FSR 1 reachable without Bevy; Solari kept honest as research, not a promise.
- **Cost:** we write and maintain a post chain, anti-aliasing and the upscaler inputs. If Solari's limits are
  lifted we will have paid for a renderer that cannot use it.
- **Risk:** the DLSS path cannot be built in default CI or tested without RTX hardware; wgpu's ray-query API will
  change under any ray-traced effect we write ourselves.
- **Licence handling:** `dlss` stays off by default and out of the default CI build; no NVIDIA binary or SDK
  enters the repository (the leak check refuses binaries). Releases that bundle `nvngx_dlss.dll` need the
  project owner to read the RTX SDKs LICENSE obligations first (credits attribution, notice to NVIDIA).
- **Follow-up outside this ADR:** move the Bevy pin from `=0.20.0-rc.2` to the final `=0.20.0` and re-check
  `rust-version` (a chore, separate from graphics).

## Revisit trigger

Re-open the question, and run the spike of option C, when **either** holds:

1. **Solari grows up.** All of: a denoiser that works without NVIDIA hardware on Vulkan or DX12; alpha-masked
   and transparent materials; point and spot lights; and a documented way to feed custom materials into its
   deferred G-buffer. Check at each Bevy release (next expected around February 2027 if the 3.5 to 5 month
   cadence holds; not announced).
2. **Build-versus-adopt tips.** After M8 ships, `blackbox-render` plus its shaders passes about 6,000 lines, or we
   are maintaining three or more passes Bevy ships (bloom, tone mapping, TAA, SSAO). The number is my
   estimate of when the shell stops being cheaper than the framework; adjust it if the owner disagrees.

The spike's exit criteria are numbers: frame time of the showroom with and without Solari on a named GPU, and
the days spent making the car materials work. If it wins, plan B for one scene (the showroom) before the city.

## Open questions and what I could not verify

- **No prototype.** The car shader on a custom Bevy material and the 80,000-instance city on Bevy's renderer were
  not tried. The case against B rests on content fit and churn, not on a measured failure.
- **Which GPUs** expose wgpu ray queries on Vulkan and DX12: not checked beyond the extension and tier
  requirements in the wgpu-hal source.
- **Pinned rc.2 versus final:** I read the `v0.20.0` tags. Differences in Solari between `rc.2` and `0.20.0` were
  not checked.
- **Alpha masks in Solari 0.20** are inferred from reading three files and from the absence of a note; confirm
  before relying on it.
- **ReShade with wgpu** (DX12 and Vulkan back ends, depth detection, OpenGL, Wine) is untested; the depth
  linearisation is derived from the header, not measured.
- **DLSS in practice** (validation noise, wgpu's Vulkan device flags, our render pass layout) is untested; the
  README says to expect validation errors from an NVIDIA bug [10].
- **Decisions for the owner:** accept the demotion of Solari from M8; whether releases should bundle the DLSS
  library; whether you have an NVIDIA RTX GPU to test on (DLSS), and an AMD or Intel GPU (FSR, vendor checks).

## Not decided here

- The settings entries for render scale and upscaler choice (video options of milestone 6/8).
- How the post chain is organised internally (a task of the pipeline layer).
- Whether a temporal upscaler other than DLSS is worth building (see "Future" above).

## Sources

Retrieved 2026-10-09. GitHub and docs.rs pages were read at the tag or release named.

1. crates.io: [bevy](https://crates.io/crates/bevy) versions via the [API](https://crates.io/api/v1/crates/bevy);
   [`bevy_render`](https://crates.io/api/v1/crates/bevy_render/0.20.0/dependencies),
   [`bevy_math`](https://crates.io/api/v1/crates/bevy_math/0.20.0/dependencies),
   [`bevy_anti_alias`](https://crates.io/api/v1/crates/bevy_anti_alias/0.20.0/dependencies) 0.20.0 dependencies.
2. Bevy 0.20 release notes: <https://bevy.org/news/bevy-0-20/> (2026-10-08).
3. Bevy 0.17 release notes (Solari, DLSS): <https://bevy.org/news/bevy-0-17/>.
4. jms55, "Solari in Bevy 0.18": <https://jms55.github.io/posts/2025-12-27-solari-bevy-0-18>;
   [bevy_solari on docs.rs](https://docs.rs/bevy_solari/latest/bevy_solari/).
5. Bevy `v0.20.0` sources: [`bevy_solari`](https://github.com/bevyengine/bevy/tree/v0.20.0/crates/bevy_solari)
   (`lib.rs`, `scene/`, `realtime/mod.rs`), [`Cargo.toml`](https://github.com/bevyengine/bevy/blob/v0.20.0/Cargo.toml),
   [`bevy_camera/src/projection.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_camera/src/projection.rs).
6. [`bevy_anti_alias/src/lib.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_anti_alias/src/lib.rs).
7. wgpu v30.0.0 [CHANGELOG](https://github.com/gfx-rs/wgpu/blob/v30.0.0/CHANGELOG.md) (2026-07-01).
8. wgpu-hal v30.0.0 [dx12](https://github.com/gfx-rs/wgpu/blob/v30.0.0/wgpu-hal/src/dx12/adapter.rs) and
   [vulkan](https://github.com/gfx-rs/wgpu/blob/v30.0.0/wgpu-hal/src/vulkan/adapter.rs) adapters.
9. wgpu ray tracing notes: <https://wgpu.rs/doc/wgpu/documentation/extensions/ray_tracing/index.html>.
10. [dlss_wgpu](https://github.com/bevyengine/dlss_wgpu) (README, `src/initialization.rs`, `src/super_resolution.rs`,
    `Cargo.toml`), [docs.rs](https://docs.rs/dlss_wgpu) (6.0.0, 2026-09-16).
11. NVIDIA DLSS SDK and its [LICENSE.txt](https://github.com/NVIDIA/DLSS/blob/main/LICENSE.txt) ("NVIDIA RTX SDKs LICENSE").
12. [NVIDIA Streamline](https://github.com/NVIDIA-RTX/Streamline) (2.14.1).
13. [GPUOpen-Effects/FidelityFX-FSR](https://github.com/GPUOpen-Effects/FidelityFX-FSR), `ffx-fsr/ffx_fsr1.h`, `ffx_a.h`.
14. [FSR 1 spatial upscaler manual](https://gpuopen.com/manuals/fidelityfx_sdk/techniques/super-resolution-spatial/).
15. [GPUOpen-Effects/FidelityFX-FSR2](https://github.com/GPUOpen-Effects/FidelityFX-FSR2) (2.2.1).
16. [FidelityFX SDK](https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK) 2.3.0: `readme.md`,
    `docs/license.md`, `Kits/FidelityFX/docs/techniques/super-resolution-ml.md`.
17. [ReShade](https://github.com/crosire/reshade) (BSD-3-Clause, v6.8.0).
18. [reshade-shaders](https://github.com/crosire/reshade-shaders) `Shaders/ReShade.fxh` (slim branch).
19. ReShade forum: [Linux version](https://www.reshade.me/forum/general-discussion/9312-linux-version),
    [Reshade and Vulkan](https://reshade.me/forum/troubleshooting/6153-reshade-and-vulkan),
    [5.1 release notes](https://reshade.me/releases/7951-5-1);
    [TinkerGame wiki](https://github.com/360900/tinkergame/wiki/ReShade). Read as search-result summaries.
20. [vkBasalt](https://github.com/DadSchoorse/vkBasalt) (Zlib).
