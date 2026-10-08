# Car sound effects: shifting, accelerate transitions, turbo, nitrous, skids, collisions

How the original triggers the effects around the engine, which banks they come from and which numbers
shape them. Companion of [engine-sound.md](engine-sound.md) (the engine mix) and
[engine-sound-ginsu.md](engine-sound-ginsu.md) (the synthesiser).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), `src/Speed/Indep/Src/EAXSound/`: `sfxctl/SFXCTL_{Shifting,AccelTrans,Engine,Wheel,Physics}.cpp`,
  `CARSFX/CARSFX_{Shifting,Turbo,Nitrous,Skids,SparkChatter,BottomOut}.cpp`, `CARSFX/SFXObj_Collision.cpp`,
  `SoundCollision.{hpp,cpp}`, `EAXSndUtil.cpp` (interpolators), `Dynamic_Mixer/NFSMixShape.cpp` (curve shapes),
  `SND_GEN/{ENGINES_AEMS2,MAIN_AEMS,TURBO}.h` (parameter structs), `Generated/AttribSys/Classes/
  {shiftpattern,turbosfx,acceltrans,audioimpact,audioscrape}.h`. Read for understanding; no code copied.
- **Data inputs:** AttribSys `shiftpattern`, `acceltrans`, `turbosfx`, `engineaudio` (`Vol_ShiftSweets`,
  `Vol_Sputters`), `audioimpact`, `audioscrape`, `simsurface` (`Aud_Skid_Type`).

Evidence tags as in the [docs README](../README.md#evidence-tags). Times are seconds unless "ms". The sound
scale for RPM (1000 to 10000) and `EngRPM`, `PhysicsRPM`, `EngTorque` are those of the engine spec.

**Interpolators.** `interp(a -> b, T ms, curve)` moves linearly (or along a curve) from `a` to `b` over `T`
milliseconds of accumulated frame time; before it finishes its value is `a + (b - a) * curve(elapsed / T)`,
afterwards `b`. `T <= 0` is read as 10 ms. `LINEAR` is the identity; `EQ_PWR_SQ` is `sin^2(pi/2 * x)` (the
equal-power curve squared). Some interpolators have a *live target*: each update the finish value is replaced
by the current physics value, so they land on wherever the physics is **[decomp]**.

## 1. Shifting

`SFXCTL_Shifting` runs a state machine on gear changes. It replaces the engine's audio RPM and torque while
active and raises one-shot sound flags. Fields are those of the car's `shiftpattern` collection.

### 1.1 Triggers

Each update, comparing the gear id with the previous update's: a higher gear starts an **up shift**, a lower
one a **down shift** (not into neutral). Up shifts are ignored when `PhysicsRPM < 3000`. A shift starts from
whatever state is running (it is cleaned up first). The gear change flag comes from the physics gear, so
automatic and manual shifts behave alike **[decomp]**.

### 1.2 Up shift

Stages `DISENGAGE -> ENGAGING -> LFO -> none`:

- **Start.** `RPM_AtShift = previous EngRPM`. `Up_Shift_Sound_Delay` seconds later the gear clunk plays
  (§1.6). The disengage sweetener plays now if `PhysicsRPM >= 7000`. Torque interpolates `PhysicsTRQ -> 0` in
  100 ms. The tachometer value interpolates `RPM_AtShift -> RPM_AtShift - 1200` over the whole shift (the sum
  of the disengage times and `Up_Engage.Time`) with `EQ_PWR_SQ`.
- **DISENGAGE.** `Up_DisengageFall` is an array (1 or 2 entries) of `{RPM, Time}` (two `i16`), each with a
  curve in `Up_DisengageFall_Curve` (a 4x4 matrix: four Bezier control points `(x, y, 0, 0)`). The curve is
  sampled at 7 equally spaced parameters (`t = k / 6`) into a polyline `(x * Time ms, y * RPM)`. During
  the stage `EngRPM = clamp(RPM_AtShift + polyline(elapsed ms), 1000, 10000)`. When the polyline ends, the
  next entry starts (re-based on the RPM reached) or, after the last, ENGAGING. With the data's curves `y`
  goes from 0 toward -1, so the RPM drops by `RPM` over `Time` ms (for example 3500 over 200 ms for the default
  pattern, or two drops of 1500 and 1200 over 210 ms each for `0x6EB87040`) **[verified]**.
- **ENGAGING.** Same with `Up_Engage` (`{RPM, Time}`) and `Up_Engage_Curve`, but based on the *current*
  `PhysicsRPM` each update: `EngRPM = clamp(PhysicsRPM + polyline(elapsed), 1000, 10000)` (the curve typically
  overshoots and settles, `y` up to 1.97 times `RPM`). Torque jumps back to `PhysicsTRQ`. The engine
  volume factor gets an **attack**: `ShiftingVOL` interpolates `Up_Engaging_Attack_Vol -> 0` over
  `Up_Engaging_Attack_T` ms, and `EngVolume = 32767 * (1 + ShiftingVOL)` so the engine is louder right after
  the shift. The engage sweetener plays if `RPM_AtShift >= 7000`. The post-shift LFO starts (§1.5).
  The engine mix does not smooth its volumes in this stage (engine spec §5.5).
- **LFO.** `EngRPM` tracks `PhysicsRPM` while the LFO decays; at its end the shift is cleaned up.

### 1.3 Down shift

Stages `DOWN_DISENGAGE -> RISE -> FALL -> REATTACH -> none` (a rev-matching blip) **[decomp]**:

| Stage | RPM | Torque | Next when |
|---|---|---|---|
| `DOWN_DISENGAGE` | `RPM_AtShift -> clamp(RPM_AtShift - Down_Disengage_Fall_RPM, 1000, 10000)` over `Down_Disengage_Fall_T`, linear | `PhysicsTRQ -> 0` in 50 ms | RPM interpolation done |
| `RISE` | `EngRPM -> clamp(PhysicsRPM + Down_Engaging_Rise_RPM * k, 1000, 10000)` over `Down_Engaging_Rise_T` (x0.7 at or below first gear), `EQ_PWR_SQ` | `EngTorque -> 100 * k` in 215 ms | done |
| `FALL` | `EngRPM -> EngRPM - Down_Engaging_Fall_RPM * k` over `Down_Engaging_Fall_T` (x0.7 at or below first gear for engine levels 0 and 1), linear | `100 * k -> 0` in 215 ms | done |
| `REATTACH` | `EngRPM -> PhysicsRPM` over `clamp(|EngRPM - PhysicsRPM| * Down_Reattach_Scale, 0, 800)` ms, `EQ_PWR_SQ` (live target) | `0 -> PhysicsTRQ` in 60 ms | done |

`k = 0` when `EngRPM < 1500` at the start of the stage (no blip at idle), else 1. The down-shift gear
clunk is queued only if `PhysicsRPM > 3000`, after `Down_Shift_Sound_Delay`.

### 1.4 What the shift changes

While a shift runs: `EngRPM` and `EngTorque` come from the shift (engine spec §4.3); the engine mix (single mode) takes new
volume values without smoothing during the up shift's ENGAGING stage and during the down shift's REATTACH
stage if the car is accelerating (dual mode: only the up engage); the redline cannot start (and ends); `AvgDeltaRPM` is fed from the audio
RPM (§5.2 of the engine spec). The shift state also publishes mixer inputs: 0 = up disengage, 1 = up engage
started, 2 = down shifting (each 0 or 32767).

### 1.5 Post-shift wobble (LFO)

After an up shift (gears 1 to 5; nothing from 6th up) the engine's RPM and volume wobble and decay:
`RPM_LFO_amp = LFO_RPM_Amp * g` decays to 0 over `LFO_RPM_Decay_Time` ms with period `LFO_RPM_Freq` ms;
`VOL_LFO_amp = LFO_Vol_Amp` (Q15 volume units) decays over `LFO_Vol_Decay_Time` ms with period `LFO_Vol_Freq` ms.
`g` is the gear scale: first 1.0, second 0.85, third 0.7, fourth 0.55, fifth 0.3. The LFO is `amp * sin(2 pi phase)`
with `phase` advancing by `dt / period` and starting at about a quarter turn (so it starts at its peak).
`EngRPM` receives it twice in the decomp (`NormalRPM = rpm + lfo` and then `+ lfo` again): keep, the
amplitudes are tuned for it. The torque LFO is unused (0) **[decomp]**.

### 1.6 Sounds

| Sound | Trigger | Bank / Csis | Volume |
|---|---|---|---|
| gear clunk | `Up_Shift_Sound_Delay` / `Down_Shift_Sound_Delay` after the shift starts | `shiftpattern.BankName` (`SOUND/SHIFTING/GEAR_*.abk`), `FX_SHIFTING_01` sample 0 up, 1 down (type `SHIFT`) | `Up_Vol_Shift` / `Down_Vol_Shift` x `cos((1 - s) * 90 deg)`, `s = 0.1 + 0.9 * clamp((PhysicsRPM - 1500) / 6500, 0, 1)`, then x the mixer output |
| disengage sweetener | start of an up shift, `PhysicsRPM >= 7000` | `SweetBank[0]` (`SWTN_CAR_nn_MB.abk`), `CAR_SWTN` sample 0 | `Vol_ShiftSweets x w(RPM) x mixer`, `w` = Q15 ramp 26000 at 4000 RPM and below to 32767 at 7000 and above |
| engage sweetener | start of the up engage, `RPM_AtShift >= 7000` | `CAR_SWTN` sample 1 | same, with `RPM_AtShift` |
| accelerate sweetener | start of an accelerate attack (§2) | `CAR_SWTN` sample 1 | same |
| engine-off sweetener | throttle release at 6000 RPM or more, §2 | `CAR_SWTN` sample 0 | same |
| reverse whine | gear id 0 (reverse) | `SweetBank[1]` = `CAR_WHINE_00.abk`, `CAR_WHINE` | mixer volume (input 8) x `PhysicsRPM / MaxRPM` |
| brake mash | local player, brake pedal reaches 1.0 from 0, more than 1 s after the last, speed above 5 mph; ends when released | `FX_SHIFTING_01` sample 2 | mixer volume (input 10) |
| transmission loop | the engine object, when `engineaudio.Tranny` and the car is the player's | `CAR_TRANNY.abk`, `CAR_TRANNY`: `magnitude = speed (m/s) * 15`, `Single_Shot = 1` while a shift runs | mixer volume (input 3) |

The sweeteners of a car are in its own bank, which holds 12 sounds for `SWTN_CAR_66_MB.abk` **[verified]**
(the sample ids 0 and 1 are the `CAR_SWTN` Csis ids; running the module shows it plays bank sound `id + 1`,
[engine-sound-aems.md](engine-sound-aems.md) §4).

## 2. Accelerate transitions

`SFXCTL_AccelTrans` adds the throttle-stab rev (not for AI racers). A car counts as accelerating when the
throttle exceeds 30 %. On the edge *not accelerating -> accelerating* one of two transitions may start
(`acceltrans` data of the engine's set) **[decomp]**:

- **From idle** (`IDLE_REVING`): when speed is at most 15 mph, the throttle rose by at least 30 points since the
  previous update, no transition or shift is running, gear is first, `PhysicsRPM <= 1500` and no
  cut-scene: `EngRPM` interpolates `PhysicsRPM -> PhysicsRPM + AccelFromIdle_PEAK_RPM` over
  `AccelFromIdle_PEAK_T` ms (linear) with an engine-volume ramp `0 -> AccelFromIdle_PEAK_VOL`; then
  `IDLE_ENGAGING`: `-> PhysicsRPM` (live) over `AccelFromIdle_RESUME_T` ms while the volume ramps back to 0.
  Data: peak RPM 2500 to 8500, peak time 260 to 700 ms, resume time 400 to 1500 ms, peak volume 0.2 to 0.6.
- **Attack** (`ATTACK`): when the throttle rose by at least 30, nothing is running, `PhysicsRPM >= 3000`, 2 s have
  passed since the last one, and (AI or gear at least second): `EngRPM` interpolates
  `PhysicsRPM + 1000 -> PhysicsRPM` (live) over 500 ms with `EQ_PWR_SQ`, a volume pulse `0.8 -> 0` over 200 ms,
  torque pinned at 100 for 10 ms, and the accelerate sweetener is queued. The engine mix does not smooth
  its volumes during the attack.

Releasing the throttle during a transition makes it `INTERRUPT`: `EngRPM` interpolates to `PhysicsRPM` over
`AccelFromIdle_INTERUPT_T` (700 ms in all data) with `EQ_PWR_SQ`. Releasing the throttle (a falling edge) when
no transition or shift runs, `PhysicsRPM >= 6000` and the gear is at least second queues the **engine-off
sweetener** (§1.6). The state publishes no mixer inputs.

## 3. Redlining

See [engine-sound.md §7](engine-sound.md#7-redlining).

## 4. Turbo and supercharger

`CARSFX_Turbo` (local player and others that carry the object), bank `turbosfx.BankName`, Csis `FX_TURBO_01`
(`SOUND/EVT_SYS/TURBO.csi`). If the car's `turbosfx` collection is the `default` one, the object **disables
itself** (cars without forced induction, such as the BMWM3GTR) **[decomp]**. Per update:

```
turbo      = EngTorque > 20 ? EngTorque * 0.01 : 0              # EngTorque is the throttle in percent
if turbo > 0.01: charge += turbo   else: charge -= Leak_Rate     # Leak_Rate: 0.5 in all data
rpm_scale  = clamp((PhysicsRPM - 1000) * 0.01, 0, 1) * 0.6 + 0.4
limit      = ChargeTime * rpm_scale
charge     = clamp(charge, 0, min(ChargeTime, limit))
spool      = charge / ChargeTime                                 # 0..1, called SpoolPercent
spool_vol  = Vol_Spool, ducked after a peak (below)
```

`ChargeTime` is 11 to 20 updates in the data (11 = the Mustang supercharger, 15 = generic supercharger, 20 for
turbos) and `Vol_Spool` 5000 to 13500. When `charge` reaches `limit` the spool volume holds for 1000 ms and then
ducks as `Vol_Spool * cos(x * 90 deg)` with `x = 307.2 / 512 * (elapsed / 2000 ms)` (to 59 % over 2 s); it
rearms when the charge drops. The **spool loop** (`FX_TURBO_01` id 0) runs all the time the object is enabled,
with `volume = spool_vol * mixer(1)`, `PSI = round(spool * 1024)`, `RPM = PhysicsRPM`,
`rotation = PhysicsTRQ * 10.24`; how PSI and RPM shape the whine is in the bank.

State machine: `NONE -> SPOOLING` when `EngTorque > 20`; `SPOOLING -> BLOWOFF` when it falls below 20: play the
**blow-off** (`FX_TURBO_01` id 1, or id 2 or 3 chosen at random when `spool > 0.75`) with volume
`Vol_Blowoff1` (id 1) or `Vol_Blowoff2` (ids 2 and 3) `* spool`, then reset the charge to 0; `BLOWOFF -> NONE` when
the sample ends. If the throttle goes above 20 % while a blow-off plays (and no shift), its volume ramps to 0
over 150 ms. `Vol_Blowoff1/2` are 0 to 26000 (0 = no blow-off, e.g. the Stage-1 superchargers). Mixer inputs
2 and 3 scale the blow-offs, 1 the spool. Turbo banks: 32 files in `SOUND/TURBO/` (`TURBO_TUN_*`,
`TURBO_EXOTIC_*`, `TURBO_SC_*`, ...), 6 sounds in the ones inspected **[verified]**.

## 5. Nitrous

`CARSFX_Nitrous`, bank `Nitrous_00_MB.abk`, Csis `FX_NITROUS` and `FX_PURGE` (`MAIN_AEMS.csi`) **[decomp]**:

- Rising edge of `nos_active`: start `FX_NITROUS` (type 0; volume = mixer input 1, azimuth = input 0, pitch and
  low-pass from inputs 3 and 4) and set mixer input 1 to 32767 (0 in a cut-scene).
- Falling edge: set `NIT_STOP = 1` on the instance and mixer input 1 to 0; the instance is deleted when its
  reference count drops.
- `nos_empty` (tank ran dry): clear the flag and play one `FX_PURGE` (volume = mixer input 2).
- A pitch-boost interpolator rises `0 -> 1` over 400 ms on start and falls over 1000 ms on stop; where it is
  read is in code not present in the sources **[unconfirmed]**.

The NOS engine boost itself is physics (`nos` collections). The AI engine mirrors `nos_active` as
`IsAccelerating = true`.

## 6. Spark chatter (backfire) and the idle mixer inputs

`CARSFX_SparkChatter` (the car's `SweetBank[0]`, Csis `CAR_Sputter`, volume `engineaudio.Vol_Sputters` x
mixer input 1) feeds `RPM = PhysicsRPM`, `TORQUE = PhysicsTRQ * 10.24`, `accel_true`, `shifting_true` to the
AEMS engine each update; the class decides when it pops. The tuner car can force a pop (`Force_Trigger`) in
first gear or neutral. The hybrid motor publishes two mixer inputs: input 0 = 32767 after 3 s of steady
speed over 30 mph (smoothing 983 up, 196 down per update), and input 1 = like `pct_acc` but from the last recorded RPM change without the +10
offset (smoothing 3000 per update), held at half of its value from the instant the redline starts, and
unchanged while shifting or airborne.

## 7. Tire skids

`CARSFX_Skids` (bank `SKID_BIG_MB.abk`, Csis `FX_SKID` of `MAIN_AEMS.csi`; `SKID_SML_MB.abk` for level 1,
`SKIDS_DRIFT_*` in the drift race type via the type parameter 1) **[decomp]**. Inputs per update:

```
for each wheel w (0 FL, 1 FR, 2 RR, 3 RL):
    fwd_slip   = physics_slip_forward  with a dead zone of 0.2 * tolerated_slip, shifted to zero
    side_slip  = - physics_skid (lateral)
    norm_fb[w] = clamp(fwd_slip  * 200, -1023, 1023)
    norm_lr[w] = clamp(side_slip *  81, -1023, 1023)
    load[w]    = ramp(wheel_load, 4000 .. 10000) * 1023
front = mean of wheels 0 and 1, back = mean of wheels 2 and 3   (all zero if neither side touches the ground)
send: Front_FB, Front_LR, Front_Load, Back_FB, Back_LR, Back_Load, SPEED (mph), UNDERSTEER, OVERSTEER,
      Surface = max(left skid type, right skid type)
```

The surface skid type is the `Aud_Skid_Type` field of the wheel's `simsurface` (a blown tire uses the
`blown_tire` surface for its side). The mixer inputs derived from the instance's own outputs:
`fwd = clamp(max(|Front_FB|, |Back_FB|) << 5, 0, 32767)` (inputs 0 and 2), `side` the same from `_LR` smoothed
500 per update (input 1), and `load = max(fwd, side)` smoothed 3000 per update (input 3). The volumes
(`VOL_Fwd`, `VOL_Side`, `VOL_Back`), the wet FX level and the pitch offset sent to `FX_SKID` come from mixer
outputs 5 to 9. When a loop's lateral or forward component exceeds its threshold is in the bank/`.csi`.

## 8. Collisions

The physics raises an *audio event* with a position, normal, velocity, magnitude 0 to 1 and the `audioimpact`
or `audioscrape` collection named by the car's `pvehicle` link (`OnHitGround`, `OnHitWorld`, `OnHitObject`,
`OnBottomOut`, `OnScrapeGround`, `OnScrapeWorld`, `OnScrapeObject`, `OnBottomScrape`: arrays of 32-byte
linkage records) **[decomp]**.

- Drop the event if no collection is linked, sounds are off, or the event is more than 100 m from every view.
- `intensity = int(clamp(magnitude, 0, 1) * 127)`. `DESCRIPTION[ ]` strings of the collection set flags:
  `SMOKABLE` 0x2, `WALL` 0x4, `TWO_CAR` 0x8, `CAR` 0x10, `ROLLOVER` 0x20, `BOTTOMOUT` 0x40, `FRONT` 0x80,
  `SIDE` 0x100, `EVENT` 0x400; flag 0x1 means the camera's target car is involved; 0x200 marks a scrape.
  Impacts with `WALL` and `SMOKABLE` together at `intensity <= 9` are dropped.
- **Impact:** the collection has up to four lists `STITCH_LEVEL_0..3` of stitch sample ids and
  `Volumes.Vol1..Vol4`. `n` = the number of non-empty lists; `level = clamp(int(magnitude * (n - 1) + 0.5), 0, n - 1)`;
  the sample is `STITCH_LEVEL_<level>[counter % len]` with a running counter; the volume is `Vol<level+1>`.
  The sample is played by the stitch system from `IG_GLOBAL/Stich_Collision_MB.abk` (171 sounds **[verified]**)
  with the stitch tables in `GLOBAL/InGameB.bun` (720 `SndStichData`, undocumented).
- **Volume slot** (a mixer output; the maps decide the level): `SMOKABLE | CAR | EVENT (0x412)` smackable car;
  `SMOKABLE` smackable world; for `CAR`: with `WALL` front (`FRONT`) or side wall, else `BOTTOMOUT`, `ROLLOVER`,
  car side (`SIDE`), car front (`FRONT`). Final volume is `mixer(slot) * collection volume >> 15`. A
  first update sets mixer inputs by kind (car-car 0, smokable 2, other 1) and input 4 to `intensity * 255`.
  Wall impacts also send an audio reflection message ("front barrier hit") to the effects system.
- **Stream sweetener:** a primary car-vs-car impact (or an `EVENT`) takes the first (lowest index) `StreamSweetner[ ]` entry
  with `Threshold <= intensity` and starts that moment stream (`aud_moment_strm`), unless world streaming is
  busy.
- **Scrape:** one `FX_Scrape` instance per event while the event is active; volume, pitch, azimuth, wet level
  come from mixer outputs; `Impulse_magnitude = intensity`; low-pass 25000. Fades out over 0.25 s after the
  event ends.
- **Landing / bottom out** (`CARSFX_BottomOut`): hang time is tracked per axle pair (front wheels 0+1, rear 2+3,
  right 1+2, left 0+3). A landing sounds when an axle pair touches again after more than 0.12 s in the air
  (a "hard" landing after 0.7 s); a heavily leaning car (up vector dotted with world up below 0.8 and no
  wheel down) landing on any side counts as hard. Intensity is
  `4 * ramp(compression_front_or_rear_sum, 0 .. 0.65) * 63.5` (AI cars use 127). The sample is a stitch sample;
  the volume is one of `{13000, 15000, 24000, 32767}` by `clamp(intensity >> 5, 0, 3)`.
  A bottom-out message from the physics plays a stitch sample chosen by intensity at mixer 1.

## 9. Road noise and wind noise

Two more continuous car sounds, read from `CARSFX_RoadNoise` and `CARSFX_WindNoise` **[decomp]**. Their final
levels are these generated volumes times the mixer map's levels ([car-sound-mixer.md](car-sound-mixer.md)).

**Road noise** (one loop per side, left = wheels 0 and 3, right = wheels 1 and 2; the loop sample is the
`simsurface` field `Aud_Roadnoise_LOOP` of that side's surface, the loop enum with 0 = gravel, see
[car-sound-mixer.md](car-sound-mixer.md) §3). With `speed` in mph and
`slip_l`, `slip_r` the length of the summed `(forward, lateral)` physics slip of the grounded wheels of a side,
`traction_l` / `traction_r` the mean absolute traction usage of the side's two wheels:

```
base   = graph(speed): (0, 0) (60, 28000) (100, 32500) (150, 24500) (175, 18000)        # Q15, linear between points
vol_L  = base * (1 + min(slip_l * 0.01, 0.15)) * (1 + min(traction_l * 0.1, 0.1))
vol_R  = base * (1 + min(slip_r * 0.01, 0.15)) * 1.1
vol    = min(vol, 32000);  0 while the side has no wheel on the ground
pitch0 = 1500 + 3000 * clamp(speed / 100, 0, 1)
pitch_L = min(pitch0 * (1 + min(slip_l * 0.01, 0.2)) * (1 + min(traction_l * 0.15, 0.15)), 6000)   # R alike, with its own slip/traction
```

The pitch is on the same 4096 = 1.0 scale as the other pitch values. The right side's traction term is absent in
the volume (only the left uses it), which looks like an oversight; it is kept. A change of surface starts the
new loop and plays a short transition sample (`Aud_RoadNoise_TransON/OFF`), a punctured or blown tire plays a
transition too. Dirt and gravel add a secondary noise flag.

**Wind noise.** `v` = speed in m/s clamped to 2 .. 40: `ratio = v / 40`, `volume = ratio * 32767` (the weight of
all three wind layers), `intensity = ratio * 1040` (crossfade weight handed to `FX_WIND`). Two wind sources circle
the car at a radius `(1 - ratio) * 65` m (not less than the car's bounding sphere) at an angle of
`1280 + ratio * 12288` (of 65536) to the left and right of the heading; that is spatial placement for the game.
A separate weather wind is scaled `1 -> 0.25` from 0 to 70 mph.

## 10. Decisions for the Rust implementation

`libs/blackbox-carsound` makes these choices where the sources leave one **[decision]** (the engine ones are at
the end of [engine-sound.md](engine-sound.md)):

- **Commands.** Everything audible is returned as plain commands: `Play` (fire and forget), `PlayVoice` +
  `SetVoice` + `Stop` (a one-shot the game keeps a handle of, for the blow-off ramp) and `SetLoop` + `Stop` (a
  looped voice, updated each call). Volumes are linear 0..1, pitch is a playback ratio (1 = unchanged), pan -1..1.
  The game resolves a `SoundRef` to a bank sound; volumes of one-shots already include the tuning level.
- **Sweetener RPM.** The accelerate and engine-off sweeteners read `RPM_AtShift`, which is stale (the last
  shift's) in the sources. The Rust code uses the `PhysicsRPM` at the moment instead.
- **Reverse whine.** Volume is the audio RPM fraction (the original divides the sound-scale RPM by the physics
  maximum, which exceeds 1); the pitch rises with it.
- **Transmission loop** (`CAR_TRANNY`) is not produced: its response to `magnitude` is bank data.
- **Turbo.** The spool duck rearms when the charge drops below the limit (the original compares with the full
  `ChargeTime`, which re-arms the duck every other update when the RPM scale is below 1). The blow-off sample
  ends after `TurboTuning::blowoff_seconds` (the game fills it with the decoded length). The spool loop's pitch
  follows `0.8 + 0.4 * spool` (the bank derives its own from `PSI`).
- **Nitrous.** The pitch boost of §5 scales `1 + 0.08 * boost` on the loop.
- **Skids.** The slip normalisation and the `fwd`, `side`, `load` smoothing of §7 are kept. The loop volume of an
  axle is `max(fwd, side) * (0.3 + 0.7 * wheel_load)` (all 0..1), the pitch `0.9 + 0.2 * max(fwd, side)`, and
  the voice stops below 0.01. The surface is the highest `Aud_Skid_Type` of the grounded wheels (a blown tire
  uses the tuning's blown-tire surface).
- **Collisions.** The game turns physics events into `CollisionEvent`s (with the `audioimpact` data it looked up);
  the lib picks the level, the sample counter, the volume class and the moment-stream sweetener. A scrape is
  a per-kind magnitude each call (0 = ended) that fades out over 0.25 s.
- **Landing.** As §8; the `Z force` of a wheel is its `compression`.
- **Random choices** (blow-off sample, compression bump) come from a seeded xorshift generator.

## 11. What the Rust game does with it

Implemented in `libs/blackbox-carsound` (`EffectsMixer`) and `crates/nfsmw/src/audio/` (the bank lookup in
`refs.rs`, the voices in `fx.rs`). Where this differs from the text above:

- **Not played:** the moment streams of an impact (`StreamSweetner`), the `FX_SKID` wet level and pitch outputs
  of the mixer maps, the transmission loop, the weather wind, road noise transition samples, and the pitch boost
  of a blown tire. The sputters are played by the `CAR_Sputter` module (`crates/nfsmw/src/audio/aems/`,
  [engine-sound-aems.md](engine-sound-aems.md) §5), with the tuner car's forced pop (`Force_Trigger`) left out;
  the sweeteners play bank sounds `id + 1` of the car's sweetener bank directly.
- **Which bank sound** each effect plays is listed in [audio.md](../formats/audio.md#which-bank-sound). Q1 is
  answered by the order and the durations in the banks, not by the `.csi` files, and the choices marked
  unconfirmed there are guesses.
- **Collisions.** The physics gives the hit's impulse (wall hits: 30,000 N s is full volume, below 800 N s is
  silent, hits closer than 0.2 s are one), the front or side of the car, and the light props it knocks over (a
  `HitWorld` of 0.15 to 0.6 by mass). A scrape is a wall rubbed at more than 3 m/s. The stitch is looked up in
  `InGameB.bun` (Q3 answered, see [audio.md](../formats/audio.md#sound-stitches)); `StreamSweetner` is read nowhere.
- **Landings** play the `OnBottomOut` collection of the surface under the car.
- **Loops** (turbo, nitrous, tires, road, wind, scrape, reverse) keep one voice each; a change of volume or pitch
  is smoothed over 60 ms and a stop fades over 120 ms.
- **Mixer levels.** Every command's volume and pitch is multiplied by the level the mixer map gives that sound
  (Q2 answered: [dynamic-mixer.md](dynamic-mixer.md), [car-sound-mixer.md](car-sound-mixer.md)); the engine's
  pitch multiplier is the map's engine pitch slot. Collisions keep their own levels. Without the map the road
  noise plays at 0.35 and the wind at 0.4 and the rest at the generated volume.
- **Telemetry the physics lacks:** a blown tire (never), the nitrous tank empty flag (set when the nitrous key is
  held on an empty tank) and `pre_race`.

## How to check it

- **Shifting:** record `EngRPM` over a 2nd to 3rd shift at 8000 RPM in the PC game (a debugger on the RPM
  passed to the Ginsu update) and compare with the stage times of the pattern (§1.2). The BMWM3GTR pattern
  `0x6EB87040` should give two drops of 1500 and 1200 RPM over 210 ms each, then a 300 ms engage.
- **Turbo:** with a turbocharged car (`skylinezt`) hold full throttle then lift: expect a blow-off at the
  lift whose volume scales with the spool (reaches id 2 or 3 above 75 %).
- **Banks:** every bank named by the sets exists in `SOUND/` (done for engine, sweeteners, shifting, turbo:
  **[verified]**).

## Open questions

- **Q1** Which sample each Csis id picks, and how `PSI`, `RPM` and `rotation` shape them, is data inside the `.abk` and
  `.csi` files; not decoded (the Rust code picks by bank order, section 11).
- **Q2** *(answered)* The mixer maps: [dynamic-mixer.md](dynamic-mixer.md); the filter and azimuth slots are not applied.
- **Q3** *(answered in part)* The stitch tables in `InGameB.bun` are read ([audio.md](../formats/audio.md#sound-stitches)); what
  the second piece field does is not known.
- **Q4** Whether the PC build changes any constant here (timings were tuned for the console builds).
