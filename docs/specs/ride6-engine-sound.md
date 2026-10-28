# RIDE 6 engine sound: what drives it, what plays, and how it maps to this game

Spec for `mods/ride6-bikes/src/sound`, `libs/blackbox-carsound/src/overrun.rs` and the host side
`crates/nfsmw/src/audio/car/own_sfx/{overrun,gear}.rs`. Written in our own words from the
shipped game's data assets and from reading its executable (`ride6-Win64-Shipping.exe`, UE 5.6.1 build `5.6.1-1697004`).
No decompiler output, no game data and no recording is in the repository ([provenance](../provenance/ride6-audio.md)).
Where the sound is made (the Wwise banks) is covered in [formats/ride6-audio.md](../formats/ride6-audio.md).

Evidence tags: **[data]** read from a data asset; **[code]** recovered from the executable, with the function address
(`0x14...`, image base `0x140000000`); **[choice]** a named modelling decision, listed in section 7.

## 1. What is known and what is not

RIDE 6 does not synthesise or crossfade engines in its own code. It feeds the numbers of section 3 to a Wwise sound
engine as *game parameters* (RTPCs) and posts *events*; the blend of the engine loops by revs, the load layers, the
pitch curves and the choice of a backfire recording are authored inside the Wwise banks.

| Part | Status |
|---|---|
| Which events a bike plays, which bank holds each, which recordings an event lists | **[data]**, all 634 chassis/variant rows |
| The values the game sends to Wwise each frame and how they are made | **[code]**, section 3 |
| When a gear change plays, when a backfire bang and flame happen, and their numbers | **[code]** + **[data]**, sections 4 and 5 |
| The engine loops themselves, their rpm points, the crossfade ranges, the pitch and load curves | **not recovered**: they are inside `WwiseAudio/KTM_EXC_450.bnk`, which the game keeps in a `.pak` whose index is encrypted ([formats](../formats/ride6-audio.md#where-the-data-is)). No key was used or sought. |
| The rule inside Wwise that picks one of the five backfire recordings | **not recovered**, same reason |

So the engine layers of a converted bike need the user's own decoded recordings *and* their rpm points
(`--layer "<rpm>:<wav>"`). Until those exist the bike plays the host's built-in engine; the manifest tables are generated
as soon as they are supplied ([modding/ride6-bikes-sound.md](../modding/ride6-bikes-sound.md)).

## 2. What a bike plays

A bike's chassis id maps to an audio data asset through the table `CHASSIS_AUDIO` (key `CH_<chassis>$Stock|Custom`) **[data]**.
The asset (class `BikeAudioDataSource`) holds Wwise events and the effect numbers; anything it leaves out takes the
value of the base asset's defaults.

| Role | KTM 450 SMR 2022, Stock | Custom (aftermarket exhaust) |
|---|---|---|
| Engine, loops while the bike is on | `KTM_EXC_450_EN` (bank `KTM_EXC_450`, media inside the bank) | the same |
| Exhaust, loops, same bank | `KTM_EXC_450_EX` | the same |
| Backfire bang | none | `Backfire_KTM_EXC_450`: five recordings `_01` to `_05`, 24 to 52 ms long |
| Backfire flame | none | none |
| Gear up | `GearUp` (shared bank `VehicleCommon`) | `GearUp` |
| Gear down | `GearDown` (shared) | `GearDown_KTM_EXC_450`: the five backfire recordings plus four `GearDown_gsxr1000-00N` clunks (two media ids each) |
| Supercharger | none | none |

The Custom variant is the one with the louder, popping exhaust; Stock never backfires audibly, whatever its numbers say.
Both have `bBackfireSimulationEnabled = true` and the new backfire system on. Over all 634 rows (317 chassis, two
variants each; 194 distinct data assets, 103 distinct engine banks): 595 name a bang event, 348 a flame event, 8 a
supercharger event; 551 turn the backfire simulation on, 550 of them with the new system, one with the old; 83 leave the
flags at the base value (off), and 53 rows name a bang event but have the simulation off (they never bang). The table of
all bikes and their banks is made by the scratch script `all_bikes_audio.py` ([formats](../formats/ride6-audio.md#tools)).

The engine's own limits come from the physics asset, not the audio asset: `MinRPM 2300`, `MaxRPM 11000`
(`KTM_450SMR_2022`, `Engine`), which become `idle_rpm` and `redline_rpm` of the manifest.

## 3. The values sent to Wwise (the motor binder, every frame)

The engine and exhaust events get these parameters **[code]** (`0x1451d0390` tick, `0x1451d51c0` input gathering). Names
are those of the game's configuration struct; the Wwise-side parameter names are strings in a constructor and were not
needed.

| Parameter | Value |
|---|---|
| Engine rpm | rpm from the vehicle data component, or `clamp((rpm - min) / (max - min), 0, 1)` when `bUseNormalizedRPM` is set. With `bApplyRPMNoise` (true for bikes **[data]**): `rpm + maxRpm * scale * noise * k`, clamped to `0 .. maxRpm * scale`, where `k` is the percentage for the surface under the bike (loose gravel 0.08, grass 0.05, gravel 0.06, kerb 0.04, sand 0.06, others 0) **[data]**; `noise` is a per-frame signal of the data component whose generator was not recovered |
| Throttle | the smoothed engine throttle (below); forced to 1 while the rev limiter is reached and that limiter's `bOverridePhysicThrottle` is true (all three of a bike's limiters are **[data]**) |
| Input throttle | the same smoothing, of the driver's throttle |
| Gear | the current gear as a number |
| Vehicle speed | `min(speed km/h, RTPC_MaxSpeedKmH)` |
| Delta speed | `min(abs(speed - previous speed), RTPC_MaxDeltaSpeedKmH)` |
| Wheel rotation speed | mean wheel angular speed in degrees per second, capped at `RTPC_MaxRotationSpeedDegreesSecond` |
| Clutch, torque efficiency, electronic throttle modulation, fuel flag | the vehicle's own values |
| Limiter status | 0, or limiter index + 1 while a limiter is reached |
| Launch-control / pit-limiter rpm | the engine rpm while that custom limiter index is active, else 0 |
| Normalized limiter reached elapsed time | the vehicle's value |
| Suspension compression, compression speed, force (front, rear, average) | the vehicle's values (clamped 0 to 1 for compression) |
| Muffler filter | `MufflerFilter`, an integer: 1 for the Custom KTM, 0 for the base **[data]** |
| Gear up / gear down | 1 / 0 for the frame an up shift happened, 0 / 1 for a down shift, else 0 / 0 |
| Level of detail | the data component's value |

**Smoothing.** `smooth(previous, target, dt, speed) = previous + (target - previous) * clamp(dt * speed, 0, 1)`
**[code]** `0x1451b5030`. `speed` is `ThrottleInterpolationSpeed`; its value is a constructor default and is not in any
asset (not recovered). The data component applies the same function to the throttle it reports.

**Gear events.** When the gear differs from the previous frame's the binder posts the gear-up event if the new gear is
higher, else the gear-down event **[code]** `0x1451d0390`. There is no minimum gear or speed at this level. Which of the
`GearDown_gsxr1000` or backfire recordings plays is decided inside Wwise. `DownShiftEnable` is 0 in the base asset, so the
rpm "spike after a down shift" (`DownShiftRpmPercentageSpike 0.1`, attack and decay 0.8 s) is off for the KTM **[data]**.

## 4. Doppler and distance

The engine, exhaust, engine-LOD, backfire and supercharger events use a Doppler exponent of 0.7; gear changes, brakes and
collisions 0 **[data]** (`BikeAudioComponent_BP`). Engine events sit at the sockets `SOCKET_AUDIO_EN`, `SOCKET_AUDIO_EX` and
`SOCKET_AUDIO_AI` (the LOD engine); backfire at the exhaust socket **[data]**. Not used by this game's mixer.

## 5. Backfire

Two systems exist; a bike chooses with `bNewBackfireSystemEnabled`. The KTM 450 uses the **new** one.
The physics-side effects element runs it every frame **[code]** (`0x1451436c0` update, `0x145137500` new system,
`0x1451377b0` old system, `0x14511be10` event emission).

### 5.1 Inputs

`throttle` is the engine throttle (after electronic intervention) and `raw` the driver's; `rpm_fraction` is the engine
speed over its maximum, clamped to 0 to 1 (so `rpm / 11000` for the KTM, not idle based; the engine speed field is
assumed to be the current rpm); `last` is the previous frame's
`throttle`. The system does nothing when `bBackfireSimulationEnabled` is false. (It also reads a fuel-tank element and
skips when the tank reports empty; this game has no fuel.)

### 5.2 New system

```
charge  = clamp(charge + AccumulatorDeltaFactor * rpm_fraction * dt * s, 0, 1)     s = throttle if throttle > 0 else -1

armed  := true, bang_time := 0     when throttle < InitialThreshold and last >= InitialThreshold

while armed:
  if throttle > TriggerThreshold or last <= TriggerThreshold:  bang_time += dt                  (still waiting)
  else:                                                       (throttle just fell through TriggerThreshold)
     if bang_time <= MaximumDeltaTime:
        x = charge * (1 - bang_time / MaximumDeltaTime)
        if x > BangAudioIntensityThreshold and random() <= P0 + (P1 - P0) * raw:             P = BackfireBangProbabilityRange
            bang(x)
     armed := false, bang_time := 0

flame:  if throttle == 0 and last == 0 and rpm_fraction >= FlameEffectMinRPM:  flame_time += dt; active |= flame_time >= FlameEffectDelayTime
        else: flame_time := 0
        if active:  y = charge * rpm_fraction;  if flame_time == 0 { y := 0; active := false };  flame(y)
```

`random()` is `rand() & 0x7FFF` over 32768. Events carry `(raw intensity, level above the threshold, type)`:

| Event | Condition | Audio level | Effect (flame/particles) level |
|---|---|---|---|
| Bang, type 0 | `x > BackfireBangParticlesIntensityThreshold` | `(x - A) / (1 - A)`, `A` = audio threshold | `(x - B) / (1 - B)`, `B` = particles threshold |
| Bang, type 2 | otherwise | `(x - A) / (1 - A)` | none |
| Flame, type 1 | `y > BackfireFlameParticlesIntensityThreshold` | `y` | `(y - C) / (1 - C)` |
| Flame, type 3 | otherwise | fixed 1 (raw) with `y` as the second argument | none |

The audio events set two Wwise parameters, `BackfireIntensity` (raw) and `BackfireThresholdNormalizedIntensity` (the
level above the threshold), `IsBackfireFlame`, and the rpm and throttle of the backfire binder (smoothed throttle with the
same `smooth`, rpm or normalized rpm), then post `BackfireBangEvent` or `BackfireFlameEvent` **[code]** `0x1451d0280`. A
bike with no such event in its data asset plays nothing (the flames may still draw).

### 5.3 Numbers

Base asset defaults **[data]** and the KTM 450 overrides; "Custom" is the variant with a bang event.

| Field | Base | KTM 450 Custom | KTM 450 Stock |
|---|---|---|---|
| `bBackfireSimulationEnabled`, `bNewBackfireSystemEnabled` | false, false | true, true | true, true |
| `AccumulatorDeltaFactor` | 0.2 | 0.2 | 0.2 |
| `BackfireEffectThrottleInitialThreshold` | 0.9 | 0.9 | 0.9 |
| `BackfireEffectThrottleTriggerThreshold` | 0.1 | 0.1 | 0.1 |
| `BackfireMaximumDeltaTime` (s) | 2.0 | 2.0 | 2.0 |
| `BackfireBangProbabilityRange` | (0.8, 0.3) | (0.8, 0.3) | (0.8, 0.3) |
| `BackfireBangAudiotIntensityThreshold` | 0.85 | **0.95** | 0.85 (no event) |
| `BackfireBangParticlesIntensityThreshold` | 0.95 | **0.8** | 1.0 |
| `BackfireFlameParticlesIntensityThreshold` | 0.7 | **1.0** | 1.0 |
| `FlameEffectDelayTime` (s) / `FlameEffectMinRPM` | 0.2 / 0.1 | same | same |

A reading that matters for the feel: with `AccumulatorDeltaFactor 0.2` the charge takes 5 s at the limit (`rpm_fraction 1`)
to fill and a bang needs `charge * (1 - t/2) > 0.95`, so only a full-throttle, near-redline pull followed by a lift
*within 0.1 s* (`t < 0.1`) bangs; the probability is 0.8 for a driver who shuts the throttle (`raw` 0) and 0.3 when the driver still holds it and an
assist cut it, so a clean lift bangs four times in five. The 450 therefore pops rarely, only after a hard pull, and loudly. Table values are the resolved merge (instance over base), see section 7.

### 5.4 Old system (other bikes)

One chassis row of 634 runs the old system (simulation on, new system off); the rest either use the new system or have the
simulation off. It is a burst model **[code]** `0x1451377b0`, partly recovered:

- A timer counts while the normalized torque (ideal torque curve value times the engine throttle) is above
  `BackfireNormalizedTorqueThreshold`, and stops while a burst is in progress.
- A burst starts when that timer exceeds `BackfireTimeThreshold`, the torque is not rising faster than
  `-BackfireNormalizedTorqueDeltaDown` per second, `rpm_fraction` is within `BackfireMinNormalizedRpm .. BackfireMaxNormalizedRpm`,
  the engine throttle is under 0.2 and the time since the last burst is at least a random time between
  `BackfireMinTimeBetweenConsecutiveEffects` and `BackfireMaxTimeBetweenConsecutiveEffects`.
- A burst is `Min..MaxDischargeEffects` pops that share an intensity budget of 1: each pop takes a random share
  (at least `min(budget, 0.2)`), the last takes the rest, and a pop under 0.2 is not emitted. Base defaults **[data]**:
  threshold 0.7, time 0.2 s, min/max rpm 0.8 / 1.0, 1 to 3 pops, delta -1.
- A gear change can instead start one pop (`OnGearSwitchUp/DownParameters`: enabled, rpm window, gear at least
  `MinRequiredGear`, probability, intensity between `MinIntesity` and `MaxIntensity`). Disabled in the base asset.
- The effect type chosen for a pop (bang alone or with flames) comes from a helper (`0x145111ff0`) that was not read.

The overrun controller of this game implements the **new** system only; the single old-system row is written without an
`[car_sound.overrun]` table (it keeps the core backfire rule).

## 6. Mapping onto this game

The host (`crates/nfsmw`) has an engine mixer that blends looping layers by engine speed and a rule-based backfire. A
RIDE 6 bike needs two things the core schema did not have, both now in it: the backfire *decision* of section 5.2
(`[car_sound.overrun]`) and the gear-change trigger of section 3 (`shift_trigger = "gear"`). The layers, curves, shifts and
pops already existed.

### Manifest extension: `[car_sound.overrun]`

```toml
[car_sound.overrun]
accumulator_rate = 0.2          # AccumulatorDeltaFactor
throttle_arm = 0.9              # BackfireEffectThrottleInitialThreshold
throttle_fire = 0.1             # BackfireEffectThrottleTriggerThreshold
window_seconds = 2.0            # BackfireMaximumDeltaTime
flame_delay = 0.2               # FlameEffectDelayTime
flame_min_rpm = 0.1             # FlameEffectMinRPM
bang_audio_threshold = 0.95     # BackfireBangAudiotIntensityThreshold
bang_effect_threshold = 0.8     # BackfireBangParticlesIntensityThreshold
flame_effect_threshold = 1.0    # BackfireFlameParticlesIntensityThreshold
probability_closed = 0.8        # BackfireBangProbabilityRange.X
probability_open = 0.3          # BackfireBangProbabilityRange.Y
```

All eleven keys are required numbers; rates, thresholds and probabilities are 0 to 1, `window_seconds` and `flame_delay`
0 to 10, `accumulator_rate` 0 to 5. The table is valid only beside a `backfire` list or nothing (a car without it keeps
the core rule). Core behaviour when present, replacing the rule "backfire when the throttle is lifted at high engine speed":

1. Each engine-mixer tick call `blackbox_carsound::overrun::OverrunBackfire::update` with `throttle` = the throttle the
   engine mixer already uses, `raw_throttle` = the driver's pedal, `rpm_fraction = rpm / redline_rpm`.
2. A returned bang plays one random file of `backfire` at volume `audio_level`; its `effect_level`, when set, lights the
   pipes with that strength ([exhaust-flames.md](../exhaust-flames.md)); a bang with no `effect_level` is audio only.
3. A returned flame lights the pipes with `effect_level` when set and plays nothing (no flame recording in the KTM data).
4. `pops` keep their core meaning (after a bang) and are not driven by this table.

The core manifest reader checks the table (all eleven keys required, the ranges above, `libs/blackbox-modding`
`content/overrun_sound.rs`) and the host plays it (`own_sfx/overrun.rs`). The generator writes it when the bike's data has
the simulation on and the new system (`OverrunParams::applies`), unless told not to (`--no-overrun-table`).

Details of the host side that the list above leaves open:

- The bang's recording volume is its `audio_level` (section 7); the pops after a bang are the core pops (one to three, as after
  a core backfire) and light the pipes only when the bang did.
- A flame lights the pipes in bursts of 0.06 s (`FLAME_PULSE_SECONDS`, the length of the pipe flame of a pop), because the
  pipes take bangs and not a strength (section 7).
- `rpm_fraction` is `rpm / redline_rpm` with `rpm = idle + rpm_pct * (redline - idle)` of the car's `[engine_sound]` span (the
  built-in engine's 900 to 7000 when it has none); the throttle is the pedal for both `throttle` and `raw_throttle`.
- The core lift-off rule (a 3 to 25 percent backfire chance and its lone pops) is not run for a car with the table; the
  turbo blow-off of the lift-off is.

### Gear-change sounds: `shift_trigger = "gear"`

Section 3: a gear number different from the previous frame's posts `GearUp` if higher, `GearDown` if lower, with no minimum
gear, speed or engine speed. The core's default trigger is the 2005 cars' gear clunk of the engine mixer (a delay after the
shift begins; up shifts under 3000 rpm ignored; down shifts only above 3000 rpm), which is a different rule, so the bike
manifest says `shift_trigger = "gear"` and the host (`own_sfx/gear.rs`) plays `shift_up` / `shift_down` in the frame the
gear number changes. The gear number is `CarInput::gear`, which the bike sim fills from `Bike::gear()` (1 for first, so 2 in
the host's numbering); `Powertrain::shifting()` is not needed, the number changing is the event. No request to the sim.

## 7. Modelling choices (not facts)

| Choice | Reason |
|---|---|
| Engine layers are the user's decoded loops at rpm points the user supplies; layers of one rpm (engine and exhaust) are summed into one | The Wwise blend (section 1) is unreadable without the bank; the core mixes layers by rpm in equal power and refuses two layers of one rpm |
| Loops are closed with a 40 ms equal-power crossfade (`SoundOptions::loop_fade_ms`) unless the WAV carries a `smpl` loop, whose body is used | A decoded loop is a clip; the game loops the whole file |
| Engine loops keep their relative level; the loudest peaks at 0.9. One-shots are each normalised to 0.9 | The host rescales the whole engine by its own level model; the Wwise gains are unknown |
| `shift_clunk = false` for a bike | The class-wide clunk of the 2005 cars is not a bike sound |
| `shift_up` / `shift_down` hold the decoded `GearUp` / `GearDown` recordings; the five backfire recordings of the Custom `GearDown` bank are also listed under `backfire` | The Wwise choice among a bank's recordings is unreadable; the core plays one at random |
| The merge "instance over base" for effect numbers | The instance JSON lists only overridden fields; property inheritance gives the rest the base value (checked: every field of the Custom asset that differs from the base is listed) |
| No overrun table for the one old-system row | Section 5.4 is partly recovered and one row does not justify a second controller |
| No fuel-tank gate, no rpm noise, no limiter-forced throttle in the overrun input | This game's engine mixer has the limiter and no noise source; section 3's rpm noise is a Wwise-side effect |
| A bang plays at volume = its `audio_level` (0 to 1) and a gear change at volume 1, both times the host's one-shot level | The original passes the level to Wwise as a parameter and posts the gear event with no level; what they do to the recordings is authored in the unreadable banks |
| A flame lights the pipes in 0.06 s bursts; the pops after a bang light the pipes only when the bang did | The pipes of this game take one bang at a time, no strength; 0.06 s is the length of the pipe flame of a pop (`docs/exhaust-flames.md`) |
| The driver's throttle and the engine's throttle are the same pedal value | The host has no electronic throttle intervention for a bike |
| The overrun rpm span is the `[engine_sound]` idle/redline of the car, else the built-in engine's | The host's `0 to 1` engine speed is relative to the span; the bike sim's own idle and redline are not passed to the audio (a request for the lead: add `idle_rpm` to `CarAudioTelemetry` for an exact fraction with the built-in engine) |
