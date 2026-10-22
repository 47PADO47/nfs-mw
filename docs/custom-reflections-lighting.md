# Custom car reflections and world lighting

This extension is a rewrite design, not an Original PC reconstruction or a
reproduction of a proprietary renderer. Original PC material equations remain
unchanged. All six probe faces continue to refresh at the selected cadence;
uncached High still refreshes every base frame. Capture processing adds no
automatic resolution reduction, temporal history, jitter or additional geometry
capture. Explicit quality presets have the budgets documented in graphics-lab.md.

## CPU preparation

The renderer buckets normal-mapped scenery, architectural glass and sky alongside
the ordinary draws. Each family walks only its own commands instead of rescanning
all instances for every blend mode/layer. Consecutive normal/window draws retain
mesh and material bindings. Draw order within each family is preserved.

Each capture face retains a draw selection keyed by the exact mesh-handle sequence
and a resource revision. Mesh/texture creation and destruction, including handle
recycling, and changed texture redirects invalidate it. Transforms are deliberately
excluded: camera uniforms and instance matrices still upload every refresh.
Streaming, LOD membership and instance-count changes rebuild selection immediately.
The main and projected views use the same structural cache rules. Refreshing a
cube's contents does not recreate its unchanged glossy bind group.

## Distance and placement

Custom car captures test current world-space scenery bounds against a finite
radius. A wall with a distant placement origin remains visible when its actual
bounds are nearby. Moved props use current bounds. Original PC capture selection
keeps its documented origin-distance rule.

Finite Custom captures fade scenery to the capture clear/fog colour over the
outer 15 percent of their radius. This spatial transition uses per-pixel depth,
not frame history or object-centre opacity. Sky depth is exempt. Unlimited and
Original PC captures preserve stored colour bytes without this fade. This does
not make a limited capture equivalent to unlimited scenery coverage; diffuse
probe irradiance also still derives from the selected capture.

The renderer retains six actual Depth32Float face attachments. Custom surfaces
below roughness 0.5 can reproject their reflection ray from the shaded point to
this depth field. Three bounded iterations intersect the ray with the sampled
radial depth, followed by a residual confidence check. Far scenery/sky, excessive
receiver offsets, behind-receiver intersections and failed depth-edge convergence
use the uncorrected ray. Correction fades out on broad rough reflections.

Depth follows the native RH camera orientation; colour storage flips vertically.
The depth sampler explicitly converts cube coordinates back to the native face.
The capture excludes the reflective car as before. Static/generated environments
disable depth correction, including capture-off and environment replacement.

This is an approximation, not ray tracing: hidden/disoccluded surfaces cannot be
recovered from one probe. It avoids a universal room-box assumption but does not
supply multiple local probes. Persistent depth uses 24 MiB at 1024 pixels per face,
20 MiB more than the former shared 4 MiB depth attachment, plus a tiny fallback.
No CPU readback is involved. Fragment cost depends on visible reflective area.

## Sun and weather

With weather matching enabled, Custom car key colour/intensity follows the world's
authored diffuse lighting at the current sun height and cloud blend. Its two
fallback fills follow authored ambient. Probe sky/ground illumination still replaces
those fills when enabled, avoiding duplicate ambient light. Missing authored world
inputs and standalone car viewers retain the previous artistic rig.

Opt-in Custom highlights add the sun angular radius in quadrature to GGX slope
variance. The existing sun-angle control supplies that radius independently of
whether soft shadows are enabled. This is a normalized distribution-broadening
approximation, not an exact finite-disc integration. The GGX denominator guard is
below the minimum supported lobe's denominator, preserving its normalization;
a larger guard could make a small sun radius increase the peak instead of reducing it.
It preserves rough-surface
lobes while limiting excessively narrow smooth-surface highlights.

F8 Lighting exposes **Local reflection correction** and **Match car lights to
weather** independently. Their defaults are on and settings persist. Car material
response still gates the Custom surface model. Original PC shader arithmetic does
not consume the new correction or angular width.

## Validation boundaries

Synthetic checks cover draw order, resource invalidation, six-axis depth
reprojection and its disabled/sky/rough fallbacks, bounds-distance selection and
separate weather direct/ambient response. Existing original-material GPU parity,
normal/window/sky and reflection capture tests remain required. Performance must
be measured at the actual output resolution; a 1080p result is not a 4K result.

## Automatic car lighting and controls

The player-facing name is **Enhanced**. Saved `glossy` values and CLI arguments
remain valid; `enhanced` is also accepted. This is a custom appearance choice,
not a claim of complete Original PC parity or a full physically based renderer.

F8 Lighting groups material response, material clearcoat, weather-matched sunlight,
local reflection placement and environment diffuse light under **Enhanced
(automatic)**. **Indirect light strength** multiplies diffuse fill only, with a
neutral default of 1.0. Detailed feature switches and upper/lower environment
multipliers remain in **Advanced car lighting**. Changing material mode retains
those tuning values and leaves reflection resolution, cadence and distance alone.
Alternative Simple and original shadowless material paths remain in Advanced Materials.

The Enhanced preset calibrates upper/lower environment multipliers to 0.85/0.45.
Previously saved manual values are retained. The same calibration is used across
weather and locations: captured radiance already contains sun/cloud lighting and
local surface colours. There is no additional global cloud or tunnel multiplier.
Fallback rig fills also respect the world's authored ambient alpha. Probe fill
replaces those fallback lights rather than adding a second ambient term.

Enhanced world captures now reuse current-frame sun visibility. The order is:
world shadow atlas, six capture faces, environment filtering/diffuse projection,
then car colour, in one frame submission. The existing atlas is rendered once;
captures use a bounded comparison instead of repeating the main view's PCSS search.
Only world-space shadow data is reused: AO, contact masks, UI and debug views do not
enter captures. Capture selection uses atlas containment, so nearby surfaces behind
the main camera can receive shadows too. Raw/reference captures keep their previous
unshadowed contract. Captured data is consumed in the same frame; optional cached
reflection cadence remains a separate quality tradeoff.

This corrects a source of bright reflected roads and walls in shade. It does not
multiply every reflection by the receiving panel's shadow: a shaded car can still
reflect genuinely sunlit scenery. Outside shadow-atlas coverage, captures retain
the unshadowed fallback. Baked ambient, LDR radiance and one-probe visibility remain
limits, especially indoors; this is not local GI or exact per-pixel ray tracing.

For reproducible comparisons, `environment clouds <0..1|location>` and
`environment hour <0..24>` use the same session state as F8 Environment. They do
not save weather or alter rain. Original-game night/tunnel exposure parity remains
separate from this custom lighting calibration.
