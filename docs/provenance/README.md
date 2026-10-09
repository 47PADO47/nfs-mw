# Provenance records

One file per crate or module whose behaviour came from a restricted source (decompiled code, GPL code,
unlicensed code). Format-only readers that just follow the docs in [`../formats`](../formats) don't need
a record. Process: [licensing.md § Spec-first](../licensing.md#spec-first).

## Records

| Module | Spec | Restricted sources read |
|---|---|---|
| [`ea-compress::huff`](ea-compress-huff.md) | [formats/huff.md](../formats/huff.md) | C&C Generals EAC source (GPL-3.0), dbalatoni13/nfsmw `LZCompress` (decompiled, CC0) |
| [`ea-audio`](audio-codecs.md) | [specs/audio-containers.md](../specs/audio-containers.md), [formats/audio.md](../formats/audio.md) | vgmstream (ISC-style, permissive) and utkencode (Unlicense): ported with notices; no decompiled code |
| [Car assembly](car-assembly.md) | [specs/car-assembly.md](../specs/car-assembly.md) | dbalatoni13/nfsmw `World/CarInfo.cpp`, `CarRender.cpp`, `CarSkin.cpp` and others (decompiled, CC0) |
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
| [Exhaust flames](exhaust-flames.md) | [specs/exhaust-flames.md](../specs/exhaust-flames.md) | dbalatoni13/nfsmw `World/CarRenderConn.cpp`, `VehicleRenderConn.cpp`, `CarRender.cpp`, `Ecstasy/EmitterSystem.cpp`, `Physics/Behaviors/DrawCar.cpp`, AttribSys class headers (decompiled, CC0) |

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
