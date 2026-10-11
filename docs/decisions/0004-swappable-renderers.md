# 0004 — Two renderers behind one interface

- **Status:** proposed (2026-10-09), for the project owner to accept. Supersedes the "Decision" of
  [0003](0003-renderer-after-milestone-8.md) (keep only `blackbox-render`) and its "What each milestone 8 item
  becomes" rows for DLSS, temporal upscalers and Bevy Solari. Amends the rule of [0001](0001-bevy.md) that
  `libs/` stays Bevy-free.
- **Milestone:** 8 (graphics) and after.
- **Plan:** [docs/plans/gfx-renderers/](../plans/gfx-renderers/README.md) holds the full design, the PR list and
  the verification plan. Section numbers (`§N`) below point into it.

## Question

[0003](0003-renderer-after-milestone-8.md) kept `blackbox-render` as the only renderer, because Bevy's renderer
did not fit the pre-lit city and its main draw (Solari) was experimental. The owner now wants both: the native
renderer stays the default and the only one for OpenGL and low-end PCs, and an optional Bevy renderer adds the
features that only Bevy offers (TAA, Solari ray tracing, DLSS), with the choice made at startup. How do we make
two renderers swappable without forking the game code, and where does each feature live?

## Facts checked on 2026-10-09

The full table is in the [plan §0](../plans/gfx-renderers/README.md). The ones that decide this ADR:

- Bevy 0.20 final is out (2026-10-08); the workspace still pins `=0.20.0-rc.2`. Bevy 0.20 replaced render-graph
  nodes with systems in the `Core3d` schedule and moved shaders to WESL.
- Solari 0.20 is experimental: `StandardMaterial` meshes only, a few light types, no documented alpha-mask
  support, and its only realtime denoiser is DLSS Ray Reconstruction (NVIDIA, Vulkan).
- `dlss_wgpu` 6.0.0 wraps DLSS SDK 310.9.1 on wgpu 30: Vulkan only, NVIDIA only, needs the SDK at build time.
  Bevy's `dlss` feature wraps it.
- "DLSS 4.5" is a set of Super Resolution models (second hand, via SDK 310.5.0 release notes). "DLSS 5" is a
  neural-rendering model reached through NVIDIA Streamline, not the NGX Super Resolution SDK that `dlss_wgpu`
  wraps.
- AMD FSR SDK 2.3.0 ships FSR 4 as a signed DX12 binary and lists Vulkan as unsupported. FSR 3.1 has Vulkan
  support in the SDK since 1.1.x. No Rust FSR 3 or FSR 4 crate exists for wgpu or Bevy.
- `.png` is a forbidden extension in `xtask/src/leak.rs`, so golden images cannot be committed.

## Decision

**Add a renderer-neutral interface, keep the native renderer as the default, and add Bevy as an optional second
renderer behind a cargo feature and a runtime switch.**

1. **`libs/blackbox-gfx`** (new, no wgpu, no Bevy): the types callers use today (vertex, texture, mesh, draw
   range, blend, shading, glossy, lighting rig, effect layer, UI layer, frame parameters, instances), a
   `RenderBackend` **trait object**, a `Capabilities` struct and typed `GraphicsSettings` with a pure
   `resolve(requested, caps)`, so a setting the backend can't run falls back gracefully
   ([§1](../plans/gfx-renderers/interface.md)). Trait objects, because the set of backends is open and compiled
   in or out by features, and calls are coarse (one `render` per frame).
2. **`libs/blackbox-render`** stays the native backend (wgpu; Vulkan, DX12, OpenGL) and implements the trait. Its
   feature set is frozen at FXAA, bloom, ACES tone mapping, render scale with bilinear upscaling and FSR 1. It is
   the default renderer and the only one for OpenGL and low-end PCs.
3. **`libs/blackbox-bevy-render`** (new, optional): a Bevy 0.20 backend added as plugins to the same Bevy `App`
   the game already runs (the 0001 shell), behind the `renderer-bevy` cargo feature of `nfsmw`, **off by
   default**, so default builds never compile `bevy_render`
   ([§3](../plans/gfx-renderers/bevy-backend.md)).
4. **The `libs/` rule changes:** Bevy-based backend crates are allowed in `libs/` as optional leaf crates that
   nothing else depends on. Everything else in `libs/` stays Bevy-free, and `crates/` still depends on `libs/`,
   never the reverse.
5. **Shared GPU code:** `libs/blackbox-gpu-passes` holds the plain-wgpu passes (UI layer, effects, soft
   particles, textured effects, filter, FSR 1) that both backends call, so there is one implementation of each,
   guarded by image digests ([§2](../plans/gfx-renderers/interface.md)).
6. **Choosing the renderer:** `renderer = blackbox|bevy` at startup (CLI, env, TOML, console, menu; needs a
   restart). A wgpu adapter probe runs before the `App` is built; a Bevy request that the build or the
   hardware can't satisfy falls back to `blackbox` with a log line. Default `blackbox`. The existing `backend` key
   (Vulkan, DX12, GL) stays; docs must keep *renderer* (who draws) apart from *backend* (graphics API).
7. **Where each feature lives** (matrix in [§4](../plans/gfx-renderers/upscalers-settings.md)):

   | Feature | Where | Notes |
   |---|---|---|
   | FXAA, bloom, ACES, render scale, bilinear, FSR 1 | both | the native set; shared passes |
   | SMAA, TAA | Bevy | |
   | FSR 3.1 | Bevy, via `libs/fsr3-wgpu` | WGSL port of the MIT upscaler; Vulkan and DX12, any vendor; the crate is backend-agnostic |
   | FSR 4 | Bevy, via `libs/ffx-api-wgpu` | **experimental**, DX12 only, AMD's DLL; cut-able; no hardware to verify |
   | DLSS Super Resolution | Bevy `dlss` feature | Vulkan, NVIDIA RTX; default off; SDK never in the repo |
   | Solari ray tracing | Bevy `ray-tracing` feature | startup-gated; DLSS-RR as denoiser on NVIDIA |

8. **Not reachable, not planned:** DLSS 5 (Streamline, not NGX Super Resolution) and frame generation (DLSS FG,
   FSR 3 FG).
9. **Settings** follow the existing layering (CLI > env > TOML > defaults): `renderer`, `post_aa` extended with
   `smaa` and `taa`, `upscaler` (`off|bilinear|fsr1|fsr3|fsr4|dlss`), `upscale_quality`, `ray_tracing`, and
   the in-flight `graphics_preset`. Unavailable values are rejected with a message in the console, hidden in
   menus and resolved with a logged downgrade at startup ([§5](../plans/gfx-renderers/upscalers-settings.md)).

## Go/no-go criteria and their result

Criteria are fixed in [plan §8](../plans/gfx-renderers/verification-risks.md): on Iris Xe (Vulkan), the testkit
strict scenes within the tolerances of §7, a real-city diff mean of at most 3/255 at five positions plus a visual
review, the CPU, GPU and hitch budgets of §7, and no Bevy render crates in a default build.

**Result (PR 8, `feat/gfx-bevy-spike`, measured 2026-10-09 on Intel Iris Xe, Vulkan, Mesa 26.2.3; the owner's
RTX numbers and any DX12 run are still to come).** The details, tables and method are in
[bevy-backend.md](../bevy-backend.md).

| Criterion | Target | Measured | Verdict |
|---|---|---|---|
| 1. Testkit strict scenes | max 4, mean 0.5, p99 2 | grid, alpha cards, sky, depth: max 1, mean at most 0.04, p99 at most 1 (Iris Xe and lavapipe) | pass |
| 2a. Real city, five positions | mean at most 3/255 | 0.22 to 1.61 at six positions | pass |
| 2b. Visual review | foliage, fences, blend order, sky | foliage and fences identical; blend order differs where blended draws overlap; the sky is off by a smooth 8 to 12 levels | pass with two known gaps |
| 3a. CPU per frame | at most native + 4 ms | +4.4 and +5.1 ms (native 2.3 to 2.5, Bevy 6.9 to 7.3); +5.3 in a sparse place | **fail by 0.4 to 1.3 ms** |
| 3b. Frame time (GPU-bound budget) | at most 1.3x native | p50 1.45x and 1.5x (5.6 to 6.1 vs 8.4 to 8.9 ms; 144 to 150 vs 106 to 110 fps), the same at half resolution | **fail** (the frame is CPU-bound, so the ratio is the CPU's) |
| 3c. Tile-upload hitch | at most 2x native | not measured; the worst frames of a 30 s drive are equal (16 to 18 ms vs 17.5 to 19.5 ms) | open |
| 3d. RSS | at most 1.5x native | 1.3 to 1.45x (503 to 506 vs 348 to 390 MiB); VRAM not measured | pass |
| 4. Default build has no Bevy render crates | none | `cargo tree -p nfsmw -e normal` has no `bevy_render`; still builds on Rust 1.95 | pass |
| Cold release build, with the feature | recorded | 120 s without, 215 s with (+95 s) | recorded |
| Binary size, with the feature | recorded | 62.4 MiB without, 111.0 MiB with (+48.6 MiB; the plan guessed +15 to 30) | recorded |
| MSRV | check | the feature needs Rust 1.97.1 (Bevy's WESL stack); only the new crate declares it | recorded |

What decides the verdict:

- **Nothing fundamental failed.** Bevy takes the game's existing window with no special handling (its window
  systems build the surface from the `bevy_winit` handle), the world renders pixel-close to the native
  renderer, headless capture works on Iris Xe and on lavapipe, and a drive through the city runs at about 105 to
  110 fps at 1080p on integrated graphics.
- **The 80,000-object city is not the problem.** The scene culls to a few hundred objects per frame, the instance
  pool costs 0.055 ms per frame, and a synthetic 40,000-instance frame costs 15 ms on this GPU. The custom
  instancing fallback of the plan would not help.
- **The two failures are one fixed per-frame cost of about 4.4 to 5.3 ms of CPU** that does not shrink with fewer
  objects, so it comes from Bevy's schedules and the idle plugins `PbrPlugin` brings, not from this crate's
  work. It is **not profiled yet**; turning on Bevy's `multi_threaded` made it worse (11.3 ms of CPU).
- **Gaps that are by design of the spike:** blended draws blend in linear space and in distance order (sky and
  overlapping layers), and the UI, effect, glossy and post layers do not exist yet.

**Recommendation: conditional GO.** Continue to PR 9 and 10 (scene layers, then post), with a hard gate at the
start of PR 9: profile the fixed Bevy cost (`trace_tracy`) and cut it (replace `PbrPlugin` by the few plugins the
material needs, turn off clustering and the other unused passes) until the CPU criterion (native + 4 ms) and the
frame-time ratio (1.3x) hold on Iris Xe. If they cannot be met with the trimmed plugin set, take **no-go path (a)**
of the plan: keep the Bevy backend for `view-car`, the showroom and menus, where the object count is small, and
leave the city on the native renderer. The criteria as written do not all hold today, so this is not a plain GO;
the reasons it is not a no-go are that the failures are small (0.4 to 1.3 ms; 1.45x), understood to be fixed
cost, and not in the code this project owns. No case for no-go path (b) (fidelity or integration) was found.
Default builds are unaffected either way: renderer `blackbox`, no `bevy_render` compiled.

## Consequences

- **Costs:**
  - Bevy release churn: every custom pass and material migrates on each release (0.21 is expected around
    February 2027, a cadence estimate). Mitigation: exact pins, a migration PR per release, parity tests.
  - Two implementations of every renderer feature. Mitigation: the shared-passes crate and the parity harness.
  - The MSRV may rise from 1.95 to 1.97.1 with the move to Bevy 0.20 final (checked in the pin-bump PR).
  - `*.wesl` shaders need `.gitignore` and the size check to learn the extension.
- **Visual parity:** the native renderer blends in gamma space, Bevy in linear. The Bevy material shader works in
  gamma space; alpha-blend and additive layers still differ and get looser tolerances. Ray tracing re-lights a
  PBR stand-in of a pre-lit world: a different look, optional, and may disappoint.
- **Licences:** FSR 3 is an MIT port (notices in every shader and `NOTICE`). DLSS is under the NVIDIA RTX SDKs
  licence (no SDK in the repo, attribution and notice obligations if a DLL is bundled); it stays an optional,
  default-off feature. FSR 4's DLL is never bundled without reading AMD's terms.
- **CI:** an optional `gpu-tests` job (Ubuntu, lavapipe) and a `features` job (clippy for
  `renderer-bevy,fsr3,ray-tracing,dlss-mock`; `cargo deny --all-features check licenses`). CI is
  `workflow_dispatch` only today.
- **Verification limits:** DLSS and Solari are implemented and unit-tested here (mock, capability tests) but need
  the owner's RTX PC; FSR 4 cannot be verified by anyone on the team (no RDNA3 or 4), so it merges only with an
  AMD tester.
- **Defaults stay safe for low-end:** renderer `blackbox`, every effect off, native resolution.

## Revisit triggers

- NVIDIA ships DLSS 5 through NGX, or `dlss_wgpu` adds it.
- AMD restores Vulkan in the FSR SDK.
- Solari gains alpha masks or a vendor-neutral denoiser.
- The Bevy backend reaches parity with the native one: reconsider the default renderer.

## Owner decisions this ADR asks for

1. Accept this ADR and the `libs/` rule change.
2. Keep `blackbox` the default until the Bevy backend reaches parity (recommended); decide whether release
   builds include `renderer-bevy`.
3. Whether releases bundle the DLSS DLL and the FSR 4 DLL.
4. FSR 4: find an RX 7000 or 9000 tester, or cut PR 14.
5. The ray-tracing look: whether pre-lit vertex colours darken albedo, and whether ray tracing covers the whole
   city or only nearby tiles and cars.
6. The MSRV rise to 1.97.1 if the pin bump needs it.
7. Accept "DLSS 5 not planned", with its revisit trigger.
8. Whether FSR 3 may later be enabled on the native backend.

The full list of risks and open decisions is in [plan §9](../plans/gfx-renderers/verification-risks.md).

## Could not verify

Kept from the plan ([§11](../plans/gfx-renderers/verification-risks.md)). PR 8 settled these three: the MSRV
(Bevy's WESL stack needs Rust 1.97.1), raw `CommandEncoder` access from `Core3d` systems (yes, through
`RenderContext::command_encoder`, read from the source and used by Bevy itself), and the compile-time and
binary-size deltas (measured above). `ViewDepthTexture` sampleability is half settled (`Camera3d` takes the depth
texture's `TextureUsages`; not exercised). The rest was not built or run:

- DLSS preset defaults per mode (second-hand quotes of the 310.5.0 release notes only).
- Whether `dlss` with `force_disable_dlss` builds without the SDK.
- Solari alpha-mask support (absent from the docs).
- Lavapipe exposing all five Solari features through wgpu.
- FSR SDK 2.3 DLL names, API and licence terms.
- That DLSS 5's 2026-09-03 launch actually happened (reported as scheduled).

## Sources

Retrieved 2026-10-09; the complete list is at the end of the
[plan](../plans/gfx-renderers/verification-risks.md#11-could-not-verify) and the earlier ones are in
[0003](0003-renderer-after-milestone-8.md#sources).

- Bevy 0.20 release notes: <https://bevy.org/news/bevy-0-20/>
- `dlss_wgpu`: <https://github.com/bevyengine/dlss_wgpu>
- NVIDIA DLSS repo and licence: <https://github.com/NVIDIA/DLSS>
- AMD FidelityFX SDK: <https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK>
- FSR 3.1 Vulkan support: <https://gpuopen.com/learn/amd_fsr_3_1_release/>
