# Cop siren audio: the state machine, the sound voice and its limits

What makes a police car's siren sound: the per-car state the chase code keeps, the sound system's siren voice that
follows it, the AEMS module that plays it and the rules that limit how many sirens play. The speech side of the chase is
in [ai-pursuit-speech.md](ai-pursuit-speech.md) (section 10 is the short form of section 2 below).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src`: `AI/Common/AIVehiclePursuit.cpp` (`UpdateSiren`), `AI/AIVehiclePursuit.h`,
  `EAXSound/CARSFX/CARSFX_Siren.{hpp,cpp}`, `EAXSound/SND_GEN/COP_SIREN_AEMS.{h,cpp}`,
  `EAXSound/States/Managers/STATEMGR_{CopCar,CarState}.cpp`, `EAXSound/EAXSOund.hpp` (`eVOL_SIREN`),
  `EAXSound/EAXSoundTypes.h`, `Physics/Behaviors/SoundCar.cpp` (`SoundCop`). Read for understanding; no code copied.
  Provenance: [provenance/cop-siren-audio.md](../provenance/cop-siren-audio.md).
- **Data inputs:** AttribSys `pursuitlevels` (six siren fields per heat row), `speechtune` (`HighIntensityMark`,
  `PursuitDurationHighIntensity`), `MIXMAPS/MAPOUTPUT.mxb` (state 4), `IG_GLOBAL/SIREN_MB.abk`.
- Tags as in the [docs README](../README.md#evidence-tags). **[decomp]** is the CC0 decompilation of the GameCube build;
  **[verified]** was checked against the PC install's data or `speed.exe` strings.

## 1. The pieces

```
AIVehiclePursuit::UpdateSiren   (chase side, per cop car, every AI update)
        |  mSirenState: off / wail / yelp / scream / die        (SoundCop copies it into the sound packet)
        v
CARSFX_Siren                    (sound side, one per cop sound state that is attached to a car)
        |  holds a lagged copy of the state, picks the TYPE word
        v
Csis class SIREN                (9 words: VOL, PITCH_OFFS, PAN, DISTANCE, TYPE, loPass, hiPass, dry_FX, wet_FX)
        |  = the SIREN module of IG_GLOBAL/SIREN_MB.abk, run by the AEMS interpreter
        v
samples of SIREN_MB.abk
```

The state numbers are `off = -1, wail = 0, yelp = 1, scream = 2, die = 3` **[decomp]**; the same numbers are the
`TYPE` word of the siren sound, with off sent as 0 (see 3).

## 2. The state machine (`UpdateSiren`) **[decomp]**

Per car it keeps the state, a flag "starting", and three clocks: **start** (restarts when contact is lost and after each
opening yelp), **wail** (restarts at the end of the starting period, and whenever the wail/yelp cycle or a pressing
formation resets it) and **scream**. All three start at the car's creation (or reset) time.

One more value is shared by all cars: the **opening gap** `SirenInitMinPeriod + random(SirenInitVariation)`, drawn the
first time it is needed and again when a car finishes an opening yelp (a function-level `static` in the original, so
every cop yelps in step).

Inputs: the car is in the pursuit (`mInPursuit`) with its heat row valid; the speech focus (below); high intensity
(`HighIntensityMark` heat or `PursuitDurationHighIntensity` seconds: heat 2 or 150 s **[verified]**, the heat row and
duration are the player pursuit's); the car's light bar is lit; the car is destroyed; the car is in a formation; the
pursuit's collapse or finisher is active, or one of the "re-initiate pursuit", "spotted", "bullhorn arrest" or "bullhorn"
lines is being said; the car belongs to a roadblock.

```
if the car is not in the pursuit:                state = off
else:
  if the pursuit is the player's only one:
     focus = pursuit flow and chase length < 5 s: the opening exchange
          no chase logic (below); the car yelps for 0.5 s once the opening gap has passed since its start clock
          (only from "off"), then goes off, draws a new gap and restarts the start clock
     focus = lost: state = off, no chase logic, the start clock restarts every tick
  if the light bar is dark:   state = die if destroyed else off          (this does not stop the chase logic)
  chase logic (unless the opening exchange or lost applied):
     t = now - start clock;  starting = t < 6 s
     if the starting period just ended:  wail clock = now; state = yelp if high intensity else wail
     if starting:  state = yelp (cool);  hot: scream for t < 3 s, then yelp
     elif pressing (collapse / finisher / one of the lines) and the car is in the formation:
          hot: if state != scream and now - scream clock > SirenScreamPeriod: scream, scream clock = now
               elif now - scream clock > SirenMaxScreamTime: yelp, scream clock = now
          cool: yelp
          wail clock = now
     else:  c = now - wail clock
            c > SirenWailPeriod + SirenMaxYelpTime: wail, wail clock = now
            elif c > SirenWailPeriod: yelp
if the car is in a roadblock: state = wail        (always, whatever the above said)
```

Installed data (`pursuitlevels`, the same in all ten heat rows **[verified]**): `SirenInitMinPeriod` 0.5 s,
`SirenInitVariation` 1.5 s, `SirenWailPeriod` 11 s, `SirenMaxYelpTime` 6 s, `SirenScreamPeriod` 60 s,
`SirenMaxScreamTime` 3 s.

Consequences worth knowing **[decomp]**: a hot chase's starting siren screams 3 s, yelps to 6 s, then keeps yelping
until the wail clock exceeds the wail period (the end of the starting period starts it at yelp, not wail); the scream
clock is re-armed by the yelp branch every `SirenMaxScreamTime`, so in practice a scream happens at the first
pressing tick when 60 s have passed since the clock was last reset, and not again until the pressing stops; the
`SoundCop` behaviour overwrites the state with `die` when the car's light bar is damaged.

### Rewrite differences

- **Light bar gate.** The original lets the chase logic overwrite the dark-bar result. The rewrite applies the gate last:
  a dark bar is `off`, a wrecked car with a dark bar is `die`, also for roadblock cars. [rewrite]
- **Speech focus.** The rewrite has no speech focus. The opening exchange is the first 5 s of the chase; "lost" is the
  cops having had no contact for 5 s or more. The "line playing" condition is not wired (it never reads true).
  [inferred]
- **Formation.** "In formation" is a cop that the formation module has assigned a slot; collapse and finisher come from
  the police state. [rewrite]
- **Shared gap.** Kept (one `OpeningGap` for all cars).

## 3. The sound voice (`CARSFX_Siren`) **[decomp]**

One voice per cop *sound state*. `CSTATEMGR_CopCar` creates **four** of them; each frame it attaches a free one to
any cop car (patrol, pursuit, roadblock or support, whatever its siren state) whose position is within **135 m**
of the closest camera, and detaches the voice of a car that has left that radius or stopped being simulated. A fifth
cop in range gets no sound state at all until one frees (cars are taken in the order the sound system lists them).
This is the original's cap: **four siren voices, 135 m**. **[decomp]**

At attach the voice starts with held state `off` and a wait of `6 + random(2)` s. Each frame:

1. The voice is **switched off** when the car's state is `off`, or when its held state is `die` and more than 10 s
   have passed since it began to die. An off voice destroys its siren sound and its mixer trigger input is 0.
2. Otherwise `wait -= dt`. When `wait < 0`, or the car's state is `scream`, the voice adopts the car's state and the wait
   becomes `random(3)` s. So the sound lags the car by up to 3 s (6 to 8 s for the first adoption), except for
   screams, which are taken at once.
3. The `TYPE` word is the held state's number, with `off` sent as 0: a voice that has not adopted anything plays the
   **wail** during its first 6 to 8 s. (The original's `off` case disables the voice but the car-side check enables it
   again the next frame; the `TYPE` input stays at its initial 0.) [decomp, inferred reading of the enable order]
4. A held state of wail, yelp or scream refreshes the death clock; entering `die` starts it.

## 4. The siren sound (`SIREN`) **[verified]**

`IG_GLOBAL/SIREN_MB.abk` holds two modules, `SIREN_BED` (1 player, unused by the game's code) and `SIREN` (5 players,
9 class words, up to 10 instances). Running the `SIREN` module with the interpreter, with a full volume word and each
`TYPE` (a class-data VOL of 20000, two seconds of 60 Hz ticks): type 0 (wail) starts samples 3 and 4 of the bank (the
mix crossfades between them) and sample 5 at volume 0; type 1 (yelp) starts samples 2 and 1 and sample 5 at volume 0;
type 2 (scream) starts sample 5 alone, swelling in; type 3 (die) starts samples 1 and 2 and then fades them out;
`TYPE -1` keeps only sample 5 at volume 0. The module crossfades by itself within about a second when the `TYPE` word
changes. The sample number is the bank sound counted from 1.

Words fed each tick **[decomp]**: `VOL` = the mixer's volume output (slot 1, 0 to 32767); `PITCH_OFFS` = the mixer's
pitch output (slot 3, Q12) minus 4096, clamped to plus or minus 16384; `PAN` = the mixer's azimuth output (slot 0, a
16-bit angle); `DISTANCE` = metres from the view times 12.8, clamped to 1024; `TYPE` as in 3; `loPass` 25000; `hiPass` 0;
`dry_FX` 32767; `wet_FX` 0.

## 5. The mixer map **[verified]**

State 4 of `MAPOUTPUT.mxb` is the cop car; the siren is its object 17. Its volume master (base -700) reads the 3D control
`(state 4, element 1)` (the distance rolloff), sub-mix 4, main-state sub-mix 4 (the volume sliders), a main-state event,
the control `(state 4, element 0)` scaled by the object's **input 5** (the trigger: 0x7FFF while the voice is on, 0
otherwise) and further events of the music and player-car states. Its pitch master reads the 3D control and a main-state
event; its reverb master (slot 4, base -100) is not read by the voice. The 3D controls read the cop **position
controller** (state 4, controller 7): inputs 0 and 1 the distance to the car and to the camera (cm), 2 and 3 the
azimuths, 13 and 14 the closing speeds (cm/s, negative approaching) and 15 the flags, the same block as the traffic
sources.

## 6. Rewrite

- `libs/blackbox-pursuit/src/siren` holds the pure state machine (`Machine`, `Input`, `Tuning`, `OpeningGap`) and the voice
  follower (`Voice`); times are passed in, so they are tested without a clock.
- `nfsmw_data::pursuit_siren` reads the six fields per heat row and the intensity marks.
- `scenes/world/ai/sirens.rs` runs one machine per police car from the police state and hands the audio a `SirenFrame`.
- `audio/siren` attaches up to four voices within 135 m, runs the state-4 mixer (one instance per voice), feeds the
  `SIREN` module of `SIREN_MB.abk` the words above and plays it in the SFX group. Doppler comes from the mixer map's
  closing-speed inputs (the traffic sources' block).

## Open

- No listening comparison with the original has been made.
- The exact play order of the original's sound tick (car-side enable before voice update) is read from the code, not
  observed. If the first adoption is not really delayed in the original, the wail-then-follow start is wrong.
- The speech-line condition of the pressing branch, the speech focus, and `DistanceToView` (taken as the camera) are
  approximations (section 2).
- Which cop cars the original treats as cop-context for the four slots (patrol cars are assumed to count) is inferred.
