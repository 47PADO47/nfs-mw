# Behaviour specs

Plain-language specifications of game behaviour (physics, AI, pursuit, streaming, camera, …), written
before the Rust implementation. Required wherever the only source is decompiled code; see
[licensing.md § Spec-first](../licensing.md#spec-first).

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
the song events and the play lists); the HUD minimap: [hud-minimap.md](hud-minimap.md) (the projection, the tiles around the player, the turning, the missing speed zoom); the user interface: [feng-runtime.md](feng-runtime.md) (scripts, messages,
drawing), [feng-input.md](feng-input.md) (pad messages, focus, navigation) and [frontend-menus.md](frontend-menus.md)
(the screens, the option rows, the game flow).

[Original PC collision particles](pc-collision-particles.md) specifies ordinary
textured sparks and contact glow separately from the experimental restoration.

[Controller settings](controller-settings.md) specifies rewrite response options and saved rebinding.

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
