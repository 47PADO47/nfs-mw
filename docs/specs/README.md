# Behaviour specs

Plain-language specifications of game behaviour (physics, AI, pursuit, streaming, camera, …), written
before the Rust implementation. Required wherever the only source is decompiled code; see
[licensing.md § Spec-first](../licensing.md#spec-first).

Loose prop scenery contacts: [prop-world-barriers.md](prop-world-barriers.md).

Driving extensions: [performance-upgrades.md](performance-upgrades.md),
[speedbreaker.md](speedbreaker.md), [vehicle-jump-stabilizers.md](vehicle-jump-stabilizers.md),
and [vehicle-ground-reference.md](vehicle-ground-reference.md) (persistent body terrain queries).

Pursuit tuning: [pursuit-tuning.md](pursuit-tuning.md) (the Bartender file: cop cap, heat growth, bust speed, fines)
and [fairbusts-reset.md](fairbusts-reset.md) (the reset penalty of the busted timer and its file).

Mods: [mod-manager.md](mod-manager.md) (the Mods page of the options: the listed mods, their on/off switches and when they take effect).

Written so far: [scenery-visibility.md](scenery-visibility.md) (exclude flags),
[scenery-lod.md](scenery-lod.md) (level of detail), [visible-sections.md](visible-sections.md) (which world
sections are loaded and drawn) and [car-assembly.md](car-assembly.md) (stock parts, wheels, brakes, paint,
texture swaps, the car shader) and vehicle physics: [vehicle-engine-drivetrain.md](vehicle-engine-drivetrain.md),
[vehicle-input-induction-brakes.md](vehicle-input-induction-brakes.md),
[vehicle-manual-shifting.md](vehicle-manual-shifting.md) (the transmission setting, manual mode, the limiter),
[vehicle-suspension-tires.md](vehicle-suspension-tires.md),
[vehicle-steering-assists-aero.md](vehicle-steering-assists-aero.md) and
[vehicle-rigid-body.md](vehicle-rigid-body.md) (and [vehicle-calibration.md](vehicle-calibration.md), the model checked against the running game); car sound: [engine-sound.md](engine-sound.md) (files, telemetry,
engine mix), [engine-sound-ginsu.md](engine-sound-ginsu.md) (the granular synthesiser) and
[engine-sound-effects.md](engine-sound-effects.md) (shifting, turbo, nitrous, skids, collisions); the
dynamic mixer: [dynamic-mixer.md](dynamic-mixer.md) (evaluating a mixer map) and
[car-sound-mixer.md](car-sound-mixer.md) (what the car sound publishes and reads); AEMS: [aems.md](aems.md)
(running a module of a sound bank) and [engine-sound-aems.md](engine-sound-aems.md) (the engine's sample layer,
the sputters, the sweeteners); the radio: [music-graph.md](music-graph.md) (the PathFinder graph of `MW_Music.mpf`,
the song events and the play lists) and [interactive-music.md](interactive-music.md) (the pursuit sets, the game state that
drives them, the cross-fades); speech: [speech.md](speech.md) (the dispatch queue, when a line may be said,
the recordings) and [speech-sentences.md](speech-sentences.md) (which recordings say a line for its parameters); [exhaust-flames.md](exhaust-flames.md) (the tail-pipe particle effects for nitrous and gear changes); the HUD minimap: [hud-minimap.md](hud-minimap.md) (the projection, the tiles around the player, the turning, the missing speed zoom) [race-minimap.md](race-minimap.md) (the map file of a running race: the event's baked route tiles, read on demand) and [hud-cop-blips.md](hud-cop-blips.md) (the police car icon: white when normal, the blue and red light bar in a pursuit); the user interface: [feng-runtime.md](feng-runtime.md) (scripts, messages,
drawing), [feng-input.md](feng-input.md) (pad messages, focus, navigation) and [frontend-menus.md](frontend-menus.md)
(the screens, the option rows, the game flow).

Cameras: [camera-views.md](camera-views.md) (the seven original views, from the install's `camerainfo` data, the camera
setting's cycle and the gameplay camera's priority; the speed curve and some field meanings are still unconfirmed).

Time of day: [time-of-day.md](time-of-day.md) (the 8 lighting presets and the blend; the clock is not yet confirmed).

Weather: [weather.md](weather.md) (the rain target from the script hook and the zone map, the easing, the drop pool and wind; rain only, no thunder or lightning in this build; static analysis, not yet confirmed in game).

[Original PC collision particles](pc-collision-particles.md) specifies ordinary
textured sparks and contact glow separately from the experimental restoration.

[Post-processing](post-processing.md) specifies the optional bloom, tone mapping and FXAA passes (new design).

[UI themes](ui-themes.md) specifies the themeable HUD and menus: the component model, the neutral draw tree, the element slots, the Original fallback, the `UiStyle` setting and the `bb.ui` mod hook (new design, see [ADR 0008](../decisions/0008-ui-themes.md)). [UI theme menus](ui-themes-menus.md) covers the menu cell, the Original menu adapters and the slot map the frontend plugin fills in T12. [UI theme custom](ui-themes-custom.md) covers the Custom radio card and the retired egui radio path. [UI theme Remastered menus](ui-themes-remastered-menus.md) covers the vertical icon menus of the Remastered style. [UI theme Remastered screens](ui-themes-remastered-screens.md) covers its dialogs, key list, credits, Blacklist, minimap and radio card.

[Contrast-adaptive sharpening](cas-sharpening.md) specifies a CAS pass for native resolution, from AMD's MIT algorithm as read in a ReShade pack (new design; overlaps RCAS, see the spec).

[Controller settings](controller-settings.md) specifies rewrite response options and saved rebinding.

[EAGL animation banks](eagl-animation.md) records the animation bank layout, the channel evaluators and the 14-byte key format found in `speed.exe`, and what is still unmapped (work in progress; the airliner is not yet animated).
[Controller UI](controller-ui.md) specifies device ownership, live binding prompts and menu rendering corrections.

[LAN and online server launcher](lan-server.md) records how the original `server.dll` is started and patched outside the game (MIT-licensed NFSLAN as the source, with what the stock `speed.exe` does and does not contain). Reference only; nothing is implemented.

[Ground contacts](ground-contact-response.md) and [prop lifetime](prop-lifetime.md)
cover measured plane response, world probes, sinking and model cleanup.

[Native impact parity](native-impact-parity.md) covers the original contact
pipeline, authored prop attributes and the limits of installed-data coverage.

[Pursuit breakers](pursuit-breakers.md) covers named scenery fragments, area
stimuli, expanding explosions, ownership and fixed-clock cop-stop regions.
[Pursuit breaker moment](pursuit-breaker-moment.md) covers the camera cut a breaker plays: its gate, the marker it
stands on, the ICE camera track and what the world does during it.
[Pursuit hiding spots](pursuit-hiding.md) covers the HIDDEN track zones, the latch that turns a spot into
"hidden", its effect on cops and the evade meter, and the HUD mark.
[Trailer constraints](trailer-constraints.md) covers named angular limits,
mass-weighted geometry correction and measured point-moment response.

[Contact camera shake](contact-camera-shake.md) covers acceleration triggers,
waveform, view transforms and fixed-tick presentation.

## Template

[Tire effects](tire-effects.md) describes this rewrite's procedural smoke and grounded marks,
consuming the vehicle's existing intensities. Its appearance and lifetime are new design choices.

```markdown
# <topic>

- **Sources read:** <URL, license, files>
- **Data inputs:** <AttribSys classes and fields, chunk types>

## Behaviour
<state machines, formulas, update order, units; pseudocode where useful, never translated source>

## Constants
<only where a constant is hard-coded in the game rather than in AttribSys; say where it was observed>

## How to check it
<what to measure in the original game to confirm the implementation matches>
```

## Police and high-speed effects

[Final driving audit](vehicle-final-audit.md) records confirmed corrections and
remaining clock, geometry, context and live-comparison limits.

- [police-tactics.md](police-tactics.md): the aim of each cop (block, pit, ram) and what the rule leaves out.
- [speed-effects.md](speed-effects.md): the high-speed screen effects; the curves are in `libs/blackbox-speedfx`.

## World entries

- [world-markers.md](world-markers.md): the ground disc at each auto shop, safe house and race start, the drive-over
  zone, and the interact card with its stub entry (the destinations are not built yet).

## Career and progression

- [career.md](career.md): the overview, the loop and the open questions.
- [career-economy.md](career-economy.md) and [career-economy-flows.md](career-economy-flows.md): bounty, infractions and fines, cash, heat, rap sheet statistics, busted and the impound strikes.
- [career-blacklist.md](career-blacklist.md): the 15 rival bins as state graphs, the challenge gate, races won, regions, the final race.
- [career-milestones.md](career-milestones.md) and [career-milestones-data.md](career-milestones-data.md): the statistics, milestones and speed traps, their states and tables.
- [career-flow.md](career-flow.md) and [career-car-lot.md](career-car-lot.md): new and resumed career, safe house, autosave, car lot and car rules.
- [career-safehouse.md](career-safehouse.md) and [career-safehouse-2.md](career-safehouse-2.md): the native safe house (zones, engage, unload and reload, heat, the respawn marker).
- [menu-backdrop.md](menu-backdrop.md) and [menu-backdrop-looks.md](menu-backdrop-looks.md): the 3D picture behind the menus: the crib room and the safe house room, which car each look shows (the profile's last car, the red BMW M3 E46 GTR by default), and the original's car stand and camera poses.
- [career-scripts.md](career-scripts.md) and [career-scripts-events.md](career-scripts-events.md): the compiled Lua state graphs (bins, prologue, boss races, final race, challenge ladder, speed traps).
- [career-text.md](career-text.md) and [career-text-2.md](career-text-2.md): the language table side (rivals, labels, SMS ids, popups, shop unlock tiers).
- [career-phone.md](career-phone.md): the phone: SMS inbox slots and states, delivery rules, call voice, who sends which id.
- [career-rapsheet.md](career-rapsheet.md) and [career-rapsheet-2.md](career-rapsheet-2.md): the rap sheet screens, ranks, history, end game.
- [career-save-screens.md](career-save-screens.md) and [career-save-screens-2.md](career-save-screens-2.md): manual save, autosave gate and triggers, the file write, load checks.
- [boot-profile-prompt.md](boot-profile-prompt.md): the profile step after the title screen: the load list of saved profiles (this build's and the original's, read only), or the create question and name prompt when none is saved; how `play` starts.
- [career-parity-menus.md](career-parity-menus.md): the start-up alias list, the main menu card, the alias manager (Create, Load, Save, Delete), the career menu, the Quick Race menu, the pause menu and the safe house menu before the world.
- [career-regions.md](career-regions.md): closed regions (barrier gates per stage), the region unlock screen, map variants.
- [race-barriers.md](race-barriers.md): the walls a race's `Barriers` list switches on (scenery groups, the star's
  mirror, collision, the door) and the road barrier flags.
- Layouts: [../formats/career-state.md](../formats/career-state.md) (memory) and [../formats/career-save.md](../formats/career-save.md) (file).

## Pursuit, traffic and the busted sequence

- [ai-pursuit-goals.md](ai-pursuit-goals.md): the cop goals that drive at a slot around the player: pit, ram (box-in and
  rolling block) and pull-over.
- [ai-pursuit-support.md](ai-pursuit-support.md): the support cars of a pursuit: heavy SUVs (ram, roadblock) and the cross cars.
- [ai-traffic-obstacles.md](ai-traffic-obstacles.md): what an ordinary traffic car does when a car, wreck, roadblock or the
  player is in its lane, and how it recovers when stuck.
- [busted-sequence.md](busted-sequence.md): from the busted meter filling to the career screen (arrest scene, packages, choices).
- [busted-aftermath.md](busted-aftermath.md): after the ticket: the impound notices, the career car select, the release, the game over and the return to free roam.
- [career-cooling.md](career-cooling.md): the `fecooling` table, paint cooling of the car's heat and the heat meter.
- [cop-health-bars.md](cop-health-bars.md): an optional health bar above each police car that is chasing the player.

## Vehicles

- [vehicle-decomp-audit.md](vehicle-decomp-audit.md): the port's vehicle physics checked against the PC decompilation, with the
  corrections it implies.

## Front end, audio and world content

- [pursuit-free-roam.md](pursuit-free-roam.md): patrol promotion, sight/search, radar, heat, soundtrack ownership, damage and roadblock integration.

- [custom-fonts.md](custom-fonts.md): optional replacement FEng fonts from a user folder, per render height, with scaling.
- [fe-shadows.md](fe-shadows.md): an optional front-end shadow pass (off by default; the original draws none).
- [hd-content-support.md](hd-content-support.md): HD font glyphs, the mouse pointer and full-motion video at the right size.
- [hd-reflections.md](hd-reflections.md): optional HD reflection options, with the original defaults.
- [custom-music.md](custom-music.md): the player's own audio folders and M3U playlists on the radio, next to the EA Trax songs.
- [mod-world-content.md](mod-world-content.md): proposed scenery packs and interiors, the concrete form of ADR 0005 for the world.

## Multiplayer and telemetry (designs, not built)

- [multiplayer/README.md](multiplayer/README.md): LAN and online multiplayer (milestones 10 and 11): requirements, protocol,
  sync model, bandwidth, transport, integration, online phases and the security design (see its own index).
- [telemetry/README.md](telemetry/README.md): the optional telemetry server (milestone 13): channels, transport and privacy,
  control endpoints, the HTTP API and what carries over to multiplayer (see its own index).
- [quick-play.md](quick-play.md): the Quick Play brief (random car and race by Blacklist stage, Randomize, Accept), with its
  Ghidra record in [../provenance/quick-play.md](../provenance/quick-play.md).
- [race-mode.md](race-mode.md): race phases, checkpoints and laps, ranking, the time limit, knockout, drag and speed-trap
  races and the result; [ai-racers.md](ai-racers.md), [ai-racers-route.md](ai-racers-route.md) and
  [ai-racers-catchup.md](ai-racers-catchup.md): the opponents, their grid and route, and the catch-up.
- [race-finish-effects.md](race-finish-effects.md): the red flares at a race's finish line and the win photo (the world seen through the
  polaroid's window from the race's replay camera), with its Ghidra record in
  [../provenance/race-finish-effects.md](../provenance/race-finish-effects.md).
- [race-restart.md](race-restart.md): when the pause menu counts a race as in progress, and what Restart resets (the clock, the
  racers, the checkpoints, the cops) without touching the career.
