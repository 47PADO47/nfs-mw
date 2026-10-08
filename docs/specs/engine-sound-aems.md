# Car sound through AEMS: the sample layer of the engine, the sputters and the sweeteners

Which AEMS classes the car sound creates, what it feeds them every update and what the graphs then do. The
evaluator is [aems.md](aems.md); the bank layout [formats/aems.md](../formats/aems.md); the engine mix that
produces the numbers is [engine-sound.md](engine-sound.md) §5 and §5.6.

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), `src/Speed/Indep/Src/EAXSound/`: `CARSFX/CARSFX_{Engine,SparkChatter,Shifting}.cpp`,
  `SND_GEN/ENGINES_AEMS2.h` (the class parameter structures), `UG/NFSUG_CarsSFXLoadData.cpp` (which banks load).
  Read for understanding; no code copied.
- **Data inputs:** `SOUND/ENGINE/CAR_nn_ENG_MB_EE.abk` (class `CAR`), `SWTN_CAR_nn_MB.abk` (`CAR_SWTN`,
  `CAR_Sputter`, `CAR_SputOutput`), `CAR_WHINE_00.abk`, `CAR_TRANNY.abk`; AttribSys `engineaudio`
  (`Vol_Sputters`, `Vol_ShiftSweets`).
- Tags as in the [docs README](../README.md#evidence-tags).

## 1. Which classes and banks

A Ginsu engine (every player car) loads the engine's **main bank** `BankName_mainRAM` (the `_EE` bank; the `_SPU`
twin is the PlayStation 2's second memory and is not used) and creates one `CAR` object with `SPU_or_EE = 1`
**[decomp]**. `SPU_or_EE` makes no difference to the EE bank's graph **[verified]**. The spark chatter loads
`SweetBank[0]` (`SWTN_CAR_nn_MB.abk`) and creates one `CAR_Sputter`; every shift sweetener creates a
`CAR_SWTN` from the same bank, and the sputter's graph creates a `CAR_SputOutput`. The reverse whine is a
`CAR_WHINE` of `SweetBank[1]` (`CAR_WHINE_00.abk`); the transmission loop a `CAR_TRANNY` of `CAR_TRANNY.abk`.

## 2. Parameters

Order of the Csis structures; all are `i32`. Values the game does not set stay as the constructor gave them (0
unless stated).

| Class | Parameters |
|---|---|
| `CAR` (26) | car class, RPM, TRQ_ENG, TORQUE, VOL_ENG, VOL_EXH, TRQ_LFO_AMP, TRQ_LFO_FREQ, RPM_LFO_AMP, RPM_LFO_FREQ, VOL_LFO_AMP, VOL_LFO_FREQ, SPU_or_EE, FX_DRY, FX_AMOUNT, AZIMUTH, PITCH_OFFSET, ROTATION, XOVER_IDLE_2_LO, XOVER_LO_2_MID, XOVER_MID_2_HI, FILTER, FILTER_RND, FILTER_Dist, FILTER_Trig, MAX_RPM |
| `CAR_SWTN` (7) | id, car class, RPM, VOL, AZIMUTH, PITCH_OFFSET, ROTATION |
| `CAR_Sputter` (11) | car class, car id, RPM, VOL, AZIMUTH, PITCH_OFFSET, ROTATION, TORQUE, Force_Trigger, accel_true, shifting_true |
| `CAR_SputOutput` (3) | volume, car class, car id |
| `CAR_WHINE` (9) | car class, RPM, VOL, AZIMUTH, PITCH_OFFSET, ROTATION, LoPass (25000), Wet (0), Dry (0x7FFF) |
| `CAR_TRANNY` (9) | car class, magnitude, VOL, AZIMUTH, PITCH_OFFSET, LoPass (25000), Wet, Dry (0x7FFF), Single_Shot |

"Car class" is the engine set's `CarID` (66 for the M3's `tvr_cerb`).

## 3. What is fed **[decomp]**

**Engine (`CAR`)**, every update of a Ginsu engine, with `dmix1` the mixer map's sample-layer volume (slot 1 of
the engine object, [car-sound-mixer.md](car-sound-mixer.md)) and `V` the hybrid motor's sample-layer volume after
the redline factor (engine-sound.md §5.5):

```
VOL_ENG = VOL_EXH = ((V * dmix1 >> 15) * 0x7FFF >> 15) - 0x7FFF        # 0 = full, -32767 = silent
MAX_RPM        = redline sample volume * dmix1 >> 15                    # engine-sound.md §7
RPM            = the Ginsu synthesiser's current pitch * 120 * pitch1   # the file-unit "RPM" of the Ginsu spec
PITCH_OFFSET   = int(PitchMultiplier * 16383) - 0x3FFF
TORQUE         = int(EngTorque * 10.24)                                 # 0 to 1024
ROTATION       = the exhaust view angle (0 here); AZIMUTH = the map's azimuth slot
```

**Sputter**, every update: `RPM = PhysicsRPM`, `TORQUE = int(PhysicsTRQ * 10.24)`, `VOL = dmix(spark chatter slot
1) * Vol_Sputters >> 15`, `accel_true` = the accelerating flag, `shifting_true` = a shift is running,
`Force_Trigger` = 0 (the tuner-car backfire sets it for one update), azimuth from the map, pitch and rotation 0.
Created with `car class = CarID` and `car id` = the owner's identity.

**Sweetener**, once per sweetener (engine-sound-effects.md §1.6): `CAR_SWTN(id, CarID, PhysicsRPM, volume,
azimuth, 0, 0)` with `id` 0 or 1 and `volume = Vol_ShiftSweets * w(RPM) * mixer level`. The object lives until its
graph fires the destroy node.

**Whine** (reverse): `RPM = PhysicsRPM`, `VOL = whine slot * PhysicsRPM / MaxRPM`. **Transmission loop**:
`magnitude = int(forward speed * 15)`, `VOL` = slot 3, `PITCH_OFFSET` = the map's pitch, `Single_Shot` = a shift
is running.

## 4. What the graphs do **[verified by running them]**

Run on the M3's `CAR_66_ENG_MB_EE.abk` with the parameters of §3 (everything else 0):

- The module has eight players, each holding one of the bank's eight looped sounds (the loops run over the whole
  sound, 14,776 to 26,604 samples). Four play off the throttle (sounds 1 to 4: low RPM to high) and three on it (5
  to 7), one is the redline sample (8, volume `MAX_RPM`).
- The pitch input of a player rises in proportion to RPM: 4096 (x1) at the sound's base RPM, 20480 (x5) for the
  lowest loop at 7500; volumes cross-fade smoothly in RPM and with `TORQUE` between the two sets; at `TORQUE`
  0 the on-throttle loops are silent and the off-throttle ones hold 12,000 to 23,000.
- `VOL_ENG = -10000` scales every volume by 0.69: the volumes are `x * (32767 + VOL_ENG) / 32767`.
- `CAR_SWTN` plays bank sound `id + 1` of `SWTN_CAR_66_MB.abk` (id 0: sound 1, 73 ms; id 1: sound 2, 147 ms) at
  pitch 4096 and volume `VOL`, and ends itself on the tick the sound has ended. This replaces the earlier guess
  (sounds 10 and 8, audio.md).
- The sputter (two players) is silent at a steady full throttle. Lifting off at 6500 RPM it played five pops over
  about 2.5 s (sounds 4, 6, 7 and 9, at 9,000 to 15,000 of `VOL` 30,000), one pop came after a shift, and
  `Force_Trigger` played sound 11 at full `VOL`. The sound is picked at random from the bank's twelve (8 ms to
  0.37 s) by tables of RPM and torque; its class controller makes the `CAR_SputOutput` whose volume goes back to
  the game (spark chatter mixer input, car-sound-mixer.md §2).

## 5. Decisions for the Rust game **[decision]**

Implemented in `crates/nfsmw/src/audio/aems/` (`params.rs` builds the parameters of §3; `host.rs` is the `blackbox-aems`
host that plays the voices; `kira_out.rs` plays them through `kira`).

- The sample layer (one instance of the engine module) and the sputter (one instance of `CAR_Sputter`) run on a
  fixed 60 Hz tick (at most four per frame) with the parameters of §3; each player is one `kira` voice of the bank
  sound its entry names, in the engine group, looping over the sound's own loop points (a sound without any is a
  one-shot: the sputter's pops); volume = input / 32767 times the map makeup of car-sound-mixer.md §5, playback
  rate = input / 4096, changes smoothed over 33 ms.
- `RPM` is the Ginsu target frequency, not the synthesiser's lagging current pitch: the audio thread keeps the
  synthesiser, and the difference is the 60 ms of its latency.
- `dmix1` is the map's raw sample-layer level (engine slot 1), `Vol_Sputters` scales the sputter's slot 1 of the
  spark-chatter object; both without the makeup, which is applied to the voices.
- The `AZIMUTH`, reverb, low-pass and dry inputs of the players are not applied.
- The sputter's `CAR_SputOutput` volume goes back to the map as the spark-chatter object's inputs 0 and 2
  (32767 while it is nonzero, car-sound-mixer.md §2). `Force_Trigger` stays 0 (the tuner car's backfire is not
  produced); the car id is a constant.
- **No `CAR_SWTN` instance.** Section 4 showed the module plays bank sound `id + 1` at pitch 4096 and volume `VOL`
  and ends with it, so the sweeteners keep the effects mixer's one-shots, now of those sounds (`refs.rs`).
- The whine and transmission loop stay as before (guessed sounds, effects spec §6); their modules run the same
  way when someone wants them.

## How to check it

`audio::aems::tests` (needs `NFSMW_GAME_DIR`) drives the M3 through the scripted drive of `audio::car::tests` (idle,
a pull to the redline, two lifts, a coast) with the real mixer map and mixes the layer's voices in software. Measured
with makeup 1.5: the layer alone peaks at 0.66 (rms 0.11 at idle, 0.21 on the pull, 0.12 on the lift), up to four
engine voices and one sputter voice sound at once, the idle voices are loops that keep playing, and the sputter
reports a volume in 37 ticks. With the Ginsu loops the whole engine peaks at 0.95 (`audio::car::tests`;
`NFSMW_AEMS_WAV` and `NFSMW_ENGINE_WAV` write the layer alone and the sum to a WAV). At 4000 RPM under full torque
four players sound (volumes 449, 715, 10430 and 24720 of 32767); the earlier Python prototype, which truncated floats
where the PC rounds to nearest, also had a fifth at 171. The gain stage is [decision]: compare it with the running
game, with the engine class values logged. Not done.
