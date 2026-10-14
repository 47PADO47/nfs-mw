# Experimental ambient occlusion

Spatial GTAO in the native `blackbox-render` backend. AO and contact shadows share
a reverse-Z depth/world-normal prepass. AO produces an ambient visibility mask
before colour shading, with no motion vectors or temporal history. It is off by
default in normal settings; the Graphics Lab launcher selects Medium. Other backends
and adapters without compute support report an unsupported-setting downgrade.

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
   horizons with a static 4-by-4 stratification of slice angles and radial offsets.
   Neighbouring pixels cover different samples, avoiding repeated hard bands around
   wide occluders. Store visibility, view depth and an octahedral normal in RGBA16F.
2. Denoise with a 5-by-5 spatial filter. Plane distance and normal agreement reject
   samples across silhouettes rather than blurring them into foreground objects.
3. Bilaterally upsample four nearby AO samples into a full-resolution R8 visibility
   mask. Scene shaders apply it to ambient lighting before fog. Direct sunlight,
   sun highlights, environment reflections and blended surfaces are unchanged.

The default contact radius is 1.25 world units, capped at 96 screen pixels. A
short range falloff and thickness heuristic limit distant silhouette occlusion.
Depth bias rejects coplanar self-occlusion. Visibility is normalized against an
unobstructed slice so open roads and slopes stay bright. AO fades between 40 and
100 view-space units; scene fog is applied after lighting. Full intensity removes
at most 45% of ambient light; default 75% intensity caps that at 33.75%.
The prelit world's ambient share remains an approximation (35% of its lighting).
For the original car rig, fill/back lights are treated as ambient illumination.

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
