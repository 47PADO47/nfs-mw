# The car sound and the dynamic mixer

What the car sound publishes to the mixer map and which outputs it reads. The evaluator is in
[dynamic-mixer.md](dynamic-mixer.md); the map layout in [formats/mixmap.md](../formats/mixmap.md); the engine
and effects it connects in [engine-sound.md](engine-sound.md) and [engine-sound-effects.md](engine-sound-effects.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), `src/Speed/Indep/Src/EAXSound/`: `sfxctl/SFXCTL_{Physics,Engine,HybridMotor,Tunnel,MasterVol,
  3DObjPos}.cpp`, `CARSFX/CARSFX_{Engine,Shifting,Turbo,Nitrous,SparkChatter,Skids,Roadnoise,WindNoise,
  BottomOut}.cpp`, `CARSFX/SFXObj_Collision.cpp`, `CARSFX/SFXObj_Enums.hpp`, `States/Managers/STATEMGR_PlayerCar.cpp`,
  `EAXCar.{hpp,cpp}`, `EAXSoundEnums.hpp`. Read for understanding; no code copied.
- **Data inputs:** `MIXMAPS/MAPOUTPUT.mxb` (the others differ in a few dwords); AttribSys `engineaudio.Master_Vol`.
- Tags as in the [docs README](../README.md#evidence-tags).

## 1. Which instances exist

The player's car is state 2, instance 0. The map also reads the main state (0, the volume sliders), the
collision state (7, one instance per sounding collision) and, through events, music (1); states with no
instance contribute nothing. This implementation instantiates states 0 and 2, publishes nothing to the others,
and leaves collisions to the effects mixer (§6). The camera state is 0 (chase).

Sound objects of the player's car (the number in an id, bits 4 to 10), from the creation ids of the original's classes
**[decomp]**: engine 1 (single Ginsu) or 2 (dual), shifting 3, turbo 4, nitrous 5, spark chatter 6, skids 7,
road noise 8, wind noise 9, rain 11, weather wind 12, bottom-out 13, reverb 16, pre-collision whoosh 18. The
player is created with objects `{1, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 16, 18}` (mask `0x53BFA`) so it plays the
*single* engine. This implementation plays the dual engine (engine-sound.md Q2) and attaches object 2 instead of
1; the map has the same entries for both (object 2 differs only by -35 and -88 on two trims). Sound controllers:
physics 0, wheel 1, shifting 2, acceleration transitions 3, engine 4, hybrid motor 5, tunnel 6, car position 7,
rear position 8, object position 9, right and left wheel position 11 and 12, left and right wind position 13
and 14.

## 2. What is published

Values are Q15 unless stated; an input never set reads 0. `(c, i)` = controller or object number and index.

| Id | Value |
|---|---|
| physics `(0,0)` to `(0,3)` | speed in mph times 1092.2334, 546.1167, 327.67, 234.05, each clamped to 0..32767 |
| physics `(0,4)` | `(10000 - PhysicsRPM) * 3.6407778` clamped (the map does not read it) |
| physics `(0,5)` | fraction of wheels on the ground times 32767 |
| physics `(0,6)` | camera view: 0 bumper, 4000 hood, 32767 otherwise (a chase camera: 32767) |
| physics `(0,10)` | 32767 while the throttle is above 30 % |
| engine `(4,0)` | 21844 (the audio upgrade level is 2 in the original) |
| engine `(4,1)` | 32767 on the tick a compression bump starts |
| engine `(4,2)` | `Master_Vol` of the engine set (22300 to 32000) |
| hybrid motor `(5,0)` | 32767 after 3 s at over 30 mph and an RPM change under 30, smoothed up 983 and down 196 per tick |
| hybrid motor `(5,1)` | `clamp(1 - (AccelDeltaRPMThreshold - |last delta|) / AccelDeltaRPMThreshold)` times 32767, smoothed by 3000 per tick; held at half its value from the moment the redline starts; not updated while shifting or with no wheel on the ground |
| tunnel `(6,0..3)`, `(6,4)`, `(6,5..6)` | 0 in the open (zone flags, occluded, reverb volumes) |
| 3D positions `(7,8,9,11..14)` `[15]` | 1 (has a position) |
| 3D positions `[0]`, `[1]` | distance to the car and to the camera in cm; this implementation: 0 and the chase camera distance |
| 3D positions `[2]`, `[3]` | azimuth to the car and to the camera, 16-bit angle; 0 (straight ahead); wind sources are placed by §5 |
| shift object `(3,7)` | 32767 during the up-shift's disengage stage |
| nitrous `(5,1)`, `(5,2)` | 32767 while the loop plays; 32767 while the purge plays |
| skids `(7,0)` and `(7,2)` | forward component (effects spec §7) |
| skids `(7,1)`, `(7,3)` | sideways component (smoothed 500), load (smoothed 3000) |
| road noise `(8,0)` | 32767 for a frame when a side's loop changes |
| spark chatter `(6,0)`, `(6,2)` | blip volume, 32767 while the sputter module's output object reports a volume (`audio/aems`) |
| bottom-out `(13,1..3)` | landing flag, bottom-out flag, landing intensity times 256 (not produced) |
| main `(0,0..5)` | the volume sliders as `(1 - level) * 32767`: 0 at full volume; the game's own volume groups do the scaling, so all 0 |

(0,1), (0,2) of the physics controller are the speed ranges the map's curves are drawn on.

## 3. What is read

Output slot `i` of an object (dynamic-mixer.md §7) **[decomp]**:

| Object | Slots read |
|---|---|
| engine 1 and 2 | 0 azimuth; 1 sample-layer volume; **2 Ginsu volume (both loops)**; 3 transmission-loop volume; **4 pitch**; 5 low-pass (the minimum of it and the engine's own cut-off) |
| shifting 3 | 1 up clunk; 2 down clunk; 3 engage sweetener; 4 disengage sweetener; 5 accelerate sweetener; 6 engine-off sweetener; 8 reverse whine; 10 brake mash |
| turbo 4 | 1 spool; 2 first blow-off; 3 the other blow-offs |
| nitrous 5 | 1 nitrous loop; 2 purge; 3 pitch; 4 low-pass |
| spark chatter 6 | 1 volume |
| skids 7 | 5 forward component; 6 rear axle; 7 sideways component; 8 pitch; 9 reverb |
| road noise 8 | per loop: 5 gravel, 6 sidewalk, 7 cobblestone, 8 deep water, 9 wet road, 10 asphalt, 11 metal, 12 stitch loop; 15 reverb |
| wind noise 9 | 2 left, 3 right, 4 rumble volumes; 5 pitch |
| bottom-out 13 | 1 landing; 2 bottom-out |

Use: a level is multiplied into the generated volume (`volume * slot >> 15`); a pitch slot multiplies the
playback rate (`int(ratio * 4096)`, 4096 = 1). The engine's loops: `vol_accel * slot2 >> 15` (engine-sound.md
§5.5; the `dmix` factor there), the playback rate `pitch1 * slot4 / 4096`.

The road noise loop of a side is the surface's `Aud_Roadnoise_LOOP` as the enum `FXROADNOISE_LOOP` with values
0 gravel, 1 sidewalk, 2 cobblestone, 3 deep water, 4 wet road, 5 and 6 asphalt, 7 metal, 8 stitch loop; **0 is a
loop, not "none"** (the loose surfaces play the gravel loop), and the sound in `ROADNOISE_00_MB.abk` is
`loop + 1` (formats/audio.md). Only the value -1 (no loop) plays nothing.

## 4. Values at a steady speed

The slots the evaluator reads from `MAPOUTPUT.mxb` with the inputs of §2 (a level car on asphalt, the chase camera,
half throttle at 4000 PhysicsRPM, nothing else going on, after one second); gains, 1 = full scale
**[computed with the Rust evaluator, `audio::mixer::tests::cruising_levels_follow_the_map`]**:

| Speed (m/s) | Engine Ginsu | Road, asphalt | Road, gravel | Wind (left+right)/2 | Gear clunk up | Sweetener (disengage) | Turbo spool | Nitrous | Skid forward | Skid side |
|---|---|---|---|---|---|---|---|---|---|---|
| 0 | 0.41 | 0.31 | 0.06 | 0.022 | 0.22 | 0.26 | 0.15 | 0.23 | 0.46 | 0.24 |
| 13.4 | 0.41 | 0.16 | 0.06 | 0.023 | 0.22 | 0.26 | 0.15 | 0.23 | 0.97 | 0.50 |
| 26.8 | 0.41 | 0.022 | 0.06 | 0.031 | 0.22 | 0.26 | 0.15 | 0.23 | 0.97 | 0.50 |
| 44.7 | 0.41 | 0.022 | 0.06 | 0.078 | 0.22 | 0.26 | 0.15 | 0.23 | 0.97 | 0.50 |
| 67.0 | 0.41 | 0.022 | 0.06 | 0.196 | 0.22 | 0.26 | 0.15 | 0.23 | 0.97 | 0.50 |

Reading them: the road noise is ducked hard with speed (a control cuts it by 23 dB between standstill and 60 mph),
the wind rises with speed, so the two cross over; the engine is steady near 0.4. The skid levels are the map's with no slip published (the skid sound itself is
silent then); they step up once the car moves. The road level times the generated volume (which rises to 0.86 by
60 mph) gives a road noise that peaks around 15 to 30 mph.

## 5. Decisions

- **Makeup gain.** The slots are relative: the engine's Ginsu volume reads about 0.41 (-7.8 dB) at cruise. How
  loud the original's 0 dB is, is the sound system's business (`SNDvol` 127), unknown here. All slot levels
  are multiplied by 1.5 (2.25 before the engine's sample layer was played), so that the whole engine, Ginsu
  loops and sample layer together, peaks near full scale (0.95 in the scripted drive of `audio::car::tests`, the
  sample layer alone 0.66); the relative levels are the data's. A scaled effect volume stops at 1. The table
  above lists the slots without the makeup.
- **Camera and positions.** The audio layer does not know the camera. The chase camera is taken to sit behind
  the car on its axis at 5.6 m at rest and 7 m from 60 m/s (the chase camera's own distance): distance to the car
  0, to the camera that, azimuth 0, for the car objects; the rear object 2 m behind, the wheels 0.9 m to each side,
  the wind sources circling the car at `(1 - v/40) * 65` m (`v` in m/s, 2 to 40, not below 3 m) at the angle
  `1280 + 12288 * v/40` (of 65536) to the left (azimuth: the complement of the angle) and right.
- **Low-pass, azimuth, reverb slots** are not applied (no filter, no panning from azimuth, no reverb).
- **Tunnel, rain, weather wind, whoosh, truck:** not produced; all their inputs read 0.
- **Wind** plays one loop (bank sound 1) at the mean of the left and right levels times the speed ratio; the
  rumble level (slot 4) and the two other wind loops of the original's `FX_WIND` are not played.
- **Collisions** keep the effects mixer's levels (the state-7 map entries are per-event and need the hit's position);
  a landing (bottom-out) takes the object 13 level.
- **Skid axles.** The rear axle's loop takes the "back" level (slot 6), the front axle's the forward or sideways
  one by its component; which axle the original's "back" volume belongs to is a guess.
- **Without the map** (`SOUND/MIXMAPS/MAPOUTPUT.mxb` unreadable) every effect plays at its generated volume,
  the road at 0.35 and the wind at 0.4 as before.

## How to check it

Compare the slots with the running game: break on `SndBase::GetDMixOutput` (or on the sound system's volume calls) while
cruising at a steady speed in the chase camera and read the engine's slot 2, road noise slot 10 and wind slot 2.
Not done.
