# Swappable renderers: verification, go/no-go, risks, docs

Part of the [swappable renderers plan](README.md): sections 7 to 11 and the sources.

## 7. Parity and verification

- **Cross-backend comparisons, no committed images.** The testkit renders the same synthetic scene on native and Bevy headless and compares in memory.

  | Testkit scene | Metric | Tolerance (out of 255) |
  |---|---|---|
  | opaque, alpha-test, sky, fog, reverse-Z | max / mean / p99 | max ≤ 4, mean ≤ 0.5, p99 ≤ 2 |
  | glossy | mean / p99 | mean ≤ 1.5, p99 ≤ 8 |
  | alpha-blend and additive stack, effects | mean / p99 | mean ≤ 4, p99 ≤ 24 (linear vs gamma blending) |
  | UI layer | max | ≤ 1 (same shared pass) |
  | post effects (bloom, tonemap, TAA, temporal upscalers) | no cross-backend comparison (different algorithms by design) | per-backend sanity only: no NaN, luminance monotonic, energy bounds, convergence |

- **Native regression guard:** 32×18 mean-RGB digests per testkit scene and per setting combination (direct path; offscreen; FXAA; bloom + ACES; FSR 1 at 67 %; GL), stored as Rust consts, with ±2 tolerance per channel. Any change to the native image needs an intentional digest update in the same commit.
- **Real install (local, never in CI):** `xtask img-diff a.png b.png [--mask opaque]` (new subcommand; reads PNGs outside the repo). Five fixed city positions plus the showroom, run as `nfsmw view-world --at X,Y --renderer {blackbox|bevy} --screenshot /tmp/…`.
- **Where tests run:**
  - CI keeps the existing jobs, plus an optional `gpu-tests` job (Ubuntu, lavapipe) and a `features` job (clippy for `renderer-bevy,fsr3,ray-tracing,dlss-mock`; `cargo deny --all-features check licenses`). CI is `workflow_dispatch` only today.
- **Performance budget on Iris Xe at 1920×1080, measured with `--bench-seconds 30` on the drive script at a fixed route; the plan's targets:**
  - Native "low" preset: today's frame time is the baseline; it may get no slower than +3 % over the stack.
  - Bevy "low": GPU-bound frame time ≤ 1.3× native; CPU frame time ≤ native + 4 ms; tile-upload hitch ≤ 2× native; RSS and VRAM ≤ 1.5× native.
  - Bevy "medium" with FXAA or FSR 1 at 67 %: ≥ 30 fps. FSR 3 Quality on Iris Xe is recorded, not budgeted.

---

## 8. Go/no-go spike (PR 8): criteria and what to cut

**GO when all of these hold on Iris Xe (Vulkan), with the owner's RTX numbers recorded:**

1. Testkit strict scenes are within the tolerances in §7.
2. Real-city diff at five positions: mean ≤ 3/255, and a visual review passes alpha-tested foliage and fences, blend ordering and the sky.
3. CPU budget, GPU budget and hitches as in §7.
4. A default build has no Bevy render crates. Compile time and binary size with the feature are recorded in ADR 0004.

**NO-GO paths:**

- **(a) Only the city fails on CPU or GPU:** keep the Bevy backend for `view-car`, the showroom and menus (a scene can declare `requires_native`). The city always uses the native backend, or uses the custom-instancing fallback (§3.2) at the price of no TAA, DLSS or Solari in the city. Re-measure on Bevy 0.21.
- **(b) Fundamental fidelity or integration failure:** stop the Bevy backend.
  - Keep PRs 3–7 (interface, settings, testkit, shared passes) and PR 13a (`fsr3-wgpu` is backend-agnostic).
  - Add TAA to native (jitter, motion vectors from `InstanceKey`), DLSS through `dlss_wgpu` directly in native `gpu/init.rs` (the ADR 0003 route), and FSR 3 in native.
  - Solari is dropped.
  - ADR 0004 records the reversal.

---

## 9. Risks and open decisions

### Risks, ranked

1. **Bevy CPU cost of tens of thousands of instance × draw-range entities** while streaming. Mitigations: the spike, the pool and change detection, the custom-instancing fallback.
2. **Visual parity:** gamma vs linear blending; Bevy's distance sort vs submission order for blends. Mitigations: the gamma-space shader, sort keys, documented tolerances.
3. **Bevy release churn:** 0.21 is expected around February 2027 (cadence estimate) and changes WESL, `Core3d` systems and material APIs. Every custom pass and material migrates each release. Mitigations: an exact pin; a migration PR per release; parity tests as the safety net.
4. **Solari content fit:** pre-lit world vs PBR re-lighting, no alpha masks, StandardMaterial only, texture and slab limits, NVIDIA-only denoising. Ray tracing is a *different look*, optional, and may disappoint.
5. **FSR 4 can't be verified** (no RDNA3/4 hardware), and DX12 interop through wgpu-hal resource states is fragile. It is the last PR and can be cut.
6. **FSR 3 port size and correctness:** the largest new code. It is isolated in its own crate with numeric tests and is useful in every outcome.
7. **DLSS expectations:** "DLSS 5" is not reachable. "DLSS 4.5" Super Resolution models are reached only through SDK default presets per mode. Build needs the SDK; licence obligations apply when the DLL is bundled.
8. **Two implementations of every renderer feature:** maintenance doubles. Mitigations: the shared-passes crate and the parity harness.
9. **MSRV, compile time, binary size:** contained by the default-off feature.
10. **ReShade:** the Bevy backend's depth (also reverse-Z `Depth32Float`, plus prepass textures) may confuse depth detection. Document as untested.

### Owner decisions

1. Accept ADR 0004 and the `libs/` rule change (Bevy-based backend crates allowed as optional leaves).
2. Keep `blackbox` as the default renderer until the Bevy backend reaches parity (recommended), and decide whether release builds include `renderer-bevy`.
3. Whether release builds bundle the DLSS DLL (RTX SDK licence: attribution, notice to NVIDIA) and the FSR 4 DLL (AMD terms to read).
4. FSR 4: find an RX 7000/9000 tester or cut PR 14.
5. The ray-tracing look: whether pre-lit vertex colours darken albedo, and whether ray tracing covers the whole city or only nearby tiles and cars.
6. The MSRV rise to 1.97.1 if PR 2 needs it.
7. Accept "DLSS 5 not planned", with its revisit trigger.
8. Whether FSR 3 may later be enabled on the native backend (the crate allows it; the owner asked for native to stay limited).
9. Optionally enable this laptop's RTX 3050 Ti driver so agents can test DLSS and Solari locally (a system change; not done here).

### ADR 0004 outline: "Two renderers behind one interface"

- **Status:** proposed. It supersedes ADR 0003's "Decision" and the "What each M8 item becomes" rows for DLSS, temporal upscalers and Solari. It amends ADR 0001's rule that `libs/` stays Bevy-free.
- **Context:** the owner's goal, the facts table (§0), and what ADR 0003 measured.
- **Decision:**
  - `blackbox-gfx` trait objects; `blackbox-render` native and default; `blackbox-bevy-render` optional through the feature.
  - Runtime `renderer` with probe and fallback; the capability matrix (§4); the settings model (§5).
  - Where each feature lives: FSR 3 through the WGSL port, FSR 4 through the DX12 DLL (experimental), DLSS through Bevy (Vulkan), Solari (Bevy, startup-gated).
  - Not reachable: DLSS 5, frame generation.
- **Go/no-go criteria and their result** (filled in by PR 8).
- **Consequences:** costs (Bevy churn, two implementations), licence handling, CI jobs.
- **Revisit triggers:**
  - NVIDIA ships DLSS 5 through NGX, or `dlss_wgpu` adds it;
  - AMD restores Vulkan in the FSR SDK;
  - Solari gains alpha masks or a vendor-neutral denoiser;
  - the Bevy backend reaches parity, at which point the default renderer is reconsidered.
- **Sources,** with retrieval date 2026-10-09.

---

## 10. Docs to write or update

- **New:**
  - `docs/decisions/0004-swappable-renderers.md`;
  - `docs/plans/gfx-renderers/` (this plan);
  - `docs/renderers.md` (user: choosing a renderer, the matrix, restart rules);
  - `docs/bevy-backend.md` (developer: façade and ops, axes, colour, instancing, systems order, spike results);
  - READMEs for `libs/blackbox-gfx`, `blackbox-gfx-testkit`, `blackbox-gpu-passes`, `blackbox-bevy-render`, `fsr3-wgpu`, `ffx-api-wgpu`.
- **Update:**
  - `docs/architecture.md` (libs table, renderer rule, render pipeline, roadmap row 8);
  - `docs/rust-stack.md` (Bevy render crates, `dlss_wgpu`, `libloading`; move "Bevy's renderer" out of "Rejected");
  - `docs/licensing.md` (DLSS SDK, FSR 4 DLL, FSR 3 MIT notices);
  - `NOTICE` (FSR 3 port);
  - `docs/upscaling.md`, `docs/post-processing.md`, `docs/low-end.md` (from the low-end layer), `docs/testing.md` (GPU tests, lavapipe, parity, digests), `docs/reshade.md` (Bevy backend untested);
  - ADR 0001 and ADR 0003 status lines.

## 11. Could not verify

- MSRV of the individual Bevy 0.20 render crates (crates.io shows `null`).
- Raw wgpu `CommandEncoder` access from Bevy 0.20 `Core3d` systems for the shared passes.
- `ViewDepthTexture` sampleability for soft particles.
- DLSS preset defaults per mode (only second-hand quotes of the 310.5.0 release notes).
- Whether `dlss` + `force_disable_dlss` builds without the SDK.
- Solari alpha-mask support (absent from the docs).
- Lavapipe exposing all five Solari features through wgpu.
- FSR SDK 2.3 DLL names, API and licence terms.
- That DLSS 5's 2026-09-03 launch actually happened (reported as scheduled).
- Compile-time and binary-size deltas.

**Sources (retrieved 2026-10-09):**
- Bevy 0.20 release notes: https://bevy.org/news/bevy-0-20/
- Bevy v0.20.0 sources: `crates/bevy_anti_alias/src/dlss/mod.rs`, `crates/bevy_core_pipeline/src/core_3d/mod.rs`, `crates/bevy_solari/src/{lib.rs, scene/types.rs}`
- crates.io API: `bevy_render`, `bevy_solari`, `bevy_anti_alias` 0.20.0; `dlss_wgpu`
- dlss_wgpu: https://github.com/bevyengine/dlss_wgpu and its `Cargo.toml`
- NVIDIA DLSS repo: https://github.com/NVIDIA/DLSS
- DLSS 4.5 SDK coverage: https://www.neowin.net/news/nvidia-released-dlss-45-sdk-for-developers-featuring-dynamic-multi-frame-generation/ and https://hwbusters.com/news/nvidia-dlss-4-5-super-resolution/
- DLSS 5: https://www.thurrott.com/games/340926/nvidia-dlss-5-to-launch-on-rtx-50-series-gpus-on-september-3 and https://80.lv/articles/nvidia-finally-announces-launch-date-for-its-controversial-dlss-5
- FSR SDK 2.3.0: https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK
- FSR 3.1 Vulkan in SDK 1.1: https://gpuopen.com/learn/amd_fsr_3_1_release/
- FSR 4 Vulkan via OptiScaler only: https://hothardware.com/news/fsr4-in-vulkan-games-optiscaler
- `fsr` crate (FSR 2): https://docs.rs/fsr
- Lavapipe ray query: https://www.phoronix.com/news/Mesa-Lavapipe-Vulkan-RayTracing
- Local checks: `lspci` (Iris Xe + RTX 3050 Ti Mobile), `nvidia-smi` (driver not loaded), `/usr/share/vulkan/icd.d` (lavapipe present), `~/.cargo/bin/rustc` 1.99.0
