# NIS player (`libs/blackbox-nis`, `crates/nfsmw/src/scenes/world/nis`)

- **Spec:** [specs/nis-player.md](../specs/nis-player.md) and [specs/nis-player-2.md](../specs/nis-player-2.md) (the
  player); [specs/nis-scenes.md](../specs/nis-scenes.md) (the data); [specs/nis-events.md](../specs/nis-events.md) and
  [specs/nis-events-2.md](../specs/nis-events-2.md) (the events and the audio). Formats: [formats/animation.md](../formats/animation.md),
  [formats/event-sequences.md](../formats/event-sequences.md), [formats/audio.md](../formats/audio.md).

## Method

- The install was scanned with a Python 3 reader, run with `python -I` and kept in the scratchpad. It reads the 87
  `NIS/Scene_*_BundleB.bun` files, the directory and the camera packs of `GLOBAL/InGameB.bun`, and the ELF objects inside
  each bundle. It uses the repository's `tools/chunkdump.py` for the chunk walk and the JDLZ inflate.
- The reader's checks gave these results: the header hash against the name (87 of 87); the entity skeleton and animation hashes
  against the bundle's own names (64 of 64 each); the directory slots against the file names (87 of 87); the camera set
  keys against the scene hashes (73, plus 4); the track length against the end frame over 30 (69 of 77).
- The event reader ports the CARP group and event readers of `libs/blackbox-carp` and `libs/blackbox-sequence`. Its
  hashes were checked against `libs/blackbox-attrib` (the vault hash) and `libs/blackbox-hash` (`bStringHash`).
- The player's rules come mostly from the decomp. Where a body is missing from the CC0 tree the spec says so, and the
  fact is marked **[unconfirmed – needs Ghidra]**. [nis-player-2.md](../specs/nis-player-2.md) section 15 lists the 21
  functions and fields for a Ghidra pass.
- The arrest handler's addresses (the chooser `0x0044d770` and the spot finder `0x0044da10`) come from
  [busted-sequence.md](../specs/busted-sequence.md), which was written from `speed.exe` 1.3 in Ghidra (read only).
- The implementation was written from the specs. The decomp files were not kept on disk.

## Sources read

Decomp (CC0, `dbalatoni13/nfsmw`, read through the GitHub API and not saved to disk), under `src/Speed/Indep/Src/`:

- Activity and interfaces: `Sim/Activities/NISActivity.cpp` (30.0 KB, partial; 852 lines), `Sim/Activities/QuickGame.cpp`
  (7.1 KB, the NIS parts), `Interfaces/SimActivities/INIS.h`, `Interfaces/Simables/INISCarControl.h`,
  `Generated/Messages/MNISComplete.h`, `Generated/Events/EPlayRaceNIS.hpp`, `Generated/Events/EPlayEndNIS.hpp`.
- Scene and animation: `Animation/AnimScene.hpp` (10.5 KB) and `.cpp` (26.7 KB), `AnimLocator.cpp` (3.5 KB),
  `AnimPlayer.hpp` (4.0 KB) and `.cpp` (17.5 KB), `AnimCtrl.cpp` (8.5 KB, `AdvanceAnimTime`), `AnimEntity.hpp` (1.4 KB)
  and `.cpp` (1.1 KB), `AnimEntity_BasicCharacter.hpp` (3.8 KB) and `.cpp` (14.4 KB), `AnimEntity_Prop.hpp` (1.8 KB) and
  `.cpp` (2.1 KB), `AnimDirectory.hpp` (3.7 KB) and `.cpp` (2.0 KB), `AnimCandidates.hpp` (9.4 KB) and `.cpp` (6.7 KB, first
  part), `AnimChooseArrest.cpp` (8.9 KB), `AnimChooser.hpp` (0.6 KB).
- Camera: `Camera/ICE/ICEManager.hpp` (5.6 KB) and `.cpp` (1.2 KB, mostly the constructor), `Camera/ICE/ICEAnimScene.hpp`.
- Audio: `EAXSound/CARSFX/SFXObj_NISStream.cpp` and `.hpp`, `EAXSound/Stream/NISSFXModule.cpp`, `EAXSound/SND_GEN/NISAudio.hpp`
  and `NISAudio.cpp`, `EAXSound/sfxctl/SFXCTL_NISReving.cpp`.
- Events: the generated `Generated/Events/ENIS*.hpp` headers and the other event headers the NIS sequences use.
- Callers, read for their NIS checks only: `AI/Activities/AITrafficManager.cpp` (around line 628),
  `AI/Activities/AICopManager.cpp` (around line 449), `AI/Common/AIPursuit.cpp` (around lines 1914 and 1981),
  `World/World.cpp` (`World_Service`), `Frontend/MenuScreens/InGame/FeBustedOverlay.cpp`.
- Zero-byte in the CC0 tree, so not read: the ICE implementation files (`ICEAnimScene.cpp`, `ICEData.cpp`,
  `ICEMover.cpp`, `ICERender.cpp`), and the generated `ENIS*.cpp` event handlers.

Install (read only):

- `NIS\Scene_*_BundleB.bun`: all 87 bundles (59 of them carry a `0x0003B811` sequence chunk).
- `GLOBAL\InGameB.bun`: the scene directory and the ICE camera packs.
- `SOUND\STREAMS\NISAudio.csi` and `SOUND\NISREVDATA\*.bin` (29 files; the engine-rev curves of the NIS cars).

Repository docs read: [formats/animation.md](../formats/animation.md), [specs/eagl-animation-bank.md](../specs/eagl-animation-bank.md),
[specs/eagl-animation-skeleton.md](../specs/eagl-animation-skeleton.md), [specs/eagl-animation-codecs.md](../specs/eagl-animation-codecs.md),
[specs/pursuit-breaker-moment.md](../specs/pursuit-breaker-moment.md), [specs/busted-sequence.md](../specs/busted-sequence.md)
(section 4), [formats/event-sequences.md](../formats/event-sequences.md), [specs/world-sequencers.md](../specs/world-sequencers.md),
[specs/career-phone.md](../specs/career-phone.md), [specs/speech.md](../specs/speech.md) and [formats/bchunk.md](../formats/bchunk.md).
The third-party note on `ENISTimeOfDay` (Xbox 360 behaviour) is a lead only and was not checked.

## Implemented by

- `libs/blackbox-nis/src/`: `scene.rs` (the `AnimScene` container and its header), `character.rs` (the entity records),
  `timeline.rs` (the scene clock, a playback run and its skip), `placement.rs` (the scene, render and world frames),
  `scene_type.rs` (the scene types). It takes bytes and has no rendering.
- `crates/nfsmw-data/src/nis/`: `bundle.rs` (a scene's bundle: the `AnimScene` chunk and its EAGL objects) and
  `cameras.rs` (the ICE camera packs by scene name). The EAGL reader is `crates/nfsmw-data/src/anim.rs`
  ([eagl-animation](eagl-animation.md)).
- `crates/nfsmw/src/scenes/world/nis/`:
  - `player.rs`: the state machine (which scene plays, how far, how it ended), on the caller's real frame time;
  - `world.rs`: the world's side (the freeze, the skip, the bars, the status line);
  - `camera.rs`: the scene's own track at its anchor, or a slow orbit for a scene with no camera set;
  - `cars.rs` and `car_world.rs`: the car channels (`ZPM_Car<n>`, `ZPM_Cop<n>`) and the cars' placement on the ground;
  - `events.rs` and `handlers.rs`: the event sequence, and what each event does (the flash, the overlay, a phone call,
    a named effect);
  - `command.rs`: the `nis` console command.
- `crates/nfsmw/src/audio/nis.rs`: the scene's audio take, started and stopped with the scene's name.
- `crates/nfsmw/src/scenes/world/ai/police/busted/arrest.rs` and `flow.rs`: the arrest scene the bust picks, and the
  bust's wait for the scene to end ([busted-sequence](../specs/busted-sequence.md), section 10.3).

## Checked against the game by

- Synthetic tests: `libs/blackbox-nis/src/tests/` (build, placement, scene, timeline), and
  `crates/nfsmw/src/scenes/world/nis/*_tests.rs` (`camera_tests.rs`, `cars_tests.rs`, `command_tests.rs`,
  `events_tests.rs`, `handlers_tests.rs`, `player_tests.rs`).
- Real-install tests, ignored unless `NFSMW_GAME_DIR` is set: `crates/nfsmw-data/tests/real_install/nis_bundles.rs`,
  `nis_scenes.rs`, `nis_cameras.rs` and `nis_audio.rs`. `story_extras.rs` checks the census of the race outro markers.
- The bundle counts and checks listed under Method were produced by the reader above.
- The framing of the scenes has not been checked on screen. The roadmap lists it as open.

## Known differences and open items

- **NIS audio take:** one shared take per branch (rival intros, arrests and endings), not a take per scene. The
  `ArrestM23`, `ArrestM23b`, `EndingNis01`, `EndingNis02`, `GarageEnter`, `GarageExit` and `OPMRivalIntro` audio names
  have no bundle in the install. The CSIS mapping from an interface to one stream is **[unconfirmed]**, and the
  `SpeechManager` side of the stream was not read ([nis-events.md](../specs/nis-events.md) sections 7 and 8).
- **Overlay packages:** the FEng packages a NIS puts over the frame are drawn as text.
- **Characters:** the character light and hide events are not played.
- **Handlers:** all 26 `ENIS` event types lack handler bodies in the decomp. Their effects in the port are the port's own,
  and not checked against the PC binary. Seven of the 26 never occur in the install
  ([nis-events.md](../specs/nis-events.md) sections 0 and 8).
- **Camera:** the ICE evaluator bodies are not in the CC0 tree (`ICEAnimScene`, `ICEManager::Update`, `GetCameraData`,
  and the choice of a track for a scene with two or more). How scene time maps to track time, and what the simulation
  speed field does, are **[unconfirmed – needs Ghidra]** ([nis-player-2.md](../specs/nis-player-2.md) section 15, items 4 to 6).
- **Loading and skipping:** the size thresholds of `CAnimPlayer::Load` (20,000 and 900,001 bytes), the source of the no-skip
  flag and of the `SkipMovies` option, and the no-skip time are unconfirmed (items 1, 3 and 15).
- **Race scenes:** the `EPlayRaceNIS` consumer is not in the CC0 tree. How a race start picks its scene and camera track is
  not known from the decomp. The race intros and outros use the race's own `IntroNIS` and `OutroNIS` fields, taken from
  the race data ([story-flow.md](../specs/story-flow.md), the race life cycle).
- **Arrest:** the arrest spot is the player's pose. The original's spot search (the finder at `0x0044da10`) is not built,
  and the heading rule, the 0.25 offset and the float the finder writes are **[unconfirmed]**
  ([busted-sequence.md](../specs/busted-sequence.md), sections 4.2 and 4.4). The chooser's variant roll is **[unconfirmed]**
  too (section 4.1), and the port fixes the perp's class at 0.
- **Arrest bundles:** the 1.3 MB arrest bundle is not checked against the loading thresholds above.
- **Scene data:** the seven special-car entries whose scene hashes match no installed scene may belong to another build
  ([nis-scenes.md](../specs/nis-scenes.md) section 13).
