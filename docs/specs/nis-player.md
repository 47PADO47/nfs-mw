# NIS player: how a cutscene scene is started, placed, timed, and ended

This spec has two parts: this file (sections 0 to 8, the start, the placement, the timing and the cars) and
[nis-player-2.md](nis-player-2.md) (sections 9 to 17, the characters, the camera, the skipping and the end).

How the original runs a NIS (cutscene) scene once its data is loaded: who starts it, where it is placed, how long it
runs, how the cars and characters move, which camera plays, what the player can do, and how it ends. The data the
player reads (the scene chunk, the entity records, the directory and the camera sets) is in
[nis-scenes.md](nis-scenes.md). The arrest flow in more detail is in [busted-sequence.md](busted-sequence.md), and the
world moment is in [pursuit-breaker-moment.md](pursuit-breaker-moment.md).

Evidence tags, as in [docs/README.md](../README.md#evidence-tags):

- **[verified]**: measured on the install (numbers quoted; the data side is in [nis-scenes.md](nis-scenes.md)).
- **[decomp]**: read in the CC0 decompilation (`dbalatoni13/nfsmw`, `src/Speed/Indep/Src/`). The file is named.
- **[derived]**: worked out from two or more [decomp] facts, not stated in the code.
- **[unconfirmed]**: a hypothesis, or a rule whose use is not proven.
- **[unconfirmed – needs Ghidra]**: the CC0 body is missing, stubbed, or empty. Section 15 of [nis-player-2.md](nis-player-2.md) lists the exact functions.

## 0. Summary

1. Three things start a scene: a police bust (the arrest scene, the main case), a race start or intro (through the race
   event `EPlayRaceNIS`, whose consumer is not in the CC0 tree), and a world moment (`ETriggerMomentNIS`, a camera-only
   scene, see [pursuit-breaker-moment.md](pursuit-breaker-moment.md)). **[decomp]**
2. Only one NIS can exist at a time. The NIS activity is a singleton, and a bust is ignored while one runs.
   **[decomp: `NISActivity::Construct`; `NISListenerActivity` in `AnimChooseArrest.cpp`]**
3. The arrest is placed at the centre of the road lane next to the player's car. Its heading is the car's heading plus
   a quarter turn; the scene's own placement then turns it back by a quarter turn. **[decomp]**
4. The scene's frame is the render frame (z up). Its origin is the marker's position with its height taken from the
   ground, and it is turned about the up axis. A scene point maps to the world by a turn, then a shift. **[decomp] and
   [derived]** Types 1 and 4 ignore the marker: origin zero, no turn. Their scenes are authored in render coordinates.
5. Time runs in seconds, but the data counts frames at 30 per second. A scene starts at `StartFrame / 30` and ends at
   `VanishFrame / 30` seconds. The camera track's length equals this in 69 of the 77 scenes that have a track.
   **[decomp] and [verified]**
6. Cars are driven by their animation channels while the scene runs. Physics input is pinned (handbrake on, brake on,
   gas off, speed and velocity zeroed). Each frame the car is placed at its animated pose, lifted 5 m above the ground.
   **[decomp]**
7. Characters play a skeletal clip (looping) and keep their feet on the ground. They are placed in scene space by
   their local matrix. **[decomp]**
8. Camera: the scene's camera track is evaluated by the ICE director. The arrest uses track 0. In the six examples the
   track's space is the scene frame for types 0, 2 and 3 and the world for type 4. **[decomp] and [verified]**
9. The arrest and the ending kinds cannot be skipped. Other scenes skip on the action button (ID 50), which jumps the
   scene to its last frame, unless they are flagged non-skippable. **[decomp: `SkipOverNIS`]**
10. When the scene ends, the NIS-spawned cops are destroyed, the other cars get their animation flag cleared, and a
    completion message (`MNISComplete`) is posted, with a null scene name, unless the scene was skipped to a target,
    in which case an `EPlayEndNIS` event carries the target name. **[decomp: `NISActivity` destructor]**

## 1. Scope of the CC0 source for the player

The CC0 tree has the NIS activity (`Sim/Activities/NISActivity.cpp`, 852 lines), the scene code (`Animation/AnimScene`,
`AnimLocator`, `AnimPlayer`, `AnimEntity*`, `AnimCtrl`), the arrest chooser (`AnimChooseArrest.cpp`) and the interface
header (`INIS.h`). Several bodies that the player needs are **not** in the tree:

| Missing or stubbed in CC0 | Why it matters |
|---|---|
| `NISActivity::Load`, `Play`, `UpdatePlaying`, `UpdateLoading`, `Release` | these run the state machine after loading and the playing frames |
| `ICEManager::GetCameraData`, `GetNumSceneCameraTrack`, `Update`, `Render` (the file is 1.2 KB, mostly the constructor) | the camera track choice and evaluation |
| `ICEAnimScene.cpp`, `ICEData.cpp`, `ICEMover.cpp`, `ICERender.cpp` (0 bytes) | the ICE evaluator |
| `EPlayRaceNIS` consumer (a search of the tree finds only the generated header) | how a race start picks its scene and camera |
| The `INISCarControl` implementation (`SetNISPosition`) | how the car takes the pose |
| `CAnimScene::Cue`, `IsCued`, `IsStopped`, `GetTime`, `CAnimPlayer::AreAnyControllingCamera`, `FindCurrentWorldCarIndex` (stripped) | some callers of the play status and the camera |
| `ForcePlayerToAnimCarPosition` (empty body), `ForceCarToAnimCarPosition` (stripped) | the replay's player placement |

Where a fact below depends on a missing body, it is marked. Section 15 of [nis-player-2.md](nis-player-2.md) lists the exact functions.

## 2. How a scene is started

### 2.1 By a police bust (the arrest) **[decomp: `AnimChooseArrest.cpp`]**

The listener `NISListenerActivity` is created when a quick race starts (`QuickGame::OnBeginState`). The career path that
creates it was not checked.
It receives the bust message (`MPerpBusted`, sent on the NIS channel by the pursuit code, `AIPursuit.cpp`). Its steps:

1. If a NIS already exists (the singleton is taken), stop. Nothing happens.
2. If there is no race parameter (free roam), set the player-busted flag of the quick-race car select.
3. Choose the scene name and the camera track (section 2.2). The track is 0.
4. Choose the location (section 2.3).
5. If a location is found: create the `NISActivity` (through the activity factory), add the player's car on the channel
   `car1`, set the start location, and load the scene with the arrest kind, playing as soon as it is loaded.
6. If no location is found: in free roam, quit to the front end on the `Infractions.fng` screen (career safehouse). In a
   race, post a pause event (`EPause`, 0, 1, 0). **[decomp]** The `#ifndef EA_BUILD_A124` build flag wraps this branch.
   The busted-sequence spec describes a "request object" for the race case; the decomp shows a pause event.

Timing: the bust message itself arrives when the pursuit ends. The 3-second delay in the busted spec is not in this
file **[unconfirmed]** ([busted-sequence.md](busted-sequence.md), section 2).

### 2.2 The arrest scene name **[decomp: `ChooseArrestAnimation`]**

The scene depends on the perp's heat level (`IPerpetrator::GetHeat()`), and on a random pick of 0 to 7
(a random number from the game's generator, scaled by 8 and rounded down).

| Heat | Pick 0 to 3 | Pick 4 to 6 | Pick 7 |
|---|---|---|---|
| 0 or 1 | `ArrestM01`, `ArrestM16`, `ArrestF02`, `ArrestF18` | `ArrestM01b`, `ArrestM16b`, `ArrestF02b` | `ArrestF18b` |
| 2 | (falls through, see below) | | |
| 3 and above | (falls through) | | |
| fall-through set | `ArrestM06`, `ArrestM19`, `ArrestF06`, `ArrestF07` | `ArrestM06b`, `ArrestM19b`, `ArrestF06b` | `ArrestF07b` |

The `FIX_BUGS` branch would add `break`s, so that heat 2 and 3 pick their own sets. Without it (the shipped code, per
the busted-sequence spec), heat 2 and 3 fall through to the last set. **[decomp; the shipped form is [derived] from the
`#ifdef`]** The busted spec's "class" value is therefore the perp's heat, not a class **[decomp]**.

The choice of track is always 0. The chooser calls the camera-count function and discards its result
**[decomp]**. `ArrestM23` and `ArrestM23b` are never chosen (no bundle exists either; [nis-scenes.md](nis-scenes.md)).

### 2.3 Where the arrest happens **[decomp: `ChooseArrestLocation`]**

1. Take the player's car transform. The position is its translation. The heading is `1 − atan2a(x, z) + 0.25`, with
   `(x, z)` the car's forward vector, in turns **[unconfirmed units]**.
2. Ask the road network for a cop-path navigation at that position and direction (`WRoadNav::InitAtPoint`). If it is
   not valid, there is no spot.
3. The spot is the midpoint of the road's left and right edges at that point (the centre of the lane).

### 2.4 By a race start, through the race event **[decomp: event fields; consumer missing]**

The race event `EPlayRaceNIS` carries a start marker (a `GMarker`), a scene name, a scene type string, the player index,
a camera track number, and optional pre and post movies **[decomp: `EPlayRaceNIS.hpp`]**. The scene type is a string,
not the numeric `SceneType` of the chunk. The race start is therefore keyed by its marker and its camera track,
both given by the race, not by the chooser. Who handles the event is **[unconfirmed – needs Ghidra]**.

The quick-race intro scene is named by the race's gameplay object (`IntroNIS()`). If one is named, the world's zone
switching is disabled while loading; otherwise the stream is set at the start point **[decomp: `QuickGame` constructor]**.
The call that starts the intro NIS is not in the tree **[unconfirmed]**.

### 2.5 A world moment **[decomp: `CAnimMomentScene`; pursuit-breaker-moment.md]**

`ETriggerMomentNIS` starts a camera-only scene (`CAnimMomentScene`). It has no bundle, no cars and no actors. The scene
holds a scene hash, a camera track, a rotation and a transform, and it ends when its elapsed time reaches its total time.
Where the event's values come from, and who sets the total time, were not located in this tree **[unconfirmed]**. The
moment is covered in [pursuit-breaker-moment.md](pursuit-breaker-moment.md).

### 2.6 Singleton rules **[decomp]**

- `NISActivity::Construct` returns nothing if a NIS exists. One NIS at a time.
- The AI cop and traffic managers check for a NIS. Traffic spawning stops while one exists (`AITrafficManager.cpp`
  line 628). Cop despawning is blocked unless the NIS is a world moment (`AICopManager.cpp` line 449).
- `QuickGame::ShouldPauseInput` returns true while a NIS exists, so the player's input is paused.

## 3. The NIS activity and its states **[decomp: `NISActivity.cpp`]**

The activity has eight states, in this order: none, creating, loading, ready to play, pre-movie, playing, post-movie,
complete.

| State | Entered when | Leaves when |
|---|---|---|
| none | construction | a load is requested |
| creating | the pre-movie ends (or no pre-movie) | loading starts (`UpdatePreloading`) |
| loading | the scene's data is requested | the scene is loaded (`ServiceLoads` / `UpdateLoading`) |
| ready to play | the scene is loaded | the start conditions hold (section 4) |
| pre-movie | a pre-movie is set | the movie reports finished (`MNotifyMovieFinished`) |
| playing | `Play` | the scene finishes or is skipped |
| post-movie | a post-movie is set and the scene ends | the movie reports finished |
| complete | the scene ends or a load fails | the activity is released |

Failing to load moves the state straight to complete, with no screen and no fallback camera **[decomp:
`UpdatePreloading`]**. This matches the busted spec's note on the missing fallback.

## 4. Loading and the start conditions **[decomp; unconfirmed where noted]**

### 4.1 Loading the bundle **[decomp: `AnimPlayer.cpp`, `CAnimPlayer::Load`]**

1. The player finds the scene's entry in the directory (section 6.1 of [nis-scenes.md](nis-scenes.md)) and adds up the
   size of its files (shared files first, then the scene file). For a NIS this is the bundle itself.
2. Memory: if the total is at least 900,001 bytes (`0xDBBA1`), no memory is granted and the load fails. If it is over
   20,000 bytes, it is taken from the track streamer's pool, or the car pool when that is full. Otherwise it comes from the
   main heap. **[decomp]**
3. The files are read in order, each aligned to 16 bytes.
4. For the arrest kind, zone switching in the track streamer is disabled during the load (the load's
   `DisableZoneSwitching` argument is set for that kind).

**Conflict [verified against decomp]:** all 26 arrest bundles are between 1,289,984 and 1,729,920 bytes, which is over
the 900,001-byte refusal limit. A straight reading would refuse every arrest. The arrests do play, so either the limit
means something else, or the sum is not the bundle size. **[unconfirmed – needs Ghidra]** (section 15 of [nis-player-2.md](nis-player-2.md)).

The audio stream for the scene is queued at the same time, and the rev data for the scene is opened if the rev manager
exists **[decomp]**. The scene's elapsed audio time (in milliseconds) comes back through a callback and is stored; the
code that uses it for pacing is not in the tree **[unconfirmed]**.

### 4.2 The start conditions **[decomp: `NISActivity::OnTask`]**

While the state is ready to play, each tick the activity:

1. Reads the player's action queue (section 12 of [nis-player-2.md](nis-player-2.md)).
2. Plays the scene only when the auto-start flag is set, the activity is not paused, and the settle counter has reached
   31. Otherwise it increments the counter and updates the scene by zero time, so the first pose is shown without the
   clock moving.

The tick is the world's fixed frame task. So the settle is 31 frames of the frame task, roughly one second at 30 frames
per second **[unconfirmed rate]**.

## 5. The scene frame and the placement **[decomp: `AnimLocator.cpp`, `AnimScene::Init`, `NISActivity::StartLocation`;
derived for the axes]**

### 5.1 Frames **[derived]**

Three frames appear:

- **World frame** (the physics and simulables). The up axis is **y**. Positions the caller passes to `StartLocation`
  and the car matrices are in this frame. **[decomp: `ANIM_GetWorldHeight` reads y as height]**
- **Render frame** (the ICE camera and the scene's own frame). The up axis is **z**. `StartLocation` converts a world
  point to this frame with the swizzle function. **[decomp: `NISActivity::StartLocation`]**
- **Scene frame** (the scene's authored coordinates). The scene's `+y` axis is the way the camera looks, and `+z` is up.
  This is the convention of the ICE keys (space 3) and of [formats/animation.md](../formats/animation.md).

The conversion between the render frame and the world frame is, for a render point `(X, Y, Z)`, the world point
`(−Y, Z, X)`. This is read from `NISActivity::GetNISStartLocation`, and it is consistent with the up axis of each frame.
**[decomp; derived]** The inverse maps a world point `(x, y, z)` to the render point `(z, −x, y)`.

### 5.2 The origin and the turn **[decomp: `CAnimLocator::GetInitialAnimMatricies`]**

For scene types 0, 2 and 3 (the "start line" types, section 6):

1. The origin is the marker point in the render frame. Its height is replaced by the ground height under it, found by
   a face query and, when that fails, by a collision ray upward from the point. If no ground is found, the height stays
   as given. **[decomp]**
2. The turn is the marker heading, in 16-bit angle units (65,536 per turn), turned about the up axis by a quarter turn
   less (the heading minus 90 degrees). **[decomp]**
3. The scene transform is the turn followed by the shift to the origin: a scene point `p` maps to the origin plus the turn
   applied to `p`. **[derived from the order of the matrix products in `CAnimScene::Init` and
   `AnimatedCars_SetMainAndWheels`]**

For scene types 1 and 4 (section 6), the origin is zero and the turn is zero, so the scene transform is the identity.
The scene frame is then the render frame itself, so such scenes are authored in render coordinates. **[decomp and
[verified]: the eye of the type-4 cameras is in the render frame (section 10.2 of [nis-player-2.md](nis-player-2.md))]**

The marker heading of the arrest is the value from section 2.3. Whether the arrest scene's `+y` lines up with the car's forward
direction depends on the unit and sign of the heading (section 2.3), which are **[unconfirmed]**.

### 5.3 Ground under the actors **[decomp]**

- The car pose is lifted so that the car's height is the ground height under its position plus 5 metres
  (section 7.4).
- A character keeps its feet on the ground: its pelvis (bone 1) is tested against the ground, and its root is moved
  vertically by the difference, plus 5 cm, smoothed half-and-half with the previous height (section 9.3 of [nis-player-2.md](nis-player-2.md)).

## 6. Scene types and what each one does **[decomp: `CAnimScene::Init`, `SetCarAnimationPositions`]**

The `SceneType` field of the header (`0x18`) selects the rules. The install uses 0, 2, 3 and 4 (1 is unused). The
counts are in [nis-scenes.md](nis-scenes.md), section 3.

| Type | Scenes (install) | Start-line placement | Car channels taken | Properties added | Notes |
|---|---|---|---|---|---|
| 0 | 25: race starts and intros (`IntroNis01`–`09`, `GenericStart`, `IntroNisToll01`), world moments (14) | yes | `car1`..`car8` (indices 0 to 7) | `ControlRaceCountdown`, `UnBindRaceCars` | the race countdown property; the camera is released at the end (section 10.1 of [nis-player-2.md](nis-player-2.md)) |
| 1 | none | no (origin) | none | none | never used |
| 2 | 26 arrests | yes | `car1`..`car8` and `cop1`..`cop8` (indices 0 to 15) | none | no automatic end unbind for the cars (section 7.5) |
| 3 | 4 replays | yes | `car1`..`car8` | `ControlRaceCountdown`, `UnBindRaceCars` | the player is forced to car slot 1 by an empty function **[unconfirmed]** |
| 4 | 32: career and D-Day starts, endings, FMVs, `IntroNis10`, `Flythrough` | no (origin) | `car1`..`car8` and `cop1`..`cop8` | none | world-space scenes |

Two more rules apply to every type:

- A moment (candidate kind 4) takes no starting-position cars.
- `HaveCarAnimation` 0 (`FMVNoCars`, `Flythrough`) creates no car controllers.

The type-0 and type-3 scenes add `UnBindRaceCars`: the cars are released when the clock passes the end (section 7.5).
That is the only automatic release.

## 7. Cars: which channel is which **[decomp; unconfirmed where noted]**

### 7.1 The channels **[decomp: `INIS.h`, `AnimScene.cpp`]**

Sixteen car slots exist. Slots 0 to 7 are the `car1` to `car8` channels (the player and the racers). Slots 8 to 15 are
the `cop1` to `cop8` channels (the police cars). A car is attached to a channel with `NISActivity::AddCar`, which marks
it as animating and zeroes its speed.

| Who | Channel | Source |
|---|---|---|
| Arrest player | `car1` | the player's own car, added by the chooser (`IVehicle::First(VEHICLE_PLAYERS)`) **[decomp]** |
| Arrest cop | `cop1` | a NIS cop car. The arrest's special car list is code 0 (one police car, `copmidsize_nis`) for all 26 arrests **[verified: table]**. Who creates it is **[unconfirmed]** |
| Race racers | `car1`..`car8` | added by the race code, not in CC0 **[unconfirmed]** |
| NIS cops | `cop1`..`cop8` | `PrepareVehicles` creates NIS police cars from the special car list (one, several, a sport cop, or one of nine traffic mixes), at the origin, facing +x, with the NIS driver **[decomp]**. Its caller is **[unconfirmed]** |

The special car list codes are in [nis-scenes.md](nis-scenes.md), section 6.2.

### 7.2 Setting up the controllers **[decomp: `CreateCarAnimationControllers`]**

For each slot that has a car, if the scene has car animation and the root skeleton exists:

1. The channel names are the scene name plus `ZPM_Car<n>` (slots 0 to 7) or `ZPM_Cop<n>` (slots 8 to 15), with suffix
   `_t` (translation) and `_q` (rotation). The `_s` channel is not used.
2. The controller's time scale is one half. So car clips run at 15 frames per second of scene time **[derived; the
   half-rate is the decomp's, the consequence is [unconfirmed]]**.
3. The controller's flags are "linear, no loop": the clip clamps at its last frame and holds it.
4. If the controller reports that it allocated (the code tests this after both channel requests), the car is activated.
   If not, the controller is discarded and the car stays inactive. Cars `car1` to `car8` are deactivated before this
   step. So a car with no clip stays deactivated **[derived]**.
5. If at least one controller exists, the property `ControlRaceCars` is turned on.

### 7.3 Pinning the physics **[decomp: `AnimatedCars_Bind`]**

When the scene starts (`BindToGame`), and only if `ControlRaceCars` is on, every car with a slot gets:

- speed 0 (`SetSpeed`), linear and angular velocity zero;
- handbrake fully on, brake fully on, gas zero.

So the car does not move by its own physics. Its pose comes from the clip.

### 7.4 Placing the car each frame **[decomp: `AnimatedCars_SetMainAndWheels`, `AnimatedCars_Update`]**

1. Each controlled car advances its clip by the scene's time step, then takes the pose.
2. The car's root matrix from the pose is multiplied by the scene's turn and shift (section 5.2), converted to the world
   frame.
3. If the ground is found under the car's point, its height is set to that ground plus 5 metres.
4. The car's position is handed to the car's NIS control interface with a flag for whether this is the first frame
   (`SetNISPosition`). If that returns false, the car's last-position counter is set to −1, a sign that the car refused the
   pose. The consequence is **[unconfirmed]**.
5. The car's last-position counter counts up each frame.

The car's wheels follow from the same pose (`AnimatedCars_SetMainAndWheels` is named for the main body and the wheels).
The wheel poses are not decoded in CC0 **[unconfirmed]**.

### 7.5 Releasing the cars **[decomp: `AnimatedCars_UnBind`, `NISActivity` destructor]**

- `AnimatedCars_UnBind` turns `ControlRaceCars` off. Each car with a slot is set not animating. Cops (slots 8 to 15)
  are also deactivated.
- It runs at the end of the scene for type 0 and 3 (section 6), and when the scene is stopped and unbound.
- When the NIS activity is released, the cops `cop1` to `cop8` are removed from the NIS and **killed**. Every other NIS car
  is removed and set not animating **[decomp: destructor]**.

So the NIS cops are destroyed at the end of the scene, and the player's car (`car1`) and the racers survive. What the
physics does with a surviving car after release (its input pins, its last pose, its speed) is **[unconfirmed]**: the
input pins of section 7.3 are not undone in the tree.

### 7.6 Replay placement **[decomp; unconfirmed]**

For type 3 the player car is forced to a start slot by `ForcePlayerToAnimCarPosition(0, 1)`. Its body in CC0 is empty and
the starting positions come from the race's own car channels. **[unconfirmed – needs Ghidra]**

## 8. Cops, traffic and the world during a scene **[decomp]**

- Traffic density is zero while a NIS exists (section 2.6).
- Cop spawning is blocked while the NIS is not a moment. Cops already on the road can be removed only by a moment
  (section 2.6).
- The NIS cops are not in the traffic or pursuit lists. They are NIS-owned vehicles (`DRIVER_NIS`).
- The NIS can also drive cars through the effect events. Their names are in the decomp's generated events, among them:
  `ENISCarPitch`, `ENISCarRoll`, `ENISCarShake`, `ENISSteering`, `ENISBurnout`, `ENISBrakelock`, `ENISNitro`,
  `ENISNeutralRev`, `ENISCarDamageReset`, `ENISDetach`, `ENISReattach`, `ENISFreeze`, `ENISHideCharacter`,
  `ENISLights`, `ENISCopCarDoors`, `ENISCopLights`, `ENISScreenFlash`, `ENISPixelate`, `ENISMotionBlur`,
  `ENISRain`, `ENISTimeOfDay`, `ENISOverlayMessage`, `ENISWorldAnimTrigger`, `ENISNukeSmack`, and others. Their handlers are in the
  scene's event sequence, which is not decoded (the `CarpEventSequence` chunk, [nis-scenes.md](nis-scenes.md)
  section 1.1). **[unconfirmed]**
- The car's NIS control interface carries pitch, roll, shake, steering, burnout, brake lock and nitro hooks
  (`INISCarControl`). **[decomp: header]** Their use is through the events above.
- Scenery can be hidden in an area. `NIS_NukeSmackablesWithinRange` hides and releases the models near a point
  (`IsValidModelToNuke`: scenery, placeable scenery, or a vehicle's model) **[decomp]**. The event that calls it is not
  in CC0.
- The cop car's doors can open by a timed animation (`StartCopDoorAnim`, `UpdateCopDoorPositions`). **[decomp]**
