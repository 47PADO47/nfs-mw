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

**Result:** filled in by PR 8 (`feat/gfx-bevy-spike`), with the compile-time and binary-size deltas, which are
unmeasured today. The two no-go paths are in §8: (a) keep Bevy for `view-car`, the showroom and menus while the
city stays native (or uses custom instancing); (b) stop the Bevy backend, keep the interface, settings, testkit,
shared passes and `fsr3-wgpu`, and move TAA, DLSS and FSR 3 into the native backend (the 0003 route).

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

Kept from the plan ([§11](../plans/gfx-renderers/verification-risks.md)); none of it was built or run:

- MSRV of the individual Bevy 0.20 render crates (crates.io shows `null`).
- Raw wgpu `CommandEncoder` access from Bevy 0.20 `Core3d` systems, for the shared passes.
- `ViewDepthTexture` sampleability for soft particles.
- DLSS preset defaults per mode (second-hand quotes of the 310.5.0 release notes only).
- Whether `dlss` with `force_disable_dlss` builds without the SDK.
- Solari alpha-mask support (absent from the docs).
- Lavapipe exposing all five Solari features through wgpu.
- FSR SDK 2.3 DLL names, API and licence terms.
- That DLSS 5's 2026-09-03 launch actually happened (reported as scheduled).
- Compile-time and binary-size deltas.

## Sources

Retrieved 2026-10-09; the complete list is at the end of the
[plan](../plans/gfx-renderers/verification-risks.md#11-could-not-verify) and the earlier ones are in
[0003](0003-renderer-after-milestone-8.md#sources).

- Bevy 0.20 release notes: <https://bevy.org/news/bevy-0-20/>
- `dlss_wgpu`: <https://github.com/bevyengine/dlss_wgpu>
- NVIDIA DLSS repo and licence: <https://github.com/NVIDIA/DLSS>
- AMD FidelityFX SDK: <https://github.com/GPUOpen-LibrariesAndSDKs/FidelityFX-SDK>
- FSR 3.1 Vulkan support: <https://gpuopen.com/learn/amd_fsr_3_1_release/>
