# Car engine sound: files, telemetry and the engine mix

How the original maps a car's state to its engine sound, and which files and AttribSys collections a
car uses. This file covers the engine itself: which files belong to a car, what the sound system reads from
the physics, how the engine's audio RPM and the mix of its layers are computed. The grain synthesis is in
[engine-sound-ginsu.md](engine-sound-ginsu.md); shifting, turbo, nitrous, skids and collisions are in
[engine-sound-effects.md](engine-sound-effects.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), under `src/Speed/Indep/Src`: `EAXSound/CARSFX/CARSFX_Engine.{hpp,cpp}`,
  `EAXSound/sfxctl/SFXCTL_{HybridMotor,Engine,Physics}.{hpp,cpp}`, `EAXSound/EAXCar.{hpp,cpp}`,
  `EAXSound/EAXCarState.hpp`, `EAXSound/SoundConn.{h,cpp}`, `EAXSound/UG/NFSUG_CarsSFXLoadData.cpp`,
  `EAXSound/States/Managers/STATEMGR_{PlayerCar,AICar,CarState}.cpp`, `EAXSound/EAXSndUtil.{h,cpp}`,
  `EAXSound/EAXAemsManager.cpp` (path table), `Physics/Behaviors/SoundCar.cpp`,
  `World/VisualTreatment.cpp` (the spline helper), `Misc/Table.cpp` (running average),
  `Generated/AttribSys/Classes/{engineaudio,shiftpattern,turbosfx,acceltrans,pvehicle,audiosystem}.h`,
  `Libs/snd/9/include/snd/sndo.h` (units). Read for understanding; no code copied.
- **Data inputs:** AttribSys `GLOBAL/ATTRIBUTES.BIN` classes `pvehicle`, `engineaudio`, `acceltrans`,
  `shiftpattern`, `turbosfx`, `audiosystem` (`mostwanted` collection) and, for the physics side, `engine`.
  Field layouts: [formats/audio.md](../formats/audio.md#tuning-classes).
- **Tags:** as in the [docs README](../README.md#evidence-tags). Anything marked **[verified]** was measured
  on the PC v1.3 install; everything else is **[decomp]** (GameCube build, so the PC build may differ).

## 1. What plays for a car

A car's engine sound is two layers started together **[decomp]**:

1. The **AEMS layer**: sample-based engine loops in an `.abk` bank (`SOUND/ENGINE/CAR_nn_ENG_MB_EE.abk`),
   driven by EA's AEMS event system through the Csis class `CAR` (`SOUND/EVT_SYS/ENGINES_AEMS2.csi`). Which
   samples play for which RPM and torque, and how they cross-fade, is **inside the bank and the `.csi`**, not
   in code; the game only feeds parameters (RPM, torque, volume, pitch offset, azimuth). Not specified here.
2. The **Ginsu layer**: one (single mode) or two (dual mode: accelerate and decelerate) granular `.gin`
   loops, synthesised in code ([engine-sound-ginsu.md](engine-sound-ginsu.md)). This is the part we can
   reimplement exactly.

Who uses which **[decomp]**: the local player's car creates the *single* Ginsu object (the accelerate `.gin`
only; flag bits 0x53BFA of `STATEMGR_PlayerCar` = engine, shifting, turbo, nitrous, skids, ...). AI racers
create the AEMS-only engine (their flags 0x91) unless a static switch `bUsingGinsu` is set, which nothing in
the decomp sets. Cops use the AEMS-only engine. Traffic uses a separate class, `FX_TRAFFIC`, picked by
`pvehicle.TrafficEngType`. The *dual* object exists (accelerate and decelerate loops) and the data is tuned
for it (every `engineaudio` has `DECEL_*` and `GINSU_Decel_*` values), but no state manager creates it in
this build. The PC build may differ **[unconfirmed]**; the Rust game should use dual mode, since both files
are on disk and the decelerate side is tuned (§5.4).

## 2. From a car to its sound files

```
pvehicle/<car>                               e.g. pvehicle/bmwm3gtr   (the car type name, lower case)
 ├─ engineaudio[ i ]  RefSpec ──► engineaudio/<set>      i = engine upgrade level (§2.1)
 ├─ ShiftSND[ j ]  {Item: RefSpec, Level}  ──► shiftpattern/<id>   j = transmission level
 ├─ TurboSND[ k ]  {Item: RefSpec, Level}  ──► turbosfx/<id>       k = induction level
 └─ engine / transmission / nos / ...  (physics; not sound)

engineaudio/<set>
 ├─ Filename_GinsuAccel / Filename_GinsuDecel        SOUND/ENGINE/<name>   (.gin, Gnsu v2)
 ├─ BankName_mainRAM        SOUND/ENGINE/CAR_nn_ENG_MB_EE.abk   (the AEMS layer, "EE" = main memory)
 ├─ BankName_auxRAM[ ]      SOUND/ENGINE/CAR_nn_ENG_MB_SPU.abk  (same sounds for the SPU RAM; PS2 split)
 ├─ SweetBank[ ]            SOUND/ENGINE/SWTN_CAR_nn_MB.abk, CAR_WHINE_00.abk   (shift/accel sweeteners, whine)
 ├─ acceltrans              RefSpec ──► acceltrans/<id>
 └─ the tuning values of §5 and the AEMS class id CarID (= the nn in the file names)
```

All names are strings stored in the data; the file is `SOUND/` + the folder of §2.3 + the name,
case-insensitive. **[verified]** on the install for the 70 `engineaudio` collections: all 129 `.gin` files
they name, all main, aux and sweetener banks exist in `SOUND/ENGINE/`; 31 of the 160 `.gin` files are not
referenced by any collection (cut cars; see [formats/audio.md](../formats/audio.md#gnsu-granular-engine-sounds-gin)).

**Example, BMWM3GTR [verified]:** `pvehicle/bmwm3gtr` has `engineaudio = [tvr_cerb]`, so the sound set is
`engineaudio/tvr_cerb`: accelerate loop `GIN_TVR_Cerbera.gin`, decelerate loop `GIN_TVR_Cerbera_DCL.gin`,
bank `CAR_66_ENG_MB_EE.abk` (aux `CAR_66_ENG_MB_SPU.abk`, sweeteners `SWTN_CAR_66_MB.abk` and
`CAR_WHINE_00.abk`), `CarID` 66, `MinRPM` 1500, `MaxRPM` 7784, `Tranny` true (so `CAR_TRANNY.abk` is used).
Its shift pattern is `shiftpattern/0x6EB87040` (bank `GEAR_MED_Lev3.abk`) and its turbo is
`turbosfx/default` (bank `TURBO_TUN_SML_0_MB.abk`; the default set switches the turbo sound off, see
[the effects spec](engine-sound-effects.md#4-turbo-and-supercharger)).

### 2.1 Which `engineaudio` entry

`pvehicle.engineaudio` is an array with one entry per audio upgrade level of the engine; most stock cars have
one entry. With `n_up = engine_upgrades` (the number of physics upgrade levels, 0 to 4), `cur = engine_current`
(the player's installed level, a save-game value, not stored in the database) **[decomp]**:

```
base   = clamp(4 - n_up, 0, 4)
offset = clamp(cur, 0, 4)
level  = 0                                   if base >= 3
       = (1 if base + offset >= 3 else 0)    if 1 <= base <= 2
       = 2 if base + offset > 2, else 1 if > 0, else 0      if base == 0
index  = clamp(level, 0, len(engineaudio) - 1)
```

AI racers instead use `cur = max(0, playerUpgrade - base)` where `playerUpgrade = clamp(base + cur, 0, 4)` of
the local player. Stock cars (`engine_upgrades` 0) always take entry 0. The data has 1 to 3 entries per car
(for example `corvette`: `corvette_z06`, `corvette_z06_v2`; `tt`: `bmw_m3_a`, `bmw_m3_c`, `bmw_m3_e`)
**[verified]**.

`ShiftSND` and `TurboSND` are arrays of `{Item, Level}` (16 bytes: a 12-byte RefSpec, then a `u8` level,
padded). Pick the last entry whose `Level` is not above `cur_upgrade`, where
`cur_upgrade = 4 - upgrades + current` of the matching physics class (`transmission_*` for shifting,
`induction_*` for turbo); entry 0 if none qualifies **[decomp]**. The shift pattern of the first local car
and its turbo set are cached and reused for later cars of the session (`g_ShiftInfo`, `g_TurboInfo`).

### 2.2 Which banks load for the player's car

`SetupLoadData` of each sound object **[decomp]**:

| Object | Loads |
|---|---|
| single Ginsu engine | `BankName_mainRAM`, `Filename_GinsuAccel` |
| dual Ginsu engine | `BankName_mainRAM`, `Filename_GinsuAccel`, `Filename_GinsuDecel` |
| AEMS engine | `BankName_mainRAM` (if `BankName_auxRAM` is non-empty) |
| shifting | `shiftpattern.BankName` (`SOUND/SHIFTING/GEAR_*.abk`) and every `SweetBank[ ]` entry |
| spark chatter (backfire) | `SweetBank[0]` |
| turbo | `turbosfx.BankName` (`SOUND/TURBO/TURBO_*.abk`) |
| nitrous | `audiosystem/mostwanted.AEMS_NOSBanks[0]` = `Nitrous_00_MB.abk` (`SOUND/NOS/`) |
| skids | `AEMS_SkidBanks[ clamp(level, 0, 1) ]` = `SKID_BIG_MB.abk` or `SKID_SML_MB.abk` (`SOUND/SKIDS/`) |

The nitrous, turbo, tire and transmission "upgrade levels" of `EAXCar` start at 0 and nothing in the decomp
changes them (only the engine level is derived), so level 0 is what these pick: `Nitrous_00_MB.abk` (the
install also has `Nitrous_01_MB.abk`, selected for levels 2 and 3) and `SKID_BIG_MB.abk`.

### 2.3 Folders and global banks

Path table of `EAXAemsManager`: `sound\`, `sound\Engine\`, `sound\evt_sys\`, `sound\FE\`, `sound\Global\`,
`sound\IG_Global\`, `sound\NOS\`, `sound\PFData\`, `sound\Skids\`, `sound\Speech\`, `sound\Turbo\`,
`sound\Shifting\`, `sound\FXEdit\` **[decomp]**. The per-game bank lists are the `audiosystem/mostwanted`
collection **[verified]**: `AEMS_SkidBanks` (`SKID_BIG_MB`, `SKID_SML_MB`, `SKIDS_DRIFT_BIG_MB`,
`SKIDS_DRIFT_MED_MB`), `AEMS_NOSBanks` (`Nitrous_00_MB`, `Nitrous_01_MB`), `AEMS_StitchBanks`
(`Stich_Collision_MB`, `Stitch_Whoosh_MB`, `Stitch_Static_MB`), `AEMS_MiscBanks` (`Siren_MB`, `FX_Rain_MB`,
`TRAFFIC_MB`, `FX_Misc_MB`, `FX_Helicopter_MB`, `FX_Camera`, `CAR_99_ENG_MB_EE`, `CAR_TRANNY`),
`AEMS_RNBanks` (`ROADNOISE_00_MB`), `AEMS_WNBanks` (`WIND_00_MB`), `AEMS_EnvBanks` (`ENV_COMMON_MB`),
`AEMS_FEBanks` (`FE_MB`, `FE_COMMON_MB`) and `EvtSys` (`MAIN_AEMS.csi`, `FE_AEMS.csi`, `ENGINES_AEMS2.csi`,
`TURBO.csi`, `ENVIRO_AEMS.csi`, `STITCH_AEMS.csi`, `COP_SIREN_AEMS.csi`). The banks of one family have
identical layouts per type (for example all 16 shifting banks hold 3 to 4 sounds).

## 3. Telemetry the sound system reads

Once per frame the physics fills a packet (`Pkt_Car_Service`) for each car; the sound system copies it **[decomp]**.
Fields used by the engine, shifting and effects:

| Field | Source | Notes |
|---|---|---|
| `rpm_pct` | `clamp((rpm - IDLE) / (RED_LINE - IDLE), 0, 1)` of the car's `engine` collection | 0 when `RED_LINE <= IDLE` |
| `throttle` | `clamp(fGas, 0, 1)` | pedal, 0 to 1 |
| `brake`, `ebrake`, `steering` | `fBrake`, `fHandBrake` (0 to 1), `fSteering` (-1 to 1) | |
| `gear` | transmission gear id: 0 reverse, 1 neutral, 2 first, 3 second, ... | a change sets a per-frame "shift" flag |
| `nos_active`, `nos_capacity` | nitrous engaged, remaining fraction | |
| `engine_blown` | 0 none, 1 blown, 2 sabotaged | plays a moment stream once |
| per wheel (4) | `on_ground`, `traction` (becomes `1 - traction`), `slip` (forward, lateral), `load`, `compression` (called Z force), `surface`, `tire blown state` | wheel order: front left, front right, rear right, rear left |
| `oversteer`, `understeer`, `slip_angle`, `health` | chassis | |
| body | world matrix, velocity (m/s; speed in mph = m/s x 2.2369) | |
| `visual_rpm` (in) | the sound system hands back its own audio RPM as a fraction | drives the tachometer; see §4.4 |

The engine's **turbo boost** (`Engine::mBoost`) exists in the sound state but is never written by the
packet in this build; the turbo sound is driven by the engine torque instead (§4 of the effects spec).

## 4. The engine's audio RPM

All engine-sound RPM values live on a "sound scale" of **1000 to 10000**, independent of the car's real RPM.

### 4.1 Physics RPM on the sound scale

Once per update (`PhysicsRPM`, `PhysicsTRQ`) **[decomp]**:

```
throttle_pct = throttle * 100                                       (player; AI cars are synthesised, see below)
PhysicsTRQ   = slew(PhysicsTRQ -> throttle_pct, max step 100 per update)     # i.e. it equals throttle_pct
r            = rpm_pct
if player car: r = bezier_y(PhysicsRPM_Map, r)                      # the per-engine remap, §4.2
PhysicsRPM   = r * 9000 + 1000
accelerating = becomes true when throttle_pct > 30 (records the time), false again when it falls to <= 30
```

So "torque" in the sound system is the throttle pedal in percent, not engine torque. The same value is used
by the engine mix (§5) and the turbo (effects spec). For AI racers the sound system invents RPM and throttle
from the car's speed and a state machine (`SFXCTL_AIPhysics`); that is only needed if AI cars get engine
sounds and is not specified here.

### 4.2 `PhysicsRPM_Map`

A 4x4 float matrix per `engineaudio`: four control points `(x, y, 0, 0)` of a cubic Bezier. The remap uses
only the `y` of the four rows and takes `rpm_pct` itself as the curve parameter `t` (not `x`):

```
bezier_y(t) = (1-t)^3 y0 + 3 t (1-t)^2 y1 + 3 t^2 (1-t) y2 + t^3 y3
```

**[verified]** values: the default collection has `y = (0, 0.2, 0.6, 1)` (control points (0,0), (0.33,0.2),
(0.75,0.6), (1,1)); `tvr_cerb` has `y = (0, 0.4136, 0.6893, 1)`. It lifts low and middle RPMs (the audio
revs rise faster than the engine) and is applied only to the local player's car **[decomp]**.

### 4.3 The audio RPM `EngRPM`

`SFXCTL_Engine` turns `PhysicsRPM` into the RPM the layers play (`EngRPM`), with these sources in priority
order **[decomp]** (times in seconds, the interpolators are in [engine-sound-effects.md](engine-sound-effects.md)):

1. a shift in progress: the shift's RPM interpolation (effects spec §1);
2. an accelerate transition in progress: its interpolation (effects spec §2);
3. otherwise `PhysicsRPM`;
4. **clutch model**: for the local player, when no shift or accelerate transition is active and `EngRPM <= 2500`,
   `EngRPM` follows `PhysicsRPM` at most `+999` up and `-60` down per update (it falls slowly at idle);
5. `EngRPM += RPM_LFO + CompressionRPM`, then `SetEngRPM`, which keeps the previous value
   (`PrevRPM`) and a smoothed copy (`smoothed = 0.95 smoothed + 0.05 rpm`).

`CompressionRPM` is a short bump played at random in steady cruising: when `UpdateMixerOutputs` of the hybrid
motor sees speed over 30 mph and `|AvgDeltaRPM| < 30` for 3 seconds it sets a flag every 60 to 210 updates
(random), which makes `CompressionRPM` rise by a random 25 to 100 over a random 25 to 100 ms and fall back
(equal-power curve). It also lifts mixer input 1. `RPM_LFO` is the shift wobble (effects spec §1.5).

`EngTorque` is `Trq` = a 3-update running average of `PhysicsTRQ` (or of the shift or accelerate-transition
torque while those are active), plus the torque LFO (0 unless shifting).

### 4.4 Visual RPM (the tachometer)

The tachometer shows what the audio does. A 2-update average of `VisualRPM` gives
`audio_pct = (avg - 1000) / 9000`; during shifts `VisualRPM` is the shift's visual interpolation, during the
accelerate attack it is `PhysicsRPM`, and during redlining it subtracts a 0 to 200 bounce. For the local
player the value is the physics `rpm_pct` before the race starts (unless it is a rolling start) and blends
from the physics value to the audio value over 0.7 s after the countdown ends. The result is returned to the physics
(`SetVisualRPM`, `GetAudibleRPMPercent`), which turns it into the gauge reading
`idle + audio_pct * (red_line - idle)` **[decomp]**.

## 5. The engine mix

`SFXCTL_HybridMotor` computes, once per update, the Ginsu target frequency and the volumes of the layers
**[decomp]**. Names of `engineaudio` fields are in `monospace`; "S" / "L" in the names mean *steady* and
*large* change of RPM (`S_RPM` = value at steady RPM, `L_RPM` = value at a large RPM change), despite the
`_RPM` suffix; the values are mix levels from 0 to 1.

### 5.1 Ginsu target frequency

```
aems_rpm   = clamp(EngRPM, 1000, 10000)
ginsu_freq = (aems_rpm - 1000) / 9000 * (MaxRPM - MinRPM) + MinRPM        # MinRPM, MaxRPM of the engineaudio
```

`ginsu_freq` is in the `.gin` file's own frequency unit, which is "engine RPM" **[verified]** (§1 of the Ginsu
spec). The file's `[min_frequency, max_frequency]` is close to but not equal to `[MinRPM, MaxRPM]`: the data
are tuned by ear, for example `tvr_cerb` has 1500 to 7784 while `GIN_TVR_Cerbera.gin` covers 1239.5 to
7784.1. A frequency below the file's minimum is handled by the engine object (§6).

### 5.2 Load, and how fast the RPM changes

```
load      = clamp(EngTorque / 35, 0, 1)                    # throttle at or above 35 % = full load
delta     = AvgDeltaRPM                                    # average over 4 updates of the RPM change per update
pct_acc   = clamp(|delta + 10| / AccelDeltaRPMThreshold, 0, 1)
pct_dec   = clamp(|delta + 10| / DecelDeltaRPMThreshold, 0, 1)
```

`AvgDeltaRPM` records, per update, depending on the state **[decomp]**: during shifting the change of the
*audio* RPM (absolute value while upshifting engage or LFO), flushed to `|physics change|` at the instant the
upshift engages; during a down shift's rise the audio change; during an accelerate attack the physics change,
otherwise the audio change; and in plain driving the change of `PhysicsRPM`. `PhysicsRPM - PrevRPM` and
`EngRPM - PrevEngRPM` are per update, so the thresholds (100 to 150 in the data) are per update; see the
open question on the update rate.

### 5.3 Single mode (the player in the decomp)

```
acc.aems   = lerp(AEMSMix_S_RPM,   AEMSMix_L_RPM,   pct_acc)
acc.ginsu  = lerp(GINSUMix_S_RPM,  GINSUMix_L_RPM,  pct_acc)
dec.aems   = lerp(DECEL_AEMSMix_S_RPM, DECEL_AEMSMix_L_RPM, pct_dec)
dec.ginsu  = Ginsu_ACL_Neg_S_RPM               # as written in the decomp (see note)
mix        = lerp(dec, acc, load)              # per component
```

Note: the decomp computes the decelerating accelerate-loop level as
`pct_dec * (Ginsu_ACL_Neg_L_RPM - Ginsu_ACL_Neg_L_RPM) + Ginsu_ACL_Neg_S_RPM`, which is a constant (the
difference of the same field); `Ginsu_ACL_Neg_L_RPM` is never effective. Whether the PC build has the same
slip is unknown; the intent was probably `S + pct_dec * (L - S)` **[unconfirmed]**. Decelerate-loop level is 0 in
single mode.

### 5.4 Dual mode (accelerate and decelerate loops)

```
acc.aems = lerp(AEMSMix_S, AEMSMix_L, pct_acc);   acc.ginsu = lerp(GINSUMix_S, GINSUMix_L, pct_acc);   acc.decel = 0
dec.aems  = lerp(DECEL_AEMSMix_S, DECEL_AEMSMix_L, pct_dec)
dec.decel = lerp(DECEL_GINSUMix_S, DECEL_GINSUMix_L, pct_dec)
dec.ginsu = Ginsu_ACL_Neg_S_RPM                                   # same note as above
f         = decel_window(ginsu_freq)                              # 0..1, below
dec.aems  = (1 - f) + f * dec.aems       # outside the window the AEMS layer is full
dec.decel = f * dec.decel
mix       = lerp(dec, acc, load); then smooth each toward the new value by at most 0.2 per update
```

`decel_window` is a 5-point polyline over the RPM axis from `GINSU_Decel_MinRPM` (`lo`) to
`GINSU_Decel_MaxRPM` (`hi`), with `w = hi - lo`:
`(0, 0), (lo, 0), (lo + w * GINSU_DECEL_FADE_OUT, 1), (hi - w * GINSU_DECEL_FADE_IN, 1), (hi, 0)`
(the first point is the origin, the last returns to 0; outside `[lo, hi]` it is 0). `GINSU_DECEL_FADE_OUT`
(0.25 in most data) is the fraction of the window over which the decel loop fades in at the low end, and
`GINSU_DECEL_FADE_IN` (0.001 to 0.02) the fraction it fades out over at the top, which makes it end abruptly.

### 5.5 Volumes

```
vol_factor    = EngVolume / 32767           # EngVolume = 32767 + 32767 * shiftVolume + VolumeLFO (effects spec §1)
vol_aems      = mix.aems  * AEMSVol       * vol_factor        # single mode: AEMSVol
                                                              # dual mode: AEMSVol and DECEL_AEMSVol blended by `load`
vol_accel     = mix.ginsu * GINSUAccelVol * vol_factor
vol_decel     = mix.decel * GinsuDecelVol * vol_factor
```

Each of the three is then slew-limited to at most **7000 per update** from its previous value (both ways), except
during a shift's engage stage and an accelerate attack, where the new value is taken at once. Then the
redline factors (§7) scale them. Units: the `*Vol` values are 0 to 32767 (Q15); the data range from 12500 to 32767
(`GINSUAccelVol` 20675 to 32767, `AEMSVol` 14500 to 32767). The Ginsu loop's volume handed to the mixer is `clamp((vol_accel * dmix >> 15) >> 8, 0, 127)`,
where `dmix` is the dynamic-mixer output (Q15, 32767 = unity, §8). At unity a `GINSUAccelVol` of 32000
gives 125 on the 7-bit 127 scale **[decomp]**. The 7-bit volume of the underlying sound system is a signed char
(`SNDvol`); its dB curve is not in the sources **[unconfirmed]**, so the Rust mixer should treat it as linear.

### 5.6 What the AEMS layer receives

Besides the volumes above (as attenuations `vol - 32767`, so 0 = full), the `CAR` instance gets:
`RPM` = the scaled RPM `(EngRPM - 1000) * (MaxRPM - MinRPM) / 9000 + MinRPM` for the AEMS-only engine, or the
**Ginsu synth's own current pitch x 120** (`m_GinsuRPM` feedback, §4 of the Ginsu spec) when Ginsu is active;
`TORQUE = EngTorque * 10.24` (0 to 1024); `MAX_RPM` = the redline-sample volume (`AEMSVol * vol_factor *
RedLineSampFactor`, times the mixer output); `PITCH_OFFSET` (`PitchMultiplier * 16383 - 16383`); `ROTATION`
(an exhaust-view angle value) and `AZIMUTH`. `Master_Vol` (22300 to 32000 in the data) is the engine control's
own mixer input 2.
The gear-whine bank `CAR_WHINE_00.abk` and the `CAR_TRANNY.abk` loop are separate Csis objects (effects
spec §6).

## 6. The Ginsu engine object

Per update, for each loop **[decomp]** (single mode: the accelerate loop only):

```
freq   = ginsu_freq
pitch1 = 1.0
if freq < gin.min_frequency:  freq = gin.min_frequency;  pitch1 = ginsu_freq / gin.min_frequency   # < 1
volume       = clamp((vol_loop * dmix_vol >> 15) >> 8, 0, 127)
playback_rate = pitch1 * PitchMultiplier          # SNDpitchmult(round(rate * 4096)); 4096 = 1.0
low_pass      = GinsuLPFVal   (dual mode: min(GinsuLPFVal, dmix_freq)); a cutoff in Hz
synth.update_frequency(freq, latency = 60 ms)
ginsu_rpm = synth.current_pitch() * pitch1 * 120          # fed back to the AEMS layer
```

`PitchMultiplier = dmix_pitch / 4096` (the dynamic mixer's pitch output, §8). On detach the object sets the
loops' volume to 0; they keep running silently while attached. Both loops of the dual mode get the same `freq`
and playback rate, and both are derived **once, from the accelerate loop's `min_frequency`** (the decelerate
loop's own minimum is never read) **[decomp]**: the decelerate synthesiser clamps the frequency to its own
range internally, so where its recording starts above the accelerate loop's (the M3 GTR set: 2019 against
1239.5) it plays at its lowest spot at natural speed, without the sub-minimum pitch ratio.

`GinsuLPFVal` is 24000 to 25000 (fully open) normally and falls with the camera distance: the engine control
computes `m_DistanceFltr = 725 + 0.7103 * Q15(curve(distance))`, clamped to 0 to 32767, where the curve is the
"down, one minus equal-power, squared" shape (in dB, converted back to Q15) of `clamp((dist - 6.5) / 44, 0, 1)`
and `dist` is the distance to the camera in metres. Beyond about 50 m the cut-off is lowest. For a first
implementation use "open" and apply the low-pass later **[decomp]**.

## 7. Redlining

When `EngRPM > 9800` (sound scale, that is `rpm_pct` above 0.978) and no shift is active (and the car is not
an AI racer), the engine enters the redline state **[decomp]**:

```
t = clamp(gear id, 1, 5)                                  # RedLineDelayPerGear
EngFactor : current -> 0.15 over 450 ms * current * t     (linear)     # multiplies vol_aems and both Ginsu volumes
SampFactor: current -> 0.85 over 120 ms * (1 - current) * t            # multiplies the redline sample (MAX_RPM)
```

and the visual RPM bounces by a 0 to 200 offset (`smooth` by 50 per update toward 200, then back). On leaving
the redline (RPM below 9800 or a shift starts) `EngFactor` returns to 1 over `50 ms * (1 - current)` and
`SampFactor` to 0 over `50 ms * current`. So at the limiter the engine and Ginsu loops duck to 15 % and the
bank's redline sample takes over at 85 %. A "redlining" edge also stores `previousDeltaRPM = output / 2`
for the mixer input described in effects spec §6.

## 8. Dynamic-mixer outputs (not specified)

The `dmix` values above (volume, pitch, low-pass frequency, azimuth, per sound object) come from EA's
dynamic mixer, driven by `SOUND/MIXMAPS/*.mxb` and the shapes in `NFSMixShape` (equal-power curves, dB
tables). The map contents and the code that evaluates them (`NFSMixMap`, 2000 lines) are out of scope. What
is known: the inputs the engine publishes (physics control): speed in mph scaled x1092 (0 to 32767),
x546, x328, x234 (four ranges), `(10000 - PhysicsRPM) * 3.64` clamped to 0..32767 (input 4, the pitch
curve input: maximum at idle, 0 at 10000), accelerating flag, wheels-on-ground x 32767, camera view
(0 for bumper, 4000 for hood, 32767 otherwise). The Rust implementation uses unity volume and pitch
(`dmix_vol = 32767`, `PitchMultiplier = 1`) and applies its own distance attenuation until the maps are read.

## Constants

All **[decomp]**, tuning that is in code rather than data: sound scale 1000 to 10000; `load` ramp 35;
`AvgDeltaRPM` window 4 updates, offset 10; volume slew 7000 per update; mix smoothing 0.2 per update;
engine volume LFO none; Ginsu latency 60 ms; redline threshold 9800, factors 0.15 / 0.85, times 450 / 120 /
50 ms; clutch below 2500, -60 / +999 per update; accelerating above 30 %; `PhysicsRPM_Map` as per
`engineaudio`; `Average` of `Trq` 3 updates, `VisRpmAvg` 2 updates.

## How to check it

- Read `engineaudio` for each car of the install and compare the file names to those in `SOUND/ENGINE`
  (done for all 70 sets: **[verified]**).
- In the PC build, log the (RPM, volume) of the player's accelerate loop while holding a constant throttle
  through a gear (a debugger on `SNDvol`/`SNDpitchmult`); compare with `vol_accel` and `playback_rate`.
- Record a full-throttle pull in 1st gear and compare the Ginsu pitch with `ginsu_freq` (the loop's own
  fundamental is `sample_rate * 120 / period`).
- Check whether a PC car creates the dual object: look for `GIN_*_DCL.gin` reads during driving.

## Open questions

- **Q1** Update rate: the sound update runs once per rendered frame with the frame time `t`; the per-update
  constants (volume slew 7000, averaging windows, clutch, delta thresholds) are only meaningful at a fixed
  rate. Which rate did the PC build use (30, 60)? **[decision]** The Rust game uses a fixed 60 Hz, see the last section.
- **Q2** Single or dual mode in the PC build (§1). The decomp builds only single for the player. **[decision]** Dual.
- **Q3** The `Ginsu_ACL_Neg_L_RPM` slip (§5.3). **[decision]** Keep the constant.
- **Q4** `SNDvol` scale (linear or dB) and the dynamic-mixer curves for volume, pitch and low-pass (§5.5, §8).
- **Q5** AEMS layer: how the bank turns `RPM` and `TORQUE` into samples (needs the `.abk` AEMS tables and
  the `.csi`); not needed while the Ginsu layer carries the engine.

## Decisions for the Rust implementation

Where the sources above leave a choice, `libs/blackbox-carsound` does the following **[decision]**. The
decompiled code was consulted again only to settle these points.

- **Q1, update rate:** the controllers run on a fixed 60 Hz tick. The caller passes real frame times; the mixer
  accumulates them, runs whole ticks (at most 10 per call, the excess time is dropped so a stall cannot cause a
  burst) and returns the last tick's output. All per-update constants (7000 slew, 0.2 smoothing, 4-update
  averages, clutch steps, the delta thresholds) apply per tick as written. Times in ms use the tick time.
- **Q2, mode:** dual mode is the default (`EngineMode::Dual`); single mode is kept for completeness.
- **Q3, `Ginsu_ACL_Neg_L_RPM`:** the constant of the shipped code is kept: the accelerate loop's level off the
  throttle is `Ginsu_ACL_Neg_S_RPM`. The data were tuned against that behaviour.
- **Single mode smoothing:** the shipped code passes the new mix as both arguments of the smoothing call, so single
  mode does not smooth the mix. The Rust single mode smooths like dual mode (0.2 per tick); the volume slew
  still applies.
- **Order inside a tick:** physics (gear, throttle, `PhysicsRPM`, `PhysicsTRQ`), shifting, accelerate transition,
  engine (clutch, LFOs, compression, `EngRPM`, `EngTorque`, volume factor, redline), hybrid motor (`AvgDeltaRPM`,
  mix, volumes, redline scaling), then the cruising compression trigger. A shift or transition therefore acts on
  the same tick it is detected.
- **Gear changes:** the previous gear starts equal to the first gear seen, so a car spawned in gear 3 does not
  "shift" on its first tick. `RPM_AtShift` is the `EngRPM` of the tick before the gear change.
- **Accelerate-transition volume pulse:** `m_InterpEngVol` (the 0 to `AccelFromIdle_PEAK_VOL` ramp and the 0.8 pulse
  of the attack) is never read anywhere in the sources, so it is not applied; only its RPM and torque act.
- **Distance low-pass:** treated as open. The output low-pass is the blended cutoff (25000 accelerate, `GINSU_LowPassCutoff`
  decelerate, smoothed by 6000 per tick) and the caller may lower it by distance.
- **Pitch:** the mixer's pitch multiplier is an input (default 1). If the tuning gives the accelerate loop's
  `min_frequency`, the output frequency is clamped to it and `playback_rate` carries `ginsu_freq / min_frequency`
  (§6); with no minimum given the ratio is 1. The decelerate loop's drive (`EngineOutput::decel_loop`) is a copy of
  the accelerate loop's, as in the original; the game hands each loop its own drive to the voice.
- **Compression bump:** length `25 + rand(100)` ms, height `25 + rand(75)` RPM, gap `60 + rand(150)` ticks, from a
  seeded generator so equal inputs give equal outputs.
- **Tachometer before the race:** the caller says whether the race has started (`pre_race`); when it flips to
  false the 0.7 s blend of §4.4 runs.
- **Volumes:** the three loop volumes and the redline volume are returned as `value / 32767` clamped to 0..1, the
  7-bit `SNDvol` being treated as linear (Q4).
