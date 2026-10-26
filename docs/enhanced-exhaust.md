# Enhanced exhaust appearance

The native renderer can replace the installed flame sprites with a small animated
volume at each original exhaust marker. `enhanced_exhaust = false` retains the
original path. `exhaust_flames = false` disables either appearance. F8 exposes both
switches; snapshots and the normal settings file retain them independently of the
global graphics style and quality. The console accepts `set enhanced_exhaust on`
and `set enhanced_exhaust off`. The environment override is `NFSMW_ENHANCED_EXHAUST`.
Other renderer backends retain the installed sprites.

## Reference and adaptation

The visual starting point is [mattatz/THREE.Fire](https://github.com/mattatz/THREE.Fire),
MIT, revision `3f558dde15d814af504d5e36a8a7a93097fa9187`. Its animated example and
`FireShader.js` supply the actual flame construction: a radial/height lookup,
three octaves of advected simplex turbulence, and integration through a bounded
volume. `assets/effects/three-fire.png` is its unmodified `Fire.png` (64x128 RGB).
This is independently licensed media, not an asset extracted from a game.
The lookup's exact bytes are reviewed in the media fingerprint allowlist.

`libs/blackbox-gpu-passes/src/shaders/flames.wgsl` ports that shader to WGSL.
`flame_noise.wgsl` ports its Ashima Arts/webgl-noise simplex implementation (MIT).
The crate retains both license texts. New integration changes are:

- Original marker +Z maps to the reference flame's +Y; dimensions suit car exhausts.
- NOS keeps a blue root, then transitions through white to a warm yellow tail.
  The gradient is applied inside the volume, before integration. Shift and
  lift-off bursts retain the reference lookup's original warm colour.
- Advected tip bending, pressure-driven length changes and brightness flicker
  use the reference simplex noise. Each new NOS activation changes the stable
  flow seed; holding NOS does not reseed it every frame. The root stays attached
  while the outer flame changes shape. Geometry bounds include room for the tip.
- NOS narrows at the nozzle and expands into a longer, denser turbulent body.
  A compressed reference height lookup preserves the outer flame instead of
  eroding it into a short pointed cone. Flow accelerates with vehicle speed;
  activation and release timing remain independent of that visual flow rate.
- NOS integrates emission with Beer-Lambert attenuation inside the volume.
  Overlapping hot samples no longer add without bound in a rear view. The bright
  white region is confined to the core; the reference lookup retains warm,
  irregular outer edges. Backfire retains the original additive integration.
- Actual NOS-burning and existing backfire events control activation. Exponential
  response uses simulation seconds, independent of render frame rate.
- The unit box is intersected analytically; 24–64 integration samples cover only the
  visible interval. Completed reverse-Z scene depth clips every ray, including
  opaque surfaces inside the box. The camera can be inside the volume. Samples
  outside the height or conservative turbulent-radius bounds skip noise evaluation.
- Additive emission is independent of opacity. Fog attenuates distant emission.
  Separate per-view inverse matrices prevent mirror/capture camera contamination.
- The flame body follows its exhaust marker. It does not leave sparse world-space
  particle samples behind a fast car. The original sprite simulation remains
  available when switching appearances.

[Pyrotechnique](https://github.com/elodin-sys/pyrotechnique), Apache-2.0,
revision `347d21543c58554f452fb8b4c9d4efa302fececf`, was evaluated for its layered
rocket plume and deterministic comparison workflow. No code or assets were copied.
The rocket plume was not selected as the car flame's visual target.

## Scope and checks

F6 > Appearance > Exhaust exposes Default, Compact and Aggressive starting points,
root/tail tints, length, width, brightness, density and motion controls. Edited
values show Custom. Default uses root RGB 80/160/255 and tail RGB 212/175/140.
All eight controls use release-relative percentages, defaulting to 100% in the
centre of a 0–200% range. Their approved authoring baselines are length 64,
width 62, brightness 150, density 137, flow 155, turbulence 45, upward curl 27
and motion response 200 in the earlier experimental scale. Resolution to those
units happens only when producing the effect. Settings store the new relative
percentages. Earlier experimental exhaust settings must be cleared rather than
interpreted as relative values. Reset exhaust restores all controls to 100% and
the baseline colours. Zero length, width, brightness or density hides the plume.
Changes use the garage's live appearance transaction without
respawning the car. Shape and motion use a shared `[exhaust_style]` settings
table, saved after a control edit is released. Switching cars and loading car
presets retain that shared style. Car presets store root/tail colour overrides;
older records with shape fields remain readable, but those fields do not replace
the shared style. F8 retains the global enhanced-exhaust switch.
These cosmetic settings do not change NOS power or original career purchases.
Preview flame animates the cosmetic effect while F6 freezes the world. Closing
F6 removes the preview; it never changes the physics NOS state or consumes NOS.

The tip has a damped response to heading changes, residual acceleration and bumps.
Forward acceleration also makes a small, bounded length change. Teleports reset
motion history. Heat rise, relative airflow and motion feed one displacement
vector, projected perpendicular to each pipe and bounded relative to flame length.
One quadratic centreline keeps the first 35% straight and bends the remaining
plume progressively. Damped angular velocity drives lag relative to each pipe's
jet direction. Sharp turns increase that lag; it responds promptly and settles
more gradually. Turn/bump displacement is capped at 55% of flame length, with
the combined bend capped at 65%. The nozzle remains fixed. Proxy bounds grow
without enlarging the visible flame.
Speed adds a smooth length stretch capped at 20% and reduces upward lift.
Side exits retain their original pipe direction at the root. Speed-dependent
airflow perpendicular to each jet sweeps the outer plume rearward. Heat rise
and this airflow share the same curve, rather than applying separate bend profiles.
Turn/bump response strength also controls this airflow bend.
The visual phase, activation envelope and damped motion update every render
frame using interpolated car poses and the world's scaled animation delta.
Speedbreaker therefore preserves slow motion without fixed-physics-tick stepping.
Ignition, NOS state and original particles remain on their original physics clock.
Neutral colours preserve the reference shading. Custom hues recolour the
emission density instead of filtering individual texture channels; blue,
purple and white therefore keep their flame body. Scene colour treatment still
applies to the result, just as it does to the rest of the car.

Resident collision is sampled along six centreline segments and four surrounding
paths per pipe. The first hit supplies a local clipping plane with a six-centimetre
soft fade; the confined flame broadens slightly near contact. Queries include
the installed blocking barriers and solid faces, skipping excluded scenery groups
and one-sided barrier backs. This is a local plane approximation, not a fluid solver
or multi-surface collision simulation; it depends on resident collision accuracy.
A small flickering additive glow projects onto the contacted surface through a
5x5 collision-sampled grid. Missing samples do not form triangles, reducing glow
across edges and gaps. The patch follows the flame palette and proximity. This
approximates light spill; it does not illuminate material properties, cast shadows,
leave scorch marks or require hardware ray tracing. Scene depth still occludes it.

This is an appearance experiment, not a physical combustion simulation. It adds
no new power, exhaust events, smoke, sparks, dynamic lights or heat distortion.
Existing rain distortion is unchanged. Bloom is not required to construct the
flame and its settings are not overridden by selecting this appearance.

## Transient backfire appearance

Enhanced shift flashes and lift-off pops have separate event-driven envelopes.
The installed shift eligibility and the existing sound-pop/console triggers remain
unchanged; the original sprite path retains its original timing. Enhanced flashes
have a fast attack, expansion, turbulent breakup and decay. Shift flashes are
larger and longer than individual lift-off pops. Variation is seeded once per
event and pipe, never regenerated per frame. At most three flashes overlap per
pipe, and no extra crackles, smoke or sparks are invented. Held NOS suppresses
new bursts. Existing NOS appearance and its release-relative baseline are unchanged.

The same licensed lookup and noise render each flash, with a warm luminous core,
orange edges and a small cooler root. Airflow and turn lag affect the short outer
plume; buoyant rise is weaker than for sustained NOS. Wall clipping and surface
glow are reused. Physical and presentation clocks keep separate event timelines
so Speedbreaker can show smooth, slowed flashes without advancing them twice.

F6 > Appearance > Backfire supplies independent root/tail colours, size,
brightness, duration and variation (100% defaults), plus one-shot shift and pop
previews. These controls persist in a shared `[backfire_style]` table, separate
from NOS colours and car presets. Preview uses wall-clock time while the garage
freezes physics and never fires a gameplay pop or consumes NOS. Runtime quality
and cost require driving feedback; a successful release build is not visual proof.

Synthetic checks cover marker direction, frame-rate-independent activation,
release/reset, the original fallback, and settings round-trip. The GPU regression
checks visibility and opaque depth clipping. The ignored installed-game check
`installed_enhanced_exhaust_drives_and_captures_original_and_reference_flames`
engages NOS through the real driving input and captures matched original/enhanced
rear and side views of an upgraded M3 GTR. `NFSMW_EFFECTS_CAPTURE_DIR` optionally
receives captures; `NFSMW_EXHAUST_SEQUENCE` adds a fixed-camera animated sequence.
`NFSMW_EFFECTS_MODS_DIR` optionally opens the install through the normal mod loader
and applies the selected profile's graphics, paint, sky, detail and world settings
at 1920x1080. Set `BLACKBOX_PROFILE_DIR` to an isolated profile for comparisons;
capture-setup.json records the loaded mod IDs and resolved graphics request.
Test success does not establish artistic quality, tunnel lighting, temporal
upscaler stability or a high-speed route performance result.
