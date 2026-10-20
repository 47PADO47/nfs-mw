# Roadmap

The milestones of the rewrite, and what is done in each. The architecture is in [architecture.md](architecture.md).

| Milestone | Content | Status |
|---|---|---|
| 1 | Workspace, guards, install discovery, bChunk/JDLZ/HUFF, solids, TPK, car viewer on Vulkan/DX12/GL | done |
| 2 | Generic `libs/` split; the streamed city: index, sections, scenery, background loading, instanced rendering, culling, fly camera | done |
| 3 | Sky dome, LODs, water, panoramas; zone-based streaming (visible sections); AttribSys reader; car assembly from the parts DB (stock parts, wheels, brakes, paint). Playtest fixes: misplaced and floating scenery, mouse look without holding a button, `--max-fps`, clearer config-file path | done |
| 4 | Engine foundation: decide on Bevy (ECS, events, UI) in an ADR and migrate the viewers if adopted; layered settings (command line > environment > per-user config file > defaults, with a settings menu in 6); input layer with controller support; developer console (F12: log view, commands such as change car, toggle free camera, change settings); performance overlay (`--show-metrics off\|basic\|advanced`). Decided: Bevy as the shell with our renderer ([ADR 0001](decisions/0001-bevy.md)), full Bevy renderer revisited in 8 | done |
 | 5 | Vehicle physics, spec-first (`docs/specs/vehicle-*.md`); world collision (`CarpWCollisionPack`); drive a car with the original HUD: read the FEng HUD packages (`HUD_*.fng` in `InGameB.bun`) and draw them with the UI layer; steering wheel controller support (wheel axes, pedals, shifters) on the input layer from 4 |  in progress: `blackbox-vehicle`, the collision reader, input actions, `view-world --drive` (placing, chase camera, one-sided walls, props, reset and fall recovery, scripted runs), manual shifting (Q/E, pad bumpers and wheel paddle buttons, with a transmission setting and an options row), the original HUD (speedometer, tachometer with its red zone and shift light, gear, nitrous bar, turbo dial, minimap; the rest of the package waits for the race and pursuit state of milestone 7), the `--show-readout` levels and the `debug collisions` command are in; steering wheel support (pedal axes, paddles, direct gears for an H-shifter and an optional clutch pedal are in; hardware testing on a real wheel is open); controller testing on real hardware and calibration against the original are done ([Driving](#driving-view-world---drive), [The HUD](#the-hud)); open: car damage (blown tyres render wrongly, on/off toggle, rear glass never breaks), a straight test road for 0-100 and 0-200 runs, controller rumble and PlayStation and Switch button faces |
 | 6 | Audio (EA-XA, EA-XAS engine loops, MicroTalk speech), VP6 movies, FEng menus (the same FEng runtime as the HUD), in-game settings menu | in progress: the codecs, banks, music and movie decoders, Ginsu synthesis, the car sound data, the engine and effects mixers, the dynamic mixer maps, the sample (AEMS) layer of the engine and the sputters, the output device, the driven car's engine and effects, a movie player and the radio (licensed songs, gapless, play lists, pause and previous/next keys, a HUD card; not yet heard by a human) are in, and so are the front end (boot movies, title screen, main menu, option screens for audio, video and gameplay, the pause menu, free roam) and the settings written to the config file; speech (the queue and the 28 phrases with banks of their own; no sentences, no triggers) and the pursuit music (four sets steered by game state, the hooks waiting for milestone 7; not yet heard) are in, and the ambience music is open; and so is the playtest report that the engine sounds muted at the rev limiter was fixed by [PR 5](https://github.com/47PADO47/nfs-mw/pull/5); `RUST_LOG=nfsmw::audio=debug` logs the limiter ([Sound](#sound), [The front end](#the-front-end)); open: confirmation menus, a full in-game rebind menu, guidance on the custom menu options, turn signals as an optional setting (default off; currently inverted) |
| 7 | AI racers, traffic, pursuit, races; career data; console commands to spawn AI | in progress: the career is in (the install data readers, the rules in `nfsmw-career`, the career being played and its profile files, Career on the main menu, the safe house, the pursuit and bust results with bounty and cost to state, closed gates, the minimap variant, the bounty row, the phone queue, and the career screens: safe house menu, Blacklist, rap sheet, save, car lot, region unlock, SMS inbox; open items in [Career open items](#career-open-items-milestone-7); [career spec](specs/career.md), [provenance](provenance/career-runtime.md)); traffic, cops, lights and trailers are in (see [AI drivers](ai-drivers.md)); the cops are on the minimap ([cop blips](specs/hud-cop-blips.md)). **Races are in** ([race mode](specs/race-mode.md), [AI racers](specs/ai-racers.md), [provenance](provenance/race-mode.md)): the race rules (`libs/blackbox-race`: gates, checkpoints in order, laps, ranking, the clock and time limit, tollbooth time bonuses, lap knockout, speed-trap points, the catch-up), the race in the world (grid, countdown, the AI racers, drag steering assist, the race card, the checkpoint and racer blips, the result), the console `race` command, Quick Race from the front end and the career's race entry and result ([open items](#race-open-items-milestone-7)). Still missing: (3) the **police helicopter** is in (flight, sight, height sheet, model and minimap blip) but its rotor sound is routed through the mixer but not yet heard in a running game, and its `DoDriving` constants are not located ([spec](specs/ai-helicopter.md), [open police items](#open-police-items-milestone-7)); the **event icons** are in for the race types (`MMICON_SPRINT_%d`, `_DRAG_`, `_CIRCUIT_`, `_KNOCKOUT_`, `_TOLLBOOTH_`, `_SPEEDTRAP_`: race starts, hidden in a race and for a locked career race; the speed traps of a speed-trap race), while `MMICON_ROADBLOCK_%d` is an object of the world map only, not of the in-game HUD ([minimap spec](specs/hud-minimap.md), section 7); open: traffic despawn fix and obstacle avoidance (go around stuck cars, change lanes past slow cars, wait for oncoming traffic); stop lights moved onto the real stop light props, with a debug stop lights command; police minimap blips that sit outside the minimap in pursuit |
| 8 | Graphics: the car shader and lighting rig, tire smoke and skid marks (`blackbox-vehicle` already reports per-wheel `skid` and `smoke`; this draws them), exhaust flames (nitrous, gear-change blow-off and a lift-off backfire on the sputter pops; [guide](exhaust-flames.md), [spec](specs/exhaust-flames.md)), post-processing, upscaling (FSR; DLSS where the backend allows it), ReShade compatibility. Bevy Solari left milestone 8 for a later spike ([ADR 0003](decisions/0003-renderer-after-milestone-8.md)) | in progress: the offscreen HDR render pipeline with a render scale, the glossy car shader with a three-light rig and a procedural environment, optional bloom, tone mapping and FXAA, FSR 1 upscaling with a texture LOD bias, the settings and Video option rows for them, and the ReShade depth contract are in ([post-processing](post-processing.md), [upscaling](upscaling.md), [ReShade](reshade.md)); tire smoke and skid marks are in ([tire effects](tire-effects.md)); FSR 3 is wired into the Bevy renderer with its prepass fixed, and Solari ray tracing works on RTX with colour stand-ins ([status](plans/gfx-renderers/status.md), sections 3d and 3e); the DLSS seams (jitter, motion vectors) are in the Bevy path, and DLSS is mock-tested (`dlss-mock`) with its real `dlss` path written but not compiled; swappable renderers (a Bevy backend with TAA, ray tracing, FSR 3 and DLSS behind a `blackbox-gfx` interface) are a spike, planned in [ADR 0004](decisions/0004-swappable-renderers.md) and the [plan](plans/gfx-renderers/README.md); open: DLSS on an RTX card, FSR 4 (no tester), turbo particle effects; exhaust flames on the different exhausts; the glossy shader makes exhausts shiny and some colours wrong (the red Ferrari, pinkish metallic gloss); bloom whitens the car on Blackbox and the whole screen on Bevy; the "acas" view is too dark on Bevy (the backlog does not say what acas is) |
| 9 | Discord Rich Presence | |
| 10 | Lan multiplayer: one host-authoritative server over our own UDP layer ([ADR 0006](decisions/0006-multiplayer-netcode.md); [specs](specs/multiplayer/README.md); the original LAN client path in [lan-server](specs/lan-server.md)) | designed (2026-10-16), not built |
| 11 | Online multiplayer: the same server on the internet, with the phases in [online](specs/multiplayer/online.md) ([ADR 0006](decisions/0006-multiplayer-netcode.md); [specs](specs/multiplayer/README.md)) | designed (2026-10-16), not built |
| 12 | Native mod API: mods are `cdylib` plugins behind a stable C ABI, replacing the scripting API ([ADR 0005](decisions/0005-native-mod-plugins.md); [modding](modding/README.md); crate `libs/blackbox-modding`) | in progress |
| 13 | Telemetry server: a local REST snapshot, live streams and opt-in control endpoints for player career and session data ([ADR 0007](decisions/0007-telemetry-server.md); [specs](specs/telemetry/README.md)) | designed (2026-10-16), not built |
| 14 | Drift mode: a handling variant with more controlled sliding at lower speeds (its own tire and assist tuning in `blackbox-vehicle`, the original has burnout and drift assists that are not modelled yet) | |

Why this order:

- **Bevy before gameplay (4).** Physics, AI, races and menus are all game objects, systems and events. Choosing the
  engine structure afterwards would mean writing them twice. The `libs/` readers stay engine-agnostic either
  way: only `blackbox-render` and the viewers would be bridged into Bevy. Bevy renders through wgpu, so the
  Vulkan / Direct3D 12 / OpenGL choice stays.
- **Settings, input, console and metrics with the foundation (4).** Every later milestone adds options,
  controls and debug commands, so they need a home first. The settings *menu* needs the front-end UI, so it
  arrives with the menus (6).
- **Graphics extras late (8).** DLSS needs NVIDIA's SDK (its own license, not MIT/Apache, so it would be an
  optional feature the user enables) and a backend that exposes it. Upscalers and ReShade want a finished
  renderer: ReShade reads the depth buffer, which is reverse-Z here, so ReShade's reversed-depth setting has to
  be documented.
- **Multiplayer and scripting (9)** need stable game state to sync and expose. **Discord Rich Presence (10)**
  is the last item on purpose.

## Open police items (milestone 7)

What the pursuit still lacks, so the work can be planned. Each item names the spec it follows.

- **RAM and leader support options:** the RAM strategy (two SUVs, `AIGoalHeadOnRam`) and the leader options (the cross cars and the priority check) are rolled and logged but not carried out. Only the heavy roadblock acts. [Formations, section 6](specs/ai-pursuit-formations.md).
- **Pull-over, ram, and pit goals:** the cops reach their slots and braking, but the `AIGoalPullOver`, `AIGoalRam` and `AIGoalPit` goals are approximated with aims and an arrival brake, not the original goals. [Formations, section 5](specs/ai-pursuit-formations.md), [tactics](specs/ai-pursuit-tactics.md).
- **Rotor sound:** the helicopter's `FX_Helicopter` rotor is routed through the audio mixer and follows the chase; it is not yet heard in a running game. [Helicopter](specs/ai-helicopter.md), section 7.
- **Busted bar:** the busted timer and its HUD value (`timer / timeout`, 0 to 1) are built: rules in `libs/blackbox-pursuit/src/busted.rs`, wiring in `crates/nfsmw/src/scenes/world/ai/police/busted.rs`, value in `HudState::busted`, read by the pursuit camera. Missing: the decomp's five-segment bar (`PursuitMeter`, -1 to 1, with the `MeterDeadZoneBustedDistance` ramp), and drawing a bar on screen, since no HUD code reads the value yet. [Heat](specs/ai-pursuit-heat.md), section 2.2; [busted provenance](provenance/ai-pursuit-busted.md).
- **Helicopter `DoDriving` constants:** the controller's function in the binary is not located (no symbols; the strings lead only to RTTI data). [Helicopter](specs/ai-helicopter.md), section 4.

## Career open items (milestone 7)

What the career still lacks. The rules and the wiring listed in the table above are done; these need their own work.

- **Race wins and the Blacklist:** done: a career race result goes to `apply_event_result` (the Blacklist counts the race,
  the rival opens, the next bins are entered, cash is paid on a win, impounded cars become releasable), the bin's
  race-win messages are queued, and the roaming-start autosave runs after the race (`scenes/world/career/events.rs`).
  A milestone banked at the end of an evaded pursuit now counts as a passed challenge of its bin, and the bin's challenge
  messages follow its count (`scenes/world/career/pursuit.rs`); the speed traps of free roam are not played yet.
  Not done: the pursuit challenges and the side bins 18 to 21 as races of their own (a challenge race runs as a plain
  race, and `challenge_series_complete` is not computed), and the region unlock screen after a race (it is reported in
  the log only).

## Race open items (milestone 7)

What races still lack. The rules and the wiring are done ([race-mode.md](specs/race-mode.md) section 13).

- **HUD:** the `HUD_SingleRace.fng` race objects (place, lap, clock, progress, 3-2-1-GO) are bound; the banner lines, the points and the result stay on an egui card; `HUD_Drag.fng` shows in drag races through the same bindings (its needle sweep and redline are unconfirmed; `Perfect Shift!` and one rpm string are not bound) (see [race-mode](specs/race-mode.md) section 10).
- **Post-race flow:** a result card only; no reward screens, outro, retry or quit choices.
- **Tollbooth and knockout details:** read from the Lua graphs, which are not decompiled. The countdown, the
  `InitialPlayerSpeed` unit and the gate size are recorded in [the provenance](provenance/race-mode.md) section 9.
- **Gate size:** the stored extents are read as half sizes on evidence, not from the trigger code.
- **AI racers:** shortcuts (`Shortcuts[]` markers) are not taken; the catch-up is the rewrite's own controller; the
  stock cars of the street rivals are a stable pick, not the game's; racers far from the player are emulated.
- **Drag:** the steering assist and lanes exist; the dragster engine, damage and input behaviours of the original
  (`EngineDragster`, `DamageDragster`, `InputPlayerDrag`) and the drag HUD are not modelled.
- **Wrong-way trigger** (`ResetsPlayer`), `SharedCheckpoints`, cash grabs and the busted-in-a-race flow are not used.
- **Roadblock icon:** on the world map's new Pursuit tab only; the in-game HUD package has no roadblock icon object.
- **Quick Race options:** the laps, opponents, difficulty and traffic rows are not bound.
- **Mods:** the `nfsmw.races` interface still answers unsupported.
- **Auto shop prices:** in career mode an auto shop opens the garage without charging: no price rule for the
  performance packages is documented.
- **Car lot stat bars:** drawn from the repo's own top speed, grip and 0-100 mph estimate, normalized over the lot
  (the game's own per-car simulation and the bar's exact fill rule are not read; [career-car-lot.md](specs/career-car-lot.md),
  open question 1).
