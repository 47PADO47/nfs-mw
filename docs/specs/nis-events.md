# NIS events and NIS audio

What a cinematic (NIS) scene fires while it plays, and how its audio and engine revs are chosen. A NIS is a scripted
cut: an animation scene, a camera track, a set of cars and an event sequence, plus a stream of sound. The cut-scene
camera is in [animation](../formats/animation.md). The pursuit breaker's cut is in
[pursuit-breaker-moment.md](pursuit-breaker-moment.md). Per-scene counts and sample timelines are in
[nis-events-2.md](nis-events-2.md).

Evidence tags, as in [docs/README.md](../README.md#evidence-tags): **[verified]** measured on the install; **[decomp]**
read from the CC0 reconstruction (dbalatoni13/nfsmw); **[lead]** a third-party source, not checked; **[inferred]** a
reasoned guess from names or values; **[unconfirmed]** a claim that needs the original binary. Where the original must
be read, the gap says **[unconfirmed – needs Ghidra]** and names the function.

## 0. Summary

- A NIS's events are one CARP engine, named after the scene. Its systems start on one stimulus, `START`. Most events
  sit on timed lists of an action. No NIS action uses the sequencer's `start`, `end` or `stop` tags; the NIS tags are
  other hashes. [verified]
- 59 of the 87 NIS bundles carry a sequence. 31 event types occur in them; 7 `ENIS` event types never occur. [verified]
- Every `ENIS` event handler is absent from the CC0 decompilation (the generated `.cpp` files are 0 bytes). Their
  layouts are known; their effects on the PC are **[unconfirmed – needs Ghidra]**. [decomp]
- The NIS audio stream is keyed by the scene name's `bStringHash`, not by the CARP engine name. The choice is made by a
  68-row table of scene names in the decomp, then by the CSIS sound interface. The four CSIS interfaces match the
  install's `NISAudio.csi`. [verified]
- The decomp's lookup of that table compares the wrong slot (section 7.8). If the PC binary does the same, no NIS
  stream or rev curve could load. **[unconfirmed – needs Ghidra]**
- The engine-rev curves (`NISREVDATA`, 29 files) drive a NIS car's RPM and torque, keyed by car slot: `car1`..`car8`,
  then `cop1`..`cop8`. [decomp] + [verified] (file layout)

## 1. How a NIS fires its events

### 1.1 The engine and its systems

Each scene bundle holds one CARP group with one engine. The engine's name is the scene name, for example `IntroNis05`.
It has one system per car channel and one for the activity itself. [verified]

- A car system's key is a 32-bit CRC of its channel name (`car1`..`car8`, `cop1`..`cop8`). The decomp finds the car by
  it. [decomp] The ordinary CRC-32 of those names does not match the keys in the data, so the exact variant is
  **[unconfirmed]**.
- The activity's own system has key `8DA18577`. Its events (`ECellCall`, `ENISOverlayMessage`, `ENISPixelate`, and
  others) are answered by the activity itself. The decomp returns the activity handle only for this key. [decomp]
- Systems per scene run from 1 (`WMBoat`) to 11 (`IntroNis05`, `IntroNisDD`). Responses per scene run from 1 to 11.
  [verified; the full table is in [part 2](nis-events-2.md#a-census-of-the-87-nis-bundles)]

### 1.2 Stimulus, start and queueing

- The stimulus `START` (hash `0xA6A56EA1`, the vault hash of the word `START`) is used by 270 of the 274 responses.
  The other four use stimulus `0` (3 responses, in `IntroNis07` and `IntroNisBL03`) and `0x1A24FA80` (1 response, in
  the FMV `FMV17to21`). What `0` and `0x1A24FA80` mean is **[unconfirmed]**. [verified]
- The activity sends `START` in one call with queue mode *allow*, after a reset of the sequencer to its start time.
  [decomp: `NISActivity::StartEvents`]
- A busy system queues a stimulus instead of dropping it. The queue rules are in [world sequencers](world-sequencers.md),
  section "Sequencer". [decomp + lead]
- The activity can also fire a named tag through `FireEventTag(name)`, which hashes the name and fires the sequencer's
  tag. [decomp] The NIS tag hashes in the data cannot be turned back into names here; see section 1.3. [unconfirmed]
- Skipping a NIS sends the sequencer's `Complete` and jumps the animation to its end. [decomp: `SkipOverNIS`]

### 1.3 Timed lists and tags

An action has a duration in seconds, a list of tags (each a hash with one event list), and a list of times (each a
float with one event list). Times are seconds from the action's start. [verified layout] [decomp layout]

- Time-zero lists fire when the action starts. A timed list fires once when the action's clock passes its time.
  [decomp + runtime; the rule matches the runtime in `libs/blackbox-sequence`]
- No response has a negative duration. The durations run from 1 to 60 s. [verified]
- No response has a filter. [verified]
- Of the 274 responses, 226 have no tag. The 48 with tags hold 100 tag references, and those tags use 31 distinct
  hashes. The sequencer's `start`, `end` and `stop` tags (hashes `0x669C6D75`, `0x4CF5D439`, `0x475830DB`) never
  appear. [verified]
- The rewrite's sequencer fires only `start`, `end` and `stop` ([blackbox-sequence](../../libs/blackbox-sequence/README.md)).
  It would therefore never fire a NIS tag list. The NIS tags must be fired by the animation scene, through the
  activity's named-tag call. Which animation event fires which tag is **[unconfirmed – needs Ghidra]**: the function is
  `NISActivity::FireEventTag`, and its caller is in the ICE and animation code, which is not in the CC0 file set.

### 1.4 Hashes used by NIS events

| What | Hash | Scheme | Evidence |
|---|---|---|---|
| Event type (ENIS and others) | the event class name | vault hash (AttribSys `lookup2`, initval `0xABCDEF00`) | [verified]: `ECellCall` = `0x885F756E`; `EFireEventList` and `ESpawnFragment` match the repo's known values |
| Engine name | the scene name | vault hash | [verified] for `IntroNis05` (`0x6AEC1B12`), `EndingNis03`, `ArrestF02` |
| `START` stimulus | the word `START` | vault hash `0xA6A56EA1` | [verified]; [decomp] (`StartEvents` uses the literal) |
| Audio, rev and camera key | the scene name | `bStringHash` (`h = 33*h + byte`, seed `0xFFFFFFFF`) | [decomp]: `NISActivity` compares `bStringHash("IntroNisFlyInDD")` with its scene hash |
| Car system key | the channel name | a CRC-32 of the channel name | [decomp] for the name; the CRC variant is [unconfirmed] |

So one scene has two hashes: the engine name (vault) and the audio key (`bStringHash`). `IntroNis05` is `0x6AEC1B12` in
the CARP data and `0x0D0E57FA` as an audio key. Do not mix them.

The CARP string records (`cS` tags, prefix `0x6353`) hold names used as parameters, such as effect names and overlay
packages. Each bundle has its own string table. [verified]

## 2. Event reference: the ENIS events

Static data starts with the event's own hash at offset 0. Each field below is at the offset given. Fields are
little-endian. The data size in a bundle is the static struct size rounded up to 16 bytes. This holds for all 31 used
types. [verified] [decomp for the layouts, from the generated headers]

Types: `f` float, `i` signed 32-bit, `u` unsigned 32-bit, `p` a CARP string tag (prefix `0x6353`, a `cS` record), `c` a
32-bit CRC of a name (not decoded here). "Scenes" is the number of NIS bundles that use the type. "Occurrences" is the
static count (section 4).

| Event | Hash | Payload (offset: type, field) | Scenes | Occurrences | Meaning and evidence |
|---|---|---|---|---|---|
| ENISAeroDynamics | `8DA05B3D` | 4: f `fAerodynamics` | 9 | 23 | Aero factor; values -1, 0, 0.1, 1, 2, 3. [inferred] |
| ENISBrakelock | `A01A259B` | 4: u `fFront`; 8: u `fRear` | 21 | 77 | Brake lock per axle, 0 or 1. [inferred] |
| ENISBurnout | `6A5265B2` | 4: f `fSpeed` | 23 | 66 | Burnout speed; values 0, 5, 30, 60, -20. [inferred] |
| ENISCarDamageReset | `991DD250` | none | 5 | 9 | Resets the car's damage. The car handle is not in the data. [inferred] |
| ENISCarPitch | `A18E1896` | 4: f `fPitch`; 8: f `fTime` | 34 | 97 | Pitch offset and blend time. [inferred] |
| ENISCarRoll | `3081837D` | 4: f `fRoll`; 8: f `fTime` | 28 | 58 | Roll offset and blend time. [inferred] |
| ENISCarShake | `FB3DAD79` | 4: f `fShake`; 8: f `fRate`; 12: f `fRamp`; 16: f `fTime` | 23 | 111 | Shake amplitude, rate, ramp-in, duration. [inferred] |
| ENISConstraint | `22AA36CE` | 4: f `fDegrees` | 0 | 0 | Not used in any NIS. |
| ENISCopCarDoors | `3F56A04E` | 4: i `fCarDoor`; 8: f `fCarDoorPos`; 12: f `fCarDoorAnimLength`; 16: f `fCarDoorEndPos` | 10 | 30 | Door index (3 in 20 uses, 0 in 8); position, animation length, end position. [inferred] |
| ENISCopLights | `83E731C7` | 4: i `fEnable`; 8: f `fLightIntensity`; 12, 16, 20: f `fLightPosX/Y/Z` | 0 | 0 | Not used in any NIS. |
| ENISDetach | `257A10A8` | 4: f `fSpeed` | 20 | 186 | Detaches a part at a speed; always followed by a reattach in the data. [inferred] |
| ENISDetail | `74E96511` | 4: i `fSceneryDetail` | 0 | 0 | Not used in any NIS. |
| ENISFakeFar | `49D34FF1` | 4: f `fFakeFar` | 13 | 41 | Fake draw distance; values 2 to 500. [inferred] |
| ENISFreeze | `7E045D1F` | 4: i `fFreezeAnimation` | 0 | 0 | Not used in any NIS. |
| ENISHideCharacter | `534BEE88` | 4: p `fAnimModelName`; 8: i `fAnimShow` | 8 | 16 | Shows or hides a named character model. [inferred] |
| ENISLights | `5E565165` | 4: u `fCopLights`; 8: u `fHeadLights` | 38 | 86 | Cop lights and head lights, 0 or 1. [inferred] |
| ENISMotionBlur | `FC382D92` | 4: i `fEnableMotionBlur` | 23 | 25 | Motion blur on or off. [inferred] |
| ENISNeutralRev | `00EB198B` | 4: u `fOn`; 8: f `fThrottle`; 12: f `fSpeed` | 5 | 18 | Neutral-revs mode with throttle and speed. [inferred] |
| ENISNitro | `EB348F62` | 4: u `fOn` | 21 | 104 | Nitrous on or off. [inferred] |
| ENISNukeSmack | `A87E176D` | 4: f `fPositionX`; 8: f `fPositionY`; 12: f `fPositionZ`; 16: f `fRadius` | 11 | 21 | Smacks breakables within a radius of a point. The decomp has `NIS_NukeSmackablesWithinRange(position, radius)`, which takes the same inputs; the link to this event is [inferred]. |
| ENISOverlayMessage | `37A3EE72` | 4: p `fOverlayName`; 8: p `fOverlayMessage` | 31 | 60 | Shows a front-end overlay package with a message. Strings seen: packages `BUSTED_OVERLAY.fng`, `BLACK_FADE_OUT.fng`; messages `FLASHTRIGGER`, `15_OUT`. [verified strings] |
| ENISPixelate | `86D368CB` | 4: i `fEnable`; 8: f `fWidth`; 12: f `fHeight` | 24 | 116 | Pixelation on or off, with a block size. [inferred] |
| ENISPlayEffect | `58AAE7A8` | 4: p `fEffect`; 8, 12, 16: f `fPositionX/Y/Z`; 20, 24, 28: f `fMagnitudeX/Y/Z` | 12 | 31 | Plays a named effect at a point, with a magnitude vector. The string table of `IntroNisBL03` holds `exp_lg2`, `env_extrasparks`, `env_carsmash1`, `dustbillow_lg1`, `glasstrail1`; that these are this event's effect names is [inferred]. |
| ENISRain | `6A465054` | 4: f `fRainIntensity` | 16 | 20 | Rain intensity. The value -1 (11 of 20 uses) probably means "leave unchanged". [inferred] |
| ENISReattach | `CF01B27E` | 4: i `fRepairDamage` | 18 | 177 | Reattaches a part; repair value 0 in all uses. [inferred] |
| ENISRoadNoise | `B9A02FA8` | 4: i `fDisableRoadNoise` | 1 | 4 | Disables road noise (`IntroNis01`). [inferred] |
| ENISScreenFlash | `9F79A4ED` | 4: f `fLength` | 0 | 0 | Not used in any NIS. |
| ENISSteering | `7282975D` | 4: f `fDegrees`; 8: f `fBlendWeight` | 4 | 8 | Steering angle in degrees and blend weight (10 or 20). [inferred] |
| ENISStopEffects | `C0582354` | none | 1 | 1 | Stops the effects the NIS started (`IntroNisDD`). [inferred] |
| ENISTimeOfDay | `F699BF04` | 4: f `fTimeOfDay`; 8: f `fUpdateRate`; 12: f `fSunAzimuth`; 16: f `fTimeOfDayOvercast` | 18 | 78 | Sets the time of day, sun azimuth and overcast. A third-party note on the PC handler describes the same four fields: [lead] ([mod note](../mods/nfsmw_enistimeofday.md)). |
| ENISVisualLook | `41D652BB` | 4: f `fDesaturation`; 8: f `fColourBloom`; 12: f `fBleachFactor`; 16: i `fUseHDR`; 20: f `fFacingScale`; 24: f `fGrazingScale` | 0 | 0 | Not used in any NIS. |
| ENISWolrdGeometry | `509649B1` | 4: p `fSceneryGroupName`; 8: i `fEnable` | 1 | 1 | Shows or hides a scenery group (`IntroNisBL07`). The name is spelled this way in the decomp. [decomp] |
| ENISWorldAnimTrigger | `62F10875` | 4: p `fAnimTreeName`; 8: f `fTimeSet`; 12: i `fAnimPause`; 16: i `fAnimHide` | 0 | 0 | Not used in any NIS. |

**Handlers.** The generated `.cpp` for every `ENIS` type is 0 bytes in the CC0 decomp. [decomp] A code search finds
these names only in the symbol list. So what each handler does on the PC is **[unconfirmed – needs Ghidra]**. The
original's handler for an event is reached through the event's `MakeEvent` callback (for example
`ENISCarShake_MakeEvent_Callback`) and its class. Find that class's vtable and its process override in `speed.exe`.

## 3. Event reference: the other events NIS sequences use

| Event | Hash | Payload | Scenes | Occurrences | Meaning and evidence |
|---|---|---|---|---|---|
| ECellCall | `885F756E` | 4: i `fID` | 2 | 3 | Phone call. Ids 13 and 11 in `EndingNis03`; id 12 in `IntroNis10`. See [career phone](career-phone.md) section 5.1. [verified] |
| ECameraPhotoFinish | `2096C371` | none | 8 | 33 | Photo-finish camera. Used in `IntroNis04`, `IntroNis06`, `IntroNis07`, `IntroNisBL07`, `IntroNisBL07End`, `IntroNisBL10`, `IntroNisBL12`, `IntroNisBL16`. The handler is in `Frontend/MenuScreens/InGame/PhotoFinish.cpp` (decomp), not read here. |
| EDDaySpeech | `A67E8286` | none | 1 | 1 | A speech line, in `IntroNisDD`. [verified]; handler not read. |
| EPlayObjectEffect | `C1AF4FDD` | 4: p `fEffect`; 8: c `fPosition`; 12: f `fIntensity`; 16: c `fID`; 20: u `fTracking`; 24: u `fAllowHiddenSpawn` | 3 | 5 | Starts an effect on an object. Used in `IntroNis07`, `IntroNisBL03`, `IntroNisBL16`. The `fID` pairs with `EStopObjectEffect`. [decomp layout] |
| EStopObjectEffect | `3338EFB8` | 4: c `fID` | 1 | 1 | Stops the effect with that id (`IntroNisBL16`). [verified] |

Defined in the decomp but used by no NIS bundle: `EAddSMS` (`97EB86D4`), `EShowSMS` (`86D32FBB`), `EFadeScreenOn`
(`5F8C3748`), `EFadeScreenOff` (`77803D76`), `EFadeScreenNoLoadingBarOn` (`D1703F59`), `EFadeScreenNoLoadingBarOff`
(`7B129988`), `EPlayRaceNIS` (`66666DC8`), `EPlayEndNIS` (`4FB8AE5C`) and `ETriggerMomentNIS` (`7DF7F613`). [verified:
zero occurrences]

## 4. Usage census

Across the 59 bundles with a sequence there are 274 responses and 1,507 event references. A reference is one event in
one action's list, so a list used by two responses counts twice. The distinct lists reached by each engine's responses
hold 740 event items. [verified]

- 31 event types appear: 26 `ENIS` types and the 5 others in section 3.
- By count of uses, the most used are `ENISDetach` (186 in 20 scenes), `ENISReattach` (177 in 18), `ENISPixelate`
  (116 in 24), `ENISCarShake` (111 in 23), `ENISNitro` (104 in 21) and `ENISCarPitch` (97 in 34).
- By number of scenes, `ENISLights` is the widest (38 scenes), then `ENISCarPitch` (34) and `ENISOverlayMessage` (31).
- The seven unused `ENIS` types are `ENISConstraint`, `ENISCopLights`, `ENISDetail`, `ENISFreeze`,
  `ENISScreenFlash`, `ENISVisualLook` and `ENISWorldAnimTrigger`.
- The per-scene table and the sample timelines are in [nis-events-2.md](nis-events-2.md).

The requested scene `IntroNisBL01` has no sequence. [verified] It is covered in section 7, because it has an audio
entry and a rev file.

## 5. Parameters of the sample scenes

Times are seconds from the action's start. Full timelines are in [nis-events-2.md](nis-events-2.md#b-sample-timelines-verified).

- `IntroNis05`: 11 systems. Eight `ENISDetach` at about 4.7 s. Burnout, neutral revs and nitrous in one car system; a
  car shake (0.003, rate 0.9, ramp 1, time 6) at 2.9 s; a brake lock at 12.0 s. [verified]
- `EndingNis03`: 8 systems. Phone call 13 at 0.05 s and call 11 at 8.0 s; detach and reattach events over 4 to 18 s;
  overlay `BLACK_FADE_OUT.fng` with message `15_OUT` at 26.3 s. [verified]
- `ArrestF02`: 3 systems. Cop door 3 is set at the start and reset at 0 s; pixelation on at the start; overlay
  `BUSTED_OVERLAY.fng` with message `FLASHTRIGGER` at 3.4 s; pixelation off at 7.0 s. [verified]
- `IntroNisBL01`: no sequence. [verified]

## 6. The rewrite subset

Used by at least one NIS: 31 types (26 `ENIS`, 5 other). They are grouped below by what the rewrite must do. A type
marked *stub* needs only to be parsed and ignored until its handler is known.

**Camera and screen effects**
- `ENISPixelate`, `ENISOverlayMessage` (an overlay package and message), `ENISMotionBlur`, `ENISFakeFar`, `ENISRain`.
  Each sets a screen-level state.
- `ECameraPhotoFinish`: a call into the photo-finish camera. It needs the photo-finish screen; the handler in
  `PhotoFinish.cpp` (decomp) is not yet read.
- `ENISTimeOfDay`: time of day, update rate, sun azimuth and overcast. Needs the world's lighting state.

**Car effects**
- Body motion: `ENISCarPitch`, `ENISCarRoll`, `ENISCarShake`, `ENISSteering`, `ENISAeroDynamics`.
- Drive state: `ENISBurnout`, `ENISNitro`, `ENISNeutralRev`, `ENISBrakelock`. These overlap with the NIS rev curves
  (section 7.7).
- Damage: `ENISDetach`, `ENISReattach`, `ENISCarDamageReset`.
- Lights: `ENISLights` (cop and head lights). `ENISRoadNoise` is a stub (one use).

**Characters**
- `ENISCopCarDoors` (cop car door animation); `ENISHideCharacter` (shows or hides a named model).

**World**
- `ENISNukeSmack` (smacks breakables in a radius; the decomp has the matching function).
- `ENISWolrdGeometry` (shows or hides a scenery group; one use, a stub until the group lookup is known).
- `ENISPlayEffect`, `EPlayObjectEffect`, `EStopObjectEffect` (named effects with a magnitude; the object pair is matched
  by `fID`).

**Audio**
- `ENISStopEffects` (stops the NIS's effects; one use). `EDDaySpeech` (one speech line). The speech itself belongs to
  the speech system ([speech.md](speech.md)).

**Phone and messages**
- `ECellCall` (three uses; the phone's message store is in [career phone](career-phone.md)).

**Not needed by any NIS.** The seven unused `ENIS` types, and the nine other events listed in section 3 as unused.
Define them in the rewrite only when other content needs them.

## 7. NIS audio and engine revs

### 7.1 Where the audio comes from

The audio is a stream from `STREAMS/NISAudio.big`, with its `.idx`, `.evt` and `.csi` (formats in
[audio.md](../formats/audio.md#speech-and-nis-streams-big--idx--evt--csi)). The stream is chosen by the scene's name
and by the CSIS selection the scene makes. [decomp]

### 7.2 When the audio is queued

1. The activity starts loading the scene's animation, with the camera track number and a flag that locks the track
   streamer for arrests. [decomp: `NISActivity::UpdatePreloading`]
2. When that load finishes, the activity queues the NIS stream with the scene's audio key, the camera track number and
   a time callback. In the same step it opens the scene's rev file (section 7.7). [decomp]
3. This happens during the loading states: before the scene plays, and again after a pre-movie has finished.
   [decomp: `OnMovieComplete`]

### 7.3 Choosing the stream

The stream queue (`SFXObj_NISStream::QueueNISStream`) does four things. [decomp]

1. It looks the scene up in a 68-row table. Each row has a scene name, its `bStringHash` and an index. The table is built
   by `GenerateNISAnimHashMap`. Its rows are:

| Index range | Count | Scenes | Path |
|---|---|---|---|
| 0 | 1 | `GenericStart` | start; the only scene that uses its camera track |
| 1 to 17 | 17 | `IntroNis01`..`IntroNis10`, `Flythrough`, `IntroNisDD`, `IntroNisDDEnd`, `IntroNisToll01`, `GarageEnter`, `GarageExit`, `OPMRivalIntro` | start: `NIS_Select_Start` |
| 18 to 35 | 18 | `IntroNisBL01`..`IntroNisBL16`, and the two end variants `IntroNisBL06End`, `IntroNisBL07End` | blacklist: `NIS_Select_Blacklist` |
| 36 to 67 | 32 | `ArrestF02`..`ArrestM23` and their `b` variants; `EndingNis01`..`EndingNis04` | end: `NIS_Select_End`, with the busted stream |

2. It picks a camera track. Only index 0 uses the NIS's track number; every other scene uses track 0. [decomp]
3. It picks the section. Normal play uses the *complete* section. A skip (the button-through path) uses the *end*
   section; for two blacklist entries it remaps indices 24 and 26 to 23 and 25. [decomp]
4. It queues the stream on the speech module: the start stream for indices 0 to 35, the busted stream for 36 to 67. It
   sets the NIS mixer input (input 6 for start, input 7 for busted) to full volume, or to zero when skipping. [decomp]

Scenes outside the table play no NIS stream. Those are the 26 bundles that are not in it: 8 FMVs, 4 replays and 14
moment scenes. The moment cut plays no sound, as [pursuit-breaker-moment.md](pursuit-breaker-moment.md) section 5 says.

### 7.4 The CSIS interfaces and the stream file

The three selectors above are EA CSIS interfaces with fixed IDs. The decomp gives each a name, a group id `0x0BE4` and a
CRC. [decomp] The install's `SOUND/STREAMS/NISAudio.csi` holds the same four interfaces, with the same names and CRCs.
[verified]

| Interface | CRC | Parameters (in order) |
|---|---|---|
| `NIS_Select_Start` | `0x67D1` | scene start (a bit flag, 20 values), track (bit flag), section |
| `NIS_Select_End` | `0x4888` | scene end (a bit flag, 21 values), track, section |
| `NIS_Select_Blacklist` | `0x3B39` | track, section, blacklist (a bit flag, 19 values) |
| `SoundFX_Select` | `0x2052` | sound type, sound parameter (used by moments and effects) |

The mapping from these parameters to one stream in the `.big` is inside the CSIS library and the `.evt` table. It is
not in the CC0 decomp outside the generated headers. Which stream a parameter set picks is **[unconfirmed – needs
Ghidra]**: the `EAXSound` CSIS runtime and the `.evt` reader.

### 7.5 When the audio starts

- The stream is ready when its channel has buffered data and its queue callback has run. The stream module's update
  sets both. [decomp]
- Playback starts when two flags are both set: the animation is ready (its start was signalled), and the stream is
  ready. The per-frame update then starts stream 2 on the speech module. [decomp: `UpdateParams`, `StartNIS`]
- The files that mention `StartNIS` include `EAXSound.cpp` and `Ecstasy/EmitterSystem.cpp`. The activity's own play step,
  which sends the start, is not in `NISActivity.cpp`. So the frame of scene time zero, relative to the audio start, is
  **[unconfirmed – needs Ghidra]**: find `NISActivity::Play` (the `READY_TO_PLAY` to `PLAYING` change) in `speed.exe`.
- The activity counts 31 ticks in the ready state before it plays, and it does not play while paused. [decomp: `OnTask`]

### 7.6 The clock and the end

- While the stream plays, each frame the update reads the stream's current time and calls the activity's time callback.
  The activity stores it in a static variable. [decomp] **The CC0 file never reads that variable**; it is written in one
  place only. Whether the scene clock follows the audio clock is **[unconfirmed – needs Ghidra]**.
- Engine revs use the same time base (section 7.7). The caller of the rev update is not in the CC0 file set, so the time
  base is **[unconfirmed – needs Ghidra]**.
- When the stream stops, the update calls the stream's *done* step, which zeroes the NIS mixer inputs and clears the
  timers. [decomp]

### 7.7 NISREVDATA: the engine-rev curves

- The loader reads `sound\NISRevData\<scene name>.bin`, where the name is the table's name for that scene. [decomp]
  All 29 files are for scenes in the table. None exist for arrests, endings or moments. [verified]
- Files: `GenericStart`, `IntroNis01`..`IntroNis10`, `IntroNisBL01`..`IntroNisBL16`, `IntroNisToll01`, `OPMRivalIntro`.
  [verified]
- Format: 16 sets. Each set is a point count (u32), then that many points. A point is a time (float), an RPM (i32) and a
  torque (i32). On the PC install the values read as little-endian; the decomp byte-swaps each word on load, which is a
  no-op on PC. [verified layout] [decomp]
- Only 9 files end exactly after set 16. The other 20 have trailing bytes, which the loader ignores. [verified]
- Slots: slots 0 to 7 are `car1`..`car8`; slots 8 to 15 are `cop1`..`cop8`. Each frame the update assigns the slots to
  the NIS's cars. [decomp: `NIS_RevManager::Update`]
- Use per file: most files use only set 0 (`GenericStart`: 187 points). `OPMRivalIntro` uses sets 0 to 3, with 161, 120,
  79 and 80 points. [verified]
- Physics per car, while a NIS is playing. [decomp: `SFXCTL_Physics::UpdateNIS`]
  1. The time into the curve is the NIS time, or the sum of frame times when the NIS time is about zero.
  2. The RPM and torque are interpolated linearly between the two points around that time. When the time passes the
     last point, the curve ends.
  3. When the race countdown is under one second, the RPM blends toward the car's physics RPM over 500 ms.
  4. Outside the curve, RPM falls by 500 and torque by 15 each tick.
  5. RPM is clamped to 1000 to 10000 and torque to 0 to 100. The result goes to the car's engine state.
- A separate pattern, `RevPat5`..`RevPat12` (hard-coded, not from a file), plays the 3-2-1 countdown. [decomp]

### 7.8 A likely decomp error in the table lookup [unconfirmed – needs Ghidra]

`GetCsisEventIndex(hash)` scans the 68-row table and returns the row whose **second** slot (the name pointer) equals the
hash. The table's first slot holds the `bStringHash` of the name. The stream queue and the rev loader both call this
lookup with the scene key. As written in the CC0 decomp, the comparison never matches a hash, so the lookup returns -1
for every scene. In that case no NIS would queue a stream or open a rev file.

The PC game plays NIS audio. So either the decomp is wrong here (the compare should use the hash slot) or the caller
passes something else. To settle it, check `GetCsisEventIndex` in `speed.exe` through its symbol, and check which slot
of the table it reads.

## 8. Facts still missing

1. The handlers for all 26 `ENIS` types on the PC. Start with their vtables; the decomp's generated `.cpp` files are empty.
2. The meaning of stimuli `0` and `0x1A24FA80`, and the names of the NIS tags (their hashes are in section 1.3). Which
   animation event calls `NISActivity::FireEventTag`.
3. `NISActivity::Play` (the start frame) and its caller; the frame of scene time zero relative to the audio start.
4. Whether the scene clock follows the audio time. The callback variable is written but not read in the CC0 file.
5. The caller of `SFXCTL_Physics::UpdateNIS`, and its time base.
6. The CSIS mapping from interface parameters to one stream in `NISAudio.big`, through `NISAudio.evt` and `.idx`.
7. Whether `GetCsisEventIndex` compares the hash slot on the PC (section 7.8).
8. Seven audio table names have no NIS bundle in the install: `ArrestM23`, `ArrestM23b`, `EndingNis01`, `EndingNis02`,
   `GarageEnter`, `GarageExit`, `OPMRivalIntro`. Find the bundles that hold them, if any.
9. The `UCrc32` variant used for car channel keys and for the `c` fields (`fPosition`, `fID`).
10. What cop door index 3 refers to, and the meaning of `fCarDoorAnimLength`.
11. Whether any NIS uses a sequence stored outside its own bundle.
12. The `SpeechManager` side of the stream (`NISSFXModule.cpp` was read; `SpeechManager` was not).

## 9. Sources

- Install (read only, `D:\Need For Speed Most Wanted Black Edition`): `NIS/Scene_*_BundleB.bun` (87 files; 59 with a
  `0x0003B811` sequence chunk), `SOUND/STREAMS/NISAudio.csi`, `SOUND/NISREVDATA/*.bin` (29 files). Parsed with a Python
  reader in the scratchpad. The reader reuses the repo's `tools/chunkdump.py` for the chunk walk and JDLZ, and ports the
  CARP group reader and event reader of `libs/blackbox-carp` and `libs/blackbox-sequence`. Hashes were checked against
  `libs/blackbox-attrib` (vault hash) and `libs/blackbox-hash` (`bStringHash`).
- CC0 decomp (dbalatoni13/nfsmw), read through the GitHub API and not saved to disk:
  - generated event headers `src/Speed/Indep/Src/Generated/Events/` (`ENIS*.hpp` and the other event headers in section 3);
  - `src/Speed/Indep/Src/Sim/Activities/NISActivity.cpp`;
  - `src/Speed/Indep/Src/EAXSound/CARSFX/SFXObj_NISStream.cpp` and `.hpp`;
  - `src/Speed/Indep/Src/EAXSound/Stream/NISSFXModule.cpp`;
  - `src/Speed/Indep/Src/EAXSound/SND_GEN/NISAudio.hpp` and `NISAudio.cpp`;
  - `src/Speed/Indep/Src/EAXSound/sfxctl/SFXCTL_NISReving.cpp`;
  - code search results for each handler name (symbol list only).
- Repo docs: [event formats](../formats/event-sequences.md), [world sequencers](world-sequencers.md),
  [audio](../formats/audio.md), [pursuit breaker moment](pursuit-breaker-moment.md),
  [speedbreaker moment audio](speedbreaker-moment-audio.md), [career phone](career-phone.md), [speech](speech.md),
  [bChunk](../formats/bchunk.md), [animation](../formats/animation.md).
- Third-party lead, not checked: [NFSMW_ENISTimeOfDay note](../mods/nfsmw_enistimeofday.md) (Xbox 360 behaviour of
  `ENISTimeOfDay`).

## Corrections to existing docs

- [event-sequences.md](../formats/event-sequences.md) names the `start`, `end` and `stop` tags as the sequencer's tags.
  NIS data uses none of them; its tags are other hashes that the activity fires by name. Add a note.
- [career-phone.md](career-phone.md) section 5.1 is correct: `ECellCall` is `0x885F756E`, with the id at offset 4, and
  ids 11, 12 and 13 in the three named scenes. [verified]
- [audio.md](../formats/audio.md) section "NISREVDATA" is correct on counts (29 files, 16 sets, 9 exact ends). It does not
  state the byte order; on the PC install the values are little-endian. [verified]
