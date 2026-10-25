# Provenance records

One file per crate or module whose behaviour came from a restricted source (decompiled code, GPL code,
unlicensed code). Format-only readers that just follow the docs in [`../formats`](../formats) don't need
a record. Process: [licensing.md § Spec-first](../licensing.md#spec-first).

## Records

Loose prop scenery contacts: [prop-world-barriers.md](prop-world-barriers.md).

| Module | Spec | Restricted sources read |
|---|---|---|
| [Cached terrain reference](vehicle-ground-reference.md) | [specs/vehicle-ground-reference.md](../specs/vehicle-ground-reference.md) | Original PC body triangle-list/query cache and CC0 world collision lifecycle |
| [Jump stabilizers](vehicle-jump-stabilizers.md) | [specs/vehicle-jump-stabilizers.md](../specs/vehicle-jump-stabilizers.md) | Original PC chassis/world-position routines and CC0 rigid-body cache lifecycle |
| [Controls list packages](controls-packages.md) | [specs/controls-remap.md](../specs/controls-remap.md) | The install's own `Options_PC_Controller.fng` and `Pause_PC_Controller.fng` object tables (`nfsmw dump-screen`) |
| [Speech sentences](speech-sentences.md) | [specs/speech-sentences.md](../specs/speech-sentences.md), [formats/speech-evt.md](../formats/speech-evt.md) | dbalatoni13/nfsmw `COPSPEECH.{hpp,cpp}`, `SpeechManager.cpp` (decompiled, CC0); `speed.exe` 1.3 speech library (Ghidra, read for behaviour only); the install's `copspeech.{evt,idx,csi}` |
| [Speedbreaker](speedbreaker.md) | [specs/speedbreaker.md](../specs/speedbreaker.md) | Original PC 1.3 meter/time manager and CC0 player/suspension reference |
| [Performance tuning](performance-tuning.md) | [specs/performance-tuning.md](../specs/performance-tuning.md) | Original PC 1.3 slider limits/gates, installed help, CC0 frontend/physics declarations |
| [Performance packages](performance-upgrades.md) | [specs/performance-upgrades.md](../specs/performance-upgrades.md) | Original PC 1.3 upgrade routines; CC0 category and attribute declarations |
| [Contact camera shake](contact-camera-shake.md) | [specs/contact-camera-shake.md](../specs/contact-camera-shake.md) | Original PC 1.3 isolated shaker routines and CC0 Rumble behavior |
| [Police helicopter tuning](ai-helicopter-tuning.md) | [specs/ai-helicopter.md](../specs/ai-helicopter.md) section 10 | NFSMWHelicopterOptions behaviour summary (MIT, key names only; its numbers not used); Ghidra not run in this pass |
| [AI pursuit escape](ai-pursuit-escape.md) | [specs/ai-pursuit.md](../specs/ai-pursuit.md) section 6 | dbalatoni13/nfsmw `AIPursuit.cpp` cool-down notes (decompiled, CC0); `speed.exe` partial Ghidra pass (strings and the 0.93 constant) |
| [Pursuit tuning](pursuit-tuning.md) | [specs/pursuit-tuning.md](../specs/pursuit-tuning.md) | install-verified per-heat values in [specs/ai-pursuit-heat.md](../specs/ai-pursuit-heat.md); NFSMWBartender summary (key names not used) |
| [FairBusts reset penalty](fairbusts-reset.md) | [specs/fairbusts-reset.md](../specs/fairbusts-reset.md) | `speed.exe` Ghidra pass (`kResetPenaltyBlock` at 0x0044447C, constants 15, 90, 0.25, 4.0); NFSMWFairBusts summary (MIT, key names only) |
| [Pursuit breaker moment](pursuit-breaker-moment.md) | [specs/pursuit-breaker-moment.md](../specs/pursuit-breaker-moment.md) | Private `speed.exe` inspection of the NIS activity, ICE playback and marker choice; CC0 structure names; installed camera packs and markers |
| [Pursuit breakers](pursuit-breakers.md) | [specs/pursuit-breakers.md](../specs/pursuit-breakers.md) | Original PC 1.3 event dispatch, explosions, models and installed hierarchies |
| [Trailer constraints](trailer-constraints.md) | [specs/trailer-constraints.md](../specs/trailer-constraints.md) | Original PC 1.3 isolated joint/angle routines and installed bounds |
| [PC object contacts](pc-object-contact.md) | [specs/pc-object-contact.md](../specs/pc-object-contact.md) | Original PC 1.3 executable (private inspection and emulation), dbalatoni13/nfsmw collision layouts (CC0) |
| [Prop placements](prop-spawners.md) | [formats/prop-spawners.md](../formats/prop-spawners.md), [specs/prop-attributes.md](../specs/prop-attributes.md) | Original PC 1.3 data and loader, dbalatoni13/nfsmw record layouts (CC0) |
| [World event sequences](world-sequencers.md) | [formats/event-sequences.md](../formats/event-sequences.md), [specs/world-sequencers.md](../specs/world-sequencers.md) | Original PC 1.3 packages/runtime, dbalatoni13/nfsmw CARP/event layouts (CC0) |
| [`ea-compress::huff`](ea-compress-huff.md) | [formats/huff.md](../formats/huff.md) | C&C Generals EAC source (GPL-3.0), dbalatoni13/nfsmw `LZCompress` (decompiled, CC0) |
| [`ea-audio`](audio-codecs.md) | [specs/audio-containers.md](../specs/audio-containers.md), [formats/audio.md](../formats/audio.md) | vgmstream (ISC-style, permissive) and utkencode (Unlicense): ported with notices; no decompiled code |
| [Car assembly](car-assembly.md) | [specs/car-assembly.md](../specs/car-assembly.md) | dbalatoni13/nfsmw `World/CarInfo.cpp`, `CarRender.cpp`, `CarSkin.cpp` and others (decompiled, CC0) |
| [Race mode](race-mode.md) | [specs/race-mode.md](../specs/race-mode.md), [specs/ai-racers.md](../specs/ai-racers.md) | Original PC 1.3 executable (Ghidra, read only): the race status calls, ranking, knockout and time rules |
| [Race goals](race-goals.md) | [specs/race-goals.md](../specs/race-goals.md), [specs/career-regions.md](../specs/career-regions.md) section 3.5 | The `basicrace` Lua graph (decoded from `gameplay.bin`): the goal order and the finish; `speed.exe` (Ghidra, read only): what `StartRace` and the race status constructor do with the region gates |
| [Race wrong-way trigger](race-wrongway.md) | [specs/race-wrongway.md](../specs/race-wrongway.md) | The `basicrace` Lua graph (restart path, `ResetsPlayer`); `speed.exe` (Ghidra, read only): the trigger constructor, the enter list and the box test |
| [Race restart](race-restart.md) | [specs/race-restart.md](../specs/race-restart.md) | Original PC 1.3 executable (Ghidra, read only): the pause menu's race flag, the restart item and the routines that put the racers back |
| [Race finish effects](race-finish-effects.md) | [specs/race-finish-effects.md](../specs/race-finish-effects.md) | Original PC 1.3 executable (Ghidra, read only, plus a byte disassembly of one function-less stretch): the trigger's flare spawn, the photo screen's picture and camera choice; the install's finish triggers, replay camera pack and photo textures |
| [Car environment captures](car-reflections.md) | [specs/car-reflections.md](../specs/car-reflections.md) | Retail PC car shader and shared environment camera setup (decompiled, CC0; spec first) |
| [World scenery LOD](world-scenery-lod.md) | [specs/scenery-lod.md](../specs/scenery-lod.md) | dbalatoni13/nfsmw `World/Scenery.cpp`, `Scenery.hpp`, `Ecstasy/eView.cpp` (decompiled, CC0) |
| [World scenery visibility](world-scenery-visibility.md) | [specs/scenery-visibility.md](../specs/scenery-visibility.md) | dbalatoni13/nfsmw `World/Scenery.cpp`, `Ecstasy/Ecstasy.cpp` (decompiled, CC0) |
| [World entry zones](world-entry-zones.md) | [specs/world-markers.md](../specs/world-markers.md) | Original PC 1.3 zone enter handler and zone state routine (Ghidra, `speed.exe` read for behaviour; enter icon not located) |
| [World visible sections](world-visible-sections.md) | [specs/visible-sections.md](../specs/visible-sections.md) | dbalatoni13/nfsmw `World/VisibleSection.cpp`, `TrackStreamer.cpp`, `Scenery.cpp` (decompiled, CC0) |
| [Vehicle engine and drivetrain](vehicle-engine-drivetrain.md) | [specs/vehicle-engine-drivetrain.md](../specs/vehicle-engine-drivetrain.md), [specs/vehicle-input-induction-brakes.md](../specs/vehicle-input-induction-brakes.md), [specs/vehicle-manual-shifting.md](../specs/vehicle-manual-shifting.md) | dbalatoni13/nfsmw `Physics/Behaviors/EngineRacer.cpp`, `PInput.cpp`, AttribSys class headers (decompiled, CC0) |
| [Vehicle suspension and tires](vehicle-suspension-tires.md) | [specs/vehicle-suspension-tires.md](../specs/vehicle-suspension-tires.md), [specs/vehicle-steering-assists-aero.md](../specs/vehicle-steering-assists-aero.md) | dbalatoni13/nfsmw `Physics/Behaviors/SuspensionRacer.cpp`, `Chassis.cpp` (decompiled, CC0) |
| [World collision queries](world-collision-query.md) | [formats/collision.md](../formats/collision.md) | dbalatoni13/nfsmw `World/Common/WCollisionMgr.cpp`, `WGrid.cpp`, `WWorldPos.cpp`, `WWorldMath.cpp`, `Physics/Bounds.h` (decompiled, CC0) |
| [Vehicle rigid body](vehicle-rigid-body.md) | [specs/vehicle-rigid-body.md](../specs/vehicle-rigid-body.md) | dbalatoni13/nfsmw `Physics/Behaviors/RigidBody.cpp`, `RBVehicle.cpp`, `Sim/Common/Simulation.cpp` (decompiled, CC0) |
| [InGameA shared models and the sky](ingamea-sky-models.md) | [formats/global-databases.md](../formats/global-databases.md), [formats/world.md](../formats/world.md) | `speed.exe` 1.3 disassembly (Ghidra, read only) |
| [ICE camera dispatch](ice-camera-dispatch.md) | [formats/animation.md](../formats/animation.md) | `speed.exe` 1.3 disassembly (Ghidra, read only); bchunk_names.py chunk names |
| [Career economy](career-economy.md) | [specs/career-economy.md](../specs/career-economy.md), [specs/career-economy-flows.md](../specs/career-economy-flows.md) | `speed.exe` 1.3 disassembly and decompilation (Ghidra, read only); install data |
| [Career blacklist](career-blacklist.md) | [specs/career-blacklist.md](../specs/career-blacklist.md) | `speed.exe` 1.3 (Ghidra, read only); `gameplay.bin` Lua state graphs and tables |
| [Career milestones](career-milestones.md) | [specs/career-milestones.md](../specs/career-milestones.md), [specs/career-milestones-data.md](../specs/career-milestones-data.md) | `speed.exe` 1.3 (Ghidra, read only); `gameplay.bin` milestone collections |
| [Career scripts](career-scripts.md) | [specs/career-scripts.md](../specs/career-scripts.md), [specs/career-scripts-events.md](../specs/career-scripts-events.md) | `gameplay.bin` Lua 5.0 bytecode (private lister); `speed.exe` 1.3 hook registry (Ghidra, read only) |
| [Career text](career-text.md) | [specs/career-text.md](../specs/career-text.md), [specs/career-text-2.md](../specs/career-text-2.md) | `English.bin` and `Labels.bin` (private decoder); `speed.exe` 1.3 label formats (Ghidra, read only) |
| [Career phone](career-phone.md) | [specs/career-phone.md](../specs/career-phone.md) | `speed.exe` 1.3 (Ghidra, read only); `English.bin`, `gameplay.bin`, saves |
| [Career rap sheet](career-rapsheet.md) | [specs/career-rapsheet.md](../specs/career-rapsheet.md), [specs/career-rapsheet-2.md](../specs/career-rapsheet-2.md) | `speed.exe` 1.3 (Ghidra, read only); the install's FEng packages |
| [Boot profile prompt](boot-profile-prompt.md) | [specs/boot-profile-prompt.md](../specs/boot-profile-prompt.md) | `speed.exe` 1.3 (Ghidra MCP, read only; the boot flow's screen list, the start-up list-ready step) |
| [Career parity menus](career-parity-menus.md) | [specs/career-parity-menus.md](../specs/career-parity-menus.md) | `speed.exe` 1.3 (Ghidra MCP, read only; label and texture keys hashed against `FrontB.lzc`) |
| [Career save screens](career-save-screens.md) | [specs/career-save-screens.md](../specs/career-save-screens.md), [specs/career-save-screens-2.md](../specs/career-save-screens-2.md) | `speed.exe` 1.3 (Ghidra, read only; disassembly of the file library) |
| [Career regions](career-regions.md) | [specs/career-regions.md](../specs/career-regions.md) | `speed.exe` 1.3 (Ghidra, read only); `gameplay.bin`, `Rivals.bin`, `TrackMaps.bin` |
| [Race barriers](race-barriers.md) | [specs/race-barriers.md](../specs/race-barriers.md) | `speed.exe` 1.3 (Ghidra, read only; a byte scan of the PE); `gameplay.bin`, `L2RA.BUN`, `STREAML2RA.BUN` |
| [Career flow and save](career-flow.md) | [specs/career-flow.md](../specs/career-flow.md), [specs/career-car-lot.md](../specs/career-car-lot.md), [formats/career-save.md](../formats/career-save.md), [formats/career-state.md](../formats/career-state.md) | `speed.exe` 1.3 (Ghidra, read only); the install's front-end attribute data; three saves read only |
| [Career reputation sheets](career-rep-sheets.md) | [specs/career-rep-sheets.md](../specs/career-rep-sheets.md) | `speed.exe` 1.3 (Ghidra, read only); the install's FEng packages and `gameplay.bin` |
| [Career core (`nfsmw-career`)](career-core.md) | [formats/career-save.md](../formats/career-save.md), [formats/career-state.md](../formats/career-state.md), [specs/career-economy.md](../specs/career-economy.md), [specs/career-economy-flows.md](../specs/career-economy-flows.md), [specs/career-phone.md](../specs/career-phone.md), [specs/career-text-2.md](../specs/career-text-2.md), [specs/career-flow.md](../specs/career-flow.md), [specs/career-safehouse.md](../specs/career-safehouse.md), [specs/career-safehouse-2.md](../specs/career-safehouse-2.md), [specs/career-rapsheet.md](../specs/career-rapsheet.md), [specs/career-rapsheet-2.md](../specs/career-rapsheet-2.md), [specs/career-save-screens.md](../specs/career-save-screens.md), [specs/career-save-screens-2.md](../specs/career-save-screens-2.md), [specs/career-regions.md](../specs/career-regions.md) | Repository specs only (no decompiled code open); three development saves read only for the optional test |
| [Career safe house](career-safehouse.md) | [specs/career-safehouse.md](../specs/career-safehouse.md), [specs/career-safehouse-2.md](../specs/career-safehouse-2.md) | `speed.exe` 1.3 (Ghidra, read only; undefined routines disassembled from the file) |
| [Career runtime](career-runtime.md) | [specs/career.md](../specs/career.md) and the career pages it indexes (flow, safe house, economy, milestones, Blacklist, car lot, rap sheet, save screens, regions, phone, text) | `crates/nfsmw/src/career`, `scenes/world/career` and `frontend/career_screens`: the career being played, its profile files and screens, written from the specs; the install-backed count test (`career/data_tests.rs`, `NFSMW_GAME_DIR`) |
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
| [Day-night clock](day-night-clock.md) | [specs/time-of-day.md](../specs/time-of-day.md) | `speed.exe` (original PC build) disassembled in Ghidra for the clock counter writer and the sun integrator; the day-night key names from the repository's note of the behaviour (no mod code) |
| [World map screen](world-map-ghidra.md) | [specs/world-map.md](../specs/world-map.md) | `speed.exe` decompiled in Ghidra (the world map controller and its screen), read-only |
| [AI pursuit goals](ai-pursuit-goals.md) | [specs/ai-pursuit-goals.md](../specs/ai-pursuit-goals.md) | `speed.exe` goal factory and constructors (Ghidra, read only); attribute names checked with a scratch lookup2 script |
| [Back-face culling](backface-culling.md) | [specs/backface-culling.md](../specs/backface-culling.md) | `speed.exe` texture state, effect parameter table and view render paths (Ghidra, read only); install geometry, texture packs and scenery measured with a scratch script and a Rust install test |
| [Traffic and obstacles](ai-traffic-obstacles.md) | [specs/ai-traffic-obstacles.md](../specs/ai-traffic-obstacles.md) | `speed.exe` traffic driving action, its vtable, speed and stopping tables (Ghidra, read only; no function created in the project) |
| [Busted sequence](busted-sequence.md) | [specs/busted-sequence.md](../specs/busted-sequence.md) | `speed.exe` arrest handler, scene chooser, registrations and FEng message names (Ghidra, read only); install screen data (read only) |
| [After the ticket](busted-aftermath.md) | [specs/busted-aftermath.md](../specs/busted-aftermath.md), [specs/career-cooling.md](../specs/career-cooling.md) | `speed.exe` infractions and car select handlers (bytes, Ghidra read only); CC0 public decompilation read for understanding; install data (read only) |
| [Cop health bars](cop-health-bars.md) | [specs/cop-health-bars.md](../specs/cop-health-bars.md) | MWHealthbars reference page (community, values only; no licence); the port's wreck model (own code). No Ghidra session |
| [Custom fonts](custom-fonts.md) | [specs/custom-fonts.md](../specs/custom-fonts.md) | NFS_CustomFont reference page (MIT; feature list only); `speed.exe` font loader, glyph draw, baseline table (Ghidra, read only); install texture data (read only) |
| [Front-end shadows](fe-shadows.md) | [specs/fe-shadows.md](../specs/fe-shadows.md) | `speed.exe` game-flow global, render states and registry settings (Ghidra, read only) |
| [HD content support](hd-content-support.md) | [specs/hd-content-support.md](../specs/hd-content-support.md) | NFS HD Content Support reference page (GPL-3.0; facts only); `speed.exe` glyph, texture, cursor and movie code (Ghidra, read only) |
| [HD reflections](hd-reflections.md) | [specs/hd-reflections.md](../specs/hd-reflections.md) | NFS HD Reflections and NFSMWHDReflections reference pages (GPL-3.0; facts only); `speed.exe` initial data and option writer (Ghidra, read only) |
| [LAN client path](lan-client.md) | [specs/lan-server.md](../specs/lan-server.md) (client path section) | `speed.exe` LAN client tag table and message handlers (Ghidra, read only; two functions created in the project, program not saved) |
| [Original race netcode](original-race-netcode.md) | [specs/original-race-netcode.md](../specs/original-race-netcode.md) | `speed.exe` 1.3 race session update, message and state name tables, bit writer, direct-connect config (Ghidra, read only; nothing created or saved) |
| [EA Trax jukebox](eatrax-jukebox.md) | [specs/eatrax-jukebox.md](../specs/eatrax-jukebox.md) | `speed.exe` EA Trax playlist builder, song pick and profile song table (Ghidra; functions created in the project) |
| [Mod world content](mod-world-content.md) | [specs/mod-world-content.md](../specs/mod-world-content.md) | `speed.exe` chunk loaders, dispatcher and car-part name builder (Ghidra, read only); installed world data (read only) |
| [Player driving parity](player-driving-parity.md) | [specs/player-driving-parity.md](../specs/player-driving-parity.md) | Original PC steering consumers and isolated selected-executable vectors; gameplay-profile calibration |
| [Player launch](player-launch.md) | [specs/player-launch.md](../specs/player-launch.md) | Original PC band, staging, timer and torque behavior; isolated numeric execution |
| [Tire capture corrections](vehicle-tire-capture-corrections.md) | [specs/vehicle-tire-capture-corrections.md](../specs/vehicle-tire-capture-corrections.md) | Exact original tire-call inputs and isolated reduced numeric vectors |
| [Player timestep](player-timestep.md) | [specs/player-timestep.md](../specs/player-timestep.md) | Explicit CarSim delta and body-probe adapter corrections; no scheduler change |
| [Shared physics clock](shared-physics-clock.md) | [specs/shared-physics-clock.md](../specs/shared-physics-clock.md) | Real-time shared cadence and scaled per-tick integration; observed original clock contract |
| [Vehicle decompilation audit](vehicle-decomp-audit.md) | [specs/vehicle-decomp-audit.md](../specs/vehicle-decomp-audit.md) | PC vehicle decompilation (no licence; summaries only, no source text kept); `speed.exe` byte searches for the range and rate tables (Ghidra) |
| [EAGL animation](eagl-animation.md) | [specs/eagl-animation.md](../specs/eagl-animation.md), [specs/eagl-animation-skeleton.md](../specs/eagl-animation-skeleton.md), [specs/eagl-animation-bank.md](../specs/eagl-animation-bank.md), [specs/eagl-animation-codecs.md](../specs/eagl-animation-codecs.md), [specs/eagl-animation-channels.md](../specs/eagl-animation-channels.md), [specs/eagl-animation-channels-2.md](../specs/eagl-animation-channels-2.md) | dbalatoni13/nfsmw `EAGL4Anim/` and `Animation/` (decompiled, CC0; several bodies are 0 bytes); `speed.exe` 1.3 (Ghidra, read only; the channel dispatch); the install's NIS and track animation banks (read only) |
| [NIS player](nis-player.md) | [specs/nis-player.md](../specs/nis-player.md), [specs/nis-player-2.md](../specs/nis-player-2.md), [specs/nis-scenes.md](../specs/nis-scenes.md), [specs/nis-events.md](../specs/nis-events.md), [specs/nis-events-2.md](../specs/nis-events-2.md) | dbalatoni13/nfsmw `Sim/Activities/NISActivity.cpp`, `Animation/*`, `Camera/ICE/*` and `EAXSound/*NIS*` (decompiled, CC0; the ICE and `ENIS` handler bodies are 0 bytes); the arrest addresses from busted-sequence (`speed.exe` 1.3, Ghidra, read only); the install's 87 NIS bundles (read only) |
| [Story flow](story-flow.md) | [specs/story-flow.md](../specs/story-flow.md), [specs/story-flow-2.md](../specs/story-flow-2.md) | `speed.exe` 1.3 (Ghidra, read only; the race finish handler and the special-setup hook); the Lua graphs of `GLOBAL/gameplay.bin` (read only) |

Audio extensions: [power effects](car-audio-power-effects.md),
[authored traffic](authored-traffic-audio.md), [common events](common-audio-events.md),
[spatial maps](spatial-parameter-maps.md), [FXEDIT](fxedit-processor.md), and
[routing, tracing and output](car-audio-output.md). The
[Speedbreaker pitch](speedbreaker-audio.md) record covers its shared authored envelope. The
[coverage ledger](../car-audio-coverage.md) records unresolved boundaries.

## Template

[Car audio integration](car-audio-integration.md) records the contact, playback,
authored scrape/wind/road and engine synchronization changes, with their remaining
original-game fidelity boundaries.

[Tire effects](tire-effects.md) records the existing source inputs and the new procedural visual design.

[Tire surface dust](tire-surface-dust.md) records the `speed.exe` tire-effect dispatcher findings and the installed
per-surface dry records.

```markdown
# <crate or module>

- **Spec:** docs/specs/<topic>.md (or docs/formats/<format>.md)
- **Sources read for the spec:** <project, URL, license, which files>, ...
- **Implemented:** <date>, from the spec only (decompiled code not open while writing).
- **Checked against the game by:** <tests, measurements, side-by-side captures>
- **Known differences from the original:** ...
```
