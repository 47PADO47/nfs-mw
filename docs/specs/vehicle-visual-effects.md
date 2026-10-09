# Vehicle visual effects

Research checkpoint, 2026-10-09. This specification precedes implementation.
The later [restored particle-motion specification](xenon-particle-motion.md)
supersedes the initial motion, size, count, fade and rear-volume approximations
below. This file retains the original research checkpoint and contact scope.
The proposed collision sparks and speed trails are optional, default off.
Those defaults preserve the current host presentation. Ordinary collision sparks
exist in PC effect data; the optional elongated Xenon spark streaks and wind
trails are console-restored extras, not verified stock-PC visual parity.

## Sources and evidence scope

- Unmodified and restoration-mod asset sets, `GLOBAL/attributes.bin`: proprietary game
  data, measured read-only with this project's `blackbox-attrib` reader.
  Values and references below are **[verified data]**, not a live-game comparison.
- [xan1242/NFSMW_XenonEffects](https://github.com/xan1242/NFSMW_XenonEffects/tree/b5070080c75a7dfc555deca0a5e005311954dbfd),
  Lovro Pleše, MIT: README, INI and `dllmain.cpp`. **[community source]** for the
  Carbon-to-MW PC restoration; its embedded reconstructed routines are used only
  to understand behavior, never copied into this project.
- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw/tree/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c),
  CC0 decompilation: `Physics/Behaviors/Effects.cpp`, `RigidBody.cpp`,
  `World/CarRenderConn.cpp`, `CarRender.cpp`, generated `fuelcell_emitter.h`,
  `fuelcell_effect.h`, and `Misc/MWAttribUserTypes.h`. **[decomp]**, with platform
  and incomplete-reconstruction limits. No claim below depends on a name alone.
- [ThirteenAG/WidescreenFixesPack](https://github.com/ThirteenAG/WidescreenFixesPack/blob/708f7528fb8b54c79d4ac3c6071ea7ccc01ae162/source/NFSMostWanted.WidescreenFix/NOSTrailFix.ixx),
  MIT: the NOS trail correction hooks the car flare renderer and scales a
  position difference between sampled frames. **[community source]**; it is
  independent of the fuelcell contrail emitter.

## Distinct effects

| Effect | Supported facts | Scope and remaining uncertainty |
|---|---|---|
| Ordinary collision sparks | BMW/default hit and scrape linkages resolve effects containing normal spark, glow and debris emitters **[verified data]**; original collision behavior drives those linkages **[decomp]** | Native PC spark particles must not be described as universally absent. Their live appearance has not been calibrated here. |
| Extra spark streaks | `fuelcell_effect/fxsprk_line` links `emsprk_line1` and `emsprk_line2`; conventional `emitterdata/emcs_sc_spark02` links it through `XenonEffect` **[verified data]** | The restoration author says extra Xenon sparks were omitted from PC **[community source]**. |
| Speed/wind contrails | `fuelcell_effect/contrail` links `fuelcell_emitter/trail3` **[verified data]** | The restoration identifies this as the wind effect behind the car. It is distinct from light trails and exhaust flames. |
| NOS/light trails | Widescreen Fix's primary source corrects frame-rate-sensitive car flare trails **[community source]** | Exact stock-PC activation and appearance remain unmeasured. Do not make speed trails into taillight trails by assumption. |
| NOS exhaust and backfire | BMW's `ecar` links `NOSEffect` to `fxcar_nos` and `MissShiftEffect` to `fxcar_exhaust_bmw` **[verified data]** | `CarRenderConn::UpdateEffects` updates exhaust on NOS, fires miss-shift effects, and can update the NOS effect while blowoff/shifting is active **[decomp]**. This is not proof that every throttle lift produces a flame. |

## Collision inputs and effect selection

The host's contact bridge must retain each contact's world point, unit normal,
surface key and section/instance identity, plus the velocities and reaction
produced by that contact. Use point velocity before collision response:
linear velocity plus angular velocity crossed with point minus world center
of gravity. Closing speed is the nonnegative component into the surface;
tangential speed is the magnitude after removing its normal component.
Reject nonfinite data and invalid normals. Whole-car speed and an audio-impact
summary cannot substitute for these contact inputs. **[host design]**

Initial emission covers supported wall contacts. Ground-body corners currently
do not provide equivalent metadata. A stationary overlap with neither closing
nor sliding motion produces no continued emission. Continuing scrape must use
an active contact with tangential motion, not repeated positional correction.
Retain prop identity so unsuitable props can be filtered by their actual material.
Do not invent a metal-name whitelist. **[host design]**

The initial contact bridge retains at most 16 deepest supported wall contacts
per simulation step and excludes light props. Contacts for an unsupported prop
or without usable material metadata do not claim original material classification.
Each output retains point, normal, pre-response point velocity, impulse divided
by mass, surface/section/instance information and prop identity. **[host design]**

Original material selection starts with a default linkage, then looks for the
contact's exact `simsurface` or an ancestor surface. Hit and scrape resolve
independently from `pvehicle.OnHitWorld` and `OnScrapeWorld`. The linkage stores
surface reference, effect reference, minimum input and maximum input. Follow
the actual effect class; an audio linkage is not automatically a visual emitter.
Require finite, increasing limits before mapping intensity. **[decomp]**

Lookup starts with the default linkage even for a null surface. `SimSurface`
maps missing hashes to unknown, or invalid surface objects to null. If no exact
or ancestor linkage overrides the default, it remains selected. Real world
contacts with hash zero therefore retain the default; missing prop metadata is
a distinct host limitation and is skipped. An explicit unsupported wood linkage
still stops fallback. **[decomp]** `Effects.cpp` linkage lookup and
`Sim/Common/SimSurface.cpp` surface lookup, same pinned revision as above.

`EffectLinkageRecord` occupies 32 bytes: surface `RefSpec` at byte 0, effect
`RefSpec` at byte 12, float minimum at byte 24, float maximum at byte 28. Each
12-byte reference stores class key then collection key; its remaining word is
not needed to resolve the database reference. The reader should validate the
record size and reference target rather than treating arbitrary bytes as a
spark selector. **[decomp layout; runtime references need validation]**

Measured BMW/default links in both installs are **[verified data]**:

| Channel / surface | `effects` target | Minimum / maximum | `emittergroup` target |
|---|---|---|---|
| Hit / default | `carhitwall` | 1 / 30 | `fxcar_impactl` |
| Hit / wood | `carhitwood` | 0.5 / 30 | `FxCS_Sc_Wood` |
| Scrape / default | `carscrapewall` | 5 / 30 | `fxcs_sc_metal` |
| Scrape / wood | `carscrapewood` | 5 / 30 | `FxCS_Sc_Wood` |
| Scrape / organic | `carscrapepavement` | 5 / 30 | `FxCS_Sc_Stone` |

`fxcar_impactl` contains ordinary spark01/spark02/glow emitters; the metal group
contains glow, spark01, spark02 and animated sparks. The wood group carries
smoke/splinters; the stone group carries stone particles and sparks. Names are
lookup identifiers, not a universal material flag. **[verified data]**

For optional streaks, resolve and validate every reference in this exact path:
the linkage's `effects` target, `effects.emittergroup`, `emittergroup.Emitters`
(array of `emitterdata` references), each `emitterdata.XenonEffect` (array of
`fuelcell_effect` references), then `fuelcell_effect.NGEmitter` (array of
`fuelcell_emitter` references). The initial host supports `fxsprk_line` and its
two known line emitters. A group without that supported path does not produce
these streaks. Other ordinary sparks, smoke, glow and debris remain outside this
first renderer. Validate referenced classes as well as collection keys; a null
or missing reference is not a fallback spark. **[verified data + host scope]**

The default hit wrapper has `EmitterQuadratic=(0,1,0,0)`; default scrape has
`(0.3,3,0,0)`; both inherit velocity at 1. Their audio fields coexist with the
visual `emittergroup` field. **[verified data]** Original curve application must
be specified before interpreting these four coefficients as an exact formula.

Original `Effects` dispatches both hit and scrape on a world collision. Hit
maps its impulse-per-mass input through the linkage range and requires closing
velocity squared above 1. Scrape requires sliding speed above 1 m/s, maps
through its range with a 0.1–1 intensity clamp, and refreshes a 0.2-second
effect timeout. **[decomp]** The host normal impulse is in different units
from a speed. Do not apply that hit range to raw N·s without conversion or an
explicit approximation. No per-hit cooldown was established by this source.

## Contrail activation

The primary restoration source enables contrails at 44 m/s (158.4 km/h);
its optional CG-style path also enables them during NOS. Outside CG style,
intensity interpolates from 0.1 at the threshold to 0.75 at twice the threshold,
then stays capped at 0.75. **[community source]**

The console reconstruction also gates contrails to the player's car, an
outside camera, player views, and no active NIS; it activates on NOS or speed
at least 44 m/s. **[decomp]** These are alternative platform/preset behaviors,
not one verified stock-PC rule. Use the chosen preset explicitly. A trail
stops spawning below its threshold; existing particles finish their lifetime.
Free-camera visibility policy must be explicit and reproducible. **[host design]**

The original contrail render call supplies the car body transform and car
velocity; `trail3` supplies its own volume, rather than an exhaust/taillight
marker. **[decomp + verified data]** A host spawn location derived from the rear
body extent is an explicit placement approximation, not a measured original
marker or a claim that all original contrails begin at the rear bumper.

## Appearance data and procedural approximation

Both inspected installs contain the following same selected fuelcell values.
Field names come from the generated class header; values are measured data.
Raw length/height values are not documented here as meters or screen pixels.

| Emitter | RGBA | Life / variance | Count / variance | Length start / delta | Height |
|---|---|---|---|---|---|
| `emsprk_line1` | 1, 0.86, 0.75, 1 | 1 / 0.05 | 30 / 0 | 255 / 140 | 62 |
| `emsprk_line2` | 1, 0.56, 0.28, 1 | 0.4 / 0.1 | 10 / 0 | 37 / 33 | 34 |
| `trail3` | 0.64, 1, 0.85, 0.13 | 0.25 / 0.25 | 15 / 0 | 200 / 50 | 255 |

Spark line1 has starting local velocity (0,0,1), variation (0.4,0.4,2),
inheritance (-1,-1,-0.1), and gravity start/delta -12/9. Line2 starts at
(0.1,0.1,1), varies by (2,2,2), inherits (-0.2,-0.2,0), and has gravity -10/2.
`trail3` starts at (0,0,-1), inherits (-0.3,-0.3,-0.1), has volume extent
(1.5,2,0.75), gravity -2/0, and `zContrail=1`. **[verified data]** Coordinate
conversion, variance distribution, dimension encoding and count-to-rate units
must be verified before treating these as an exact runtime recipe.

A first procedural renderer can use bright warm short streaks for sparks and
faint pale green/white elongated streaks for the wind effect, informed by those
colors and lifetimes. Contact position, outgoing velocity, widening, texture
mask, additive brightness, gravity, spacing and geometry size remain explicit
**[host approximations]** until calibrated. No copied texture is required.
Particles must be depth-tested against scene geometry, rendered before the HUD,
bounded by live-count and vertex budgets, aged when simulation does not advance,
cleared on disable/reset as appropriate, and emitted from elapsed time rather
than rendered-frame count. Fixed-step captures need deterministic seeds and age.

Upstream source defaults include 10000 particles, effect-list size 500 and a
30 FPS contrail limit. **[community source]** The host chooses independent budgets and
lifetime fade, but must label them as new rendering behavior.

## Verification before a parity claim

The first host implementation uses one strongest reacting impact and one
strongest supported sliding contact per fixed step, to avoid multiplying sparks
across body probes. Impact bursts contain at most 48 particles; scraping uses
160 times intensity particles/second with at most 24 owed per step and no debt
after contact loss. Sparks cap at 768 and wind trails at 256. Wind trails use
240 times intensity particles/second and the speed-only restoration rule;
NOS does not bypass the threshold. Spark life is capped at 1.5 seconds and trail
life at 0.5 seconds. These limits, particle gravity/motion, additive masks and
fade are **[host approximation]**, not native emitter unit conversions.

Unknown prop materials are skipped. Wind particles spawn in a bounded volume
around the body's rear and retain their own world orientation; they finish
their lifetime below threshold. Free camera disconnects trails and ages sparks.
Reset and disable clear the relevant particles. Pausing freezes simulation,
while an options change refreshes the effect buffers. Native bounce, the
wrapper's quadratic curve and its scrape timeout are not recreated; current
contact motion controls emission. Detailed smoke still composites in its
existing later pass; cross-effect transparency is not globally sorted.

Check exact resolved material/effect paths and finite linkage ranges. Exercise
an impact, sustained wall scrape, stationary overlap, unsuitable surface, reset,
disable, and lifetime expiry. Verify speed below/at/above threshold, NOS preset
differences, camera gates, HUD separation and bounded resources in actual
captures. Native appearance requires a controlled original comparison with
documented platform/plugin state. This research started/stopped no game process
and establishes no original-game visual calibration.
