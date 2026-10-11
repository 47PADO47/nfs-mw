# Controller settings and saved bindings

These are rewrite options **[ours]**, layered on the existing action mapping. The default response remains
the current linear, rescaled 15 percent stick deadzone; this document does not claim that curve has been
calibrated against the original game. No restricted source was used for these options.

The optional cutoff response is informed by [xan1242/NFS-XtendedInput](https://github.com/xan1242/NFS-XtendedInput)
(MIT, `src/Main.hpp`, per-axis deadzone handling); no source code was copied.
The plugin leaves values outside the cutoff unchanged, so matching its
percentage alone with a rescaled curve does not produce the same response.

## Response settings

- Steering and camera stick deadzones are independent, from 0 to 95 percent (default 15).
- Analog-trigger deadzone is independent, from 0 to 95 percent (default 0).
- Steering, camera-stick and mouse sensitivity are independent, from 1 to 400 percent (default 100).
- Invert camera Y reverses vertical look and orbit for mouse and stick; default is off.
- Deadzone response can be rescaled (default) or cutoff. Both map magnitudes below `d` to zero;
  rescaled uses `(abs(v)-d)/(1-d)`, while cutoff leaves values outside the cutoff unchanged.
  Non-finite device input contributes zero; finite axes/triggers are bounded before rescaling.
- Steering sensitivity applies to analog steering, not digital steering buttons. Camera sensitivity applies
  to the stick; mouse sensitivity applies to mouse motion. Menu navigation keeps its existing threshold.
- Stick camera motion is integrated per second. Mouse deltas are already per frame and are never multiplied
  by the frame duration. Inversion is applied exactly once after the existing source direction convention.
- Config file, environment, console and the Controls menu share the same validated types. Invalid values
  leave the lower layer or current value intact. Menu edits persist through the existing settings writer.
- Expanded lists reuse the package's visible row objects while scrolling, so every response option retains
  its title, value, focus and edit controls in both the main-menu and pause-menu layouts.

## Rebinding

The per-user settings file can contain a `[bindings]` table: action names map to arrays of source strings.
An absent action keeps the shipped defaults (including configured wheel paddles); an empty array unbinds it.
Malformed action entries are warned about and ignored as a unit, so a typo does not silently remove controls.

Sources name a physical key, mouse button/motion/scroll, gamepad button/axis or analog trigger. A source can
carry a finite signed scale and an explicit per-second flag, retaining bidirectional axes and camera rates.
Every action is bindable, and bindings are resolved once per frame through the existing action layer.

Console commands can replace or append a binding, unbind an action, reset assignments to defaults and save
the current assignments explicitly. Replacing one device family keeps the other families of the same action.
The live `keys` listing shows the effective bindings. Saving preserves unrelated settings and unknown keys;
invalid existing TOML is an error rather than overwritten. Ordinary console edits are session-only until saved.

## Verification

Synthetic input and Bevy message tests cover unchanged default response, independent settings, endpoints,
invalid/non-finite values, analog triggers, inversion, per-frame mouse versus per-second stick motion,
rebinding while another device remains assigned, signed axes, suppression while the console is focused,
tap edges and persistence round trips. Runtime captures check the original Gameplay menu and live console.
Hardware controller validation remains a distinct claim and requires an actual connected-device exercise.

## Steering wheel

**[ours]**; the original's wheel handling was not read. Wheel axes are `Source::PedalAxis { axis, inverted }`
(`pedal:` and `pedal_inv:` in text): `(1 + raw) / 2`, or `(1 - raw) / 2` inverted, so a pedal at either end of an
axis gives 0 to 1. Non-finite input is 0 and an axis that has not reported counts as released, so an unplugged or
silent wheel never floors the car. The trigger deadzone applies. The steering wheel itself is an ordinary axis and
takes the steering deadzone and sensitivity (a wheel usually wants the deadzone near 0).

Actions added: `gear_reverse`, `gear_neutral`, `gear_1` to `gear_7` (no defaults) and `clutch` (default key Z).
`manual_clutch` and `h_shifter` are off by default, are layered like the other settings (config, environment,
console) and reach the car through `Scene::set_wheel_options`. Their physics is in
[vehicle-manual-shifting.md](vehicle-manual-shifting.md), section 6. The input layer logs unnamed buttons and non-stick
axes so the player can find codes. Tests are synthetic (parsing, round trips, conversion, latching, physics); no real
wheel, H-shifter or clutch pedal was available.

## Backend filtering

Inspected the Cargo.lock-pinned `bevy_input` and `bevy_gilrs` 0.20.0 sources (`src/gamepad.rs`,
`src/lib.rs` and `src/gilrs_system.rs`). Bevy's gilrs adapter disables gilrs's default filters and only
converts D-pad axes to buttons. Bevy's own default 5 percent analog zones and 1 percent change thresholds
still discard small updates; although `Gamepad` stores accepted raw values, a return to zero can leave a
stale sub-threshold value. The rewrite neutralizes analog zones and change thresholds on gamepad addition,
including reconnection, before raw events are processed. Digital button press/release settings remain
unchanged. Actual Bevy connection and raw-event tests cover initial input, sub-5-percent and sub-1-percent
changes, returns to zero, trigger hysteresis and reconnection with existing settings.
