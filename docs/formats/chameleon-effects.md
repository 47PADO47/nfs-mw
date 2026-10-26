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

The starting radius is uniform between 0x08 and 0x0C (floor at 0x10). The radius over the life is the starting radius times
`lo * rise + base + hi * fall` with `t` the age over the life, `lo = min(t, peak)`, `hi = max(t, peak)`; peak at 0x14, base at
0x18, rise at 0x1C, fall at 0x20, floor at 0x24. The sprite's half width is the radius **[infer]** (the renderer's draw scale
is not decoded).

### `UpdatePosVel` (61) **[exe]**

Each step the position gains `velocity * dt` and the velocity becomes `velocity * (1 - drag)` **at every step, whatever its
length or `dt`**, plus `drag * wind_blend` towards the wind, plus `lift * dt` upwards, plus a random kick of up to
`turbulence * dt` on each axis. At 0x0C lift, 0x10 drag, 0x14 wind blend, 0x18 turbulence; bytes at 0x1C and 0x1D index `P`
for the scales of the lift and the turbulence. A drag of 0.75 leaves a quarter of the velocity after each step.

### `ParametricSetColourGradient` (28) **[exe]**

Five colours (red, green, blue, brightness) at 0x20, 0x30, 0x40, 0x50, 0x60; the weights `a` at 0x70 and `b` at 0x80. The colour
at age fraction `t` is `c0 + sum(w[i] * c[i + 1])` with `w[i] = clamp(frac(t * scale) * a[i] + b[i])` (scale at 0x9C); red, green
and blue are multiplied by the brightness, which goes above 1. The alpha is a piecewise linear curve through five points: heights
as bytes at 0x90..0x94, positions as bytes at 0x95..0x99 (value / 255), the position scale at 0xA0. **The curves start over at
`t = 1`, where a particle is dead**: read the end just before it. A byte at 0x9B says the effect's `ColourOverride` input
replaces the colour.

### `InitialiseVelocityInheritance` (22), `Initialise2dRotation` (10) **[infer]**

Inheritance: at 0x00 and 0x04 the least and the greatest share of the emitter's velocity a particle takes at birth. Rotation:
the starting rotation is uniform between 0x00 and 0x04, in turns (a span of 1 is a full circle **[infer]**), and 0x14 is a spin
whose unit is not decoded.

### The space of the particles **[exe]**, **[map]**

`InitialisePosVelToWorldSpaceWithLocalTransform` puts a particle into the world once, at birth, from the emitter's place:
afterwards it is a world particle (the nitrous glow). `ParametricPosVelToWorldSpace` and `...WithLocalTransform` put the particles
into the world **every frame** from the emitter's current place: they ride with it (the flames, the dust and the smoke puffs).

### `RenderSprite` (36), `RenderSpriteSubUV` (39), `RenderSpriteUVDistortion` (40)

A camera-facing sprite with the effect's material (the first texture is the picture). The sub-picture variant takes the
cell from `ParametricSetSubUV` (29): its rule is not decoded; the flame sheet is 5 x 5 cells, read off the picture. The
distortion variant refracts what is behind it (heat haze). The blend of a material is a shader technique and is not read.

## Decoded and not

Decoded with evidence: the structure, the operation table, the spawn rate and its window, the speed and rate relation, lifetime,
the radius and its curve, the motion (lift, per-step drag), the colour gradient and the alpha curve, the inheritance share, the
space of the particles. Written into the profile of the 2012 nitrous variant by
[`vfx_profile.rs`](../../mods/nfsmw-2012-content/src/vfx_profile.rs).

Not decoded: the effect's parameter array `P` (the nitrous effects name the indices 0, 1, 3 and 4; the executable fills it
from the eight inputs in a way not read, so **the rate, shell and lift scales are taken as 1**); the azimuth ranges of the cone;
the shell scale; wind, turbulence and spin; how the flipbook picks its cell; the blends; the start delay and burst on sprites
(zero here); the light operations (`InitialiseAndSubmitLight`: radius 0.5, no picture; not played); the distance fade and the
quality gate; the sprite draw scale. These are what the numbers in a profile cannot be checked against without seeing the 2012 game.
