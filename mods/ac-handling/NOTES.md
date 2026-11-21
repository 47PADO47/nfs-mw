# ac-handling: implementation notes (stages 2 to 5)

Written by the implementer sessions, 2026-11-20 and 2026-11-21, from the specs alone (the firewall of
[ADR 0016](../../docs/decisions/0016-handling-hooks.md) section 5: no Assetto Corsa file, no other project's code, no restricted
source was opened). Sources used: AGENTS.md, CONTRIBUTING.md, ADR 0016, the plan, `docs/modding/api-handling*.md`, the specs of
`docs/specs/ac-handling/` and the SDK of `libs/blackbox-modding` for how a mod is built. Nothing here was measured against Assetto Corsa.

## Result

| What | State |
|---|---|
| Tyre force T1 to T14 and its properties | all reproduced (`src/tests/tyre_force_vectors.rs`) |
| Wheel dynamics at the default `inertia_scale` 1.0: the one-substep tables, W1 to W9b with 6-decimal slip angles (1e-5), W10 convergence and the three host rates; the `inertia_scale 0.3` check | all reproduced (`src/tests/wheel_vectors.rs`) |
| Parameter mapping: the four cars, per-axle grip, overrides, the forced-compound table and its invariant, precedence, car ids, step 9 clamp | all reproduced (`mapping_vectors.rs`, `mapping_forced.rs`, `loader.rs`) |
| Tyre thermal T1 to T12 (the 900 s warm-up and the 1800 s cool-down included) and the properties | all reproduced (`thermal_vectors.rs`) |
| Suspension V1 to V17 and the properties | all reproduced (`suspension_vectors.rs`) |
| Aids TC1 to TC6, ABS1 to ABS4, BIAS, BIAS2, ESC1 to ESC7, FLAG | all reproduced (`aids_vectors.rs`) |
| Steering S1 to S10, F1 to F6, the steering flag | all reproduced (`steering_vectors.rs`) |
| Scenarios (convention E): the ten scalars of `targets.md` 2.1, S2t, the 11-rung S5 ladder, the inertia sweep, the two class tables, 1/144 s | all inside 3 % (0.05 s, 0.5 m) of the planar values and inside the design bands (`tests/scenarios/`) |
| Rows skipped | the Assetto Corsa reference of every row is `unmeasured` in `targets.md`: skipped with a printed reason, never guessed |
| Real loader, fake host | switches, hotkey, command, every unit, master off, a refused hook, online refusal, a real vehicle on all three hooks without a fault, the steering flag (`tests/real_loader/`) |

No scenario was added for the aids, the suspension or the steering: their specs add vectors, not scenarios.

## Spec questions

Resolved by the spec revision of 2026-11-20 (struck, kept for the record):

1. ~~`inertia_scale` 0.3 breaks the host's drivetrain.~~ The default is now 1.0 (`Keys::default()`, no preset key); a value below 0.5 is
   accepted and logs one warning per car. The planar harness runs the key at 0.3, 0.5, 1.0 and 2.0 (the sweep of scenarios.md).
2. ~~The car has no name.~~ The id `spec-<kg>-<cm>-<kW>` of parameter-mapping.md 2.1 (`CarSpec::key_name`, rounding half away from zero, `spec-invalid`).
3. ~~`cars/<car>.toml` collides with the content registry.~~ The folder is `tuning/`; a file in `cars/` is not read (tested).
4. ~~A forced preset kept the car's own level (slick below sport).~~ Re-based on the compound's reference (`map_car`), with the invariant tested.
5. ~~Precedence of the compound.~~ Setting, then the tuning file, then automatic.
6. ~~Ranges after the last step.~~ Step 9 clamps every key.
7. ~~Time and distance conventions of the planar appendix.~~ Convention E (end-of-tick samples and stamps).
8. ~~`slip_angle` rounding and the W7 corner mass.~~ Six decimals, 1e-5; the airborne rows send 350 kg.
9. ~~The `dt` guard.~~ `0.1 * (1 + 1e-6)` in the tyre, suspension and controls hooks.
10. ~~The thermal seam.~~ Implemented: `GripScale` now has `grip_scale`, `end_tick`, `reset` and `hud`; `ThermalGrip` keeps four tyres.

Still open, or new:

11. **The SDK delivers a payload-less `FRAME_START` as `Event::Unknown`.** The mod matches the frame start by `event.kind()`.
12. **Settings and the hotkey.** `bb.settings` has no `set`, so a flip by the hotkey or the console is not reflected in the settings row; `status` says
    what is true. The default hotkey `key:F1` is a choice (F2 to F12 are taken by the game's tools and other mods) and unverified in the game window;
    a refusal is logged and the other switches still work. Each switch-on registers a new hook closure that stays in the SDK's keep list until the mod
    unloads (a few hundred bytes per switch and hook).
13. **tyre-thermal.md has no per-compound table**, although section 5 says the compound group is overridden per compound. All five compounds use the
    road defaults (`ThermalParams::default()`), and the constants are not in the TOML schema (no `[thermal]` table).
14. **No spec for the unit defaults and the aids' starting levels per preset** ("defaults per preset" is a requirement of the task, not of a spec). The
    `[units]` table of the compound presets and tuning files carries them: suspension and thermal on, aids and steering off (the aids and the steering
    change how the controls feel: the player opts in), starting levels economy 3/3/2, road 2/2/1, sport 1/1/0, semislick and slick 0 (traction
    control, ABS, stability control). All design values.
15. **A level slider has no "unset" state.** A slider the player never moved follows the preset's level; a moved one (even back to 0) is the player's
    (the saved value is read at load: a saved 0 is indistinguishable from the default, so it follows the preset until the slider is touched again).
16. **`p_slide` and `v_slide`** (tyre-thermal.md 2.2) are the mean over the substeps of the values at the end of each substep (the wheel call has no
    other sample point); they feed the thermal state of the same tick.
17. **The steering seed.** `delta` starts from the last suspension call's inside front `steer_angle_rad` only while the suspension hook is
    registered, else 0; the host's rate limit is bypassed while the flag is set, so a seed of 0 on a car whose wheels are turned snaps them (a
    one-time effect when the steering unit is switched on mid-corner).
18. **Aids need the car's constants.** If the spec cannot give them (a mass, track or brake torque that is not positive), the controls hook declines
    every tick (the steering part with it) rather than guess.
19. **The tyre HUD** is a message through `bb.hud` every 5 s of simulated driving (shown 2 s), only with the HUD setting and the thermal unit on and
    a HUD present; it is never an error. The core temperature of the four tyres, the pressure of the front left and the lowest remaining life.
20. **Not verified in the game window:** the suspension and thermal units on a real NFS car, the hotkey binding, the settings rows. The loader tests
    drive a real `blackbox-vehicle` through all three hooks (with a test car, one gear and one tyre) and find no fault.

## Decisions

- **All model maths is `f64`**, narrowed once at the edge (`bridge.rs`). A result with a number the host would refuse (not finite, or past the
  ranges of the interface pages) is never sent: the call answers "not handled" and the state reseeds.
- **One concern per file**, none over 500 lines. Tyre: `tyre_force.rs`, `substep.rs`, `wheel.rs`. Thermal: `thermal.rs`, `grip.rs`. Suspension:
  `suspension.rs`. Controls: `aids.rs`, `steering.rs`, `controls.rs`. Parameters: `keys.rs` (one macro generates the struct, the TOML table and the
  ranges of the tyre, suspension and steering key sets), `mapping/`, `presets/`. Glue: `control.rs` (requests and the plan), `control_hooks.rs` (the
  three hook entry points), `control_commands.rs` (command, status, HUD), `model.rs`, `setup.rs`, `lib.rs`.
- **Switches never register in a handler.** The settings handlers, the hotkey and the command only change the request; the frame start applies it
  (twice: once for the master and the units known without a car, once after the car's spec is read, when the preset's unit defaults are known).
- **Hooks follow the switches independently.** The tyre hook follows the master; the suspension hook the master and the suspension unit; the controls
  hook the master and (aids or steering). Unregistering goes controls, suspension, tyre; registering the other way. A refused suspension or controls
  hook is blocked until a switch changes; a refused tyre hook switches the master off.
- **The spec is read when the revision changes**, not on a timer: any hook that sees a `vehicle_revision` other than the parameters' keeps the old
  parameters for that tick and asks the next frame start to read `spec()` once.
- **The mod never asks the host for anything it has not offered.** Camber and load come from the suspension hook only through the host (the
  tyre query's `camber_rad`); with the unit off the tyre sees 0.
- **`config.toml`** gives only first values. A saved settings choice other than the default wins over it; `enabled = true` in either place switches
  the model on.

## Left for later

- A `[thermal]` table and per-compound thermal constants once the spec has a table (question 13); presets for the whole roster and per-car files
  (stage 6); a name field in the spec would replace the car id (`CarSpec::key_name` is the one place).
- A measured row in `targets.md` (a maintainer with their own install) would replace the design values; nothing in the mod depends on their exact value.
- Force feedback has no host output; `Control::ffb_signal()` is the accessor if the game ever gets one.
- The owner's drive (gates A and B) decides the feel; every constant is a design value.
