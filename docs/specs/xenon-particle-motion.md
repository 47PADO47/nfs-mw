# Restored spark and wind particle motion

Research checkpoint 2026-10-09, written before the implementation revision.
The target is the Xenon Effects restoration, not verified Xbox 360 footage.
The base game's installed fuelcell attributes provide the tuning. No modded
install, texture pack or injected DLL is needed by the rewrite.

## References and verification

Reference: [xan1242/NFSMW_XenonEffects](https://github.com/xan1242/NFSMW_XenonEffects/tree/b5070080c75a7dfc555deca0a5e005311954dbfd),
Lovro Pleše, MIT. Read the emitter bridge, spark/contrail dispatch, spawn,
particle aging, bounce, and sprite construction. Embedded reconstructed
routines are reference only; no assembly or decompiled code is ported.

The spawn routine was executed in an isolated x86 emulator with controlled
uniform inputs and identity transforms. This verifies numeric conversions,
not live rendering or native collision queries. The source's spark hook forces
intensity to its configured spark intensity; it does not use contact strength
as a particle-rate multiplier. **[community source + emulated routine]**

## Emitter recipe

Read volume center/extent, velocity start/delta/inherit, gravity start/delta,
color, life/variance, particle count/variance, and length/height from each
supported runtime `fuelcell_emitter`. Reject missing or nonfinite fields and
out-of-budget values rather than silently substituting invented motion.

Use independent uniform values in [0,1) for each variable component:

- Local spawn position is center plus (uniform minus 0.5) times extent.
  Extent is the full width of the sampling interval, not a radius.
- Rotate velocity start into world space. Add componentwise inheritance times
  the incoming world velocity. Multiply each resulting world component by
  `1 + delta * (2 * uniform - 1)`. Variation is multiplicative, not an extra
  outgoing velocity along the collision normal.
- Gravity coefficient is start plus delta times `(2 * uniform - 1)`.
- Lifetime is `life * (1 - life_variance)`, without a random lifetime offset.
  Selected supported profiles have positive results and zero count variance.
- Count is authored count for intensity up to one. Above one it scales with
  intensity; the source subtracts `count * intensity * count_variance * 100`. Fractional
  positive counts round up. Budgets bound malformed or excessive requests.
- Length is start plus uniform times delta, clamped to [0,255] then truncated.
  Height is clamped to [0,255] then truncated. Length divided by 2048 is a
  **time offset**; height divided by 2048 is the full width in world units.
- At intensity exactly one, retain authored alpha. Otherwise use the truncated
  value `min(intensity * 42, 42) / 255`; this replaces authored alpha rather
  than multiplying it. Speed changes brightness, not density below intensity one.

For identity rotation, world velocity (60,0,0), and uniform values 0.1/0.5/0.9,
the first spark profile produces X velocity -40.8/-60/-79.2, Z velocity
-0.6/1/2.6, gravity -19.2/-12/-4.8, lifetime 0.95, count 30, alpha 255,
height 62 and length 255. Wind at intensity 0.1 produces count 15, alpha 4,
velocity (-18,0,-1), gravity -2, lifetime 0.1875 and lengths 205/225/245.
These are emulator measurements, with tuning supplied by the installed data.

The one-shot collision recipe rotates local Z toward the outgoing normal.
The continuing world-effect path instead rotates local Y toward the outgoing
render-space normal. For a horizontal wall this keeps local Z vertical, so
the spark start velocity and its variation can rise and fall above the contact.
The complete ordinary emitter hierarchy remains incomplete. Velocity
inheritance uses the owner's current linear velocity, scaled by the wrapper's
`InheritVelocity`; pre-response point velocity is used only for contact eligibility.
The native collision behavior also suppresses scrapes below the linkage's
minimum speed, before its intensity clamp. **[decomp]**
[Effects.cpp](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/Physics/Behaviors/Effects.cpp),
CC0, read as a reference only.
The effect bridge distinction comes from
[WorldConn.cpp](https://github.com/dbalatoni13/nfsmw/blob/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c/src/Speed/Indep/Src/World/WorldConn.cpp),
same CC0 revision, one-shot and continuing world-effect dispatch. **[decomp]**

Read-only inspection of the restoration's `MAIN` texture and selected emitter
UVs found an opaque atlas whose RGB supplies the glow mask. The supported
profiles select UV 0..0.25 on both axes. That region has average red energy
about 0.105 and concentrates light toward the segment end, rather than filling
the whole line. The rewrite uses a procedural narrow core and dim exponential tail with
comparable average energy, plus a separate subtle head glow. No atlas pixels
are shipped or embedded. **[verified texture measurement + host enhancement]**

## Movement and geometry

World Z is up. Position at age t is initial position plus velocity times t
plus Z times gravity times t squared. Instantaneous velocity is initial
velocity plus Z times twice gravity times t. Gravity is a parabola coefficient,
not an acceleration to multiply by another half.

The reference builds a segment between positions at t and t plus length/2048.
It constructs width along world Z. The rewrite may face width toward the camera
to keep grazing sparks legible; this is an intentional rendering improvement.
Opacity remains constant until expiry, as in the reference without optional fade.
The procedural mask remains a replacement for the mod's texture atlas.

Follow the authored interval for both sparks and wind. Comparison screenshots
show long grinding streaks in the restoration; an experimental 1/60-second cap
made the rewrite too point-like and is superseded by this checkpoint. Keep a
narrow core with a dim tail rather than shortening the underlying segment.

As an explicit host improvement, clip spark segments against the active car's
oriented root collision box at its interpolated render pose. Preserve the
unclipped segment's longitudinal mask coordinates on surviving pieces. Suppress
spark head glows whose center is inside that box. This prevents birth volume or
predicted streak geometry from visibly crossing the car's interior; it is not
native car-particle collision or a mesh-exact silhouette. Wind retains its
authored body volume and is not subject to this spark-only exclusion.

The reference reflects instantaneous velocity at a collision and scales the
whole result by byte elasticity/255. Defaults for the two spark profiles are
160 and 120. Wind particles bypass bounce. The host should sweep spark motion
against resident world faces and barriers, use the hit fraction for substep
motion, preserve remaining lifetime, and limit bounces to bound query cost.
Its collision implementation is independent of the reference's predicted
collision-time routine. Moving props and car-particle collisions remain out
of scope. Birth positions should stay on the outgoing side of the source
contact plane to avoid a wide emitter spawning inside its wall.

## Dispatch and trail placement

Choose one strongest supported contact per physics tick; do not multiply
effects by body-probe count. Impacts emit the authored burst. Sustained moving
scrapes dispatch at 60 Hz independent of rendering frame rate. Hit takes
priority when both channels occur. Stationary correction remains silent.
Use the authored 30:10 spark-profile ratio, not a 50:50 alternation.

Wind uses total velocity magnitude at threshold 44 m/s. Intensity rises from
0.1 to 0.75 between 44 and 88 m/s. The host uses a fixed 60 Hz dispatch for
repeatable presentation; the restoration allows configurable dispatch limits.
Use the full car body transform and authored volume around its origin. Do not
move the volume to a rear bumper or scale it by approximate body dimensions.
Motion and segment direction follow particle velocity, including negative
world velocity inheritance, rather than a fixed car-forward axis.

The host may spread births over the fixed dispatch interval to reduce a visible
single-frame burst. Emission position follows interpolated body motion through
that interval. This improves frame-rate stability without changing density.

## Limits and impact glow

The initial host limits are 2048 sparks and 256 wind particles. Overflow retires
the oldest particle. Disable/reset/camera behavior remains as documented in
[vehicle visual effects](vehicle-visual-effects.md).

An ordinary PC glow emitter is a separate channel from the restored line sparks.
The screenshot supplied by the user shows bright particles and a local bright
area at the contact; it does not alone distinguish a glow sprite, bloom or a
dynamic wall light. A short procedural contact flash and spark head glow may
be added as explicit host enhancements. A single sustained-scrape glow may
refresh while contact moves, then expire shortly after contact ends. Glows remain contact-local,
depth-tested, bounded, and must not claim to reproduce native dynamic lighting.
