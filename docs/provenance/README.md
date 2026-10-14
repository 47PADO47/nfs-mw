# Provenance records

One file per crate or module whose behaviour came from a restricted source (decompiled code, GPL code,
unlicensed code). Format-only readers that just follow the docs in [`../formats`](../formats) don't need
a record. Process: [licensing.md § Spec-first](../licensing.md#spec-first).

## Records

| Module | Spec | Restricted sources read |
|---|---|---|
| [Cached terrain reference](vehicle-ground-reference.md) | [specs/vehicle-ground-reference.md](../specs/vehicle-ground-reference.md) | Original PC body triangle-list/query cache and CC0 world collision lifecycle |
| [Jump stabilizers](vehicle-jump-stabilizers.md) | [specs/vehicle-jump-stabilizers.md](../specs/vehicle-jump-stabilizers.md) | Original PC chassis/world-position routines and CC0 rigid-body cache lifecycle |
| [Speedbreaker](speedbreaker.md) | [specs/speedbreaker.md](../specs/speedbreaker.md) | Original PC 1.3 meter/time manager and CC0 player/suspension reference |
| [Performance tuning](performance-tuning.md) | [specs/performance-tuning.md](../specs/performance-tuning.md) | Original PC 1.3 slider limits/gates, installed help, CC0 frontend/physics declarations |
| [Performance packages](performance-upgrades.md) | [specs/performance-upgrades.md](../specs/performance-upgrades.md) | Original PC 1.3 upgrade routines; CC0 category and attribute declarations |
| [Contact camera shake](contact-camera-shake.md) | [specs/contact-camera-shake.md](../specs/contact-camera-shake.md) | Original PC 1.3 isolated shaker routines and CC0 Rumble behavior |
| [Pursuit breakers](pursuit-breakers.md) | [specs/pursuit-breakers.md](../specs/pursuit-breakers.md) | Original PC 1.3 event dispatch, explosions, models and installed hierarchies |
| [Trailer constraints](trailer-constraints.md) | [specs/trailer-constraints.md](../specs/trailer-constraints.md) | Original PC 1.3 isolated joint/angle routines and installed bounds |
| [PC object contacts](pc-object-contact.md) | [specs/pc-object-contact.md](../specs/pc-object-contact.md) | Original PC 1.3 executable (private inspection and emulation), dbalatoni13/nfsmw collision layouts (CC0) |
| [Prop placements](prop-spawners.md) | [formats/prop-spawners.md](../formats/prop-spawners.md), [specs/prop-attributes.md](../specs/prop-attributes.md) | Original PC 1.3 data and loader, dbalatoni13/nfsmw record layouts (CC0) |
| [World event sequences](world-sequencers.md) | [formats/event-sequences.md](../formats/event-sequences.md), [specs/world-sequencers.md](../specs/world-sequencers.md) | Original PC 1.3 packages/runtime, dbalatoni13/nfsmw CARP/event layouts (CC0) |
| [`ea-compress::huff`](ea-compress-huff.md) | [formats/huff.md](../formats/huff.md) | C&C Generals EAC source (GPL-3.0), dbalatoni13/nfsmw `LZCompress` (decompiled, CC0) |
| [`ea-audio`](audio-codecs.md) | [specs/audio-containers.md](../specs/audio-containers.md), [formats/audio.md](../formats/audio.md) | vgmstream (ISC-style, permissive) and utkencode (Unlicense): ported with notices; no decompiled code |
| [Car assembly](car-assembly.md) | [specs/car-assembly.md](../specs/car-assembly.md) | dbalatoni13/nfsmw `World/CarInfo.cpp`, `CarRender.cpp`, `CarSkin.cpp` and others (decompiled, CC0) |
| [Car environment captures](car-reflections.md) | [specs/car-reflections.md](../specs/car-reflections.md) | Retail PC car shader and shared environment camera setup (decompiled, CC0; spec first) |
| [World scenery LOD](world-scenery-lod.md) | [specs/scenery-lod.md](../specs/scenery-lod.md) | dbalatoni13/nfsmw `World/Scenery.cpp`, `Scenery.hpp`, `Ecstasy/eView.cpp` (decompiled, CC0) |
| [World scenery visibility](world-scenery-visibility.md) | [specs/scenery-visibility.md](../specs/scenery-visibility.md) | dbalatoni13/nfsmw `World/Scenery.cpp`, `Ecstasy/Ecstasy.cpp` (decompiled, CC0) |
| [World visible sections](world-visible-sections.md) | [specs/visible-sections.md](../specs/visible-sections.md) | dbalatoni13/nfsmw `World/VisibleSection.cpp`, `TrackStreamer.cpp`, `Scenery.cpp` (decompiled, CC0) |
| [Vehicle engine and drivetrain](vehicle-engine-drivetrain.md) | [specs/vehicle-engine-drivetrain.md](../specs/vehicle-engine-drivetrain.md), [specs/vehicle-input-induction-brakes.md](../specs/vehicle-input-induction-brakes.md), [specs/vehicle-manual-shifting.md](../specs/vehicle-manual-shifting.md) | dbalatoni13/nfsmw `Physics/Behaviors/EngineRacer.cpp`, `PInput.cpp`, AttribSys class headers (decompiled, CC0) |
| [Vehicle suspension and tires](vehicle-suspension-tires.md) | [specs/vehicle-suspension-tires.md](../specs/vehicle-suspension-tires.md), [specs/vehicle-steering-assists-aero.md](../specs/vehicle-steering-assists-aero.md) | dbalatoni13/nfsmw `Physics/Behaviors/SuspensionRacer.cpp`, `Chassis.cpp` (decompiled, CC0) |
| [World collision queries](world-collision-query.md) | [formats/collision.md](../formats/collision.md) | dbalatoni13/nfsmw `World/Common/WCollisionMgr.cpp`, `WGrid.cpp`, `WWorldPos.cpp`, `WWorldMath.cpp`, `Physics/Bounds.h` (decompiled, CC0) |
| [Vehicle rigid body](vehicle-rigid-body.md) | [specs/vehicle-rigid-body.md](../specs/vehicle-rigid-body.md) | dbalatoni13/nfsmw `Physics/Behaviors/RigidBody.cpp`, `RBVehicle.cpp`, `Sim/Common/Simulation.cpp` (decompiled, CC0) |
| [Movies](video.md) | [formats/video.md](../formats/video.md) | ruffle-rs/nihav-vp6 (MIT, dependency); FFmpeg `electronicarts.c` (LGPL, facts only); vgmstream (ISC-style) |
| [FEng packages and runtime](feng.md) | [formats/frontend.md](../formats/frontend.md), [specs/feng-runtime.md](../specs/feng-runtime.md) | dbalatoni13/nfsmw `FEng/*`, `Frontend/FEngRender.cpp`, `FEngFont.cpp`, `HUD/*` (decompiled, CC0); FEngLib (no license, facts only) |
| [FEng input and the front-end menus](frontend-menus.md) | [specs/feng-input.md](../specs/feng-input.md), [specs/frontend-menus.md](../specs/frontend-menus.md) | dbalatoni13/nfsmw `FEng/FEngine.cpp`, `FEButtonMap.cpp`, `Frontend/FEJoyInput.cpp`, `Frontend/MenuScreens/*` (decompiled, CC0); the install's own packages |
| [Car engine sound](engine-sound.md) | [specs/engine-sound.md](../specs/engine-sound.md), [specs/engine-sound-ginsu.md](../specs/engine-sound-ginsu.md), [specs/engine-sound-effects.md](../specs/engine-sound-effects.md) | dbalatoni13/nfsmw `EAXSound/Ginsu/*`, `CARSFX/*`, `sfxctl/*`, `EAXCar*`, `SoundConn*`, `SoundCollision*` (decompiled, CC0) |
| [Dynamic mixer](dynamic-mixer.md) | [specs/dynamic-mixer.md](../specs/dynamic-mixer.md), [specs/car-sound-mixer.md](../specs/car-sound-mixer.md), [formats/mixmap.md](../formats/mixmap.md) | dbalatoni13/nfsmw `EAXSound/Dynamic_Mixer/*`, `SndBase.cpp`, `EAXSound.cpp`, `sfxctl/*`, `CARSFX/*` (decompiled, CC0) |
| [AEMS module banks](aems.md) | [specs/aems.md](../specs/aems.md), [specs/engine-sound-aems.md](../specs/engine-sound-aems.md), [formats/aems.md](../formats/aems.md) | dbalatoni13/nfsmw `Libs/snd/9/*`, `EAXSound/CARSFX/*`, `SND_GEN/ENGINES_AEMS2.h` (decompiled, CC0); `speed.exe` disassembled locally for the function numbering |
| [HUD minimap](hud-minimap.md) | [specs/hud-minimap.md](../specs/hud-minimap.md), [formats/minimap.md](../formats/minimap.md) | dbalatoni13/nfsmw `Frontend/HUD/FeMinimap*.cpp`, `FeHudElement.cpp`, `FEPkg_Hud.cpp`, `World/TrackInfo.hpp` (decompiled, CC0); `speed.exe` 1.3 disassembled locally to confirm the formulas and read the globals |
| [Music graph and radio](music-graph.md) | [specs/music-graph.md](../specs/music-graph.md) | `speed.exe` PathFinder and EA Trax code (disassembly); dbalatoni13/nfsmw `SFXObj_Pathfinder.cpp`, `FEDatabase.cpp`, `FEManager.cpp` (decompiled, CC0) |
| [Speech](speech.md) | [specs/speech.md](../specs/speech.md), [formats/audio.md](../formats/audio.md) | dbalatoni13/nfsmw `EAXSound/Stream/Speech*.cpp`, `GameSpeech.cpp`, `SND_GEN/COPSPEECH.cpp`, AttribSys class `speech` (decompiled, CC0); header-only SPCH and CSIS |
| [Exhaust flames](exhaust-flames.md) | [specs/exhaust-flames.md](../specs/exhaust-flames.md) | dbalatoni13/nfsmw `World/CarRenderConn.cpp`, `VehicleRenderConn.cpp`, `CarRender.cpp`, `Ecstasy/EmitterSystem.cpp`, `Physics/Behaviors/DrawCar.cpp`, AttribSys class headers (decompiled, CC0) |
| [Weather](weather.md) | [specs/weather.md](../specs/weather.md) | `speed.exe` (original PC build) decompiled and disassembled in Ghidra; name-hash scan of the install's data files |

## Template

[Tire effects](tire-effects.md) records the existing source inputs and the new procedural visual design.

```markdown
# <crate or module>

- **Spec:** docs/specs/<topic>.md (or docs/formats/<format>.md)
- **Sources read for the spec:** <project, URL, license, which files>, ...
- **Implemented:** <date>, from the spec only (decompiled code not open while writing).
- **Checked against the game by:** <tests, measurements, side-by-side captures>
- **Known differences from the original:** ...
```
