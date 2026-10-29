# nfs-mw docs

Extra options: [F5 camera, testing and presets](extra-options.md).

Runtime customization: [garage-overlay.md](garage-overlay.md).
Normal garage coverage: [garage-parity.md](garage-parity.md).

Original menu UI: [fixes and parity audit](menu-ui-parity.md).

Native physics work: [collision and destruction parity](native-physics-parity.md).
Driving state flow: [downstream consumer audit](driving-state-audit.md).

Documentation for the Rust rewrite of Need for Speed: Most Wanted (2005, PC v1.3 / Black Edition): how
the game stores its data, what prior work exists, and how the rewrite is built. Format claims are checked
against a local install selected with `NFSMW_GAME_DIR`.

## Short answers

- **Language:** C++, Visual C++ .NET 2003, Direct3D 9, EA Black Box's in-house engine.
  → [research.md](research.md)
- **Prior art:** a CC0 matching decomp (PC 1.3 at 14%, GameCube at 61%), format readers for most
  data, a working MW world viewer (noclip), and no existing engine rewrite. → [research.md](research.md)
- **Container:** a tree of **bChunks** (`u32 id, u32 size, payload`; bit 31 = container), often
  compressed with **JDLZ** (LZ77) or **HUFF** (Huffman + run-length).
  → [formats/bchunk.md](formats/bchunk.md), [formats/huff.md](formats/huff.md)
- **Models:** "solids" in `GeometryPack` chunks, with 104-byte material records, 36- or 60-byte vertices
  and u16 indices. → [formats/models.md](formats/models.md)
- **Textures:** TPK packs, either plain or one compressed blob per texture; DXT1/3/5, P8, ARGB.
  → [formats/textures.md](formats/textures.md)
- **Maps:** one world (`L2RA`): a metadata file plus a 533 MB stream of 720 sections. Models are placed by
  name hash. → [formats/maps.md](formats/maps.md), [formats/world.md](formats/world.md)
- **Gameplay data:** AttribSys `VPAK` vaults (57 classes, Jenkins lookup2 hashes).
  → [formats/attributes.md](formats/attributes.md)
- **Animation:** EAGL4 skeletons and banks stored as ELF32 MIPS relocatable objects.
  → [formats/animation.md](formats/animation.md)
- **Audio / video:** EA-XA, EA-XAS and MicroTalk audio in EA containers; VP6 movies.
  → [formats/audio.md](formats/audio.md), [formats/video.md](formats/video.md)

## Contents

### The rewrite

| Doc | What it covers |
|---|---|
| [renderers.md](renderers.md) | The `renderer` setting (native Black Box or Bevy), what each can do, every graphics key with its CLI, environment and TOML names, restart rules, the `gfx` console command |
| [bevy-backend.md](bevy-backend.md) | The optional Bevy renderer spike: how it fits the game's `App`, axes, colour, instancing, the measured results against the native renderer, known gaps |
| [low-end.md](low-end.md) | Weak GPUs and laptops: what each graphics setting costs, the low, medium and high presets, a low-end config and how to read the performance overlay |
| [post-processing.md](post-processing.md) | Optional tone mapping, bloom and anti-aliasing (FXAA; SMAA and TAA on the Bevy renderer): settings, menu rows, what each does and how to compare them |
| [visual-treatment.md](visual-treatment.md) | Installed PC midday colour treatment for the native renderer |
| [ambient-occlusion.md](ambient-occlusion.md) | Spatial and temporal ambient-only AO: shared geometry, history rejection, comparison and GPU validation |
| [optional-reflections.md](optional-reflections.md) | Optional cached local probes and selective current-frame car reflections |
| [cached-shadows.md](cached-shadows.md) | Optional exact static shadow reuse, dynamic casters, memory costs and invalidation |
| [vehicle-effects.md](vehicle-effects.md) | Original PC sparks, experimental restored sparks and wind trails using base-game assets |
| [architecture.md](architecture.md) | `libs/` (engine-generic) vs `crates/` (MW), finding the install, the streamed city, graphics backends, Windows/Linux, roadmap |
| [world-interactions.md](world-interactions.md) | Contact, dynamic scenery and frame-rate parity work, validation and remaining gaps |
| [traffic-interactions-audit.md](traffic-interactions-audit.md) | Traffic/cargo merge regressions, verified fixes, collision-placement findings and remaining parity work |
| [specs/traffic-presets.md](specs/traffic-presets.md) | Reference/Extended traffic ranges, F8 density and budget controls, collision support and remaining parity gaps |
| [world-interactions-todo.md](world-interactions-todo.md) | Deferred collision, cleanup, trailer and pursuit-breaker work with acceptance checks |
| [testing.md](testing.md) | the kinds of tests, the real-install tests and how to run them |
| [profiling.md](profiling.md) | Build profiles (`release`, `profiling`, `dist`) and the mimalloc allocator; the Tracy workflow is in [profiling-bevy.md](profiling-bevy.md) |
| [performance-audit.md](performance-audit.md) | The 2026-10-17 performance audit: frame and load findings, the fixes, before and after numbers, the cache verdict, follow-ups |
| [benchmarks/](benchmarks/) | Dated measurement reports: [Bevy vs native, 2026-10-10](benchmarks/2026-10-10-bevy-vs-native.md) and [performance sweep, 2026-10-17](benchmarks/2026-10-17-perf-sweep.md) |
| [window-modes.md](window-modes.md) | Windowed, borderless and exclusive fullscreen, display selection, resolution and runtime switching |
| [hud-layout.md](hud-layout.md) | PC widescreen, centered and Xbox-scaled HUD presets |
| [hud-settings.md](hud-settings.md) | The Custom HUD's `[custom_hud]` table: presets, size, map, information, accessibility, camera shake and FOV, console keys |
| [HUD_DESIGN_SYSTEM.md](HUD_DESIGN_SYSTEM.md), [HUD_REFERENCE_BOARD.md](HUD_REFERENCE_BOARD.md) | The Custom HUD's visual source of truth (Rockport / Signal System: colour, type, layout, the map from the game's own picture and icons) and its reference images |
| [upscaling.md](upscaling.md) | Render scale, bilinear and FSR 1 upscaling, the temporal upscalers and their quality modes, ReShade limits, seams for DLSS |
| [native-dlss.md](native-dlss.md) | Optional Streamline DLSS SR/DLAA, Reflex and Frame Generation on native Windows Vulkan/DX12 |
| [reshade.md](reshade.md) | Using ReShade or vkBasalt: backends, the reverse-Z depth settings, render scale limits |
| [reshade-shaders/](reshade-shaders/README.md) | Survey of a local ReShade effect pack (33 files): what each does, licence tiers, ideas worth taking and what the repo already has |
| [tire-effects.md](tire-effects.md) | Tire smoke, ground-following marks, runtime controls and reproducible driving checks |
| [tire-surface-dust.md](tire-surface-dust.md) | Dry tire particles from the installed surface records: grass, sand, dirt and leaves |
| [vehicle-effects.md](vehicle-effects.md) | Optional collision sparks and high-speed wind trails using stock-install definitions |
| [exhaust-flames.md](exhaust-flames.md) | Tail-pipe flames for nitrous, gear changes and lift-off sputter pops: the `exhaust_flames` setting, console, budgets and checks |
| [controller-settings.md](controller-settings.md) | Independent response options and saved keyboard, mouse and gamepad rebinding |
| [rust-stack.md](rust-stack.md) | Crates used and planned, with versions and licenses; rejected options |
| [decisions/](decisions/0001-bevy.md) | Architecture decision records: [0001 Bevy as the game framework](decisions/0001-bevy.md) (proposed), [0003 The renderer after milestone 8](decisions/0003-renderer-after-milestone-8.md) (proposed), [0004 Two renderers behind one interface](decisions/0004-swappable-renderers.md) (proposed), [0005 Mods are native plugins behind a stable C ABI](decisions/0005-native-mod-plugins.md) (proposed), [0006 Multiplayer netcode](decisions/0006-multiplayer-netcode.md) (proposed), [0007 The optional telemetry server](decisions/0007-telemetry-server.md) (proposed), [0008 Themeable UI](decisions/0008-ui-themes.md) (proposed), [0009 Multiplayer netcode revisions](decisions/0009-multiplayer-netcode-revisions.md) (accepted), [0010 Co-op and the N-player world](decisions/0010-coop-and-the-n-player-world.md) (accepted), [0012 Localization](decisions/0012-localization.md) (accepted), [0013 The Custom HUD 2026](decisions/0013-custom-hud.md) (accepted) |
| [modding/](modding/README.md) | Native plugin mods (milestone 12): the catalogue of what a mod can do, the API in three parts (core, world, content) and the NFS:MW interface, with [ADR 0005](decisions/0005-native-mod-plugins.md). Mod cars are cars of their own: [stats](modding/car-stats.md), [bikes](modding/bikes.md) (two wheels, the [integration](specs/vehicle-two-wheel-integration.md)), [shop and paints](modding/car-customization.md), [sound](modding/car-sound.md), [collision](modding/car-collision.md), [lamps](modding/car-lamps.md), [police cars](modding/cop-cars.md); the worked example is the [2012 cars mod](../mods/nfsmw-2012-content/README.md) |
| [plans/gfx-renderers/](plans/gfx-renderers/README.md) | Plan for swappable renderers (native Black Box and Bevy) behind one interface: [interface](plans/gfx-renderers/interface.md), [Bevy backend](plans/gfx-renderers/bevy-backend.md), [upscalers and settings](plans/gfx-renderers/upscalers-settings.md), [PR list](plans/gfx-renderers/pr-list.md), [status and handoff](plans/gfx-renderers/status.md), [verification and risks](plans/gfx-renderers/verification-risks.md) |
| [mods/redux-v3.md](mods/redux-v3.md) | Reference notes on the REDUX V3 community mod pack (unlicensed): contents, install, target build, what the port already covers |
| [mods/redux-v3-plugins.md](mods/redux-v3-plugins.md) | Per-plugin reference for the pack's `.asi` plugins, every shipped config key with its default, and the Xbox 360 shader-swap script |
| [mods/nfsmw-extra-options.md](mods/nfsmw-extra-options.md) | NFSMW Extra Options (GPL-3.0, facts only): hotkeys, quick race and traffic limits, pursuit and weather tuning, fixes, every settings key with its default |
| [mods/nfsmw-unlimiter.md](mods/nfsmw-unlimiter.md) | NFSMW Unlimiter (unlicensed script mod): fixes and limits lifted for added cars, modder hooks, the settings file keys with defaults, required assets |
| [mods/nfsmw-unlimiter-essentials.md](mods/nfsmw-unlimiter-essentials.md) | NFSMW Unlimiter Essentials (unlicensed car parts pack): paint, neon, wheels, plates, brakes and tyres; Binarius install steps and known issues |
| [mods/nfsmw-2012-plates.md](mods/nfsmw-2012-plates.md) | The 2012 content mod's licence plates: the default text of each car's game pack, the retypeable plate, and the shared set of 469 plate designs (files, size, default, what is not checked) | Texts and design data verified on the install; the look on a car not seen in a window
| [mods/README.md](mods/README.md) | Index of 42 community mod and tool repositories: one page each (mechanism, config, licence, how the rewrite could match it) |
| [licensing.md](licensing.md) | Project license, what each source license allows, unlicensed repos, the spec-first process |
| [ai-drivers.md](ai-drivers.md) | The AI drivers in the code: traffic and cops, which spec each layer follows, deliberate differences, what is open |
| [pursuit-system.md](pursuit-system.md) | Current police/pursuit behaviour, repaired placeholders, original-game parity gaps, enhancements and verification |
| [specs/career.md](specs/career.md) | Career and progression overview: economy, blacklist, milestones, flow, car lot, save; the loop, what the rewrite needs and the open questions |
| [provenance/career-core.md](provenance/career-core.md) | `crates/nfsmw-career`, the career slices (settings, stable, economy, SMS inbox, safe house, rap sheet, save screens, regions, profile file glue): what is covered, the choices made where the specs are silent, the known gaps |
| [provenance/career-runtime.md](provenance/career-runtime.md) | The career at run time (`crates/nfsmw/src/career`, the world career and the career screens): profiles, start and autosave, the choices marked unconfirmed, how it is checked |
| [specs/](specs/README.md) | Behaviour specs written before implementing it from decompiled sources: [scenery visibility](specs/scenery-visibility.md), [scenery LOD](specs/scenery-lod.md), [visible sections](specs/visible-sections.md), [car assembly](specs/car-assembly.md), [FEng input](specs/feng-input.md), [front-end menus](specs/frontend-menus.md), [LAN server](specs/lan-server.md), the themeable UI: [ui-themes](specs/ui-themes.md) (component model, element slots, the `UiStyle` switch, the `bb.ui` hook; built), the [Custom HUD 2026](specs/custom-hud/README.md) (the from-scratch Custom HUD and menus of ADR 0013; designed, being built), and the designs [multiplayer](specs/multiplayer/README.md) (milestones 10 and 11) and [telemetry](specs/telemetry/README.md) (milestone 13), not built yet |
| [provenance/](provenance/README.md) | Records for modules whose behaviour came from restricted sources |

### Research

| Doc | What it covers |
|---|---|
| [research.md](research.md) | Prior art: decomps, sibling-engine RE, rewrites and viewers, hook projects, format tools, reference repos |
| [install-layout.md](install-layout.md) | What each folder of the install contains; full-scan numbers; mod changes; saves and registry |
| [TOOLS_AND_SKILLS.md](TOOLS_AND_SKILLS.md) | The RE toolchain on the development machine (Ghidra, Ghidra MCP, ReAgent, MSVC) |
| [tools/chunkdump.md](tools/chunkdump.md) | The Python chunk-tree dumper |

### File formats

[Prop spawners](formats/prop-spawners.md) connect original physics placements to scenery and gameplay attributes.

| Doc | What it covers | Status |
|---|---|---|
| [formats/bchunk.md](formats/bchunk.md) | bChunk container, padding, JDLZ/RAWW/HUFF wrappers, bare JDLZ blobs | Complete |
| [formats/huff.md](formats/huff.md) | EA HUFF codec (`0x30FB`): bit stream, numbers, canonical code, run-length clue | Complete; all 10,813 blobs decode |
| [formats/models.md](formats/models.md) | Solids: chunk tree, header, shading groups, vertex buffers per effect run, indices, LODs, compressed add-on cars | Verified; Rust reader |
| [formats/textures.md](formats/textures.md) | TPK: both pack forms, streaming entries, `TextureInfo`, platform record, pixel formats, alpha / blend modes | Verified; Rust reader |
| [formats/chameleon-bundle.md](formats/chameleon-bundle.md) | The 2012 game's `bnd2` bundles (Chameleon engine): header, resource table, zlib blocks, import table, resource type ids | Verified on all 2,021 files; Rust reader |
| [formats/chameleon-gfx.md](formats/chameleon-gfx.md) | Chameleon textures (and their `.dds` export), vertex layouts, renderables, materials, shaders and models | Verified on 93 cars; Rust reader |
| [formats/ride6-rider.md](formats/ride6-rider.md) | The RIDE 6 rider: the exporter's `R6RA` animations and `R6RM` skinned meshes, the runtime pack `.bbsk`, the files the rider importer writes; the data is the user's and never committed | Exported and converted; runtime tested in code, not built yet |
| Specs for the rider | [specs/skeletal-animation.md](specs/skeletal-animation.md) (the generic runtime `blackbox-skeleton`), [specs/ride6-rider-animation.md](specs/ride6-rider-animation.md) (how the rider is posed and attached), [specs/ride6-rider-render-integration.md](specs/ride6-rider-render-integration.md) (wave 2), [provenance/ride6-rider.md](provenance/ride6-rider.md) | Written; not integrated |
| [formats/ride6-physics-data.md](formats/ride6-physics-data.md) | The RIDE 6 physics data assets: where they are, Blueprint inheritance and the merge rule of the dump, units; field tables in [vehicle](formats/ride6-physics-vehicle.md), [tyre](formats/ride6-physics-tyre.md) and [elements](formats/ride6-physics-elements.md) with the KTM 450 SMR 2022 values | Dump verified on 3 bikes; units mostly confirmed in the binary |
| Specs for the RIDE 6 bike physics | [specs/ride6-lean-controller.md](specs/ride6-lean-controller.md), [ride6-rider-round6](specs/ride6-rider-round6.md) (rider drag, tuck-in, lean sign, animation script, [provenance](provenance/ride6-physics-C.md)), [ride6-tyre](specs/ride6-tyre.md), [ride6-suspension](specs/ride6-suspension.md), [ride6-engine-drivetrain](specs/ride6-engine-drivetrain.md), [ride6-rider-help](specs/ride6-rider-help.md), [ride6-tricks](specs/ride6-tricks.md) (wheelies, stoppies, rider help, auto-rider), [ride6-rider-body-aero](specs/ride6-rider-body-aero.md), [ride6-tyre-mf94](specs/ride6-tyre-mf94.md) (the tyre model object), [ride6-tyre-thermal](specs/ride6-tyre-thermal.md) (temperature, wear, efficiency), [ride6-gearbox-auto](specs/ride6-gearbox-auto.md) (shift decision, clutch feathering), [ride6-powertrain-round6](specs/ride6-powertrain-round6.md) (shift timing, starting rules, filters, brake coupling sign), [ride6-aero-drag](specs/ride6-aero-drag.md) (drag and slipstream), [ride6-chassis-round6](specs/ride6-chassis-round6.md) (roll closure, lean help, fall tests, the day, end of travel); [ride6-rider-fall](specs/ride6-rider-fall.md) (the rider leaving the bike, his ragdoll, the way back; [provenance D](provenance/ride6-physics-D.md)); the mapping to `BikeSpec` and the model changes: [specs/vehicle-two-wheel-parameters.md](specs/vehicle-two-wheel-parameters.md); [provenance](provenance/ride6-physics.md), [round 6](provenance/ride6-physics-A.md), [round 6 B](provenance/ride6-physics-B.md), [D](provenance/ride6-physics-D.md) | Partly read from the binary; each item is tagged |
| [formats/chameleon-language.md](formats/chameleon-language.md) | The `LocalisedText` string tables of `UI/LANGUAGE`: ids, UTF-16 texts, markup and colour codes | Verified on all 14 languages (byte-exact round trip); Rust reader and writer |
| [formats/chameleon-games.md](formats/chameleon-games.md) | How the bundles of Hot Pursuit 2010 and Remastered, Burnout Paradise and the console ports differ from the 2012 PC ones | Community tool's source only, not read |
| [formats/chameleon-vehicle.md](formats/chameleon-vehicle.md) | A 2012 car: graphics spec, wheel placement, names, surface kinds | Verified on 93 cars; Rust reader |
| [formats/chameleon-traffic.md](formats/chameleon-traffic.md) | The 11 traffic bundles (`_TR`: Audi A3 to GMC Topkick, no stats or paints) and the 93 low-detail bundles (`_LO`: a mid-detail body level the car bundles lack) | Bundles, names, levels and sizes verified on the install; handling in `TRAFFICATTRIBS.BNDL` not decoded per car; Rust reader `chameleon-vehicle::traffic` |
| [formats/chameleon-driver-animation.md](formats/chameleon-driver-animation.md) | How the people in the 2012 cars are animated: the 46-bone skeleton, the 28 clips (30 keys a second, packed rotations in the rest world axes), the pose rule, the skin, the steering clip the game scrubs, the arms reached to the wheel grips, the per-car pose clips | Skeleton, clips and pose rule verified (hands within 5 mm of the stored positions on every key); state machine and roles read from `NFS13.exe`, clip names unconfirmed; Rust reader `chameleon-anim`, rig in `chameleon-vehicle::driver_rig` |
| [formats/chameleon-active-aero.md](formats/chameleon-active-aero.md) | The spoilers of a 2012 car that rise with its speed: the input binder and animation state of the executable, the state machine's conditions, the animation's packed keys, spoilers, vents, the Agera and the Veyron wing machine decoded and implemented, the operator table; McLaren, Venom and Huayra driven from their graphs with flagged assumptions | Decoded from the data and `NFS13.exe` for 17 bundles |
| [formats/chameleon-hinged-parts.md](formats/chameleon-hinged-parts.md) | The hinged parts of a 2012 car (bonnet, bumpers, mirrors, doors): the `Bodypart` fields, the skeleton, the skin that says which triangles each part owns, the zone codes | Verified on the 93 cars; Rust readers; the engine's consumer not found |
| [formats/chameleon-collision.md](formats/chameleon-collision.md) | What a 2012 car keeps for collision, physics shapes and damage: the collision mesh (decoded), the convex hulls and the physics body (partly), the damage zones and hinged parts | Mesh verified on 92 of 93 cars; Rust reader |
| [formats/chameleon-lamps.md](formats/chameleon-lamps.md) | A 2012 car's lamps: the locator groups of the graphics spec, the `VehicleVfxBehaviour` corona and light records, the light-state word, the lamp meshes | Groups and records verified on 93 cars (981 coronas, 549 lights); bit meanings partly from `NFS13.exe`, mode and reverse unconfirmed; Rust reader `chameleon-vehicle::lamps` |
| [formats/chameleon-exhaust.md](formats/chameleon-exhaust.md) | A 2012 car's tail pipes: the spot effect records (nitrous bits 8 and 9) that name the locator group, the mirror flag, a per-car table of pipes, the cars without one | Verified on all 93 cars; bit meanings partly from `NFS13.exe`; Rust reader `chameleon-vehicle::lamps` |
| [formats/chameleon-corona.md](formats/chameleon-corona.md) | The flares, glows and beams of a 2012 lamp: the `Corona` objects of a car bundle (layers, curve blocks, pictures), the chain that sizes and dims them by angle and distance, the flash patterns and colours of the police sirens | Layout and the police data verified on the 93 cars; the evaluation chain from `NFS13.exe`; units and brightness scale unconfirmed; Rust readers `chameleon-effects::corona`, `chameleon-vehicle::read_corona_records` |
| [formats/chameleon-police.md](formats/chameleon-police.md) | Which 2012 bundles are police vehicles (five: Crown Victoria and Charger cops, Explorer Interceptor, Corvette Z06, SWAT van), the siren corona object, the game's role table; what was not decoded (heat level unit tables) | Marks verified on all 93 bundles; role table meaning unconfirmed; Rust reader `chameleon-vehicle::police` |
| [formats/chameleon-vehicle-stats.md](formats/chameleon-vehicle-stats.md) | A 2012 car's own stats: the Genesys handling definition (torque curve, gearbox, mass, tyres, brakes, top speed, nitrous) | Layout verified on 93 cars, several field meanings unconfirmed; Rust reader |
| [formats/chameleon-camera.md](formats/chameleon-camera.md) | The gameplay cameras of a 2012 car: the `VehicleCameraContainer` (chase, bonnet, bumper, rear view, shared globals), the type layouts, the per-car variation, and the behaviour model built from them | Layout and per-car values verified on the 93 cars and the traffic bundle; every field meaning and the whole model unconfirmed (the executable's camera code was not found); Rust reader and model `chameleon-camera` |
| [formats/chameleon-engine-state.md](formats/chameleon-engine-state.md) | The 2012 engine on/off state and the car's `Player Vehicle Ignition` start-up sound, and the standstill auto-stop model built around them | Ignition sequences and the `EngineOn` gating verified on the 93 cars; the standstill rule itself not found, its numbers unconfirmed; Rust reader and model `chameleon-engine-state` |
| [formats/chameleon-nitrous.md](formats/chameleon-nitrous.md) | What the 2012 game does when the nitrous burns: the sound sequences and their samples, the post effect states and keyframes, the HUD signals; what is not in the data | Sounds and post effect values verified on the 2012 install; roles and field names marked; flames, camera and speed effects not found |
| [formats/chameleon-damage.md](formats/chameleon-damage.md) | The 2012 damage model: the 8 per-zone clips (21 keys over the damage, a turn, a slide and a broken scalar per bone) that tilt and break off the pieces of a car, the damage fractions at which they go (the same on every car), the ladders, the damage bar, and the global tables that are not decoded | Verified on the 93 cars; Rust reader in `libs/chameleon-vehicle`; meaning of the ladders and the global tables unconfirmed |
| [formats/chameleon-drift.md](formats/chameleon-drift.md) | What a drift is and pays in the 2012 game: the `NitrousParameters.Drifting` payout (6 units a second of a 100 unit tank from 30 mph, after 0.75 s), the physics' `DriftBehaviour` trigger and exit (speed, slip angle, hold time), the HUD text, and the state machine built from them | Payout and physics numbers verified on all 93 cars; units and the meaning of the fields inferred; the code that reads them not found; Rust reader and model `chameleon-vehicle::drift` |
| [formats/chameleon-drift-effects.md](formats/chameleon-drift-effects.md) | What else the 2012 game shows and plays while a car drifts: the free-roam "DRIFTING" line, the Drift Attack score HUD (strings, bindings, score tick), the per-car tyre smoke and skid mark numbers, the tyre sound parameters, and the negative scan (no drift camera, post effect or rumble) | Strings, bindings and numbers read from the install; the score tick and the roles of the tyre numbers inferred (`NFS13.exe`); nothing ported |
| [formats/chameleon-effects.md](formats/chameleon-effects.md) | The BearEffect particle effect resource: file layout, the 78 operations, the decoded parameter blocks (spawn, life, radius, motion, colour and alpha, space) | Decompiled from the executable and mapped; the parameter array, flipbook rule, blends and lights marked as not decoded |
| [formats/chameleon-collision-effects.md](formats/chameleon-collision-effects.md) | The collision sparks of the 2012 game: the `VfxVehicleCollisions` objects, the impact and scrape effects, the spawner on the collision mesh, bounce, spark streak, bindings and their curves, materials; what is not in the data (contrail, speed lines) | Decoded from the data and the executable on the 2012 install; the surface index, segment-length constant and trigger code marked as not found |
| [formats/chameleon-sound.md](formats/chameleon-sound.md) | The 2012 game's `.SPS` streams (EA Audio Core, `EL31` Layer III frames) and the sample resources inside the car bundles | Streams verified on all 56 files; Rust reader `ea-audio::sps`; engine layer roles not read |
| [formats/chameleon-shared-sounds.md](formats/chameleon-shared-sounds.md) | The 2012 game's shared one-shots (`METALCAR.BNDL`, `WAVES.BNDL`) and the named graphs that play them: body hits, scraping, pops | Graph names verified (text in the graph); sample roles measured; layer mixing not decoded |
| [formats/chameleon-sound-graph.md](formats/chameleon-sound-graph.md) | The sound graphs of a 2012 car bundle: sequences, items, curves, mixer gains, the version-3 Ginsu engine resources, and the sound of speed (the wind loops of `GLOBAL.BNDL`, the silent `SpeedSoundSet`) | Class layouts verified on the Porsche; Rust reader `chameleon-sound-graph`; window direction, curve tangents and whine pitch rule inferred; the DSP is not ported |
| [formats/chameleon-customization.md](formats/chameleon-customization.md) | What the 2012 game offers per car for its looks: the `VehiclePaint` records (443 in 93 cars), liveries as vehicle entries, and what it does not have (vinyls, rims, spoilers, tint) | Paint records verified on all 93 cars; Rust reader `chameleon-vehicle::paints`; pack and list fields partly open |
| [formats/chameleon-plate.md](formats/chameleon-plate.md) | The licence plate of a 2012 car: the registration is the player's, its five defaults (`NEED4SPD` and four DLC packs) are in the executable and chosen by the car's game pack, the eight-quad plate mesh and its 36-glyph sheet, the 666 Autolog plate designs | Defaults, packs, mesh and sheet verified on the install; text rules from the executable; Rust reader `chameleon-vehicle::plate`; design fields and blank-slot drawing partly open |
| [formats/chameleon-plate-designs.md](formats/chameleon-plate-designs.md) | The 666 plate designs of Autolog (469 pictures of 512 x 256, a name and a game pack each; no category, country, glyph colour or default), what they cover on a car, and the 4 MB shared set the content mod writes | List, records and pictures verified on the install; Rust reader `chameleon-vehicle::plate_designs`; unlock table and default open
| [formats/chameleon-manufacturers.md](formats/chameleon-manufacturers.md) | The brands of the 2012 game: the manufacturer table of the vehicle list (name, logo picture, the `ManufacturerId` of each vehicle), the logo bundles under `UI/IMAGES/STREAMED/` | Table and 34 logos verified on the install; two record fields unconfirmed; Rust readers `chameleon-vehicle::manufacturers`, `chameleon-gfx::Texture::to_rgba8` |
| [formats/maps.md](formats/maps.md) | The L2RA world: metadata chunks, streaming index, section families, scenery placement and rotation encoding | Verified; Rust readers |
| [formats/world.md](formats/world.md) | World grid and road network, collision packs, bounds, triggers, emitters, sky, minimap | Partial (decomp only for several) |
| [formats/minimap.md](formats/minimap.md) | The minimap: 8 x 8 map tiles, the per-track calibration, the HUD objects and textures | Verified; Rust reader |
| [formats/collision.md](formats/collision.md) | World collision packs, the collision grid, car and prop bounds, surface types, query semantics | Layouts verified; Rust reader |
| [formats/event-sequences.md](formats/event-sequences.md) | World event engines, actions and fragment events | All 73 installed packages read; 75 engines |
| [formats/model-hierarchies.md](formats/model-hierarchies.md) | Named intact and internal fragment models | Validated reader and installed breakup checks |
| [specs/world-sequencers.md](specs/world-sequencers.md) | State/queue timing, damage stimuli and cargo | Sequencer tested; world effect integration in progress |
| [specs/sphere-contacts.md](specs/sphere-contacts.md) | Sphere contacts and relative sphere/box sweeps | Isolated PC vectors and world integration tests |
| [formats/global-databases.md](formats/global-databases.md) | Every file in `GLOBAL/`: the databases, UI and HUD packs, texture packs, and what the repo reads | Catalogue; read status per file |
| [formats/attributes.md](formats/attributes.md) | AttribSys `VPAK` packs, vaults, exports, hash, the 57 classes | Layout verified |
| [formats/cardata.md](formats/cardata.md) | Car types, parts database, slot types, presets, light materials, solid markers, `ecar`, vinyls | Car tables verified; vinyls partial |
| [formats/animation.md](formats/animation.md) | EAGL4 ELF objects, skeletons, banks, NIS cutscenes | Container verified; codecs open |
| [formats/audio.md](formats/audio.md) | Sound banks, GIN engine loops, MPF/MUS music, speech, reverb | Codecs known; speech `.idx`, `.evt`, `.csi` verified ([speech-evt.md](formats/speech-evt.md)) |
| [specs/car-audio-tyres-transmission.md](specs/car-audio-tyres-transmission.md) | Authored skid and transmission producers | PC bounds and installed playback regressions |
| [specs/car-audio-road-weather.md](specs/car-audio-road-weather.md) | Road transitions, stitch road and weather wind | PC parameters; render-clock policy explicit |
| [specs/car-audio-environment.md](specs/car-audio-environment.md) | Engine environment, distance filtering and software effects | Authored spatial maps, zone selection and FXEDIT wet bus; backend limits explicit |
| [car-audio-coverage.md](car-audio-coverage.md) | Audio coverage, verification boundaries and remaining gaps | Runtime audit and trace comparison tool |
| [specs/car-audio-observability.md](specs/car-audio-observability.md) | Single-owner routing, retained controller history and bounded tracing | Event and voice lifecycle regressions |
| [specs/car-audio-power-effects.md](specs/car-audio-power-effects.md) | Nitrous, purge, shift, sweetener, turbo and whine controllers | PC contracts and installed playback |
| [specs/authored-traffic-audio.md](specs/authored-traffic-audio.md) | Moving engines, horns and passing whooshes | Installed profiles and cadence checks |
| [specs/common-audio-events.md](specs/common-audio-events.md) | Speedbreaker activation and pursuit breaker modules | PC slots and retained event timing |
| [specs/spatial-parameter-maps.md](specs/spatial-parameter-maps.md) | Spatial Reverb map and enclosure zone context | Reader and PC lookup vectors |
| [specs/fxedit-processor.md](specs/fxedit-processor.md) | Authored effect graph parsing and processing | All installed patches; isolated PC PCM comparison |
| [specs/car-audio-output.md](specs/car-audio-output.md) | Shared wet bus and dry/pan/send ordering | Device-rate and irregular-block regressions |
| [specs/speedbreaker-audio.md](specs/speedbreaker-audio.md) | Shared real-time Speedbreaker pitch envelope | Original producer and installed routing checks |
| [formats/aems.md](formats/aems.md) | AEMS module banks: the dataflow graphs and code inside `.abk` files | Verified on all 301 banks; Rust reader |
| [formats/mixmap.md](formats/mixmap.md) | Dynamic-mixer maps (`MIXMAPS/*.mxb`): states, controls, events, 3D controls, channels, presets | Verified; Rust reader |
| [formats/video.md](formats/video.md) | EA VP6 container and decoder options | Complete |
| [formats/frontend.md](formats/frontend.md) | FEng UI packages, compressed packages, fonts | Packages known; font glyphs open |
| [formats/text.md](formats/text.md) | Language string blocks, labels, subtitles, memory-card locale files | Mostly verified |
| [formats/shaders.md](formats/shaders.md) | The 31 D3DX effects embedded in `speed.exe` | Located; not decompiled |
| [formats/ride6-audio.md](formats/ride6-audio.md) | RIDE 6 audio: the Wwise events, banks and media of a bike, the 634-row chassis table, the KTM 450 data, why the banks are unreadable (encrypted pak index), the bank chunks the importer reads. Specs: [specs/ride6-engine-sound.md](specs/ride6-engine-sound.md) (the values sent to Wwise, the backfire system with its numbers, the mapping to this game), [modding/ride6-bikes-sound.md](modding/ride6-bikes-sound.md), [provenance/ride6-audio.md](provenance/ride6-audio.md) | Events, effects and backfire read; engine loops and curves not recoverable without the pak key; generator written, not built |
| [formats/ride6-materials.md](formats/ride6-materials.md) | RIDE 6 vehicle materials: the instance chain, textures, the packed specular/roughness/metallic/occlusion picture, and what the bike importer keeps | Read from assets; shader graph not available |
| [formats/saves.md](formats/saves.md) | Save files: header, MD5, payload order (superseded by career-save.md for the layout) | Partial |
| [formats/career-save.md](formats/career-save.md) | Save layout: header CRCs, settings, stable, statistics, controls, MD5; editor offsets mapped | Verified on three saves |
| [formats/career-state.md](formats/career-state.md) | The career in memory: profile, settings, car stable, career records, rap sheet sums, script tables | Read in `speed.exe` |

## Original gameplay cameras

[Camera views](specs/camera-views.md) indexes native profile resolution, rolling
follow, projection, noise, clearance and stable high-FPS host timing. Each
contract has a companion provenance record and an explicit validation boundary.

## Project tools

Police AI: [implementation coverage and remaining parity limits](police-ai-parity.md).

- **Rust:** `nfsmw check-install | list-cars | view-car | view-world` (see the [root README](../README.md)).
  The engine-generic libraries are in [`libs/`](../libs), each with its own README.
- [`tools/chunkdump.py`](../tools/chunkdump.py) dumps the bChunk tree of a data file (JDLZ/RAWW aware).
- [`tools/bchunk_names.py`](../tools/bchunk_names.py) maps 330 known chunk IDs to names.
- [`tests/`](../tests) has the Python tests (`python -m unittest discover tests`).

## Conventions

- One file per format under [`formats/`](formats). Where a format isn't reverse-engineered yet, the doc
  says so instead of guessing offsets.
- PC data is **little-endian** unless a doc says otherwise.
- No game data and no decompiled code in docs: layouts, offsets, IDs and measured numbers only
  ([licensing.md](licensing.md)).

## Evidence tags

| Tag | Meaning |
|---|---|
| **[verified]** | Checked against the files in this install (with the Rust readers, `tools/chunkdump.py` or a short script); the numbers quoted come from those runs |
| **[decomp]** | From the [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) decompilation or its symbol files |
| **[community]** | From community tools such as NFS-ModTools, VaultLib, vgmstream |
| **[unconfirmed]** | Observation or hypothesis not yet proven |

## Open questions

| Question | Where to look |
|---|---|
| Meaning of `PermSize` in `TrackStreamingSection` | `World/TrackStreamer.cpp` in the decomp |
| `TroughBoundary.bin` chunk `0x00034190` (not in the decomp's chunk list) | — |
| EAGL bank header and channel layouts | `EAGL4Anim/*` in the decomp |
| Where a compressed palettized texture keeps its palette | no example in car packs; check world/frontend streams |
| Reverb and mix presets; the table after the records of `copspeech.evt` | [formats/audio.md](formats/audio.md), [formats/speech-evt.md](formats/speech-evt.md) |
| FEngFont glyph format | [formats/frontend.md](formats/frontend.md) |
| Save header bytes 0x08–0x33 | answered in [formats/career-save.md](formats/career-save.md) (CRCs and descriptor) |
| What moves the career stage on after bin 15; the bounty divisor at run time; the Lua rules (safe house heat reset) | [specs/career.md](specs/career.md#open-questions-all-pages) |
| How the `CarShader` paint, reflections and lighting work | the `fx_2_0` effects in `speed.exe` ([formats/shaders.md](formats/shaders.md)) |
| Water animation and reflections (`ANM_WATERA_` frames, `SKY_POND_REFLECTION`) | `TextureAnimPack` `0xB0300100`, the effects in `speed.exe` |
| What the subtractive / overbright blend types look like in the game | the effects in `speed.exe` |

Camera audit coverage and remaining scene-validation boundaries: [original gameplay camera coverage](specs/original-camera-coverage.md).

Graphics profiles and missing original-rendering coverage: [coverage ledger](graphics-coverage.md), [settings contract](specs/graphics-profiles.md).

World rendering: [material routing](specs/world-material-routing.md),
[normal maps](specs/world-normal-materials.md), [atmosphere](specs/world-atmosphere.md),
[sky shading](specs/world-sky-shading.md).

Road and glass: [reflective surfaces](specs/world-reflect-materials.md),
[architectural windows](specs/world-window-materials.md),
[projected wet-road captures](specs/wet-road-reflections.md).

Car rendering: [original material families](specs/car-material-shading.md),
[light rigs and authored specular scale](specs/car-lighting.md).

Projected shadows: [camera and strength](specs/original-projected-shadows.md),
[world surface receivers](specs/world-surface-shadow-receivers.md).
