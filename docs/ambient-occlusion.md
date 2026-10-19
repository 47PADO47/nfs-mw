# Experimental ambient occlusion

Spatial GTAO, XeGTAO, temporal half-resolution XeGTAO and a hemisphere-kernel SSAO reference in the native `blackbox-render` backend. AO and contact shadows share
a reverse-Z depth/world-normal prepass. AO produces an ambient visibility mask
before colour shading. Spatial methods use no history; temporal XeGTAO uses an
independent camera/object motion pass. It is off by
default in normal settings; the Graphics Lab launcher selects Medium. Other backends
and adapters without compute support report an unsupported-setting downgrade.

F8 Effects has an independent **AO method** selector: `gtao` uses horizon search and
analytic slice integration; `ssao` projects cosine-weighted hemisphere probes and
counts depth occlusion. The latter is a comparison baseline, not HBAO+ or CACAO.
These two reuse the same geometry, radius limits, strength, depth hierarchy and spatial
filter. Switching methods does not allocate another set of images or retain history.
The selection persists in profiles and A/B snapshots; `ao_method = "ssao"`,
`NFSMW_AO_METHOD=ssao` and `--ao-method ssao` provide the same control.

F8 Effects offers opt-in **Depth-prefiltered AO**, also controlled by `ao_depth_mips`,
`NFSMW_AO_DEPTH_MIPS` and `--ao-depth-mips true|false`. Disable it to compare the
direct-depth reference at the same radius, intensity and quality (the default). Profile saves,
A/B snapshots and startup restores retain this choice independently of AO quality.
The hierarchy uses up to three half-resolution-and-smaller R32F levels, with a
farthest-depth-weighted reduction that rejects unrelated foreground depths. Near
taps continue reading the original depth; distant taps choose a coarser footprint.
Odd image edges are included. It adds approximately 10.4 MiB at 4K and skips its
reduction passes and releases full-size resources when disabled. It has no frame
history and does not change material lighting or grading. Include hierarchy
construction in AO GPU timings when comparing performance.

Authored alpha-tested leaf cards use their camera-facing geometric plane in that
prepass. Their bent lighting normals stay in the colour pass; treating them as
surface planes creates false self-occlusion that changes as the camera moves.

The game defaults to XeGTAO at 80% strength when no method/strength is saved.
AO remains opt-in without an appearance preset. Enhanced and Black Box High/Very High
use the depth prefilter; explicit saved overrides are preserved on startup.
Reducing strength changes ambient composition, not the sampling budget.

All methods use a single depth layer. Thin foreground pipes can over-occlude
ceilings behind them; stronger AO also compounds baked shading. These limitations
remain and are not fixed by the depth prefilter or changing methods.

## Try it

```sh
nfsmw play --ao medium --ao-intensity 75
nfsmw play --ao medium --ao-compare
```

Video options offer **Ambient Occlusion (Experimental)**, **AO Intensity** and
**AO Comparison**. Changes apply immediately and persist with the other
settings. In split mode, the left half is unchanged and the right half uses AO
from the same depth and camera. The console also accepts:

```text
ao medium
ao_intensity 60
ao_compare on
ao off
```

Config keys are `ao = "medium"`, `ao_intensity = 75`, `ao_compare = false`.
Environment equivalents are `NFSMW_AO`, `NFSMW_AO_INTENSITY` and
`NFSMW_AO_COMPARE`. CLI settings take precedence. No addon assets are required.

GTAO reference budget:

| Quality | AO resolution | Slices | Depth samples per slice |
|---|---|---|---|
| Off | No AO allocation or passes | 0 | 0 |
| Low | Half scene width and height | 2 | 6 |
| Medium | Half scene width and height | 3 | 8 |
| High | Full scene resolution | 4 | 12 |

Medium is the suggested starting point. AO follows the internal scene resolution
when render scale changes. Zero intensity also skips the passes and releases their
resources. Existing graphics presets retain the independent AO selection.

## Approach and prior art

[XeGTAO](https://github.com/GameTechDev/XeGTAO) (MIT, Intel) supplies the horizon
search and analytic slice integration used by this WGSL adaptation. The complete
XeGTAO implementation also has depth mip prefiltering, more elaborate denoising
and tuning, and can rely on the host engine's TAA. This experiment implements a
smaller spatial method suitable for the existing immediate renderer; it is not a
complete XeGTAO port. The adapted shader retains Intel's MIT notice.

The provided [Quasar SSAO shader](https://github.com/dev25bansal-ops/quasar/blob/main/assets/shaders/ssao.wgsl)
(MIT) was considered as a hemisphere-kernel reference. Its position and normal
inputs would require additional scene outputs here. No Quasar code is copied.

One shared geometry prepass precedes three AO passes and the colour draw:

1. Reconstruct view positions from depth and transform authored world normals into
   view space. Missing normals use edge-aware depth reconstruction. Search nearby
   horizons with a static Hilbert/R2 sequence over 64-by-64 pixel regions, adapted
   from XeGTAO's sampling. This replaces the visibly repeating 4-by-4 ordered strata.
   Neighbouring pixels cover different samples, avoiding repeated hard bands around
   wide occluders. Store visibility, view depth and an octahedral normal in RGBA16F.
2. Denoise with a 5-by-5 spatial filter. Plane distance and normal agreement reject
   samples across silhouettes rather than blurring them into foreground objects.
3. Bilaterally upsample four nearby AO samples into a full-resolution R8 visibility
   mask. Scene shaders apply it to ambient lighting before fog. Direct sunlight,
   sun highlights, environment reflections and blended surfaces are unchanged.

Normal-mapped opaque and cutout pavement/road materials now consume that mask too.
Previously their prepass contributed to AO, but their colour shader omitted the
ambient multiplier, leaving the ground bright beside an occluded pole. Regression
fixtures cover bump and reflective receivers, both estimators, transparent exclusion,
and preservation of direct sunlight, specular highlights and reflected images.

The default contact radius is 1.25 world units. Its screen cap scales with scene
height (96 pixels at 1080p, 192 at 4K), preserving angular reach across resolutions. A
short range falloff and thickness heuristic limit distant silhouette occlusion.
Depth bias rejects coplanar self-occlusion. Visibility is normalized against an
unobstructed slice so open roads and slopes stay bright. AO fades between 40 and
100 view-space units; scene fog is applied after lighting. Intensity blends between
unoccluded ambient light and the full GTAO visibility. The former 45% reduction cap
has been removed; approximately 34% intensity matches the previous default 75% response.
The prelit world's ambient share remains an approximation (35% of its lighting).
Custom Glossy treats the rig's fill/back lights as ambient illumination. Original
PC car materials bypass screen-space AO. Direct sunlight and reflections remain unchanged.

Static sampling avoids animated grain. There is no accumulation to ghost behind
cars or survive a camera cut. Smoke, sparks, other effect particles and the UI
are drawn afterward. Direct rendering remains supported: AO does not require
an extra offscreen scene-colour image.

## Limits and validation

Screen-space depth cannot see offscreen or hidden occluders. Thin objects and
normals inferred at depth discontinuities remain approximations. Fixed sampling
can alias as geometry moves between pixels, especially at Low quality. It does
not establish perfect stability at speed. Authored texture/vertex lighting is
retained, so baked occlusion can still overlap AO. Keep intensity conservative
on already dark assets. Reflection captures omit screen-space AO.

Depth/normals cost eight bytes per scene pixel, shared with contact shadows.
The full-resolution R8 mask costs one byte per scene pixel; raw and filtered
RGBA16F textures total sixteen bytes per AO pixel. Disabling AO releases its
images and rebinds a tiny placeholder; the prepass remains only if contacts need it.

Shader validation runs without a GPU. Explicit GPU regression tests check
contact shading and smooth raised-body ground gradients, small camera-move stability,
bounded darkening, unchanged open planes and sky, unchanged UI,
fog suppression, identical repeated frames and camera-return captures, exact
on/off round trips, odd sizes, render scaling and 2D projection bypass. Integration
tests distinguish direct-only and ambient-only surfaces, preserve glossy highlights
and reflections, and exercise combined AO/contact/SMAA resize and live toggles.

```sh
cargo test -p blackbox-render gpu::ao::tests -- --ignored --nocapture
cargo test -p blackbox-render ao_gpu_timings -- --ignored --nocapture
```

The diagnostic benchmark uses GPU timestamps around the AO passes on synthetic
depth at 1080p and 4K, after warmup. It excludes geometry, CPU setup and capture
readback; its numbers are not whole-game frame times. Before considering a default
change, compare repeatable driving routes and inspect car silhouettes, vegetation,
road slopes and building corners with the split view on multiple adapters.

## XeGTAO spatial comparison

`xegtao` in F8 Effects, `--ao-method xegtao`, the console or the saved settings
selects a separate WGSL port of Intel's spatial core. The pinned upstream revision is
[`a5b1686c7ea37788eeb3576b5be47f7c03db532c`](https://github.com/GameTechDev/XeGTAO/tree/a5b1686c7ea37788eeb3576b5be47f7c03db532c).
The shader files retain Intel's full MIT notice. The original `gtao` method remains
available as the smaller earlier adaptation described above.

This mode uses the reference radius multiplier (1.457), quadratic sample placement,
linear horizon falloff (0.615 of the effective radius), projected-normal correction,
visibility power (2.2), slope-adjusted packed edges, symmetric edge rejection and
the nine-tap spatial denoiser (four intermediate passes with beta 0.24, final beta 1.2).
Five spatial passes suppress undersampling noise without temporal history or extra
images: the existing two images alternate as input and output. High uses four slices
and four steps on both sides: 32 depth samples at full resolution. Medium uses 24
and Low uses 12 at half width/height. These budgets trade some fine detail for camera
stability without Intel's host TAA; they differ from its reference 18-sample High preset.

The optional depth hierarchy uses four half-resolution-and-smaller R32F levels for
this mode, with reference radius-scaled depth weighting. It costs about 10.5 MiB at
4K. Distant taps read point samples from the selected mip; near taps use original
depth. The direct-depth toggle remains an independent comparison. Coarse positions
are reconstructed at their selected texel centers to avoid false slopes.

Integration differences from Intel's complete implementation: FP32 shader arithmetic,
RGBA16F intermediates instead of integer working-term textures, exact `acos`, shared
authored normals and reverse-Z geometry, fixed Hilbert/R2 noise, normalization against
each unobstructed slice and a small coplanar rejection bias to preserve open slopes,
engine distance fade and common R8
ambient composition. There are no bent
normals, TAA, history accumulation or animated noise. The port does not promise the
quality or cost of Intel's full renderer. Open surfaces, ground contacts, material
coverage, repeated frames and live method switches are covered by GPU regressions.

## Larger alternatives

[AMD CACAO](https://gpuopen.com/fidelityfx-cacao/) is an MIT-licensed ASSAO derivative
with Vulkan and DX12 implementations, adaptive sampling and edge-aware filtering.
It is a suitable separate integration experiment, not yet included here. Its HLSL/GLSL
passes and deinterleaved resources require a WGPU port or explicit native API bridge.
[NVIDIA HBAO+](https://github.com/NVIDIAGameWorks/HBAOPlus) uses NVIDIA's own source
license rather than this project's permitted port licenses; no code is imported.
Ray-traced AO additionally needs acceleration structures, dynamic geometry updates
and denoising. None of these methods can compensate for a material omitting AO.

## Optional temporal half-resolution method

`xegtao-temporal` in F8 Effects always estimates visibility at half resolution.
It rotates sampling over frames, uses three edge-aware spatial denoise passes, then
reprojects AO-only history using independent camera/object motion. This works
without a temporal upscaler and does not read presented frames or UI. Existing
spatial methods and defaults retain their paths.

History rejects camera cuts, projection/size changes, long frame gaps, changed AO
settings, out-of-view samples, depth discontinuities and normal disagreement.
Independent motion retains previous view depth for each visible surface, so a
following camera can reuse a moving car's history. Four validated bilinear taps
use a surface-plane depth check, including sloping roads and odd-sized targets.
A bounded neighborhood margin prevents current sample noise from erasing the
accumulated estimate. Fully open surfaces clear immediately; invalid coverage
uses current AO. AO's RGBA motion/depth target is separate from the existing RG
motion targets consumed by FSR/DLSS and frame generation.
The approach builds on the temporal sampling discussion in the GTAO research
already cited above; the reprojection and rejection shader is a custom implementation.

Synthetic GPU coverage checks contact visibility, newly exposed open planes,
camera-cut cold starts, odd-size resizing, and exact spatial-method round trips
on Vulkan and DX12. This does not establish visual quality during a full drive.

## Spatial sample coverage and stability checks

The spatial XeGTAO depth search uses 2 slices x 6 steps per side at Low,
3 x 6 at Medium and 4 x 8 at High. Low/Medium remain half resolution; High is
full resolution. The denser radial search reduces fixed-pattern sampling error
without depending on a host TAA pass. It costs more than the previous 2 x 3,
3 x 4 and 4 x 4 budgets; decreasing intensity does not recover that cost.
Temporal XeGTAO retains the cheaper 2 x 3, 3 x 4 and 4 x 4 estimator, always
at half resolution, and accumulates its coverage over time.

A dense 24 x 24 sample reference checks spatial contact error against the former
budgets. A static/following-camera sequence measures actual temporal mask variation,
not just the presence of AO contacts. Jittered FSR/DLSS combinations, occluder
removal, camera cuts and odd-sized resize coverage remain separate checks.
These synthetic metrics do not establish in-game visual quality; inspect fine
car geometry and moving contacts in a driving test. Single-layer depth limitations
and thin-geometry over-occlusion still apply.
