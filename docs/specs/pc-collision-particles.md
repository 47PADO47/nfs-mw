# Original PC collision particles

Written before implementation on 2026-10-09. This specifies the ordinary PC
particle path separately from the experimental Xenon restoration.

## Evidence

- [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw/tree/1f2cdd7996791c81a580b3f7b36b44d4f9f6719c),
  CC0, read-only reference: `EmitterSystem.cpp`, generated `emitterdata.h`,
  `WorldConn.cpp` and `bMatrix.cpp`. No source is copied.
- **[verified]** The ordinary hit group in base-game attributes has two sprite
  spark emitters and a glow. The scrape group additionally references a four-frame
  animated spark sprite. These reference textures in `GLOBAL/INGAMEB.BUN`.
- **[verified]** Read-only inspection of the original PC executable corroborates
  rotation of camera-facing quads, uniform particle size and the ordinary spawn
  path. Runtime data supplies tuning; no game bytes or extracted images are shipped.

## Selection

Retain the per-car hit/scrape surface links and their inherited surface lookup.
Resolve each ordinary `emittergroup.Emitters` entry directly to `emitterdata`;
do not traverse `XenonEffect`. Explicit wood/stone effects must not become metal
sparks. Missing or invalid profiles must not silently select experimental streaks.
Normalize input between the linkage minimum and maximum, then evaluate the
wrapper's four-term intensity polynomial and clamp the result to zero through one.
Hit input is collision impulse delta-v; scrape input is tangential contact speed.
The host supplies its existing collision events, not a second collision simulation.

## Timing and counts

Each emitter maintains its own fractional spawn accumulation. Its effective rate
is `NumParticles * intensity * (1 - NumParticlesVariance)` per second. PC variance
reduces this rate; it is not the Xenon percentage-byte rule. Whole births are
consumed each update; sub-unit accumulation releases one after exceeding one.
Start delay, on/off intervals and additive interval variance are emitter-local.
A forced hit group is one-shot. With no on interval it uses the reduced authored
lifetime as its spawn interval; with an on interval it emits until that interval
expires. Continuous emitters without an off interval remain on. Finishing a
contact stops new births, but already released particles finish their lives.
Keep the native clock's boundary handling: the first interval starts with one
step already consumed; delayed starts pass their negative remainder to spawning;
the final on-cycle dispatch uses the step plus the previous remaining interval.
This intentionally retains the original's frame-dependent count truncation.
Pause freezes the layer. Reset, disable, car replacement and mode changes clear
both emitter state and particles. The PC path has a 1024-particle budget and drops
new births at capacity. It adds neither extra streaks nor procedural contact glow.

## Birth and movement

Sample a uniform box around `VolumeCenter`, using `VolumeExtent` as full widths,
then transform to the contact frame. Speed, life and motion inheritance sample
from `value * (1 - variance)` through `value`; inheritance is clamped to [0,1].
Speed-driven particles use the contact direction with independently sampled
spread rotations. Disc emitters sample azimuth and half-spread tilt. Vector-driven
particles use the authored start/delta vectors; `EliminateUnnecessaryRandomness`
selects one-sided start-plus-uniform-delta sampling rather than centered deltas.
The former vector path uses world vectors, while the centered velocity path uses
the emitter basis. Non-live motion inheritance adds owner velocity at birth,
including the effect wrapper's inheritance multiplier.

Ordinary particles have no Xenon trajectory segments or world bounces. Each step
first applies speed-proportional drag, clamped to prevent reversal, then gravity
or acceleration (nonzero gravity takes precedence), then advances position using
the new velocity. Remaining life uses integer 1/1024-second ticks. Color/size age
is `1 - remaining_ticks / (authored_life * 1024)`, not randomized birth lifetime.
The PC executable stores velocity and acceleration as floats; do not introduce
the compressed vectors found in the console reference implementation.

## Color, size, rotation and UVs

Four color/size/relative-angle controls are sampled by the unique cubic through
the four authored `KeyPositions / 3`. This is algebraically the polynomial formed
by the native basis construction; it can be evaluated with Lagrange weights.
Packed colors are RGBA, high byte first. Clamp/truncate output channels to bytes.
Size is full width; the native size coefficient is half width. Kill at/below the
authored alpha threshold unless `NoKillAtAlpha` is set. Initial rotation samples
the centered angle range; rotation variance controls signed relative rotation.

`ParticleTextureRecord` is eight bytes: texture-name hash then texture index.
`ParticleAnimationInfo` is eight bytes: square-grid size, FPS byte and random-start
flag. Grid zero is a full texture. Animated UVs select a grid cell and wrap the
16-bit animation phase; do not replace the animated contact sprite with a radial mask.
The native animation wrap tests the updated phase plus another increment and
subtracts 65535 before narrowing, which differs slightly from ordinary modulo.
Camera-facing quads rotate about their viewing normal. Authored axis constraints
select fixed XY/XZ/YZ planes or the camera basis. Use the runtime texture blend
mode: base sparks/glow/animated contact sprite use source-alpha additive light,
depth testing, no depth writes. Ordinary smoke/debris use their own blend modes.
Do not add the experimental car-box exclusion or emitter offset to PC geometry.

## Presentation boundary

The menu offers Off, Original PC and Restored (Experimental). Configuration keeps
the enable switch and a separate style, defaulting the style to Original PC.
Speed Trails is explicitly experimental and remains independently off by default.
The host runs particle updates with its fixed 60 Hz simulation. This specifies
the non-live ordinary collision profiles, not a complete generic particle engine.
The original PC mode reproduces these profiles against the installed assets;
it cannot promise identical final pixels while the port's lighting, postprocessing,
camera and collision solver differ. Compare with an unmodified install for stock
parity, and do not equate the Xenon-effects mod with verified Xbox output.
