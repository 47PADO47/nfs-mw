# NIS player, part 2: characters, camera, skipping and the end of a scene

Part 1 is [nis-player.md](nis-player.md) (sections 0 to 8). Section numbers continue from part 1. Evidence tags
are as in part 1.

## 9. Characters **[decomp: `AnimEntity_BasicCharacter.cpp`]**

### 9.1 Setup **[decomp]**

Each entity record (type 1) becomes one character:

1. A space node is created under the scene's node, at the record's local matrix (scene space). Its name is the record's
   instance name hash. The parent hash is 0 for every record in the install.
2. A world model is made from the record's three LOD name hashes. The hashes do not resolve to strings in the bundle, so
   the solids they name are **[unconfirmed]**.
3. An animation controller is made. Its time scale is the play speed (1.0 in all 64 records). If the record has a
   skeletal clip, the time scale is halved again (0.5), so a character clip runs at 15 frames per second. **[derived]**
4. The play flags (0x20 in all 64) become controller flags 0x2A: linear, with the loop bit 0x20 and the bit 0x2 (its
   meaning is open, see section 15). The loop bit means a character's clip repeats until the scene ends. The clip frames are in [nis-scenes.md](nis-scenes.md),
   section 4.1.
5. The skeleton is fetched from the bundle by hash. The skeletal clip is created from the record's clip hash. If the
   record has no skeleton (none in the install), the root skeleton is used.
6. The bone-map type is set only for 48-bone rigs: `Bip23` (the perp) is type 3, other 48-bone rigs (`Bip20`, `Bip21`)
   are type 2. The 3-bone rig (`Bip24`, the cuffs) has no map (type −1). **[decomp; verified bone counts]**
7. The loop range is applied only if the play flags have the 0x40 bit (none in the install).

### 9.2 Each frame **[decomp: `UpdateTimeStep`]**

The clip advances by the time step and the pose is taken. If the controller uses blending, the pose matrices are given
to the space node. Otherwise the local matrix is multiplied with the bone matrix. The character's velocity is set from
its displacement per second.

### 9.3 Keeping on the ground **[decomp]**

When the keep-on-ground flag is on (the default), the pelvis (bone 1) is raised 2 metres and tested against the ground.
The ground height is smoothed half-and-half with the previous height, and the root is moved vertically by the difference
plus 5 cm. The arrest's cuffs and cop are all keep-on-ground.

### 9.4 Shadows and the face **[decomp]**

The character draws a soft shadow quad under its feet (two feet bones from the bone map), using the game's global
character-shadow texture. For bone-map type 3 (the perp), it sets the face-pixelation point to the head bone.

### 9.5 Seeking **[decomp: `SetTime`]**

Setting the time sets the clip to frame 0 and then advances it by the given time. Seeking is therefore an absolute
evaluation from the start, not a jump in the clip.

## 10. The camera **[decomp; verified for the data]**

### 10.1 Who has the camera **[decomp: `CAnimScene::BindToGame`, `SetCameraControl`]**

- On binding (start of play), the scene takes camera control (`mControllingCamera` on) and the scene's camera track
  number is the one passed when the scene was loaded (`CAnimScene::GetCameraTrackNumber`).
- For type 0 at the end of the scene (`UpdateTime`, when the scene is finished), camera control is released.
- For types 2 and 4 the end branch calls the camera-control query and does nothing with the result. The camera stays
  with the scene until the activity releases it **[derived; the consumer of camera control is not in the tree]**.
- A moment scene (`CAnimMomentScene`) always controls the camera and does not fix the elevation of the camera. A NIS scene
  fixes it. **[decomp]**

### 10.2 Which track **[decomp: `AnimChooseArrest.cpp`; race track from the event; verified data]**

| Scene | Track | Source |
|---|---|---|
| Arrest | 0 | the chooser sets it to 0 **[decomp]** |
| Race start, intro | from the race event's camera track | `EPlayRaceNIS`; its consumer is missing **[unconfirmed]** |
| World moment | the track the moment is given | [pursuit-breaker-moment.md](pursuit-breaker-moment.md); the event's source is not checked here |
| Scene with two tracks (`GenericStart`, `IntroNis09`) | not decided here | the choice rule is **[unconfirmed]** |

Every installed scene with a camera set has a `Track 0` ([nis-scenes.md](nis-scenes.md), section 8). `IntroNisBL01`
and `IntroNisBL08` have no set and no track; a scene with no track has no camera data, and the player has no authored
camera for it **[verified]**. What the game then shows (the gameplay camera, or nothing) is **[unconfirmed]**.

### 10.3 The space of the track **[verified: first keys; decomp: space bytes in the key layout]**

The first key of the track gives the space of its eye and look point:

- Scene types 0, 2 and 3: scene space (3). Example: `ArrestF02`, `IntroNis05`, `IntroNis09`, `WMGasStation`.
- Scene type 4: world (1), which is the render frame. Examples: `IntroNisDD`, `IntroNisBL13`, `EndingNis04`.
- Car space (0) occurs in 33 keys of pack `0x8003B200`; the scenes that hold them were not identified **[unconfirmed]**.
  The replay photo uses car space in its own pack ([formats/animation.md](../formats/animation.md)).

This matches section 5.2 of [nis-player.md](nis-player.md): type-4 cameras are authored in the render frame, so the type-4 origin and turn are zero.

### 10.4 The camera's start point **[decomp: `NISActivity::GetNISStartLocation`]**

The activity can give a start point for the camera: the first eye of the track (in world or scene space, converted to
the world frame). If no track exists, the marker is used. The callers of this start point are not in the tree, so
what it is used for (for example, streaming the world around the camera) is **[unconfirmed]**. **[decomp]**

### 10.5 Evaluation **[decomp: `formats/animation.md`; evaluator missing in CC0]**

The ICE track is evaluated as a Hermite curve between keys, with its field of view and dutch angle. The rules are in
[formats/animation.md](../formats/animation.md) ("ICE camera keys"). The evaluator's code is not in the CC0 tree
**[unconfirmed – needs Ghidra]**. The key's simulation speed (100 in most scenes, 87 in `IntroNisBL13`) is a per-track
time multiplier **[decomp: format doc; consumer unconfirmed]**.

## 11. Overlay and letterbox **[decomp; verified for the names]**

- A scene's `SeeulatorOverlayName` is activated when the scene is initialised and deactivated when it is purged
  (`TheVisibleSectionManager.ActivateOverlay`). **[decomp]** Twelve scenes name the overlay `FLYBY`
  ([nis-scenes.md](nis-scenes.md), section 3).
- What the `FLYBY` overlay draws is in the visible-section code, which is not in this tree **[unconfirmed]**.
- The key byte that the ICE layout calls "letterbox" is not checked here, because its offset is in question in the
  format doc (the two bytes at 0x7C and 0x7D are equal in every key sampled). **[unconfirmed]**

## 12. HUD and input during a scene

| Item | Rule | Evidence |
|---|---|---|
| HUD | hidden while a NIS exists | stated in [busted-sequence.md](busted-sequence.md), section 5 (not re-read here) **[unconfirmed]** |
| Player input | paused while a NIS exists | `QuickGame::ShouldPauseInput` **[decomp]** |
| Skip button | the action ID 50 on the player's HUD controller config skips the scene (not in an online race) | `NISActivity::JoyHandle` **[decomp]** |
| Pause | a paused NIS does not start; the pause flag stops the start | `NISActivity::Pause`, `UnPause`, `OnTask` **[decomp]** |
| Busted text | the arrest scene shows a busted overlay with the pursuit summary (roadblocks dodged, cops destroyed, cost, time, infractions) | `FeBustedOverlay.cpp` **[decomp]** |
| Speech | a skip clears the speech queue and releases the cop speech module | `SkipOverNIS` **[decomp]** |

The player's car in the arrest has no autopilot in the CC0 tree. The pursuit-breaker spec mentions an `EBecomeAiCar`
event for moments, which is not confirmed here **[unconfirmed]**.

## 13. Skipping **[decomp: `NISActivity::SkipOverNIS`, `JoyHandle`]**

A skip is accepted only when all of these hold:

1. The editor is off.
2. The scene is in the playing state.
3. The scene is not an arrest and not an ending. **These two kinds cannot be skipped.**
4. The scene is skippable: the non-skippable flag is off, or the skip-movies option is on.
5. If the scene has a skip target and the elapsed time has passed the no-skip time, the skip is refused. If the scene has a
   skip target and the time is inside the no-skip window, the skip is marked as "to target".

An accepted skip:

- completes the event sequence;
- sets the state to complete;
- clears the speech and releases the cop speech module;
- calls the ICE "complete event tags" routine;
- jumps the scene to its total length (every entity and every car is set to the end time);
- posts a screen-fade event (`EFadeScreenOn` with the off flag).

When the activity is later released, a skip-to-target posts `EPlayEndNIS` with the target name; otherwise it posts the
completion message (section 14).

The rule that arrests and endings cannot be skipped comes from the decomp's type check. **[decomp]** The source of the
non-skippable flag and of the skip-movies option is **[unconfirmed]**.

## 14. How a scene ends **[decomp: `NISActivity` destructor, `OnTask`, `Unload`]**

### 14.1 The natural end **[decomp; partly unconfirmed]**

- The scene's clock reaches its total length (`VanishFrame / 30` seconds). For type 0 and type 3 the cars are then
  released (section 7.5 of [nis-player.md](nis-player.md)), and for type 0 the camera is released (section 10.1).
- The state machine moves to complete. The code that does this in the playing state (`UpdatePlaying`) is not in the tree
  **[unconfirmed – needs Ghidra]**.
- In the complete state the activity clears the full-speed flag and releases itself (`Release`, body missing)
  **[decomp for the call; unconfirmed for the body]**.

### 14.2 The release **[decomp: destructor]**

1. The scene's sound is told the NIS has finished. The "DDAY flying" sound-state change is cleared if the scene is
   `IntroNisFlyInDD`.
2. The cops `cop1` to `cop8` are removed from the NIS and killed.
3. Every remaining NIS car is set not animating, and its record is deleted.
4. The scenery detail override is cleared.
5. A game-state sound message (state 4, off) is sent.
6. The completion message is posted (below).

Unloading (`Unload`) releases the sequencer, stops the scene in the animation player (which unbinds the cars, see 7.5),
and unloads the bundle from memory.

### 14.3 The completion message **[decomp: `MNISComplete.h`, `NISActivity` destructor, `EPlayEndNIS.hpp`]**

- Normal end: `MNISComplete` with a null scene name, posted on the gameplay channel. The name field is part of the
  message but is null in this path.
- Skipped to a target: `EPlayEndNIS` with the target scene name. This is an event, so the next scene plays from it.
- The consumer of `MNISComplete` (the arrest's next screen, the race's start, the career flow) is not in the tree
  **[unconfirmed – needs Ghidra]**. The busted spec says the infractions screen follows the arrest **[community]**.

### 14.4 Where the cars end up **[derived]**

- The player's car (`car1`) stays in the world at its final pose, with its speed and velocities at zero and the
  physics pins of section 7.3 of [nis-player.md](nis-player.md) still set, unless something clears them. Nothing in the tree clears them, so this is
  **[unconfirmed]**.
- The racers (`car2` to `car8`) are set not animating, and any with no clip are left deactivated.
- The cops that the NIS spawned are killed.

## 15. Facts still missing (for the supervisor's Ghidra pass)

Each item names the function or field and what to find. Addresses are in `speed.exe` 1.3 (image base 0x400000).

1. **`NISActivity::Load(CAnimChooser::eType, const char *scene, int cameratrack, bool)`**: how it sets the scene hash,
   the camera track and the kind; whether it calls `PrepareVehicles` (and with which special car list); how it treats
   the `PlayAsSoonAsLoaded` flag.
2. **`NISActivity::Play`, `UpdatePlaying(float)`, `UpdateLoading`, `Release`**: the playing frame (does it call
   `CAnimScene::UpdateTime` with the frame time, and at what point it sets the state to complete); the loading wait; the
   release path.
3. **`CAnimPlayer::Load`**: the size thresholds (20,000 and 900,001 bytes). Which size is compared (the bundle or the
   listed scene files), and what happens to a 1.3 MB arrest bundle. The "UNSOLVED" tag in the CC0 means the author did
   not finish this.
4. **`ICEManager::GetCameraData(scene_hash, camTrack)`**: the lookup of the pack, and how a `Track <n>` name is chosen.
5. **`ICEManager::GetNumSceneCameraTrack`** and **`ChooseCameraPlaybackTrack`**: the rule for a scene with two or more
   tracks (for example `GenericStart`, `IntroNis09`), and whether car facing picks it.
6. **`ICEManager::Update` and `Render`**, and the **`ICEAnimScene`** implementation (0 bytes in CC0): how the scene's
   time maps to the track's time; how the simulation speed field is used.
7. **The `INISCarControl` implementer** of `SetNISPosition`: whether it sets the physics body's position directly, what
   `initial` does, and what a false return means (the car's counter set to −1).
8. **`CarpEventSequence` handler**: how the NIS event names (`ENIS*`) are bound and what each does.
9. **`EPlayRaceNIS` consumer**: how the race start picks the scene and the camera track, and where it sets the start marker.
10. **`MNISComplete` consumer**: which screen or game step follows a normal end (the arrest's infractions screen, the
    race start).
11. **`ForcePlayerToAnimCarPosition(int, int)` and `ForceCarToAnimCarPosition(Car*, int)`** (empty or stripped in CC0):
    the replay's player placement and the start grid.
12. **`CAnimScene::UpdateTime` callers**: which code advances the scene each frame (the NIS task is the only caller found).
13. **`AnimChooser::ChooseAnim`** (stripped): whether it maps the chooser kind to a camera track.
14. **`CAnimPlayer::AreAnyControllingCamera`** (stripped) and the camera-control consumer: who switches the gameplay
    camera off and on during a scene.
15. **The NIS skip flags**: the source of `mNonSkipableNIS` and the `SkipMovies` option; the value of the no-skip time.
16. **The `EA_BUILD_A124` build flag**: which build the PC 1.3 image is (the skip and the `Infractions` branches depend on
    it).
17. **The overlay drawing** (`TheVisibleSectionManager.ActivateOverlay`), for the `FLYBY` name.
18. **The heading unit and sign** of `atan2a` in `ChooseArrestLocation` (turns or radians), and the meaning of the
    constant 0.25 added to it.
19. **The meaning of the 5 m car lift and the 5 cm character lift** (tuning constants or ground-query offsets).
20. **The frame rate of the fixed frame task** (for the 31-tick settle).
21. **`AnimCtrl` flag bits**: confirm that 0x20 is loop and 0x2 is the skeletal bit, from the `SetFlags` callers.

## 16. Existing docs that disagree with the decomp

- [busted-sequence.md](busted-sequence.md), section 2 step 6c: a race with no spot posts an `EPause` event (`EPause(0, 1, 0)`),
  not a "request object" with an unknown consumer. **[decomp: `AnimChooseArrest.cpp`]**
- [busted-sequence.md](busted-sequence.md), section 4.1: the chooser keys on the perp's heat (`GetHeat()`), not a class
  value. The fall-through explains the class-2 and class-3 names; there is no break unless `FIX_BUGS` is defined.
  **[decomp]**
- [busted-sequence.md](busted-sequence.md), section 4.4: the arrest camera track is 0 by the chooser; the "camera track
  chosen by the cutscene system" is not what the decomp does. **[decomp]**
- [busted-sequence.md](busted-sequence.md), step 8: the NIS end is partly located here. The release posts the completion
  message and kills the NIS cops; the next screen is still open. **[decomp]**
- [formats/animation.md](../formats/animation.md), "NIS scene chunk": see [nis-scenes.md](nis-scenes.md), section 12 for
  the entity counts and the header size.
- [specs/eagl-animation-bank.md](eagl-animation-bank.md), section 10.2: the NIS time is in seconds from `StartFrame` to
  `VanishFrame` at 30 frames per second (confirmed here). The car and character clips run at half that rate through
  their time scale (new; [decomp] for the half, [unconfirmed] for the effect).

## 17. Sources

- **Decomp, `src/Speed/Indep/Src/`** (CC0, `dbalatoni13/nfsmw`): `Sim/Activities/NISActivity.cpp` (30.0 KB, partial);
  `Sim/Activities/QuickGame.cpp` (7.1 KB, NIS parts); `Animation/AnimChooseArrest.cpp` (8.9 KB); `Animation/AnimScene.hpp`
  (10.5 KB) and `.cpp` (26.7 KB); `Animation/AnimLocator.cpp` (3.5 KB); `Animation/AnimPlayer.hpp` (4.0 KB) and `.cpp`
  (17.5 KB); `Animation/AnimCtrl.cpp` (8.5 KB, `AdvanceAnimTime`); `Animation/AnimEntity.cpp` (1.1 KB);
  `Animation/AnimEntity_BasicCharacter.cpp` (14.4 KB); `Animation/AnimEntity_Prop.cpp` (2.1 KB);
  `Animation/AnimCandidates.hpp` (9.4 KB) and `.cpp` (6.7 KB, first part); `Animation/AnimChooser.hpp` (0.6 KB);
  `Animation/GenericNISControlScenario.cpp` (world animation, not used by the NIS); `Interfaces/SimActivities/INIS.h`;
  `Interfaces/Simables/INISCarControl.h`; `Camera/ICE/ICEManager.hpp` (5.6 KB) and `.cpp` (1.2 KB);
  `Camera/ICE/ICEAnimScene.hpp`; `Generated/Events/EPlayRaceNIS.hpp`; `Generated/Events/EPlayEndNIS.hpp`;
  `Generated/Messages/MNISComplete.h`; `AI/Activities/AITrafficManager.cpp` (NIS check, line ~628);
  `AI/Activities/AICopManager.cpp` (line ~449); `AI/Common/AIPursuit.cpp` (lines ~1914 and ~1981);
  `World/World.cpp` (`World_Service`); `Frontend/MenuScreens/InGame/FeBustedOverlay.cpp`.
- **Repo docs read**: [nis-scenes.md](nis-scenes.md) (this spec's data side), [formats/animation.md](../formats/animation.md),
  [specs/eagl-animation-bank.md](eagl-animation-bank.md), [specs/eagl-animation-skeleton.md](eagl-animation-skeleton.md),
  [specs/pursuit-breaker-moment.md](pursuit-breaker-moment.md), [specs/busted-sequence.md](busted-sequence.md).
- **Install measurements** (scan scripts in the reader's scratch folder, not committed): entity skeleton and clip
  names, the directory, the camera sets, the candidate table, and the bundle sizes (see [nis-scenes.md](nis-scenes.md),
  section 14).
