# Controller response and rebinding

Xbox prompts use bundled [Kenney CC0 icons](https://kenney.nl/assets/input-prompts); base PC assets work.
Use A to accept, B to go back, D-pad or left stick to navigate, and Menu/Start to pause or resume.
View/Back quits from the main menu. Prompts follow saved and live rebinding and switch on meaningful
keyboard, mouse or controller input. Console hotkeys and idle sticks do not switch the displayed device.
Disconnecting the active controller pauses driving. Menu stick engagement is 55%, release is 35%; this
is independent of steering/camera deadzones. See [UI policy and references](specs/controller-ui.md).

Open **Options > Controls** from the main menu or pause menu. The response rows apply immediately;
backing out saves the changed settings. Deadzones move in 5 percent steps and sensitivities in 25 percent
steps. The config file and console accept finer integer percentages.

| Setting | Default | Range |
|---|---|---|
| Deadzone Mode | Rescaled | Rescaled / Cutoff |
| Steering Deadzone | 15% | 0–95% |
| Camera Deadzone | 15% | 0–95% |
| Trigger Deadzone | 0% | 0–95% |
| Steering Sensitivity | 100% | 1–400% |
| Camera Sensitivity | 100% | 1–400% |
| Mouse Sensitivity | 100% | 1–400% |
| Invert Camera Y | Off | On / Off |

Rescaled response removes the deadzone and stretches the remaining range to full input. Cutoff response
keeps the raw value outside the deadzone, matching the per-axis cutoff approach in
[XtendedInput](https://github.com/xan1242/NFS-XtendedInput). These are per-axis settings; steering sensitivity
affects analog steering, and camera sensitivity affects the stick independently of mouse sensitivity.
The rewrite's default mappings remain available; a mod's ASI does not supply bindings to the rewrite.

## Config file

Run `nfsmw check-install` to find the per-user `config.toml`. On Windows it normally lives at
`%APPDATA%\nfsmw\config\config.toml`. Put response keys at the root, before any table:

```toml
deadzone_mode = "cutoff"
steering_deadzone = 10
camera_deadzone = 10
trigger_deadzone = 0
steering_sensitivity = 100
camera_sensitivity = 100
mouse_sensitivity = 100
invert_camera_y = false

[bindings]
throttle = ["key:KeyW", "trigger:RightTrigger2"]
steer = ["key:KeyD", "key:KeyA:-1", "axis:LeftStickX"]
nos = ["key:ShiftLeft", "button:East"] # Xbox B
```

## Steering wheel

A wheel shows up as a gamepad. Button and axis codes depend on the device, so nothing is bound by default except a
clutch key. The log (`info`) prints the code of each unnamed button the first time it goes down and of each non-stick
axis when it is first seen or moves by half its travel; press a button or move a pedal and read the name.

| Source | Meaning |
|---|---|
| `axis:LeftStickX` | the wheel itself, -1 to 1; the steering deadzone and sensitivity apply to `steer` |
| `pedal:Other(2)` | a full-range pedal axis, -1 released to +1 pressed, read as 0 to 1 |
| `pedal_inv:Other(2)` | the same for a pedal that reads +1 while released (common on Linux) |
| `trigger:Other(7)` | a pedal reported as an analog button |
| `button:Other(20)` | paddles, gear buttons, a clutch button |

A pedal axis that has not reported yet counts as released. The trigger deadzone applies to pedals. Example:

```toml
manual_clutch = true   # reads the clutch action; off by default
h_shifter = true       # gear_* actions hold a gear; off by default
paddle_up = 5          # shortcut for shift_up = ["button:Other(5)"]

[bindings]
steer = ["axis:LeftStickX"]
throttle = ["pedal_inv:Other(2)"]
brake = ["pedal_inv:Other(1)"]
clutch = ["pedal_inv:Other(0)"]
gear_1 = ["button:Other(20)"]    # gear_neutral, gear_reverse, gear_2 ... gear_7 the same way
gear_reverse = ["button:Other(26)"]
```

`manual_clutch` and `h_shifter` (also `NFSMW_MANUAL_CLUTCH`, `NFSMW_H_SHIFTER` and `set manual_clutch on`) only matter
with a manual transmission for the gears; the clutch works with either. Without `h_shifter` a gear button asks for its
gear once when pressed. With it, the gear whose button is held is kept and no button means neutral; the brake pedal
then never selects reverse. The hardware layouts are not verified: see
[the specification](specs/controller-settings.md#steering-wheel).

An omitted action keeps its default assignments. An empty array unbinds the action. Invalid entries
produce a warning and keep that action's defaults. Response environment variables use the upper-case
key with `NFSMW_`, for example `NFSMW_STEERING_DEADZONE=4`. Environment values override saved settings.

## Console

Press **F12**. `keys` lists effective controls with the action names used for rebinding:

```text
set steering_deadzone 4
set deadzone_mode cutoff
bind throttle key:J
bind nos button:b
bind steer key:D
addbind steer key:A:-1
unbind nos gamepad
bind-reset nos
bind-save
```

`bind` replaces that action's assignments from the same device family (keyboard, mouse or gamepad).
`addbind` appends an assignment. `unbind <action>` removes all assignments; an optional family narrows it.
`bind-reset` restores all defaults, or just the named action. These edits apply immediately and remain
session-only until **bind-save**. Console `set` edits also last for the session; use the Controls menu
or config file to persist response settings. Saving bindings preserves other settings and unknown keys;
comments and formatting are rewritten by the TOML serializer.

Sources use `kind:name[:scale[:per_second]]`:

- `key:KeyW` (or `key:W`), `key:ArrowUp`: physical Bevy key names.
- `button:South` (or `button:a`), `button:Other(7)`: gamepad buttons, including wheel paddles.
- `trigger:RightTrigger2`: analog pedals; `axis:LeftStickX`: signed analog axes.
- `mouse:Left`, `mouse:scroll`, `mouse:look_x`, `mouse:look_y`, `mouse:orbit_x`, `mouse:orbit_y`.
- `axis:RightStickY:-700:per_second`: signed scale and frame-rate-independent camera motion.

Scales must be finite, nonzero and within ±10000. Mouse motion already contains the frame's delta;
`per_second` is only valid for key/button/axis/trigger sources. Each action supports up to 32 assignments.

The radio has three button actions, bound like any other: `radio_toggle` (pause and resume; M, the media
play/pause key, pad right stick click), `radio_next` (`.`, media next, pad D-pad right) and `radio_previous`
(`,`, media previous, pad D-pad left). They work while driving and are ignored in the pause menu, which uses the
D-pad itself. A press of `radio_previous` within 3 s of a song's start goes back to the song before; later it
starts the song again. The console `radio` command does the same without keys (`radio prev|pause|resume|toggle`).

The existing 0.75 press / 0.65 release hysteresis for digital gamepad buttons remains separate from
analog trigger and stick response. See [the controller specification](specs/controller-settings.md)
for implementation boundaries and verification.
