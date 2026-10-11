# Swappable renderers: upscalers, anti-aliasing and settings

Part of the [swappable renderers plan](README.md): sections 4 and 5.

## 4. Upscalers and anti-aliasing: capability matrix

| Method | blackbox (native) | bevy | API / hardware | Inputs it needs | Licence / feature |
|---|---|---|---|---|---|
| Off | ✔ | ✔ | any | — | — |
| Bilinear | ✔ | ✔ (Bevy `upscaling` + `MainPassResolutionOverride`) | any | — | — |
| FSR 1 (EASU + RCAS) | ✔ | ✔ (shared pass) | any | anti-aliased, display-referred colour; after tone mapping | MIT, already in NOTICE |
| FXAA | ✔ | ✔ (Bevy's) | any | — | — |
| SMAA | ✗ | ✔ (optional) | any | — | MIT (Bevy) |
| TAA | ✗ | ✔ | Vulkan, DX12 | jitter, motion vectors (stable `InstanceKey`), depth prepass, mip bias; `camera_cut` resets history | — |
| FSR 3.1 upscaler | ✗ (the crate is backend-agnostic, so it could be added later) | ✔ `fsr3` | **Vulkan and DX12**, any vendor (compute) | HDR colour at render size, reverse **infinite** depth flag, motion vectors, jitter (Halton), exposure (auto), optional reactive mask from effect alpha, mip bias `log2(s) - 1`, reset | MIT FSR 3.1 HLSL ported to WGSL in `libs/fsr3-wgpu`; notices in every shader and in NOTICE |
| FSR 4 (ML) | ✗ | ✔ `fsr4`, **experimental** | **DX12 only**, RX 7000/9000; falls back to FSR 3 | same as FSR 3, plus D3D12 resource-state handling | AMD signed DLL, not in the repo; licence terms to check before bundling |
| DLSS Super Resolution ("DLSS 4.5" models L and M at Performance and Ultra Performance by SDK default) | ✗ | ✔ `dlss` | **Vulkan only**, NVIDIA RTX | `Dlss` component (jitter, motion vectors, depth, `Hdr`), `DlssProjectId` | `dlss_wgpu` MIT/Apache; DLSS SDK under the NVIDIA RTX SDKs licence; build needs the SDK; off by default |
| DLSS Ray Reconstruction (Solari denoiser) | ✗ | ✔ with `ray-tracing` and `dlss` | Vulkan, RTX | Solari G-buffer | same as DLSS |
| DLSS 5 (neural rendering) | ✗ | **✗ not reachable** (Streamline, not NGX Super Resolution / `dlss_wgpu`) | — | — | revisit trigger |
| Frame generation (DLSS FG, FSR 3 FG) | ✗ | ✗ not planned | — | — | — |

Upscaling settings:

- `upscale_quality = auto|native|quality|balanced|performance|ultra_performance` drives the temporal methods. It maps to DLSS modes, and to FSR 3/4 ratios of 1.0, 1.5, 1.7, 2.0 and 3.0 per axis.
- `render_scale` (50–200 %) keeps driving bilinear and FSR 1.
- FSR 1 sharpness stays.

**Who can test what:**

| Item | Here (Linux, Iris Xe, lavapipe; implementer agents) | Owner (RTX PC, Windows) |
|---|---|---|
| Interface, settings, `resolve` | CPU tests | — |
| Native parity digests | lavapipe and Iris Xe Vulkan; GL on Iris Xe | DX12 |
| Bevy raster parity, TAA, FSR 1, FXAA, bloom | lavapipe and Iris Xe | DX12 and RTX look |
| FSR 3 numerics (convergence, reprojection) | headless on Iris Xe and lavapipe | DX12 run, image quality |
| DLSS | compile with `dlss-mock` only (unless the owner enables this laptop's RTX 3050 Ti driver) | **hardware-verified** |
| Solari | capability downgrade tests; maybe a 64×64 lavapipe smoke test (unverified) | **hardware-verified** |
| FSR 4 | compile on Windows only; not even that on Linux | **nobody can verify: the owner has RTX, not RDNA3/4.** Needs an AMD tester, or cut. |

---

## 5. Settings and low-end

Keys follow the existing layering: CLI > env > TOML > defaults. Each key has a `FromStr`/`Display` type, an entry in `Partial` and `write.rs`, a console `get`/`set`, and a menu row.

| Key | Values | Default | Restart? | Notes |
|---|---|---|---|---|
| `renderer` | `blackbox`, `bevy` | `blackbox` | yes | Falls back to `blackbox` (with a log) if not built or no adapter. |
| `post_aa` (kept for compatibility; owner's "anti_aliasing") | `off`, `fxaa`, `smaa`, `taa` | `off` | no | `smaa` and `taa` exist only with Bevy. |
| `upscaler` | `off`, `bilinear`, `fsr1`, `fsr3`, `fsr4`, `dlss` | `fsr1` (inactive at 100 %) | no (DLSS: first enable may need a restart if `DlssInitPlugin` wasn't registered) | — |
| `upscale_quality` | `auto`, `native`, `quality`, `balanced`, `performance`, `ultra_performance` | `quality` | no | temporal upscalers only |
| `render_scale`, `upscale_sharpness` | unchanged | — | no | spatial upscalers only |
| `post_bloom`, `post_tonemap` | unchanged (+ Bevy's tonemappers later) | `off` | no | — |
| `ray_tracing` | `off`, `low`, `medium`, `high` | `off` | off ↔ on: yes; quality: no | Bevy, ray-query hardware |
| `graphics_preset` (in flight in the low-end layer) | `low`, `medium`, `high`, `ultra`, `custom` | `custom` | no | a lower layer than explicit keys |

Preset mapping. The resolver applies backend capabilities after the preset.

| Preset | blackbox | bevy |
|---|---|---|
| low | direct path: no post, no upscaler, `car_shading = simple`, no sparks or trails | no post, no `Hdr`, no prepasses, MSAA off, simple car shading |
| medium | FXAA, glossy cars | FXAA, glossy cars |
| high | FXAA, bloom low, ACES | TAA, bloom low, ACES-fitted |
| ultra | = high | TAA, or DLSS Quality (NVIDIA on Vulkan) or FSR 3 Quality (others); `ray_tracing = medium` if supported |

- **Nothing runs or is allocated when off.** Native already builds pipelines lazily (low-end layer). On Bevy, camera components are inserted only for enabled features: no `TemporalAntiAliasing`, `Dlss`, `Bloom` or `Hdr` when off, so no prepass and no history textures.
- **Validation:**
  - Console `set upscaler dlss` on blackbox prints `dlss is not available with the blackbox renderer (available: off, bilinear, fsr1)` and leaves the value unchanged.
  - A new console command `gfx` prints the renderer, the API, the capabilities, and requested vs effective with reasons.
  - Menus: `options::rows(category, caps)` hides rows the backend can't do (ray tracing on blackbox), and toggles cycle only supported values. FEng widget rows have no greyed-out state, so hiding is the honest equivalent.
  - The startup log lists every downgrade once.
- **Defaults are safe for low-end:** renderer `blackbox`, every effect off, ray tracing off, native resolution. This is the frame the stack produces today.
