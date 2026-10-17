# Original world atmosphere

Sources: retail PC executable, independently inspected read only, and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0), generated
`timeofdaylighting` layout/names and `SkyRender`/`rain` reference descriptions.
No source or assets are copied. See [provenance](../provenance/world-atmosphere.md).

## Preset inputs [PC checked]

The existing two-stage colour blend also applies to all scalar inputs. Blend
presets 0 and 2 for high sun, 1 and 3 for low sun, by cloud intensity. Blend
these results by `1 - (clamp(sun_z, .25, .5) - .25) * 4`. The PC lighting
update then unconditionally sets ambient alpha to **4.0**, which scales
the ambient term separately from its RGB. This final runtime assignment
takes precedence over the alpha authored in each preset.

Normal-mapped world materials also consume the blended `SpecularColour`
RGB (`0x5855B1E7`). It follows the same cloud/high-sun/low-sun blend as
ambient and diffuse. The matched PC normal-map material setup supplies a
specular exponent of **11.0**. Deliver RGB and exponent separately to the
renderer; plain world, tree, prelit and sky families must not gain this
specular contribution. Car material lighting remains its separate rig.

| Attribute | Hash | Purpose |
|---|---|---|
| FogSkyColourScale | `0x9095E8B8` | Sky scattering coefficient scale |
| FogHazeColourScale | `0x02C1F6CA` | Haze scattering coefficient scale |
| FogSunFalloff | `0x2EF8A8BF` | Mie angular parameter |
| FogDistanceScale | `0x72606030` | Shader view-distance multiplier |
| FogInLightScatter | `0x00756239` | Shader in-scattered light multiplier |

The distance coefficient conversion is `field*.01*.0099 + .0001`.
The nonpositive haze-distance branch produces zero haze coefficients.
Match shader formulas separately; names alone do not establish
the distance multiplier's position in the shader or its units.

Missing or invalid required fields disable this reconstructed atmosphere;
do not silently restore constants measured at one position. Scalar bounds
and nonfinite rejection are remake robustness decisions.

## Spatial cloud blend [PC checked, timing adapted]

The lighting blend is the precipitation object's cloud intensity, not a
constant, generic tunnel weight or clock hour. Its target comes from the
`Clouds` parameter-map layer, queried in original planar world `(x,y)`.
Field0 is intensity; field1 is the time-gate threshold. A selected set is
active when the weather time-gate is less than or equal to field1; otherwise
use the default cloud amount, zero in the inspected build. Missing layers,
wrong field types and nonfinite values use that same safe default.

Positive desired rain takes precedence over the spatial cloud target.
The original advances cloud intensity toward its target with `dt/10` and
caps the result at one. Its original zero-target near-zero snap is retained
at .01. Do not use frame count as elapsed time. The remake uses the exact
exponential equivalent of the original 60 Hz step:
`alpha = 1 - (1 - 1/(60*10))^(60*dt)`. This is an explicit high-FPS
adaptation, preserving the 60 Hz response for a constant target.
Zero/nonfinite/negative elapsed time holds the state.

The ordinary free-roam weather **clock** is constant one second, not the
gate. The gate also depends on original planar X: angle in degrees is
`weather_seconds * 0.71428573 + x * .02`. Truncate `angle*65536`, divide
the resulting integer by 360 (toward zero), and use its low 16 bits as the
engine sine angle. Gate is `1 - abs(sine)`; in ordinary unscripted free
roam, negative sine is replaced by zero before this expression. The remake
uses the quantized angle with standard sine; the engine sine-table's tiny
interpolation differences remain a precision limit. Scripted race
weather's clock and automatic rain selection remain incomplete.
Forced rain remains the existing user override. Do not claim full weather
parity or tunnel exposure from this cloud map.

## Sky composition boundary

The stock assets supply main/cap textures for blue, cloud, overcast, low
resolution and reflection layers. The reference swaps texture slots and
places a dedicated sky model around the active camera, rather than drawing
a fixed solid in the city. Blue/cloud/overcast are distinct draws, with
cloud and overcast alpha controlled by cloud intensity. Reflection and
rear-view views have separate scales. These facts do not establish the PC
shader's complete compositing formula, state or chosen model; implementation
must be checked against the matched PC draw path before claiming parity.

### Programmable PC sky placement [PC branch and declaration checked]

The programmable PC reconstruction uses the stock `SKYDOME_XENON` model,
retaining its authored diffuse and independent UV1. The name also exists in
the PC installation; it does not imply dependence on console or mod assets.
Replace its fixed world placement with a dedicated camera-relative draw.
Do not duplicate it with `SKYDOME` or draw `SKY_SPECULAR` into the player view. Construct
the model matrix anew from identity each view: camera translation and uniform
scale 1 in the main view, .03 in the rear mirror, .0065 in environment faces.
Bypass scenery distance/LOD/frustum gates for this draw. Reflection-specific
texture substitutions remain outside this batch. The three programmable main
layers are specified below.
The existing legacy graphics path retains its previous placements when the
original-world reconstruction is disabled. Stable per-view instance identity
must not overlap scenery or vehicles. No frame-count movement is introduced.

The stock track supplies `SKYDOME`, `SKYDOME_XENON` and `SKY_SPECULAR` in
its shared X0 stream section. The global in-game pack supplies sky textures;
it does not supply these dome models. Resolve the layered sky from the same
already loaded stream models as ordinary shared residency, with global models
only as fallback. Installed validation must create all three native layers
from these real model and texture sources and verify idempotent release.
The programmable skybox effect selects the Xenon dome's 44-byte declaration.
The ordinary `SKYDOME` has 36-byte vertices and is selected by the separate
legacy effect path. It may be used by the remake's documented legacy backend
fallback, but must not supply duplicated UV0 to the three programmable passes.

### PC shader state [PC FPU instructions checked]

The modern sky effect supplies miscellaneous textures `SKY_NEXGEN_CAP_A`,
`SKY_NEXGEN_CLOUDS_B`, `SKY_NEXGEN_CAP_B`. It supplies cloud shader weights
from the same precipitation cloud amount: an exact zero becomes .1 before
weight calculation, then `x=clamp(cloud*10/3,0,1)`,
`y=clamp(2*cloud*cloud,0,.8)`, `z=clamp(cloud*cloud,0,.8)`.
The unused fourth component is zero in the remake. The explicit debug
overcast override is out of scope. Texture scroll advances at .02 per
simulation second with the default lighting rate one. It advances once
per world update, never once per view or pass; pause holds it and mirror
or six environment faces must not accelerate it.

The main modern geometry uses the low-resolution layer transform (scale
one), with multiple shader passes over the same model. Do not apply the
legacy separate-layer scales .9/.8 to those modern shader passes.
The programmable PC path uses the model's authored skybox effect.
Preserve independent UV1, as required by the shaders in
[world-sky-shading.md](world-sky-shading.md). Missing required assets or an
unsupported renderer must use the documented legacy fallback rather than
white textures or an incomplete subset of the passes.

The legacy effect path replaces the ordinary dome's
`SKY_MIDDAY_A_CLOUDS_A`/`SKY_MIDDAY_A_CAP_A` bindings with environment
textures for its low-resolution draw. That replacement is excluded from the
programmable path. The Xenon dome's authored diffuse is `SKY_NEXGEN_CLOUDS_A`.
For the three programmable shader passes, base uses that authored diffuse with
miscellaneous1, cloud2 samples miscellaneous2, cloud3 samples miscellaneous3.
These are three passes over the same geometry, ordered base then cloud2 then
cloud3. The full native renderer owns these registered meshes/materials for
the world lifetime. Keep every material and mesh handle in the sky resource
owner and destroy them before replacing that owner or closing its scene.
Scene replacement must forward the close hook through pause wrappers; calling
close repeatedly is harmless. Renderer shutdown destroys any final resources.

## Validation

Synthetic tests must cover both stages of scalar/ambient-alpha blending,
missing fields, nonfinite values, map field selection and threshold equality,
rain precedence, zero elapsed time, and cloud response at 30/60/144/240/360 Hz.
Installed checks require multiple cloud-map sets and verify values from the
authored track. Asset-dependent mods are excluded from the baseline. Static
analysis is not a live screenshot comparison; remaining visual gaps must be
listed after runtime validation.

## Reproducible environment comparison controls [remake controls]

The F8 Environment panel distinguishes a requested condition from its current
simulation state. It exposes the current sun phase/direction, cloud amount,
authored cloud target, rain intensity and cloud-map availability. These readbacks
are diagnostic state; they are not measurements of a simultaneously running
original game.

Three sun modes are independent comparison controls. Custom clock retains the
existing remake hour-to-phase mapping and lighting schedule. Fixed phase holds a
phase in the original gameplay interval [.05,.95]. Original cycle advances that
phase at 1/1200 per simulated second and reverses at the endpoints: eighteen
minutes per leg. Its initial phase is the current selection, not a claim about
the original game's random initial phase. High-FPS reconstruction reflects
overshoot at the endpoints rather than discarding a frame's excess elapsed time.
Freeze time holds either moving mode without accumulating catch-up time.
Changing a time or phase must refresh paused lighting and invalidate cached
reflections without stepping vehicle simulation.

Cloud comparisons can follow the installed location/rain target, or hold a
manual amount. Sunny, Cloudy and Overcast select 0, .5 and 1 immediately. These
are comparison presets over the original blend input, not three discovered
original weather state IDs. A manual cloud amount affects the original sky,
world lighting, fog and car specular blend together; it does not create rain.
Returning to location resumes authored targets and easing. Settle clouds snaps
to the currently selected target for a repeatable screenshot without waiting.
All overrides are session-local and finite, clamped inputs. A zero simulation
delta holds the automatic easing; a deliberate comparison selection may update
lighting while paused.

The old Clear rain label is replaced by No rain: disabling forced rain does not
force the independent location cloud target to zero. Rain particle amount scales
the existing custom particle density; nonzero forced rain still targets full
rain intensity. Automatic regional storms, exposure under tunnels/overpasses,
wet-road dampness history and planar reflection captures remain incomplete.
The custom night shortcut remains a dusk-lit clock extension. A full-night
renderer is an optional custom feature, not a missing ordinary PC sun-cycle
behavior. Neither the custom clock nor comparison overrides establish scripted
race/NIS weather parity.
