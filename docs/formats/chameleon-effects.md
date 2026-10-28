# BearEffect: the particle effects of the Chameleon engine (NFS: Most Wanted 2012)

The resource kind `0x301` ("BearEffect") of `GLOBALEFFECTS.BNDL` (214 effects, 129 textures, 102 materials) is a small program
that spawns, moves and draws particles. This page is what is known of it; the reader is [`libs/chameleon-effects`](../../libs/chameleon-effects).

Evidence tags: **[exe]** the executable's code was read (`NFS13.exe`, decompiled); **[map]** the loader's copy from the file to
the runtime object was read, so the offset and the use of a field are known, the use less certainly than with [exe]; **[data]**
seen in the install's files; **[infer]** the layout of the block says it, nothing more. Nothing here is game data: no number
of the install is copied, only how to read them. The numbers of the three nitrous effects are in
[chameleon-nitrous.md](chameleon-nitrous.md).

## The program

An effect is a list of **emitters**. An emitter is a list of **tasks**; a task is an operation (an index into the executable's
operation table) and a parameter block. The operations run on a bucket of particles stored as separate arrays of x, y, z, w
(structure of arrays, four at a time with SSE), in the order the emitter's stage lists give: stage 0 is the update, 1 the
render, 2 and 3 the shadow map.

**The operation table** **[exe]** is at `0x010C2D48`: 78 records of 16 bytes: the name, the instantiate function, the execute
function (a stub that forwards to the real one) and a signature. An operation's id is its record's index. The names:
`EmitterConePlus` (0), `EmitterCollisionMesh`, `EmitterNull`, `EmitterTrail`, `EmitterVolume`, `EmitterWheel`,
`EmitterWheelContacts`, `EmitterWheelContactPlus`, `EmitterWheelTrack`, `EmitterWheelTracks`, `Initialise2dRotation` (10),
`Initialise3dRotation`, `InitialiseAgeAndLifetime` (12), `InitialiseAlpha`, `InitialiseAndSubmitLight` (14),
`InitialiseAndSubmitLightLocalSpace`, `InitialiseBakedLighting`, `InitialiseCollisionRadius`, `InitialiseRadius` (18),
`InitialiseBouncePlane`, `InitialisePosOffset`, `InitialisePosVelToWorldSpaceWithLocalTransform` (21),
`InitialiseVelocityInheritance` (22), `ParametricApplyPerParticleAlpha`, `ParametricEmitterVolume`,
`ParametricEmitterVolumeSubUV`, `ParametricPosVelToWorldSpace` (26), `ParametricPosVelToWorldSpaceWithLocalTransform` (27),
`ParametricSetColourGradient` (28), `ParametricSetSubUV` (29), `Render3dSprite` and `Render3dSpriteSubUV`,
`RenderCubeMapSprite`, `RenderLightBeam`, `RenderMesh`, `RenderMeshNoPaintColour`, `RenderSprite` (36),
`RenderSpriteMotionBlur`, `RenderSpriteShadowMapSubUV`, `RenderSpriteSubUV` (39), `RenderSpriteUVDistortion` (40),
`RenderSpriteVelocity`, `RenderTrail`, `SuperEmitter`, `Update2dRotation`, `Update3dRotation`,
`Update3dRotationBasedOnVelocity`, `UpdateAge`, `UpdateAndSubmitLight` (48), `UpdateAxis`, `UpdateAxisLocalSpace`,
`UpdateBindings`, `UpdateBindings_Wheel`, `UpdateOccluders`, `UpdateParametric2dRotationAlignToVelocity`,
`UpdateParametricColour`, `UpdateParametricFadeOverDistance`, `UpdateParametricLighting` (and `WithOccluders`,
`WithTunnels`), `UpdateParametricRadius` (60), `UpdatePosVel` (61), `UpdatePosVelWithBouncePlane`,
`UpdatePosVelWithVortex`, `UpdatePosVelWithWorldCollision`, `UpdateTrail`, `UpdateWheelBindings`, `RenderSpark`,
`UpdateBucketBoundingBox`, `FrustumCullBucket`, `FreezeOffscreen`, `InitialiseRandomSeed`, `UpdateParametricRandomTint`,
`PackVelocity`, `UnpackVelocity`, `InitialisePackedVelocity`, `EmitterSplatter`, `RenderSplatter` (77). The random numbers are
a four-lane xorshift generator kept per bucket **[exe]**.

## File layout **[data]**, **[map]**

| At | What |
|---|---|
| 0x00 | magic `0x269987A1` |
| 0x04 | u16 emitter count |
| 0x12 | u16 offset of the emitter table: one u16 offset per emitter |
| 0x20 | the effect's inputs: a u16 count (8), type bytes, then the names as C strings: `Transform`, `Velocity`, `ColourOverride`, `PreCrashVelocity`, `Intensity`, `ImpactClosingVelocity`, `IsEmitting`, `MovementSinceLastFrame` |

An emitter (offsets from its start): u16 at +4 the task table, u16 at +6 the stage table, u16 at +0x0C a flag word whose
second byte (+0x0D) is the **task count**. The task table is `n` u16 offsets of parameter blocks, `n` operation ids and `n`
second bytes. The stage table is four u16 offsets of stage lists (0 for none); a stage list is four header bytes (the sizes of
two groups of pairs, then two more) followed by `(task index, count)` pairs. A block runs to the next structure that starts
after it. Words such as `0xEB400000`, `0x000804D0` inside blocks are references to slots of the bucket's arrays
(`value & 0x3FFF0` the byte offset, bit 0 the second buffer): they say where a task reads or writes, not a number to play.

## The parameter blocks

Offsets are in the block. "Per effect parameter" `P[i]` is a float of the effect's parameter array that a block names by a
byte index; **what the array holds is not decoded** (see below).

### `EmitterConePlus` (0), 0x80 bytes **[exe]** for the use, **[map]** for the offsets

| At | Field |
|---|---|
| 0x00 f32 x3 | velocity scale: a particle leaves in a direction uniform on the sphere with the speed `random(0..1) * scale` per axis |
| 0x10, 0x14 | the window of the effect's time, in seconds, in which it spawns (`[0, FLT_MAX]`: as long as the effect lives) |
| 0x18, 0x1C | a range the emitter's squared distance to a reference position must lie in (`[0, 90000]`) |
| 0x20, 0x24 | the camera distance range the spawn rate fades out over (`[10, 150]` or `[15000, 15999]` in the nitrous effects) |
| 0x28..0x34 | two ranges in degrees, divided by 360 and shaped into a polynomial used in the azimuth of the direction; the data has `[0, 5]` twice on the flames and zeros elsewhere. **Use not decoded** |
| 0x38, 0x3C | the "shell": the particle starts at a random distance in this range, times `P[idx72]`, from the emitter, on its direction |
| 0x40, 0x44 | the emitter's own speed must lie in this range for it to spawn (`[0, FLT_MAX]`) |
| 0x48..0x5C | slot references |
| 0x60 f32 | spawn rate at rest, particles a second |
| 0x64 f32 | spawn rate for each m/s of the emitter's speed |
| 0x68 f32 | the spawner starts after a random delay of up to this many seconds (0 in the nitrous effects) |
| 0x6C u32 | burst: particles spawned at once when it starts (the spawn counter starts at this plus 0.9999; 0 on the sprites, 1 on the light) |
| 0x70..0x73, 0x74 | bytes: the indices into `P` of the rate scale, the carry-over scale, the shell scale and the speed scale; and a quality level that must be below the game's detail setting |

Spawn count per step **[exe]**: `(rate_per_speed * speed + rate) * dt * P[idx70] * fade(distance)`, added to an accumulator,
and `round(accumulator)` particles are born; the fraction stays. Spawning goes on while the effect's time is inside the window
or while the accumulator still holds a whole particle. So a flame "burst" is a window (0.5 s) of a rate (100 a second), not a
count.

### `InitialiseAgeAndLifetime` (12) **[exe]**

0x0C and 0x10: the shortest and longest life in seconds; a particle's life is uniform between them, times a scale that runs from
the floor at 0x14 (0.001) to 1 by the parameter whose index is the byte at 0x18.

### `InitialiseRadius` (18) **[infer]**, `UpdateParametricRadius` (60) **[exe]**

The starting radius is uniform between 0x08 and 0x0C (floor at 0x10). The radius over the life is the starting radius times a
factor that is three values joined by straight lines: the file's 0x18 at age 0, 0x1C at age `peak` (0x14) and 0x20 at age 1
(floor at 0x24). The loader turns those into the line `min(t, peak) * rise + base + max(t, peak) * fall` that the executor
evaluates **[exe]** (`rise = (v1 - v0) / peak`, `fall = (v2 - v1) / (1 - peak)`, `base = v0 - (v2 - v1) * peak / (1 - peak)`).
Reading the three file values as `rise`, `base` and `fall` is wrong and made the glow 5 times too wide in an earlier version of
this reader. The executor also multiplies by `floor + (1 - floor) * P[i]` for a byte index `i` at 0x28. The sprite's half width is
the radius: the quad's corners are at plus and minus one radius from the centre when the pivot (the two floats after the scale in
the render block, 0.5 and 0.5) is the middle **[exe]**.

### `UpdatePosVel` (61) **[exe]**

Each step the position gains `velocity * dt` and the velocity becomes `velocity * (1 - drag)` **at every step, whatever its
length or `dt`**, plus `drag * wind_blend` towards the wind, plus `lift * dt` upwards, plus a random kick of up to
`turbulence * dt` on each axis. At 0x0C lift, 0x10 drag, 0x14 wind blend, 0x18 turbulence; bytes at 0x1C and 0x1D index `P`
for the scales of the lift and the turbulence. A drag of 0.75 leaves a quarter of the velocity after each step. **Possibly wrong**: the bounce-plane variant of this operation (`UpdatePosVelWithBouncePlane`, [chameleon-collision-effects.md](chameleon-collision-effects.md#updateposvelwithbounceplane-62-exe-for-the-use-map-for-the-offsets)) computes `pow(1 - drag, dt)` in the disassembly, i.e. a share lost every second; this operation was not re-read for it.

### `ParametricSetColourGradient` (28) **[exe]**

Five colours (red, green, blue, brightness) at 0x20, 0x30, 0x40, 0x50, 0x60; the weights `a` at 0x70 and `b` at 0x80. The colour
at age fraction `t` is `c0 + sum(w[i] * c[i + 1])` with `w[i] = clamp(frac(t * scale) * a[i] + b[i])` (scale at 0x9C); red, green
and blue are multiplied by the brightness, which goes above 1. The alpha is a piecewise linear curve through five points: heights
as bytes at 0x90..0x94, positions as bytes at 0x95..0x99 (value / 255), the position scale at 0xA0. **The curves start over at
`t = 1`, where a particle is dead**: read the end just before it. A byte at 0x9B says the effect's `ColourOverride` input
replaces the colour.

### `InitialiseVelocityInheritance` (22), `Initialise2dRotation` (10) **[exe]**

Inheritance: at 0x00 and 0x04 the least and the greatest share of the emitter's velocity a particle takes at birth. Rotation
(the loader's copy was read): the starting angle is uniform between 0x00 and 0x04, in turns, and the angular speed uniform
between minus and plus the value at 0x14, in turns a second.

### `ParametricSetSubUV` (29) **[exe]**

The cell value of a flipbook is `age * recip_lifetime * scale` (the scale is the float at 0x0C, 1 here) when the byte at 0x10
is 1, so the sheet plays once over the particle's life; otherwise it is the first slot times the scale. A byte at 0x11 adds a
random offset per particle. The renderer turns the value into a cell of the sheet; the 5 x 5 grid of the flame sheet is read off
the picture.

### `UpdateBindings` (51) and the parameter array **[exe]**, contents not evaluated (the bindings are read since: see the collision effects below)

Every emitter starts with this task. It fills the effect's parameter array `P`: for each of four bindings it takes a source
(a time, the length of the effect's `Velocity` input or of a position difference, a scalar input, a constant, ...; the selector
is a byte), runs it through a response curve of 10-byte points, adds a bias and writes `P[1..4]`; a second routine
combines those into `P[5..8]` (product, minimum, maximum, mean, sum, ...). `P[0]` is not written here. The spawn rate of the
start flames is scaled by `P[1]`, that of the start smoke puffs by `P[3]`, of the pop smoke by `P[4]`, of the glow and its
lifetime and radius by `P[0]`. The curves of the three nitrous effects were dumped but not evaluated, so the scales are assumed
1, and the puffs scaled by `P[3]` and `P[4]` are left out (the inputs of those indices are the crash inputs).

### The space of the particles **[exe]**, **[map]**

`InitialisePosVelToWorldSpaceWithLocalTransform` puts a particle into the world once, at birth, from the emitter's place:
afterwards it is a world particle (the nitrous glow). `ParametricPosVelToWorldSpace` and `...WithLocalTransform` put the particles
into the world **every frame** from the emitter's current place: they ride with it (the flames, the dust and the smoke puffs).

### `RenderSprite` (36), `RenderSpriteSubUV` (39), `RenderSpriteUVDistortion` (40)

A camera-facing sprite with the effect's material (the first texture is the picture). The sub-picture variant takes the
cell from `ParametricSetSubUV` (29): its rule is not decoded; the flame sheet is 5 x 5 cells, read off the picture. The
distortion variant refracts what is behind it (heat haze). The blend of a material is a shader technique and is not read.

## Wheel effects (tyre smoke)

The surfaces of the road (`HAWAII/SURFACELIST.BNDL`) name effects for a tyre. A `SurfaceType` object (kind `0x15`,
type `Genesys.Gen.SurfaceType`) has three effect references, at +0x0C, +0x14 and +0x1C **[data]**. +0x0C differs by surface (a
grey-blue smoke for the tarmac types, brown dust for dirt, sand and gravel), +0x14 is the same effect for every road surface (it
is named by about thirty types) and +0x1C is one of two spark effects (a dry and a wet one: orange sparks and a small bright
sprite). Field names are hashed and are not in the executable's strings, so which field is the tyre smoke is **[infer]** from
this: the effect at +0x14 is the one that has seven smoke emitters, is on every road surface and does not change with the
surface. The `SurfaceVfx` objects (+0x118 to +0x148) name the effects of impacts and scrapes, one of which is the
`EmitterWheelContacts` effect `0x7C950` (one sprite emitter, 0.4 to 0.5 s); they are not used. The mod finds the effect by the
count of references (`tire_smoke::find_effect`); no id is written in its code.

The wheel operations are `EmitterWheel` (5), `EmitterWheelContacts` (6) and `UpdateBindings_Wheel` (52). The reader is
[`libs/chameleon-effects/src/wheel.rs`](../../libs/chameleon-effects/src/wheel.rs) (`Emitter::wheel`, `Emitter::bounce_drag`).

### `EmitterWheel` (5), 0x7C bytes **[exe]** for the use, **[map]** for the offsets

The execute function (`0x0044FB20`) was decompiled. The runtime copy of the block has four floats added in front and one
accumulator after the rate; counting the nine slot references that are in both gives the offsets of the file:

| At | Field |
|---|---|
| 0x00, 0x04 | the squared distance range of the wheel from a reference position in which it spawns (`[0, FLT_MAX]`) |
| 0x08, 0x0C | the camera distance the rate fades out over, near and far (`[10, 150]`; `[10, 65]` or `[0, 65]` on the smaller emitters); the shape of the fade is not decoded |
| 0x10, 0x14 | the speed of a new particle, a range in m/s (`[0.125, 0.5]`); its direction is built from the angle fields below and the road, not decoded |
| 0x28, 0x2C | the angle around the axle, degrees (`[0, 360]`: all round the tyre; `[145, 160]` and `[-45, -45]` for the emitters that spray to one side) |
| 0x30, 0x34 | where on the tyre a particle starts, as a share of the wheel radius (`[0.9, 1.2]`) |
| 0x38, 0x3C | and across the tread as a share of half its width (`[-1, 1]` is all of it) |
| 0x40, 0x44 | the height above the road, a range in metres, times an effect parameter (`[0.01, 0.04]`; `[-0.2, -0.15]` for the grit) |
| 0x6C, 0x70, 0x74 | rates: `w * (rate + rate_per_speed * speed)` particles a second, where `rate` is 0x74 and `rate_per_speed` is 0x70, or 0x6C when 0x70 is 0 (the first emitter has 18 and 2) |
| 0x78..0x7B | bytes: the index into the effect's parameter array `P` of the rate scale (1, 5, 6 or 7), of the speed scale, of the height scale, and a quality gate |

`w` is a float of the wheel's binding record (it is read at +0x64 of it) and is not decoded; a tyre that smokes more has a larger
`w`. `speed` is the larger of the car's speed and the speed of the tyre's contact patch (so a spinning tyre on the spot smokes).
The rate is also multiplied by `P[rate index]` and by the camera fade. `UpdateBindings_Wheel` (116 bytes) fills `P` for the
emitter from four sources chosen by a byte each (inputs of the wheel's record, the ratio of its first input to a constant, a
flag, or a computed value) run through response curves; the curves are not evaluated, so `P` is taken as 1 (and the rewrite
scales the rates down, see below). The seven emitters use different `P` (1, 7, 6, 5, 5, 5, 6): they are the smoke of different
slip regimes, and the rewrite plays them all at the same time.

### The seven emitters of the road smoke **[data]**

All are `RenderSpriteSubUV` sprites with a random cell of a sheet (`ParametricSetSubUV` with its byte at +0x10 zero and the
random-offset byte at +0x11 one: the cell is picked at birth and kept) and the gradient's `ColourOverride` byte set (the colour
the effect is given replaces the gradient's colour; the alpha is the effect's). The pictures are BC3: `512 x 512`, four puffs of
grey smoke in a 2 x 2 sheet (emitters 0 to 2), and `512 x 256`, eight puffs in a 4 x 2 sheet (emitters 3 to 6; the sheets are
read off the pictures).

| # | Rate a second (+ per m/s) | Life (s) | Radius (m), times the curve | Notes |
|---|---|---|---|---|
| 0 | 18 (+2) | 0.75 to 1.0 | 1.0 to 1.25; 0.25 at birth, 0.75 at 0.15 of the life, 1.0 at the end | alpha 0 to 0.31 at 0.09, 0.22 at 0.48, 0 at the end; takes the car's velocity; spins 0.25 turns a second |
| 1 | 10 | 3 to 4 | 1.0 to 2.5; 0.2, 0.4 at 0.2, 0.8 | alpha up to 0.87; takes 0.8 to 1.0 of the velocity |
| 2 | 10 (+1) | 4 to 6 | 1.5 to 3.0; 0.2, 0.75 at 0.3, 2.32 | alpha up to 0.89; takes 0.65 to 0.85 of the velocity; a faint cloud left behind |
| 3 | 32 (+4) | 1.0 to 1.75 | 0.3 to 0.6; 0.25, 0.5 at 0.125, 1.5 | starts on one side of the tyre, 0.6 to 0.8 of the half width |
| 4 | 480 | 0.125 to 0.16 | 0.1 to 0.125 | grit: the only one with turbulence (5); left out of the rewrite |
| 5 | 48 (+4) | 0.8 to 1.25 | 0.85 to 1.25; 0.35, 0.65 at 0.25, 1.25 | speed 3 m/s, to the side |
| 6 | 48 (+5) | 0.8 to 1.25 | the same as 5 | the same as 5 under another `P` |

The gradient colour is blue-grey, `(0.456, 0.503, 0.604)` at a brightness of 2.0 falling to about `(0.33, 0.37, 0.45)` at 0.5
(`ParametricSetColourGradient`; a brightness above 1 clamps), which the override replaces. A sprite is drawn `2 x radius x curve`
wide. The motion is `UpdatePosVelWithBouncePlane` (62), a 0x40 byte block: the particle bounces on the road plane, and the
number at +0x34 (0.025 for emitter 0, 0.125 for 1, 0.25 for 2, 0.5 for 5) is, read as the per-step drag of `UpdatePosVel`, what
leaves the clouds where the tyre made them **[infer]**; the others (a vertical acceleration at +0x2C, 0.8 at +0x30) are not read.

### In the rewrite (`mods/nfsmw-2012-content/src/tire_smoke.rs`)

The mod writes a `TIRESMOKE/MW2012/profile.toml` ([effect-variants.md](../modding/effect-variants.md)) with a layer for each of
emitters 0, 1, 2, 3 and 5 (6 repeats 5; 4 is grit): the picture, the blend (alpha), `life`, the width over the life
(`2 x mean radius x curve`, at most 8 m), the colour and alpha at the keys, `inherit`, `damping` (the bounce plane's number at
+0x34), the angle range and spin of the rotation, the spawn rate and the speed range. The 4 x 2 sheet is written as a 4 x 4 sheet
with its two rows again, because the profile's sheets are square. **Not decoded or not carried**: `P` and `w` (the host's own
wheel smoke, the slip of the vehicle library, stands for `w`), the angles around the tyre and the start on the tread (a puff starts
at the contact patch, on a cone of 90 degrees), the camera fade, the colour override (the layers carry the gradient's colour) and
the sprite draw scale (taken as 1). **Chosen, not decoded**: all rates are multiplied by 0.25 (the seven emitters at their full
rates fill the screen; the file scales each by an unevaluated `P`), a rise of 0.25 m/s2, the 8 m width cap, and leaving out the
grit.

### Collision effects: `EmitterCollisionMesh` (1), the bounce plane (19, 62), `RenderSpriteVelocity` (41), the bindings (51)

The spawner that throws sparks and dust from the collision mesh of the object that was hit, the bounce, the spark streak and the
four bindings that turn the effect's inputs into `P[1..4]` (so the response curves are now known: `P[n] = curve(source) + bias`)
are decoded in [chameleon-collision-effects.md](chameleon-collision-effects.md); the readers are `libs/chameleon-effects/src/collision.rs`.

## Calibration against the screenshots of the game

Two screenshots of the 2012 game (the burning glow at the pipes of the 918 Spyder, and the start burst of orange fire at the
pipes) were compared with renders of the converted effects (`mods/nfsmw-2012-content/src/vfx_profile.rs`). What had to differ
from the decoded numbers is listed there as constants and is **not decoded**: the sprite draw scale (a glow 1.0 of the width
the radius gives, a flame 2.0), the rate term per speed (read as the movement of one frame, a 60th), and that the glow rides
with the car instead of being left behind (the file says: world particles with 0.8 to 0.9 of the car's velocity and a drag of
0.75 a step, which would be a trail of large sprites that neither screenshot shows).

A render with the glow left behind as the file says was a wide blue wash over the road and the car, so the glow still rides. To keep it
like the references (a compact blue disc with a bright centre in each pipe, a blue plume out of a side exit) the rewrite also caps
its width at 0.7 m (the width the file gives it at the peak of its alpha; it grows it to 5.6 m by the end of its life, when in the
2012 game it is a faint wash left behind, which a first render with a 1.2 m cap showed as far too big: the burning glow of the 2012
game is small), gives it a small lightened core sprite (the 2012 bloom), and lets the flames leave the pipe along its axis at a few
metres a second and the glow at 1 m/s (the file has them at the pipe's origin, which buries them in the body of a side exit and
glues them to the pipe's end). None of the four is decoded.

## Decoded and not

Decoded with evidence: the structure, the operation table, the spawn rate and its window, lifetime, the radius and its curve,
the motion (lift, per-step drag), the colour gradient and the alpha curve, the inheritance share, the rotation, the flipbook
value, the space of the particles, how the parameter array is made. Written into the profile of the 2012 nitrous variant by
[`vfx_profile.rs`](../../mods/nfsmw-2012-content/src/vfx_profile.rs).

Not decoded: the response curves of the bindings (so `P`); the azimuth ranges of the cone; the shell scale; wind and
turbulence; the blends; the light operations (`InitialiseAndSubmitLight`: radius 0.5, no picture; not played); the distance
fade and the quality gate; the renderer's other parameters (the first two floats after the slots, the brightness multiplier
1.2247 of the flames).
