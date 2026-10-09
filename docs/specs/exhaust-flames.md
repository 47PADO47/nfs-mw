# Exhaust flames (gear-change blow-off and lift-off backfire)

What the original shows at the tail pipes: when, where, with which particles. It covers the three "pipe effects"
of the car renderer (nitrous, gear change on an upgraded engine, miss-shift) and the particle emitter system that
draws them. Section 8 is the rewrite's own design; the rest is the original.

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), under `src/Speed/Indep/Src`: `World/CarRenderConn.cpp` (`UpdateEffects`, `UpdateEngineAnimation`,
  `OnEvent`, the constructor, the pipe and engine effect lists), `World/VehicleRenderConn.cpp` and `.h`
  (`Effect::Update`, `Fire`, `Stop`, the event ids), `World/CarRender.cpp` (`InitEmitterPositions`,
  `GetEmitterPositions`, `FXMarkerNameHashMappings`, `CarEffectPosition`), `World/Car.hpp` (`CarEffect`),
  `Ecstasy/EmitterSystem.cpp` and `.h` (the emitter system), `Physics/Behaviors/DrawCar.cpp` (what the render
  connection is told), `Physics/Behaviors/EngineRacer.cpp` (gear changes), `Physics/PhysicsUpgrades.hpp`,
  `Generated/AttribSys/Classes/{ecar,emitterdata,emittergroup,pvehicle}.h`. Read for understanding; no code copied.
- **Data inputs (read from the install, checked on `attributes.bin` and on the 2005 `attributes.bak`, identical):**
  AttribSys `ecar` (`NOSEffect`, `MissShiftEffect`, `ShiftSpeed`, `ShiftAngle`), `pvehicle`
  (`engine_upgrades`), `emittergroup`, `emitterdata`; the `EXHAUST` / `LEFT_EXHAUST` / `RIGHT_EXHAUST` position
  markers of the car solids; the `PARTICLE` texture pack of `GLOBAL/InGameB.bun`.

Evidence tags as in the [docs README](../README.md#evidence-tags): **[decomp]** read in the decompiled sources,
**[verified]** read from the install's data, **[inferred]** concluded from names or structure, **[not found]**
searched for and absent from the sources.

## 1. Summary

1. The only exhaust visuals in the sources are **two particle effect groups** started on the car's tail-pipe
   markers: the *nitrous* group (`fxcar_nos`: a short blue-to-orange flame plus a faint grey glow) and the
   *miss-shift* group (`fxcar_exhaust_bmw`: grey smoke puffs). [decomp, verified]
2. The nitrous group plays **while the nitrous burns**, and also **after every gear change** (up or down) of a
   player or AI-racer car whose engine is upgraded (or cannot be upgraded), for a time that depends on the car
   (about 0.28 s on most cars, 0.06 s on the FXX Evo) and only above 10 m/s. This is the "pop" at a shift. [decomp]
3. The miss-shift smoke is raised only by the drag-race engine class. [decomp]
4. The **sputters** (the crackle of the engine sound, `CARSFX_SparkChatter`) are sound only. The car renderer never
   reads them; no visual is tied to a lift-off crackle. [decomp: not found] The rewrite adds one on purpose (section 8.3).
5. The textures are `FX_FIRE02_ADDITIVE`, `FX_SMK06_BLEND`, `FX_SMK03_BLEND` and `FX_SMK05_BLEND` of the particle
   texture pack. [verified]

## 2. The pipe effects (every frame, per tail pipe)

Each tail-pipe marker owns one *effect* (an emitter group plus a local matrix). The car renderer updates them
after the engine animation each frame: [decomp]

```
for each pipe effect:
    if nitrous engaged:                      Update(group = ecar.NOSEffect)        // continuous
    else if the miss-shift flag is set:      Fire(group = ecar.MissShiftEffect)    // one burst
    else if blowoff flag and shifting != 0:  Update(group = ecar.NOSEffect)        // continuous
    else                                     Stop()
clear the miss-shift flag
```

- `Update` creates the group the first time (and whenever the group key changes), places it at
  `marker matrix x car render matrix`, enables it, sets intensity 1, makes it non-one-shot, hands it the car's
  velocity to inherit and runs one emitter-group update of `dt`. `Stop` only **disables** the group: no new
  particles spawn, those alive finish their life. `Fire` creates a separate fire-and-forget group, forced one-shot,
  that deletes itself. The effects are skipped (and stopped) when the car is not visible or farther than a
  distance factor away; the player's car is always visible. [decomp]
- "Nitrous engaged" is the physics' `IsNOSEngaged` (the nitrous is burning), passed in the render packet. [decomp]
- **Blow-off flag.** Set once when the car render connection is created, only for usage *player* (0) or *AI racer*
  (2): `engine_current != 0 || engine_current == engine_upgrades`, with `engine_current` the installed engine
  upgrade level and `engine_upgrades` the number of upgrade levels of the car's `pvehicle` collection. The bodies
  of `Upgrades::GetLevel` / `GetMaxLevel` are not in the sources; reading them as those two `pvehicle` fields is
  [inferred]. A stock engine of a car that can be upgraded gives no flame at a shift; a car with
  `engine_upgrades = 0` (BMW M3 GTR, FXX Evo, SF90, SL 65, Camaro, Corvette C6.R, Porsche 911 GT2 and the career
  start BMW M3 E46 among them) always has it; every upgrade level above 0 turns it on. [verified values]
- **`shifting`** (a float, 0 when idle): set to `+1` by an up-shift event and `-1` by a down-shift event
  (`VehicleRenderConn::E_UPSHIFT = 3`, `E_DOWNSHIFT = 4`; `E_MISS_SHIFT = 0`), through
  `CarRenderConn::OnEvent`. **Who raises these events is [not found]**: nothing in the sources calls
  `VehicleRenderConn::HandleEvent`; the physics raises `EPlayerShift` (every gear change of the player's car,
  `DoGearChange`) and `EMissShift` / `EPerfectShift` (drag only). Reading it as "once per gear change of the car" is
  [inferred] from the names and from the engine animation below.
- Each frame, before the effects (`UpdateEngineAnimation`), `shifting` is advanced:

  ```
  if not visible within 30 m: shifting = 0
  else if shifting != 0:
      if ShiftSpeed > 0 and ShiftAngle > 0 and gear >= first and speed > 10 m/s:
          shifting moves towards 0 by dt * ShiftSpeed / ShiftAngle     // both in degrees; the ratio is per second
      else: shifting = 0
  ```

  so the flame lasts while `shifting != 0` **after** that step: `ceil(1 / (dt * ShiftSpeed / ShiftAngle)) - 1`
  frames. `racers` have `ShiftSpeed 8`, `ShiftAngle 2.25` (rate 3.56 per s: 16 frames at 60 Hz, 0.27 s); the FXX Evo
  has `ShiftAngle 0.5` (rate 16 per s: 3 frames, 0.05 s). Cops, traffic and the rest have neither field (0): no
  flame. `gear >= first` is `data.mGear - G_FIRST >= 0`: neutral and reverse never show it. The speed is the
  length of the car's velocity vector (m/s). [decomp, verified values]
- The same `shifting` also pitches the body (the shift squat); that part is not this document's subject.

## 3. Where the pipes are

`FXMarkerNameHashMappings` maps the `CARFXPOS_EXHAUST` slot (10) to the marker names `EXHAUST`, `LEFT_EXHAUST`,
`RIGHT_EXHAUST`. `GetEmitterPositions` walks the highest-detail model of **every** car part slot and takes every
position marker with one of those names: one pipe effect per marker. A car without such a marker has no pipe
effect. [decomp] A census of the installed cars [verified]: one to four markers per car, all on the body solid
(`<CAR>_KIT00_BODY_A`, or `<CAR>_BASE_A` for the FXX Evo and SF90), at the rear (x about -2.0 to -2.4 m,
y 0 to 0.8, z 0.0 to 0.4 in the car frame). Cars with none: `LEVIN`, `SEMI`, `COPHELI`, `BRAKES` and the part
folders. The marker matrix (rows x, y, z, translation, as `docs/formats/cardata.md`) has its **z axis pointing
backwards** in every car checked (e.g. M3 GTR: (-1, 0, 0)); the emitters shoot along the local z axis.

## 4. The effect groups

`ecar.NOSEffect` and `ecar.MissShiftEffect` are references to `emittergroup` collections. Both are set only on
the `default` ecar collection, so every car with an `ecar` collection uses the same two groups. [verified]

| Group | Emitters (in order) | Used for |
|---|---|---|
| `fxcar_nos` | `emcar_nos_fire`, `emcar_nos_glow` | nitrous, gear-change blow-off |
| `fxcar_exhaust_bmw` | `emcar_exhaust_bmw`, `emcar_exhaust_bmw2` | miss-shift |

`emittergroup` fields: `Emitters[]` (references), `IntensityRanges[]` (per emitter `(min, max)`; `min >= max`
means 0..1), `FarClip`. The group is created with intensity 1; an emitter spawns only if
`min <= intensity <= max`. Both groups have `(0, 0)` ranges, so every emitter plays. [decomp, verified]

Emitter values ([verified]; units: metres, seconds, degrees, colours RGBA):

| Field | `emcar_nos_fire` | `emcar_nos_glow` |
|---|---|---|
| `NumParticles` per second (`NumParticlesVariance` 0) | 500 | 50 |
| `Life` s, `LifeVariance` (fraction) | 0.075, 0.8 | 0.5, 0.5 |
| `Speed` m/s, `SpeedVariance` | 6, 0 | 16, 0.6 |
| `SpreadAngle` (cone), `SpreadAsDisc` | 0, no | 1, no |
| `InitialAngleRange` | 15 | 360 |
| `RotationVariance`, `RandomRotationDirection` | 0, yes | 0, yes |
| `Size` at the 4 keys (full width) | 0.35, 0.45, 0.1, 0 | 0, 0.4, 1.0, 1.5 |
| `RelativeAngle` at the 4 keys | 0, 0, 0, 0 | 150, 100, 75, 25 |
| `Color1..4` (RGBA) | (0,40,80,60) (15,30,130,40) (100,70,40,50) (150,75,60,0) | (130,130,135,20) (140,140,150,76) (150,150,160,38) (125,125,125,0) |
| `KeyPositions` | 0, 1.8, 2.6, 3 | 0, 1, 2, 3 |
| `Drag`, `Gravity` | 0.05, 0 | 0.75, -1 |
| `MotionInherit` (variance), `MotionLive` | 1.0 (0), 1 | 0 (0), 1 |
| `VolumeCenter` (local x, y, z), `VolumeExtent` | (0, 0, -0.15), 0 | (0, 0, 0.1), 0 |
| `EliminateUnnecessaryRandomness` | yes | yes |
| `OnCycle`, `OffCycle`, `StartDelay`, `IsOneShot` | 0, 0, 0, no | 0, 0, 0, no |
| `AlphaToKillAt`, `NoKillAtAlpha` | 0, no | 0, no |
| `Texture` | `FX_FIRE02_ADDITIVE` | `FX_SMK06_BLEND` |
| `FarClip` | 100 | 150 |

`emcar_exhaust_bmw` (and `bmw2`, its child): 12 particles per second, life 2 s, no speed (it uses
`VelocityStart (-0.25, -0.45, 0.5)` `VelocityDelta 0.25` and inherits 0.75 of the car's velocity), sizes
0.5 / 0.75 / 1.5 / 2.5, grey-blue (about 80 to 90, 94, 102) with alpha 0, 51, 38, 0, textures `FX_SMK03_BLEND` and `FX_SMK05_BLEND`,
gravity -0.1, drag 0.125.

The texture name carries the blend: `..._ADDITIVE` has `alpha_blend = 2`, `..._BLEND` has `alpha_blend = 1` in the
TPK record, and all four are DXT3, 32x32 (smoke) or 64x64 (fire). The platform draw code
(`PlatStartParticleRender`) is not in the sources; additive (`src * alpha + dst`) for 2 and alpha blending for 1 is
[inferred] from the names and flags.

## 5. Spawning

An emitter is "on" from its first update (no delay, no cycle). Each update `Emitter::SpawnParticles(dt, intensity)`
spawns, in this order: [decomp]

```
rate   = intensity * NumParticles                    (variance 0 here)
n_f    = rate * dt                                   (a one-shot emitter uses Life instead of dt)
n      = int(n_f)
if n == 0: acc += n_f; if acc > 1 { n = 1; acc -= 1 }         // the fraction is lost when n >= 1
life_lo  = Life * (1 - LifeVariance);   speed_lo = Speed * (1 - SpeedVariance)
for each particle:
    speed = speed_lo + rand(Speed * SpeedVariance)                    // rand(r) is uniform in [0, r)
    if speed != 0:
        direction = local z axis (cone: three independent angles about x, y, z, each in
                    [-Spread/2, +Spread/2]); disc variant for SpreadAsDisc
        velocity  = direction * speed
    else (none of the two here): VelocityStart +- VelocityDelta along the local axes, AccelStart +- AccelDelta
    position = local matrix * (VolumeCenter + rand(VolumeExtent) - VolumeExtent/2)
    life     = life_lo + rand(Life * LifeVariance)       (capped at 60 s)
    if MotionInherit != 0 and not MotionLive: velocity += inherit_velocity * clamp(MotionInherit +- variance, 0, 1)
    angle    = -InitialAngleRange/2 + rand(InitialAngleRange); direction flag = random low bit
    rotation offset = clamp(rand(RotationVariance), 0, 1)
    size and colour start at curve parameter 0
```

`MotionLive` set means a spawned particle does **not** take the car's velocity: it stays where it was born and
only moves with its own speed, so at speed the flame leaves a trail along the path of the pipe. The local matrix is
set from the pipe effect every update, before spawning. The system caps live particles at 1024 and emitters at
500 (the player's effects have priority); no budget problem arises from two pipes. [decomp]

## 6. Particle update and drawing

Each frame, after all spawns, every live particle is stepped by `dt` (a particle born this frame also moves): [decomp]

```
remaining life -= dt                          (the particle dies when it would reach 0)
if Drag > 0:      v += v * max(-1, -dt * Drag * |v|)                  // quadratic drag
if Gravity != 0:  v.z -= Gravity * dt                                  // z is up; a negative gravity lifts
else:             v += acc * dt
position += v * dt
t = 1 - remaining_life / Life                 // Life = the emitter's nominal Life, not the particle's own
size, angle_delta = cubic(t) through the 4 (key, value) pairs        // key = KeyPositions[i] / 3
colour            = cubic(t) of the 4 colours, each channel x 255 clamped to 0..255
angle  = initial angle +- angle_delta * (1 + rotation offset)         // + or - by the direction flag
dies if colour alpha <= AlphaToKillAt, unless NoKillAtAlpha
```

Because `t` uses the nominal `Life`, a particle with a shorter random life starts further along the curves.
`Size` is stored doubled: the **half** size is `Size * 0.5`, so `Size` is the full quad width. Colours are packed
`r << 24 | g << 16 | b << 8 | a`. `RelativeAngle` is in degrees. [decomp]

The curve basis comes from `hermite_basis(controls, key0..key3)`, whose body is **not in the sources**. Reading
it as "the cubic polynomial through the four control values at the four keys" is [inferred] (four controls, four
keys, evaluated with the vector `(t^3, t^2, t, 1)`).

Drawing: one camera-facing quad per particle, centred on the particle, half extent `size` along the view right and
up vectors rotated by `angle` about the view forward axis, texture over the whole quad (`TextureAnimation` is
none), vertex colour as computed, blended per the texture (section 4), per emitter in list order, with no sort
between emitters. [decomp for the vectors; inferred for the blend]

## 7. Not found, or not done in the original

- Who raises `E_UPSHIFT` / `E_DOWNSHIFT` (section 2) and the body of `hermite_basis` (section 6).
- A link from the sputters (sound) to any visual. The sputters have no render-side reader; the rewrite's lift-off
  backfire (section 8.3) is its own design, not the original's.
- The platform blend state of particles (section 4).
- AI racers' own flames follow the same code (usage 2); only the player's car exists in the rewrite.

## 8. The rewrite

Everything in this section is the rewrite's own design **[decision]** unless it says otherwise.

### 8.1 Structure

- `libs/blackbox-particles` holds the emitter simulation of sections 5 and 6 (plain structs, no game names; start
  delay and on/off cycles are not modelled). `blackbox-render` draws the textured billboard batches (additive or
  alpha-blended) of its effects layer. `nfsmw-data` reads the groups, the markers and the textures from the
  install at run time; **no game texture or data is stored in the repository**.
- `crates/nfsmw/src/scenes/world/exhaust/` owns the trigger (section 2), the pipes (section 3) and the per-frame
  step at the 60 Hz physics rate. The gear change is read from the physics' gear: a change between two forward
  gears (or into one) counts as an up-shift or down-shift event.
- The career's installed engine level does not exist yet: it defaults to **0** and the console command
  `exhaust-flames engine <n>` sets it, so cars with upgradable engines show the shift flame only after
  `exhaust-flames engine 1` (or higher).
- The miss-shift smoke (drag races) is not drawn; there are no drag races yet.

### 8.2 The setting

`exhaust_flames` (on or off) switches the whole feature; the default is **on**. Setting, command line
(`--exhaust-flames` / `--no-exhaust-flames`), environment (`NFSMW_EXHAUST_FLAMES`), config file, F12 console
(`set exhaust_flames off`) and the Video option row work as for `tire_smoke`. What the switch does:

- **Off:** the car's pipe markers, particle group and textures are not read, nothing is uploaded to the GPU, no
  emitter exists, the physics step and the frame skip the flames after one `Option` check, and no vertex is
  built. Sputter notices from the sound are dropped.
- **On:** the flames of the driven car are loaded the first frame after the car is placed (or the setting is turned
  on), so a menu change takes effect at once without restarting the car. Turning it off drops the emitters and
  frees the textures. At rest the cost is the same `Option` check plus, per pipe emitter, a check that it has no
  particles; a flame costs at most the budget below.
- The default is on because an idle car pays almost nothing and the feature is rare to see (nitrous, a shift
  pop, a lift-off crackle); the particle counts are capped (8.4). A machine that cannot afford even that sets it off.

### 8.3 The triggers

A pipe flames, in this order of strength, when:

1. **The nitrous is not flamed.** The original's nitrous effect differs from the one used here; it is left for later
   and the group is not played while the nitrous burns.
2. **A gear-change blow-off runs** on an engine that allows it: the group at full intensity (section 2).
3. **A sputter pops while the driver is off the throttle** (the *lift-off backfire*): the group at intensity 0.5 for
   `BACKFIRE_SECONDS` (0.06 s, about the life of one fire particle) after each pop.

The third is the rewrite's own: the original ties no visual to the sputters (section 1, item 4) but the roadmap asks
for the crackle to show. Each time the `CAR_Sputter` module of the sample layer starts a pop voice
([engine-sound-aems.md](engine-sound-aems.md) section 4: about five pops over 2.5 s after a lift-off at high RPM,
one after a shift, none at a steady throttle) the sound reports one *sputter notice* to the scene. The scene turns it
into a backfire when the throttle of that physics step is below 0.15 and the gear is a forward one; a pop with the
foot down (the pop after a shift under power) shows only what the blow-off rule of item 2 shows. A notice is raised
when the voice starts, so the flame leads the sound by the audio latency (tens of milliseconds) and trails it by the
frame the notice waits for the next physics step. The tuner car's forced backfire (`Force_Trigger`) is not produced
(engine-sound-aems.md section 5).

Without sound (`--no-sound`, no output device, screenshot runs, no sputter bank in the car's set) there are no
notices and so no lift-off backfire; the blow-off does not depend on the sound. `exhaust-flames pop` and the `pop` key of a
`--drive-script` raise one notice by hand, which is how the backfire is screenshot.

### 8.4 Budgets

- At most **4 pipes** (the census of section 3 found 1 to 4) and at most **96 live particles per emitter**
  (`Emitter::set_limit`; the group's own rates need about 40 for the fire and 25 for the glow, so the cap is only
  reached by a modified install). That is at most 4 x 2 x 96 = 768 sprites, 4,608 vertices, per frame.
- The emitter's particle storage is reserved to its limit when it is created, so stepping never allocates; a spawn
  past the limit is skipped. The vertex buffers of the sprite batches go back to the flames after each upload and
  are reused; the GPU side reuses its buffers (`blackbox-render`). Nothing is allocated for a frame without
  particles.
- Emitters with no particles and not flaming are not stepped.
- The flames are drawn depth-tested without depth writes and are not part of the tire or collision effect budgets.

### 8.5 Brightness

The emitter data gives the fire an alpha of 40 to 60 of 255, and the platform blend state is not in the sources
(section 6). Drawn as `src * alpha + dst` the flame is nearly invisible against a lit road in a screenshot of the
rewrite (tried). The rewrite therefore multiplies the alpha of **additive** particles by `ADDITIVE_GAIN` (4,
clamped at 255); alpha-blended ones are untouched. This is a host enhancement to be tuned against the running
original, not a reading of it.

### 8.6 Checks

`exhaust-flames` (F12) prints the pipes, the engine level, the live particles and the state; `exhaust-flames engine
<n>` sets the level and `exhaust-flames pop` fakes one sputter notice. A scripted run screenshots the flames at the
end of its script: the backfire with a last, very short segment `0.05:pop` (a scripted run has
no sound, so the key stands for the notice).
