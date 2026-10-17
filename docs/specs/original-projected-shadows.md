# Original PC projected shadows

Status: static producer/receiver research and an isolated original-instruction
camera-fit oracle. Caster selection and runtime vendor sampling remain partial.
This is separate from the custom cascaded shadow enhancement. See
[provenance](../provenance/original-projected-shadows.md).

## Map production

The inspected PC producer allocates one 1024 by 1024 color target and a matching
vendor depth texture. NVIDIA uses a D24S8 depth texture, ATI uses DF16 when the
corresponding device-format check succeeds. Color is ARGB8. The receiving shader
binds color for the color fallback mode and depth for the hardware modes.
The shadow render target receives a depth clear of one and color clear black.
The main-view shadow toggle controls production and receiving techniques.
The original pass sets slope-scale depth bias 4.0 and a constant depth bias of
four 24-bit depth units, resetting both afterwards. Native floating depth and
alternate resolutions require separate validation of equivalent bias.

Higher map resolutions are an optional asset-independent enhancement. Keep fit
coverage unchanged and recompute texel offsets, projection bias and filter
footprints from the selected map size. The original map is not a cascade atlas.

## Single-map camera fit

The fitting point set is in the original camera's view coordinates. It contains
the camera-frustum corners plus eight corners of each selected object bound.
The original frustum-building defaults use near 0.01 and far 50. Exact original
bound collection/culling is not fully reconstructed; an implementation supplying
its own eligible bounds must label this remaining difference.

Use column-vector matrices below. Let V be the original world-to-view transform,
d the normalized view-space direction towards the sun,
`r = normalize(cross(d,(0,0,-1)))`, and `u = normalize(cross(d,r))`.
Construct B whose first three rows are r, d, u. Do not replace this with an
ordinary light-view orthographic fit.

Transform all fitting points through B and find component-wise bounds lo/hi.
With parameters near, offset, widthScale, heightScale and farScale:

`eye = ((lo.x+hi.x)/2 + offset.x, (lo.y+hi.y)/2 + offset.y, lo.z-near)`

For each transformed point p with positive `z = p.z-eye.z`, accumulate
`sx = max(abs((p.x-eye.x)/z))`, `sy = max(abs((p.y-eye.y)/z))`,
and `zmax = max(z)`.

Create a left-handed perspective P with near plane `near`, far plane
`zmax*farScale`, width `2*sx*near*widthScale` and height
`2*sy*near*heightScale`. Let T translate by negative eye.
The resulting world-to-shadow-clip matrix is `post * P * T * B * V`.

Original parameters are near 20.1, widthScale 1.5, heightScale 10,
farScale 10, zero offset. The original fixed post transform, as column arrays,
is `[[1,0,0,0], [0,0,-0.5,0], [0,2,0,0], [0,-1,0.5,1]]`.
It combines an axis warp and a left-handed orthographic transform with width 2,
height 1, near 0.5 and far 2.5. These game constants belong in the game adapter.

The receiver texture transform scales clip x by 0.5 and y by -0.5, preserving
z and w, then adds `(0.5 + 0.5/resolution) * w` to x/y. The original 1024 map
therefore uses 0.50048828125. Output depth is the original 0 to 1 convention;
adapting the scene camera's handedness is a separate caller responsibility.
On a half-integer-center rasterizer, shift the caster's clip X/Y by
`(+1/resolution,-1/resolution)*W` to emulate the source integer-center grid.
Pair this with the receiver bias; applying only the receiver offset displaces
shadows by half a texel. The CPU camera oracle measures the unshifted source
matrix, independently of this graphics API adaptation.
Reject empty, nonfinite or singular fitting inputs explicitly. Such rejection
is a modern robustness guard, not evidence of original degenerate behavior.

## Receiver and strength boundaries

Original receivers attenuate selected lighting terms, rather than multiplying
the final material color uniformly. World direct diffuse is shadowed; ambient
and in-scattering bypass it. Reflective normal-world direct diffuse/highlight
are shadowed once; reflection bypasses it. Cars apply the visibility factor twice
to specular and once to diffuse. Window glints/direct diffuse receive shadowing,
while authored reflection bypasses it.
Tree foliage has no projected-shadow receiving program in the inspected PC
family. Keep it unshadowed in this mode; custom cascades may shadow trees as an
explicit enhancement. NoFog and Prelit retain their unshadowed contracts.

Hardware-depth techniques and color-fallback techniques differ. The inspected
NVIDIA path caps its receiver level at two; the second car technique uses four
samples displaced by half a map texel. Its depth comparison average is combined
with a color classification mask and a minimum visibility floor of
`0.2 + inverseShadowStrength`. Other world techniques use different projected
edge terms. A shared custom CSM visibility factor is not original filter parity.
The car interpolation has no saturation on that floor. Low-sun or overcast
strength can therefore produce a car factor above one; preserve it until the
material's final SDR saturation rather than imposing a physical-visibility cap.

The inspected second hardware WorldReflect technique averages four projected
depth-comparison samples at `(±0.5,±0.5)/resolution`. Its direct-light factor is
`clamp(average + clamp((0.2-shadowUV.y)*5,0,1) + inverseStrength,0,1)`.
The additional car visibility floor must not be applied to this world technique.
These samples depend on the vendor depth-texture comparison behavior; bytecode
alone does not establish its underlying point/linear hardware filtering state.

Shadow strength depends on sun altitude and cloud coverage. A bounded altitude
factor is multiplied by one minus cloud coverage, then mapped between the
configured minimum/maximum strength. Original defaults are:

`strength = 0.1 + 0.8 * (1-cloudCover) * clamp((sunVector.z-0.14)/(0.677-0.14),0,1)`.

Per-draw inverse strength is `drawContribution + (1-strength)`. The draw-specific
contribution is not fully traced here. Fifteen synthetic cases execute the
original scalar updater and agree with this equation within 0.000001.
Runtime draw constants and hardware comparison filter state require additional
capture before claiming complete equivalence.

## Native integration boundary

The first native integration offers the recovered single-map fit separately
from custom cascades. The game supplies original fit constants and a .01 to 50
view frustum; the native right-handed scene view is converted by reversing view
Z before supplying world points. The map uses the existing depth rasterizer and
resident caster feed. Original per-object culling/classification and vendor
raster/filter states remain partial; this mode is labelled incomplete.
The game supplies the inspected four-unit constant bias and slope scale four
separately from custom cascades. The native Depth32 map is a precision difference
from the original vendor formats. Flat and sloping self-casting planes at city
coordinates must remain lit; the custom one-unit bias is insufficient here.

The integration includes resident caster bounds whose projection along the sun
overlaps the fitted receiver frustum. It supplies all eight corners of each
selected world-space bounding box, including upstream objects behind the main
camera. Fitting only the camera corners can crop a caster whose shadow falls
inside that same frustum. The conservative selection is a remake policy; it
does not claim to reproduce the original object classifier. Camera/light
alignment uses an alternate perpendicular axis when the source cross product
degenerates, so aiming along the sun cannot disable the entire map. Ordinary
nondegenerate fitting retains the independently measured original equations.

Perspective culling must use homogeneous clip planes, including clip W. An
affine bounding-sphere test cannot correctly cull the fitted perspective map.
Higher map resolution changes allocation/texel offsets rather than scene assets.

## Validation

Six synthetic cases execute the original x87 basis/fitting/matrix-store segment
in an isolated emulator, with library matrix operations and bound storage as
callbacks. They cover two sun directions, source-view rotation/translation,
fit offsets, near distance and added caster bounds. A separate mathematical
reconstruction agrees within 0.000005 per matrix element. This verifies those
camera stages, not live gameplay culling, rendering or shadow pixels.
