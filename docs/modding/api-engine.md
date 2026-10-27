# Mod API: the player's engine (`bb.engine`)

Part of the [modding catalogue](README.md). **W** marks a write that is denied in an online session (this one is not yet
enforced; see Open).

**Delivered (`bb.engine` 1.0): read the driver's pedals, switch the player's engine off, start it with a sound.** Built and unit
tested (the ABI layout, the host table, the game's port, the engine switch). Not yet run in the game window.

A mod that gives the car an engine that can stop (the 2012 content mod's **2012 engine auto-stop**, see
[chameleon-engine-state.md](../formats/chameleon-engine-state.md)) needs three things the other interfaces do not give: the
raw pedals (while the engine is off the drive no longer sees the throttle), a switch that really cuts the engine, and a start
sound from the mod's own files. This interface is that, and nothing else; it is small on purpose, and the rule that decides
when to stop is the mod's.

## What an engine that is off does

While the switch is off (`stop`, until `start` has run its delay) the player's car:

- takes **no throttle and no nitrous** (the steering, the brakes, the handbrake and the gear requests go on, so the car coasts
  and can be braked and steered);
- shows **no revs** (the tachometer reads 0, and `bb.vehicles` `state().rpm` with it);
- makes **no engine sound**: the engine voices, the sample layer, the spark chatter and the turbo are silent. Tyres, wind,
  hits and the gearbox are not touched.

Nothing else changes: the physics, the camera, the HUD's other numbers and the AI are as they were. A game with no mod that
calls `stop` is exactly as it was: the switch is idle until used.

## Calls

| Call | Does | Errors |
|---|---|---|
| `state()` | the phase (`Off`, `Starting`, `Running`), `free_roam`, the throttle and brake pedals (0 to 1, **as the driver presses them, whether or not the engine takes them**), the speed along the car in m/s | `NOT_READY` while no car is driven |
| `car()` | the id of the player's car (`POR911CARR12`), through a sink | `NOT_READY` |
| `stop()` **W** | switches the engine off; queued, made on the scene after the mods have run | `NOT_READY` |
| `start(sound, gain, delay)` **W** | plays `sound` now and runs the engine after `delay` seconds; queued like `stop`. Nothing happens if the engine already runs or starts | `ARG`, `NOT_READY` |

`free_roam` is true while the player drives free of any event: no race (running or queued), no chase or arrest, no cutscene or
movie, no garage, and the chase camera on the car. A mod that keeps a rule for free driving only (the auto-stop) reads it.

`start` takes `sound` as a path in the mods' `files/` layers (`SOUND/MODENGINE/<ID>/IGNITION.WAV`; the empty string plays
nothing), a 16-bit PCM WAV, mono or stereo, read through the install's layers like the nitrous variants' sounds. The path
is plain relative text of at most 512 bytes: no `..`, no leading `/` or `.`, no drive and no backslash (`ARG`). `gain` is 0
to 8 (the sound plays at most at 4, as the game's other mod sounds), `delay` is 0 to 10 seconds; a number outside, or not
finite, is `ARG`. A file that cannot be read is a warning in the log and a silent start: a start never fails the engine.

**One engine, many mods.** The game remembers the mod that stopped the engine. If that mod unloads (or is reloaded), the engine
runs again at once with no sound. Two mods that both stop it are not arbitrated: the last call wins.

## SDK

```rust
if let Some(engine) = ctx.engine() {
    let state = engine.state()?;                      // phase, free_roam, throttle, brake, speed (m/s)
    if state.free_roam && state.speed.abs() < 0.5 {
        engine.stop()?;                                // later, when the throttle is pressed:
        let car = engine.car()?;
        engine.start(&format!("SOUND/MODENGINE/{car}/IGNITION.WAV"), 1.0, 0.6)?;
    }
}
```

`ctx.engine()` is `None` when the game does not offer `bb.engine`; ask for it with `[optional.interfaces]` in the manifest
(`"bb.engine" = "1.0"`) and cope with `None`. The calls are made from a mod callback on the main thread, like every other call.

## Host side

`blackbox_modding::host::EngineBackend` (`state`, `car`, `stop`, `start`, and `release`, called when the mod's table is dropped on
unload); the registry adds it with `InterfaceRegistry::add_engine`. The game's port is `crates/nfsmw/src/modding/backends/engine.rs`:
`state` and `car` read the bridge's copy of the scene taken before the mods run (`Scene::engine_readout`), `stop` and `start`
queue a `WorldRequest::Engine` that `Scene::engine_command` makes. The scene's state is `crate::engine_switch::EngineSwitch`
(a pure machine with tests), held by the player's `Drive`:

| Part | Where |
|---|---|
| throttle and nitrous cut | `scenes/world/drive/engine_switch.rs` (`DriveInput::engine_stopped`, applied in `simulation.rs` next to the story chase's sabotage) |
| tachometer at 0 | `Drive::hud_state` |
| engine silent, start sound once | `CarSoundState::engine` (`EngineAudio`), `audio/car/update.rs` (levels), `audio/car/engine_start.rs` (the one-shot) |
| `free_roam` | `scenes/world/mod_hooks.rs` |

## ABI

`BbEngine` (`abi::iface::engine`): `head`, `ctx`, `state`, `car`, `stop`, `start`, in that order (48 bytes on 64-bit); fields are
only ever appended. `BbEngineState` is size-prefixed: `size`, `phase` (`BB_ENGINE_OFF` 0, `BB_ENGINE_STARTING` 1,
`BB_ENGINE_RUNNING` 2), `free_roam`, `throttle`, `brake`, `speed` (24 bytes). The layout is locked by `abi/iface/engine_tests.rs`.
**Versioning:** a new interface is a new id at 1.0: no table of an existing interface changes and the ABI minor stays, so every
mod built before loads as it did (ADR 0005 sections 1 and 7).

## Open

Online sessions (the call should be denied for a peer, like the other **W** calls), an engine switch for cars other than the
player's, a stall that the game itself decides (a flooded engine, water), and a start that the player must hold the key for.
Creep on a car whose engine is off (the drive only cuts the throttle; the car's idle torque is the physics' own) was not
checked in a window.
