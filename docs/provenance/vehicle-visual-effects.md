# Vehicle visual effects

- **Spec:** [vehicle-visual-effects.md](../specs/vehicle-visual-effects.md).
- **Research date:** 2026-10-09.
- **Implementation:** stock-data reader, collision contact bridge, optional host
  effects and generic additive streak renderer authored from the spec.
  Validation below covers this implementation; native visual parity remains
  uncalibrated.
- **Method:** read-only install/data inspection and original/reconstructed source
  research; own-words specification. No source, game asset, texture, raw dump,
  executable, Ghidra database or decompiler output was copied into the repository.

## Sources read and credits

| Source | License / role | Material used |
|---|---|---|
| [Lovro Pleše / xan1242, NFSMW_XenonEffects](https://github.com/xan1242/NFSMW_XenonEffects/tree/b5070080c75a7dfc555deca0a5e005311954dbfd) | MIT; primary restoration author's source; reconstructed native routines remain reference-only | [README](https://github.com/xan1242/NFSMW_XenonEffects/blob/b5070080c75a7dfc555deca0a5e005311954dbfd/README.md), [configuration](https://github.com/xan1242/NFSMW_XenonEffects/blob/b5070080c75a7dfc555deca0a5e005311954dbfd/NFSMW_XenonEffects.ini), [source](https://github.com/xan1242/NFSMW_XenonEffects/blob/b5070080c75a7dfc555deca0a5e005311954dbfd/dllmain.cpp): extra Xenon sparks/contrails, Carbon backport, activation/intensity modes, frame limits and budgets |
| [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw/tree/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c) | CC0; decompilation, no code ported | Original collision effect dispatch and selection, exhaust effect selection, console contrail camera/activation gates, class/record names and layouts |
| [ThirteenAG/WidescreenFixesPack](https://github.com/ThirteenAG/WidescreenFixesPack/tree/708f7528fb8b54c79d4ac3c6071ea7ccc01ae162) | MIT; primary mod source, no code ported | [NOSTrailFix.ixx](https://github.com/ThirteenAG/WidescreenFixesPack/blob/708f7528fb8b54c79d4ac3c6071ea7ccc01ae162/source/NFSMostWanted.WidescreenFix/NOSTrailFix.ixx) establishes a distinct frame-sensitive car flare trail; installed INI documents separate NOS/light-streak controls |
| Unmodified and restoration-mod asset sets | Proprietary assets, read-only; field values/reference facts only | AttribSys classes, effect links and emitter tuning |
| This project's `blackbox-attrib`, `blackbox-vehicle`, world collision host | MIT OR Apache-2.0; existing implementation | Decoded values, point-velocity calculation, retained contact fields and limitations of the current host |

Exact decompilation navigation sources:

- [Physics/Behaviors/Effects.cpp](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/Physics/Behaviors/Effects.cpp):
  `EffectLookup`, world hit/scrape dispatch, input ranges and timeout.
- [Physics/Behaviors/RigidBody.cpp](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/Physics/Behaviors/RigidBody.cpp):
  contact closing/sliding velocity and reaction-per-mass inputs.
- [World/CarRenderConn.cpp](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/World/CarRenderConn.cpp):
  `UpdateContrails`, its render call, `UpdateEffects` and engine animation.
- [World/CarRender.cpp](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/World/CarRender.cpp):
  car effect inventory and exhaust marker terminology.
- Generated [fuelcell_emitter.h](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/Generated/AttribSys/Classes/fuelcell_emitter.h),
  [fuelcell_effect.h](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/Generated/AttribSys/Classes/fuelcell_effect.h),
  [emitterdata.h](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/Generated/AttribSys/Classes/emitterdata.h),
  [effects.h](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/Generated/AttribSys/Classes/effects.h),
  [MWAttribUserTypes.h](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/Misc/MWAttribUserTypes.h),
  and [symbols/vlt.txt](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/symbols/vlt.txt):
  schema names and linkage interpretation, checked against decoded collections.

## Install measurements

The installations are independent evidence scopes, not interchangeable binaries.
The executable was not launched or stopped for this research.

The unmodified asset set and the restoration-mod asset set were compared
independently. No assertion is made that an installed plugin matches the upstream
source revision cited above. Local paths, configuration fingerprints, machine
details and raw measurement logs are retained privately rather than published.

Read-only probes decoded both attribute packs using the project's existing
reader. All eight `fuelcell_emitter` and four `fuelcell_effect` records' inspected
fields match across the two asset sets, despite different pack order. `fxsprk_line`
and `contrail` references resolve to the emitters listed in the spec. Independent
linkage probes resolved BMW/default hit/scrape material records and their actual
`effects` wrappers, visual `emittergroup` targets and child emitters. References
were resolved through the database, rather than inferred from a collection name.

Private measurement output is not a contribution artifact. The spec records
only measured scalar/reference facts and original behavior in our own words.

## Initial evidence checkpoint

- **Verified data:** ordinary spark/debris groups; attached extra spark-line
  effects; contrail emitter graph; selected colors/lifetime/raw tuning;
  material linkage ranges; BMW NOS/miss-shift references.
- **Decomp/source evidence:** original collision input selection and timeout;
  default linkage retained for null or unresolved world surfaces (Effects lookup
  and SimSurface fallback); missing prop materials remain a host exclusion;
  console camera/contrail gates; exhaust NOS/miss-shift selection; restoration
  modes. These do not establish every platform's on-screen behavior.
- **Inference:** the user's speed lines likely mean Xenon wind contrails, given
  the restoration effect inventory. NOS car-flare trails are a distinct plausible effect.
- **Unconfirmed:** exact installed ASI revision, whether it was active for the
  user's observation, native dimensions/count units, variance distribution,
  shader/texture calibration, and controlled original visible appearance.
- **New host design:** optional settings, per-contact bridge and its 16-contact
  cap, supported emitter filtering, rear-body placement approximation, procedural
  streaks, time-based emission, lifetime fade and independent bounded resources.
  Supporting extra streaks does not implement all ordinary debris/sparks/exhaust.

Before describing a renderer as matching the original, perform the controlled
checks in the spec and record the selected PC/console/restoration preset.
An emitter's name does not substitute for a live visual comparison.

## Initial implementation validation (superseded motion)

On 2026-10-09, the combined playable build passed 1048 workspace tests, formatting,
warning-denied Clippy and the leak/size guards. The independent main-based PR also
passed those gates. Explicit ignored BMW attribute-reader and actual main/pause
menu tests each ran against both asset sets. Synthetic tests cover unsupported
materials, malformed references, settings priority, disable/reset, activation,
finite geometry, expiry and bounded budgets.

Selected custom option titles stay opaque through the native highlight loop,
preserving their authored RGB. Main/pause asset tests cover repeated loops,
deselection and exit fades; retail title RGBA matches the untouched asset.

Actual DX12 and Vulkan GPU tests exercised additive RGB energy, taper, opaque
occlusion, unchanged depth, fog attenuation, smoke composition, resize and clear.
The existing high-smoke depth tests passed on both backends after the new batch.

Fifteen 1920x1080 rewrite captures were made using the packaged executable:

- Unmodified assets / DX12: default-spawn impact at 2 seconds, held-wall at 4 seconds, left-wall
  sliding on the straight near (1979, 1617), and speed trails at 7 and 8 seconds.
  The impact emitted 25 sparks; all expired by 3 seconds, with no new sparks
  during the stationary hold. The sliding route emitted 113 sparks by 6 seconds.
- The straight reached 183.8 km/h at 7 seconds: 13 trail particles remained live,
  with 73 emitted. No trails emitted below the 158.4 km/h forward-speed threshold.
- Restoration-mod assets / Vulkan: impact, sliding and the same high-speed route
  completed with visible streaks. Main and actual driving-pause Video captures
  showed the two independent rows. A replacement font atlas produced existing
  glyph mismatches; stock labels rendered correctly.
- Unmodified assets / DX12 disabled runs retained zero emitted sparks/trails on
  the same impact and speed routes. Vulkan reset removed the preceding trail.

Private screenshots and logs remain excluded from contribution artifacts.
Additional main-based release captures rendered the unmodified-asset impact,
sliding and speed routes independently of the other gameplay contributions.
Visual inspection confirms working presentation in the rewrite. It does not
establish an original-PC/Xenon pixel comparison, native size/count calibration,
particle bounce, transparent-particle sorting or a frame-rate improvement.

## Restored motion revision

The later [motion specification](../specs/xenon-particle-motion.md) was written
before replacing the initial approximation. The same pinned restoration source
was used only as a behavioral reference. A private x86 sandbox executed its
spawn routine with controlled uniform inputs, identity transforms and stubbed
collision queries. The numeric measurements in the spec cover velocity,
gravity, lifetime, opacity, count and dimension encoding. This is stronger
evidence for those conversions than visual tuning, but does not establish
native rendering, emitter transform hierarchy or actual Xbox appearance.

The revised host reads the full supported runtime emitter profiles and uses
their body-relative volumes, multiplicative velocity variation, negative world
velocity inheritance, parabola coefficients and time-based lengths. Spark
dispatch retains the two authored populations; wind brightness increases
without changing dispatch density. Swept world collision applies the restoration's
elasticity defaults, with independent host lifetime and bounded query handling.
Camera-facing width, distributed wind births, particle head glows and a short
contact flash are documented host enhancements. Procedural masks remain a
replacement for the restoration's texture atlas. No proprietary asset or
reconstructed code is included in this revision.

Further read-only inspection of `World/WorldConn.cpp` at the credited decomp
revision established the owner's current linear-velocity inheritance and the
different one-shot/continuing emitter frames. The contact bridge's pre-response
point velocity remains the eligibility input. Read-only atlas inspection showed
that the restored texture carries an opaque RGB light mask with a compact bright
end; the earlier broad procedural mask incorrectly filled the segment with light.
The revised shader uses an independently authored compact kernel. Original atlas
pixels and private inspection tools remain excluded from the contribution.
An experimental shorter spark exposure was discarded after comparison screenshots
showed long streaks in the restoration. The revised presentation retains authored
time intervals and a narrow procedural core with an exponential tail. Spark-only
geometry exclusion uses the active car's collision bounds at its interpolated
render pose, including its pivot and rotation. The surviving pieces retain the
original mask coordinates. A single refreshed contact glow accompanies moving
scrapes. These are independent host improvements, not native mesh collision or
dynamic illumination. Colour tuning remains authored; the original's overall
grading is outside this effect contribution.

## Revision validation

The revised combined build passed 1059 workspace tests; the isolated main-based
VFX branch passed 966. Both passed formatting, warning-denied Clippy and repository
leak/size checks. On the main-based branch, the installed BMW effect-reader test
and both main/pause-menu tests passed against unmodified and restoration-mod
assets. Four explicit GPU tests passed: streak/glow blending, depth and fog, and
the existing soft-smoke depth/resize checks on both DX12 and Vulkan.

Nine captures from the revised combined release covered impact, held-wall expiry,
sustained scraping, high-speed trails, reset and disabled effects across the two
asset sets/backends. The held-wall case retained 80 cumulative spark emissions,
with zero live particles by four seconds and no continuing stationary emission.
Disabled impact/speed runs kept both emission counts at zero. Three further
captures from the separately compiled main-based release exercised impact,
scraping and speed trails using unmodified assets.

Visual inspection confirmed long exterior streaks and a contact-local scrape glow.
Synthetic tests verify rotated/pivoted car-box exclusion, preserved mask coordinates,
glow expiry, bounded one-sided-barrier skipping and original-segment hit fractions.
The supplied comparison screenshots use different cars, motion, camera placement
and overall grading. They guide the presentation corrections but do not establish
controlled native pixel parity. The collision box is a conservative mesh
approximation, and transparent effects remain outside global sorting.
