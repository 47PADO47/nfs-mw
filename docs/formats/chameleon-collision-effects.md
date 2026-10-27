# Chameleon collision effects: the sparks of NFS: Most Wanted (2012)

What the 2012 game's data says about the particles it plays when a car hits something or slides along it: which effects they
are, how they spawn, move and look, and what scales them. They are `BearEffect` resources (kind `0x301`, see
[chameleon-effects.md](chameleon-effects.md)) of `GLOBALEFFECTS.BNDL`, read by `libs/chameleon-effects` (`collision.rs`) and
turned into the `SPARKS` and `XENON` effect variants by `mods/nfsmw-2012-content` (`spark_assets.rs`,
[the profile spec](../modding/effect-variants-sparks-xenon.md)).

Evidence tags as in the other pages of this folder: **[exe]** the executable's code was read (`NFS13.exe`, decompiled);
**[map]** the loader's copy from the file to the runtime object was read, so the offset of a field is known and its use less
certainly; **[data]** seen in the install's files; **[infer]** the layout or a number says it, nothing more. Nothing here is game
data: numbers are listed so a reader can check the converter, the install holds them.

## Where the effects are named **[data]**

`GLOBALEFFECTS.BNDL` has a Genesys object `Genesys.Gen.VfxGlobals` (`0x24F12`) that holds handles to the rest of the visual
effect data: `VfxDebug`, two `Corona` sets, `VfxMisc`, and **two `Genesys.Gen.VfxVehicleCollisions` objects** (`0x250A5`,
`0x250A6`). A `VfxVehicleCollisions` object is 36 handles to effects, laid out as two plain handles (offsets `0x0C`, `0x14`),
then 16 handles (`0x1C` to `0x94`), two plain handles (`0x9C`, `0xA4`) and 16 more (`0xAC` to `0x124`); the first two are empty
effects (no emitters) in both objects. The 16 are two runs of 8 that repeat each other in both objects. The field names are
hashed, and the code that indexes the runs was not found, so **which of the 8 is picked by what is not known** **[unconfirmed]**
(a surface class is the obvious guess: the run is `A B B C C C C C` in `0x250A5` and `D E E E D D D E` in `0x250A6`).

| Object | Its effects | What the effect is |
|---|---|---|
| `0x250A5` | `0x14021C` (20 of its 32 handles), `0x1402F0` (8), `0x117ECA` (4) | an **impact**: a burst, once |
| `0x250A6` | `0x140279`, `0x07BCD0` (16 each) | a **scrape**: a rate, for as long as the effect lives |

The mod takes the effect each object lists most, ties going to the first handle: `0x14021C` for impacts, `0x140279` for scrapes.
The other effects of each object differ in details only (`0x1402F0` and `0x117ECA` add ground sprites and debris meshes;
`0x07BCD0` has a wider contact patch and a slower second emitter). Also in `VfxGlobals`: a `HangingBumperSparks` structure
(four numbers, 0.1, 0.8, 0.03 and 0.4, and two small integers) and a `VehicleCollisionSettings` structure (0.01, 5 and 0.2); their
roles are not read. `VfxMisc` names `0x24F44` (`MotionBlur` sparks at a rate of 50 plus 50 for each m/s, life 0.8 to 1.2 s, lift
-5, drag 0.75: **[infer]** the bumper that hangs and drags) and `0x24F66` (wreck debris with the same sparks); `VfxWheelEffects`
names `0xAF154` (a burst of 128 rim sparks, restitution 0.8 to 0.9, gravity -12) and `0xAF168`. These are not converted.

The inputs of the effects are ten, in this order (`collision::input_names`): `Transform`, `Velocity`, `ColourOverride`,
`VehicleParams`, `PreCrashVelocity`, `CollisionMeshParams`, `IsEmitting`, `MovementSinceLastFrame`, `ImpactClosingVelocity`,
`Intensity` **[data]**. The executable fills them for the effect it plays; the game code that decides *when* it plays one (the
thresholds of impact and scrape) was not found, so the host keeps its own triggers.

## The effect of an impact (`0x14021C`) **[data]**

Seven emitters. Everything spawns from `EmitterCollisionMesh`; nothing is spawned at a point.

| Emitter | Draws | Count / m² | Life (s) | Look |
|---|---|---|---|---|
| 0 | flipbook sprite, 1 x 1 cell | 64 | 0.25-0.35 | white, brightness 12, 0.5 m radius shrinking to 0.4 and fading: the flash |
| 1 | `RenderSpriteVelocity` | 200 | 0.25-0.85 | orange (1, 0.407, 0.153), brightness 12 falling to 5; a jet within 10° of the surface normal, speed 4-8 m/s |
| 2 | `RenderSpriteVelocity` | 200 | 0.25-0.85 | the same orange at brightness 12 to 9.5; a spray over the whole hemisphere, 6-10 m/s, segment 0.5 s long |
| 3 | `RenderSpriteVelocity` | 200 | 0.5-0.85 | dark red (1, 0.238, 0.127), brightness 8 falling to 2; embers, 2-12 m/s, restitution 0.2-0.5, turbulence 6 |
| 4 | mesh | 70 | 7-10 | debris, with a world collision (not converted) |
| 5 | flipbook sprite, 2 x 2 cells | 128 | 0.5-3 | grey (0.376), 2.5-3.5 m radius growing to 1.85 times, alpha 0.77 at the peak: dust |
| 6 | mesh | 200 | 0.5-1 | small debris, lift -3.26 (not converted) |

A spark is a segment `radius` wide (a radius of 0.015 to 0.025 m, times the radius curve: 2.0 at birth, 0.5 at 25 % of the life,
0.125 at the end), whose colour and opacity follow the gradient (`decode::Gradient`) and whose brightness goes well above 1: the
2012 game blooms it. The mod keeps the hue and the brightness apart.

## The effect of a scrape (`0x140279`) **[data]**

Six emitters, a rate of 2000 particles a second and square metre for the whole life of the effect: emitter 0 is a control emitter
that draws nothing (a `UpdateBucketBoundingBox` only; its use is not read), 1 a short-lived dot (life 0.125-0.2 s, speed 0-0.5 m/s,
colour brightness 6) whose streak block has an opacity of 0 and a brightness floor of 1 (**[infer]** a point glow, not a streak),
2 and 3 long-lived streaks (1-2.5 s, speed 1-4 m/s, brightness 12, restitution 0.2-0.5, friction 0.3-0.45, segment 1.0 s), 4 the
flash (life 0.25-0.45 s) and 5 a mesh (not converted). The patch of the mesh used is 0.25 m (`0x07BCD0`: 0.4 to 0.5 m).

## Operations

### `EmitterCollisionMesh` (1) **[exe]** for the use, **[map]** for the offsets

A block of 100 bytes. The spawner walks the triangles of the collision mesh of the object that was hit, keeps those within `radius`
(+0x48, times `P[params[0]]`) of the effect's position, less than `height` (+0x4C) above the contact plane and with
`(dot(normal, up) + 1) / 2 > facing` (+0x50), and gives each `area * (burst * P[b] + rate * dt * P[r]) * fade(camera distance)`
particles: **the counts are per square metre of the mesh**. A particle starts at a random point of the triangle and leaves at
`speed.0 .. speed.1` metres a second times `P[params[3]]`, at an angle from the normal drawn from the first range of degrees.

| At | Field |
|---|---|
| 0x00, 0x04 | speed range, m/s |
| 0x08..0x14 | two ranges in degrees (`[0, 10]` twice on the jets, `[0, 90]` twice on the sprays, `[90, 90]` and `[70, 110]` on debris); the first is the tilt from the surface normal, the second read as a roll **[infer]** |
| 0x18, 0x1C | window of the effect's time for `rate` (`[0, 0]` for an impact, `[0, FLT_MAX]` for a scrape) |
| 0x20, 0x24 | camera distance range the count fades out over (10 to 100 m; the dust 20 to 50) |
| 0x28..0x44 | slot references (position, the mesh parameters input, output arrays) |
| 0x48, 0x4C, 0x50 | radius 0.5 m, height 0.5 m, facing 0.35 (scrape: 0.25, 0.25, 0.5; ground sprites 3, 0.4, 0.3) |
| 0x54, 0x58 | `rate` and `burst`, u32 (the first is spent as a rate times `dt`, the second once and then reset to 0) |
| 0x5C..0x61 | bytes: indices into `P` of the radius scale, the rate scale, the burst scale, the speed scale, the angle scale, and a quality level below the detail setting |

In the impact effect the radius and rate scales are `P[0]` and the burst and speed scales `P[3]`; in the scrape effect the rate
scale is `P[1]`.

### `UpdatePosVelWithBouncePlane` (62) **[exe]** for the use, **[map]** for the offsets

| At | Field |
|---|---|
| 0x1C, 0x20 | restitution: a random share of the normal speed kept at a bounce (jets 0.8-0.9, embers 0.2-0.5, dust 0.1-0.15) |
| 0x24, 0x28 | friction: a random share of the tangent speed lost at a bounce (0.01-0.05; embers 0.3-0.45) |
| 0x2C | gravity, m/s², vertical (-9.8; dust -1; rim sparks -12) |
| 0x30 | drag (0.4; dust 0.9) |
| 0x34, 0x38 | wind blend and turbulence (jets 0; embers 6) |

A particle that falls through the contact plane (the plane of the triangle it was born on) is put back on it and bounced; a bounce
too weak to lift it for one step becomes a slide. **Drag**: the executable computes `pow(1 - drag, dt)` (a call into the C library
with `1 - drag` and the step as arguments, then multiplies the velocity by the result) **[infer]**, so the drag is **a share of
the velocity lost every second**: 0.4 keeps 0.6 of it after a second. The page on [the operations](chameleon-effects.md) reads
the same field of `UpdatePosVel` as a loss at every step; the call there is the same one, so that reading is probably wrong too
(**not checked**; it would also explain why the nitrous glow of that page does not trail the car).

### `RenderSpriteVelocity` (41) **[map]**, **[exe]**

A block of 80 bytes. The segment drawn per particle is `|velocity| * k * P[scale]` long, `k` being 0.125 here (+0x48: 0.5 on the
sprays, 0.15 on the embers, 1.0 on the scrape sparks) times a vector constant of the executable that is built at start-up and not
in its file (taken as 1, **not decoded**), clamped to +0x30 .. +0x34 (0 to 100 m); it is as wide as the particle's radius. The
material is the untextured motion blur sprite (below). +0x40 (brightness floor) and +0x44 (opacity scale) are 0 and 1 except
on the scrape dot (1 and 0).

### `UpdateBindings` (51) and the parameter array **[exe]**

Four bindings fill `P[1..4]` (`P[0]` is never written here and is taken as 1 **[infer]**): `P[n] = curve(source) + bias`. The
source is chosen by a byte at +0x60 + n and multiplied or divided by the float at +0x40 + 4n: 0 or 1 the effect's age, 3 the
speed of the effect's `Velocity` input, 4 the camera distance, 9 a constant, 0x11 `ImpactClosingVelocity` divided by the float,
0x12 `MovementSinceLastFrame` divided by it (others exist and are not read). The curve is 5 points in 10 bytes at +0x18 + 10n
(five y bytes, then five x bytes, each over 255; a range factor of the executable that is built at start-up is taken as 1);
straight lines between the points, flat outside. In the collision effects:

| | Source | Curve |
|---|---|---|
| Impact `P[3]` | `ImpactClosingVelocity / 22.35` (50 mph) | 0.153 at rest, 1.0 from x = 0.988 (22.1 m/s) |
| Impact `P[1]`, `P[2]`, `P[4]` | speed x 0.0224 or x 1 | 0.30 to 1 (`P[1]`), 1 to 0.145 (`P[2]`), 1 to 0 (`P[4]`) over 0 to 44.7 m/s (`P[4]` over 0 to 1 m/s) |
| Scrape `P[1]` | speed x 0.5 | 0 up to x = 0.365 (0.73 m/s), 1 from x = 0.686 (1.37 m/s) |

So **an impact scales both the count and the speed of the sparks** by `0.153 + 0.847 * min(v / 22.1, 1)` for a closing speed `v`,
and a **scrape** plays at its full rate from 1.37 m/s of sliding speed and not at all below 0.73 m/s.

## Materials and pictures **[data]**

| Material | Shader (`SHADERS.BNDL`) | Texture |
|---|---|---|
| `0x250C5` (all sparks) | `VfxParticles_MotionBlurSpriteUntex` | none: the shader makes the streak (`gVfxSparkBrightness` is its brightness constant) |
| `0x140280` (flash) | `VfxParticles_Diffuse_SubUV_NoBlend` | 512 x 512 BC3, 1 x 1 cells (second constant 1, 1) |
| `0x140200` (dust) | `VfxParticles_Diffuse_SubUV_NoBlend` | 512 x 512 BC3, 2 x 2 cells |
| `0x1402AF`, `0x0F58DD` | `VfxParticles_Diffuse_SubUV` (`0x1402AF`), `..._NoBlend` | 256 x 128 and 512 x 512, 4 x 4 and 2 x 2 cells (ground sprites; not converted) |

The executable also has the operation `RenderSpark` (67); no effect of the install uses it.

## What the 2012 data does not have

The restoration of the 2005 game's console effects ([vehicle effects](../vehicle-effects.md)) has, besides sparks, a **wind
contrail** behind the car at speed and **view streaks** over the screen. Nothing in the 2012 install matches either: no effect
of `GLOBALEFFECTS.BNDL` is driven by the car's speed alone (the closest are the wheel and exhaust effects), the post effect
states ([chameleon-nitrous.md](chameleon-nitrous.md#post-effects-verified-values-field-names-unconfirmed)) hold vignette, bloom
and colour grade, and the executable names `MotionBlurAmounts` as a shader constant. The wind is a sound: the layers of
`Genesys.Gen.SpeedSoundSet.WindLayer` are silence, the loops are a sequence of `GLOBAL.BNDL`
([chameleon-sound-graph.md](chameleon-sound-graph.md#the-sound-of-speed)). Lamp flares and glare are the `Corona`, `Corona.Flare` and `Corona.Glow` objects of
`GLOBALEFFECTS.BNDL` (`0x25169`, `0x2518C`, `0x49258`) and the coronas of the cars ([chameleon-lamps.md](chameleon-lamps.md));
their shape and curves are not decoded here.

## Not decoded

The index of the 16 handles; the second range of angles; the constant of the segment length and the range factor of the curves
(both set at start-up); `P[0]`; the control emitter of the scrape; the roles of `HangingBumperSparks` and
`VehicleCollisionSettings`; what plays the effect and when; how the dust and the flash blend (the shader family
`NoBlend` suggests the colour is not alpha-blended; the mod draws both additively); the drag of `UpdatePosVel` (above).
