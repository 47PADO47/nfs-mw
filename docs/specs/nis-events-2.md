# NIS events: census and sample timelines (part 2)

Companion to [nis-events.md](nis-events.md). Holds the per-scene census and the timelines of four scenes. Everything
here is **[verified]**: read from the install's `NIS/Scene_*_BundleB.bun` files with the CARP reader described in
[event formats](../formats/event-sequences.md). Counts are static: how many times an event appears in an action's
event lists. They are not runtime fire counts.

## A. Census of the 87 NIS bundles

Of the 87 bundles, 59 carry a CARP event sequence (one engine each, named after the scene) and 28 do not:

- No sequence (28): the five `FMV` bundles `FMV20T1`, `FMVBLIntro`, `FMVNoCars`, `FMVPSA`, `FMVTest`; `Flythrough`,
  `GenericStart`, `IntroNis09`, `IntroNisBL01`, `IntroNisBL08`, `IntroNisDDEnd`; the four replays `ReplayBurnout`,
  `ReplayCorner`, `ReplaySlide`, `ReplaySpeed`; and 13 of the 14 moment scenes (all `WM*` except `WMBoat`):
  `WMAmphitheatre`, `WMConsScaffold`, `WMDriveIn`, `WMFishingHouse`, `WMGasStation`, `WMGate`, `WMPoliceGround`,
  `WMPorch`, `WMRadioTower`, `WMStripMall`, `WMTorusRoofShop`, `WMTrailerPark`, `WMWaterTower`.

The census table below lists the 59 with a sequence. "Systems" are the engine's named systems (one per car or
cop channel, plus the activity's own system). "Responses" are its state-and-stimulus pairs.

| Scene | Systems | Responses | Events (static occurrences) |
|---|---|---|---|
| ArrestF02 | 3 | 3 | CarPitch 2, CarRoll 2, CopCarDoors 3, Lights 1, OverlayMessage 1, Pixelate 4 |
| ArrestF02b | 3 | 3 | CarPitch 2, CarRoll 2, CopCarDoors 3, Lights 1, OverlayMessage 1, Pixelate 4 |
| ArrestF06 | 1 | 1 | CarPitch 1, CarRoll 1, HideCharacter 2, OverlayMessage 2, Pixelate 9 |
| ArrestF06b | 1 | 1 | CarPitch 1, CarRoll 1, HideCharacter 2, OverlayMessage 2, Pixelate 9 |
| ArrestF07 | 2 | 2 | Lights 1, OverlayMessage 2, Pixelate 4 |
| ArrestF07b | 2 | 2 | Lights 1, OverlayMessage 2, Pixelate 4 |
| ArrestF14 | 2 | 2 | CarPitch 1, CarRoll 1, Lights 1, OverlayMessage 1, Pixelate 5 |
| ArrestF14b | 2 | 2 | CarPitch 1, CarRoll 1, Lights 1, OverlayMessage 1, Pixelate 5 |
| ArrestF18 | 3 | 3 | CopCarDoors 4, Lights 1 |
| ArrestF18b | 3 | 3 | CopCarDoors 4, Lights 1 |
| ArrestF23 | 2 | 2 | Lights 1, OverlayMessage 2, Pixelate 4 |
| ArrestF23b | 2 | 2 | Lights 1, OverlayMessage 2, Pixelate 4 |
| ArrestM01 | 3 | 3 | Lights 1, OverlayMessage 1, Pixelate 4 |
| ArrestM01b | 3 | 3 | Lights 1, OverlayMessage 1, Pixelate 4 |
| ArrestM04 | 3 | 3 | CarPitch 4, CarRoll 4, CopCarDoors 4, Lights 1, OverlayMessage 1, Pixelate 6 |
| ArrestM04b | 3 | 3 | CarPitch 4, CarRoll 4, CopCarDoors 4, Lights 1, OverlayMessage 1, Pixelate 6 |
| ArrestM06 | 3 | 3 | HideCharacter 2, Lights 1, OverlayMessage 2, Pixelate 4 |
| ArrestM06b | 3 | 3 | HideCharacter 2, Lights 1, OverlayMessage 2, Pixelate 4 |
| ArrestM07 | 3 | 3 | CarPitch 2, CarRoll 2, CopCarDoors 2, HideCharacter 2, Lights 1, OverlayMessage 2, Pixelate 2 |
| ArrestM07b | 3 | 3 | CarPitch 2, CarRoll 2, CopCarDoors 2, HideCharacter 2, Lights 1, OverlayMessage 2, Pixelate 2 |
| ArrestM14 | 3 | 3 | CarPitch 1, CarRoll 1, Lights 1, OverlayMessage 2, Pixelate 5 |
| ArrestM14b | 3 | 3 | CarPitch 1, CarRoll 1, Lights 1, OverlayMessage 2, Pixelate 5 |
| ArrestM16 | 3 | 3 | CarPitch 2, CarRoll 2, CopCarDoors 2, Lights 1, OverlayMessage 1, Pixelate 7 |
| ArrestM16b | 3 | 3 | CarPitch 2, CarRoll 2, CopCarDoors 2, Lights 1, OverlayMessage 1, Pixelate 7 |
| ArrestM19 | 3 | 3 | HideCharacter 2, Lights 1, OverlayMessage 2, Pixelate 4 |
| ArrestM19b | 3 | 3 | HideCharacter 2, Lights 1, OverlayMessage 2, Pixelate 4 |
| EndingNis03 | 8 | 8 | ECellCall 2, Brakelock 1, CarPitch 1, CarShake 2, Detach 7, OverlayMessage 1, Reattach 7 |
| EndingNis04 | 10 | 10 | AeroDynamics 2, Brakelock 11, Burnout 2, Detach 5, Lights 8, MotionBlur 1, NeutralRev 4, Reattach 3 |
| FMV17to21 | 10 | 10 | AeroDynamics 2, Burnout 3, CarPitch 2, Detach 12, Lights 2, Reattach 12 |
| FMVPIN11X06 | 5 | 5 | Brakelock 2, Burnout 2, CarPitch 4, CarRoll 1, CarShake 4, Detach 1, Lights 2, Nitro 1, Reattach 1 |
| FMVroc02 | 9 | 9 | AeroDynamics 2, Burnout 3, CarPitch 1, Detach 10, Lights 2, Nitro 2, Reattach 10 |
| IntroNis01 | 5 | 5 | Brakelock 2, Burnout 3, CarShake 2, MotionBlur 1, NeutralRev 2, Nitro 12, RoadNoise 4 |
| IntroNis02 | 3 | 3 | Brakelock 1, Burnout 4, CarShake 1, MotionBlur 1 |
| IntroNis03 | 5 | 5 | Brakelock 4, Burnout 4, CarShake 12, MotionBlur 1, Nitro 14 |
| IntroNis04 | 5 | 5 | ECameraPhotoFinish 3, Burnout 2, CarPitch 3, CarRoll 1, CarShake 1, MotionBlur 1, NeutralRev 6, Nitro 7 |
| IntroNis05 | 11 | 11 | Brakelock 2, Burnout 2, CarShake 1, Detach 8, MotionBlur 1, NeutralRev 2, Nitro 2 |
| IntroNis06 | 5 | 5 | ECameraPhotoFinish 3, Brakelock 2, Burnout 1, CarShake 4, MotionBlur 1, NeutralRev 4, Nitro 2 |
| IntroNis07 | 5 | 5 | ECameraPhotoFinish 4, Brakelock 4, CarShake 7, MotionBlur 1, Nitro 8, EPlayObjectEffect 1 |
| IntroNis08 | 5 | 5 | Brakelock 2, Burnout 1, CarShake 4, MotionBlur 1, Nitro 6 |
| IntroNis10 | 5 | 5 | ECellCall 1, Brakelock 2, CarShake 4, FakeFar 1, Rain 1, TimeOfDay 2 |
| IntroNisBL02 | 5 | 5 | CarPitch 3, CarShake 12, Detach 2, Lights 2, MotionBlur 1, Nitro 3, Rain 1, Reattach 2, TimeOfDay 2 |
| IntroNisBL03 | 9 | 10 | AeroDynamics 2, Burnout 10, CarDamageReset 3, CarPitch 2, CarRoll 4, Detach 16, FakeFar 2, Lights 4, Nitro 2, NukeSmack 2, OverlayMessage 1, PlayEffect 7, Rain 1, Reattach 28, TimeOfDay 6, EPlayObjectEffect 3 |
| IntroNisBL04 | 3 | 3 | CarPitch 2, CarShake 10, FakeFar 3, MotionBlur 1, NukeSmack 2, OverlayMessage 1, PlayEffect 3, Rain 1, TimeOfDay 2 |
| IntroNisBL05 | 10 | 10 | Burnout 2, CarPitch 5, CarRoll 1, CarShake 4, Detach 7, MotionBlur 1, Nitro 4, NukeSmack 1, PlayEffect 1, Rain 1, Reattach 1, Steering 2, TimeOfDay 5 |
| IntroNisBL06End | 7 | 7 | CarDamageReset 1, CarPitch 7, CarRoll 1, CarShake 6, Detach 8, Nitro 12, NukeSmack 2, OverlayMessage 5, PlayEffect 2, Rain 3, Reattach 13, TimeOfDay 4 |
| IntroNisBL06 | 7 | 7 | CarDamageReset 1, CarPitch 4, CarRoll 1, CarShake 3, Detach 12, FakeFar 3, MotionBlur 1, Nitro 6, NukeSmack 1, OverlayMessage 5, PlayEffect 2, Rain 3, Reattach 13, TimeOfDay 3 |
| IntroNisBL07End | 10 | 10 | ECameraPhotoFinish 1, AeroDynamics 4, Brakelock 2, Burnout 3, CarPitch 3, CarRoll 1, Detach 13, Lights 10, NukeSmack 3, PlayEffect 5, Rain 1, Reattach 11, TimeOfDay 2 |
| IntroNisBL07 | 10 | 10 | ECameraPhotoFinish 2, AeroDynamics 7, Brakelock 2, Burnout 2, CarPitch 3, CarRoll 1, Detach 13, FakeFar 3, Lights 10, MotionBlur 1, NukeSmack 3, PlayEffect 5, Rain 1, Reattach 11, TimeOfDay 3, WolrdGeometry 1 |
| IntroNisBL09 | 4 | 4 | Brakelock 10, Burnout 5, CarPitch 5, CarRoll 1, Detach 3, FakeFar 3, MotionBlur 1, Nitro 2, NukeSmack 2, PlayEffect 1, Rain 1, Reattach 3, Steering 2, TimeOfDay 5 |
| IntroNisBL10 | 3 | 3 | ECameraPhotoFinish 7, AeroDynamics 2, Brakelock 4, Burnout 2, CarDamageReset 1, CarPitch 2, CarRoll 1, CarShake 1, Detach 2, FakeFar 4, Lights 2, Nitro 2, Reattach 2, TimeOfDay 7 |
| IntroNisBL11 | 8 | 8 | AeroDynamics 1, Brakelock 3, Burnout 6, CarPitch 6, CarRoll 2, CarShake 6, Detach 2, FakeFar 6, Lights 5, MotionBlur 1, Nitro 4, NukeSmack 1, Rain 1, TimeOfDay 8 |
| IntroNisBL12 | 4 | 4 | ECameraPhotoFinish 8, Brakelock 2, CarPitch 3, CarRoll 1, CarShake 9, Detach 14, FakeFar 2, Lights 2, MotionBlur 1, Nitro 6, NukeSmack 3, OverlayMessage 1, Rain 1, Reattach 14, Steering 2, TimeOfDay 3 |
| IntroNisBL13 | 6 | 6 | Burnout 2, CarRoll 3, Detach 2, FakeFar 1, MotionBlur 1, Nitro 3, NukeSmack 1, OverlayMessage 8, PlayEffect 1, Rain 1, Reattach 2, Steering 2, TimeOfDay 4 |
| IntroNisBL14 | 8 | 8 | Brakelock 3, Burnout 1, CarShake 6, Detach 3, FakeFar 2, Lights 6, MotionBlur 1, Nitro 4, PlayEffect 1, Rain 1, Reattach 3, TimeOfDay 6 |
| IntroNisBL15 | 3 | 3 | Brakelock 2, Burnout 2, CarPitch 2, CarShake 5, FakeFar 5, Nitro 2, PlayEffect 2, Rain 1, TimeOfDay 7 |
| IntroNisBL16 | 3 | 3 | ECameraPhotoFinish 5, Brakelock 3, Burnout 2, CarPitch 1, CarShake 2, Lights 2, MotionBlur 1, Rain 1, TimeOfDay 3, EPlayObjectEffect 1, EStopObjectEffect 1 |
| IntroNisDD | 11 | 11 | EDDaySpeech 1, AeroDynamics 1, Brakelock 13, Burnout 2, CarDamageReset 3, CarPitch 12, CarRoll 13, Detach 46, FakeFar 6, Lights 5, MotionBlur 1, PlayEffect 1, Reattach 41, StopEffects 1, TimeOfDay 6 |
| IntroNisToll01 | 2 | 2 | CarShake 5, MotionBlur 1 |
| WMBoat | 1 | 1 | MotionBlur 3 |

Two notes on the table. `ECellCall` appears twice in `EndingNis03` and once in `IntroNis10` (ids 13, 11 and 12;
see [career phone](career-phone.md) section 5.1). `IntroNisBL01` has no sequence, although it has a camera track, an
audio stream entry and a rev file. Its cut plays audio and engine revs but no events.

## B. Sample timelines [verified]

Times are seconds from the start of the action that holds the event. A system's action starts when the `START`
stimulus is delivered (see the main spec, section 1). Values are the static-data fields, decoded with the layouts in
the main spec, section 3. The strings are the CARP string records of the same bundle.

### B.1 IntroNis05 (11 systems, 11 responses)

| Time | Event | Values | Where |
|---|---|---|---|
| 0.000 | ENISBurnout | speed 30 | system `9D4052A1`, action of 8.0 s |
| 0.000 | ENISNeutralRev | on, throttle 1, speed 3 | system `9D4052A1` |
| 0.000 | ENISMotionBlur | off | activity system `8DA18577`, action of 1.0 s |
| 1.700 | ENISNitro | on | system `9D4052A1` |
| 2.400 | ENISNitro | off | system `9D4052A1` |
| 2.867 | ENISCarShake | shake 0.003, rate 0.9, ramp 1, time 6 | system `32AB6B91`, action of 4.8 s |
| 3.220 | ENISBurnout | speed 0 | system `9D4052A1` |
| 3.220 | ENISNeutralRev | on, throttle 0, speed 2 | system `9D4052A1` |
| 3.220 | ENISBrakelock | front 0, rear 0 | system `9D4052A1` |
| 4.687 to 4.707 | ENISDetach | speed 1 | eight car systems, one each |
| 12.013 | ENISBrakelock | front 1, rear 0 | system `32AB6B91` |

Each of the eight `ENISDetach` events is in a different car's system. Their times differ by 0.02 s.

### B.2 EndingNis03 (8 systems, 8 responses, duration 9.4 to 60 s)

| Time | Event | Values |
|---|---|---|
| 0.047 | ECellCall | id 13 |
| 4.173 to 5.180 | ENISDetach | speed 1 (four systems) |
| 5.407 to 8.133 | ENISReattach | repair 0 (four events in four systems) |
| 7.033 | ENISCarShake | shake 0.002, rate 0.005, ramp 0.5, time 30 (system `663D7304`) |
| 7.420 | ENISBrakelock | front 1, rear 1 |
| 7.800 | ENISCarShake | shake 0.002, rate 0.005, ramp 0.5, time 30 (system `32AB6B91`) |
| 7.987 | ECellCall | id 11 |
| 8.267 | ENISCarPitch | pitch 0.1, time 1 |
| 14.013 to 14.480 | ENISDetach | speed 1 (three events) |
| 15.233 to 17.907 | ENISReattach | repair 0 (three events) |
| 26.293 | ENISOverlayMessage | name `BLACK_FADE_OUT.fng`, message `15_OUT` |

The overlay's two strings are the CARP string records `0x63530002` (the package) and `0x63530001` (the message).

### B.3 ArrestF02 (3 systems, 3 responses, durations 4 to 15.2 s)

| Time | Event | Values |
|---|---|---|
| 0.000 | ENISLights | cop lights on, head lights off (system `35BD77E2`, duration 8 s) |
| 4.467 | ENISCarRoll | roll 0.05, time 0.3 (system `35BD77E2`, actions of 8 s; the lights in the same action) |
| 4.467 | ENISCarPitch | pitch 0.1, time 0.3 |
| 6.487 | ENISCarRoll and ENISCarPitch | roll 0.05 and pitch 0.1, time 0.3 |
| on start | ENISCopCarDoors | door 3, position 1, anim length 0, end 1 (system `8DA18577`, duration 15.2 s) |
| on start | ENISPixelate | on, 100 by 100 (same system) |
| 0.000 | ENISCopCarDoors | door 3, position 0 |
| 1.867 | ENISPixelate | on, 40 by 40 |
| 2.667 | ENISPixelate | off, 16 by 16 |
| 3.427 | ENISOverlayMessage | name `BUSTED_OVERLAY.fng`, message `FLASHTRIGGER` |
| 6.207 | ENISCopCarDoors | door 3, position 1, anim length 0.3, end 0 |
| 7.020 | ENISPixelate | off, 16 by 16 |

"On start" marks events in an action's tag list, not in its timed lists. See the main spec, section 1.3.

### B.4 Other scenes named in the census

- `WMBoat` (1 system, 1 response, 2 events): a moment scene with a sequence. Its events are not described here.
- `FMVroc02`, `FMV17to21`, `FMVpin11X06`: sequences with 9 to 10 systems and a second stimulus (`0x1A24FA80`,
  unknown). They are FMVs, not NIS cuts. See the main spec, section 8.
