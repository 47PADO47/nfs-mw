# Speedbreaker

Sources: original PC 1.3 executable inspected locally; and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0 declarations and
reference behavior in `LocalPlayer.cpp`, `PInput.cpp`, `QuickGame.h` and
`SuspensionRacer.cpp`. Read for behavior only; no decompiled expression copied.
The action, meter and HUD bar are built. PC meter/time behavior below was
researched on 2026-10-13. The HUD geometry still needs visual comparison.

## 1. The action [built]

- `Action::Speedbreaker` (`input/action.rs`), a button. Its name is `speedbreaker`, and the menu calls it "Speedbreaker"
  (`input/describe.rs`).
- Default keyboard binding: **Left Ctrl** (`input/bindings.rs`). The original's Speedbreaker is on Ctrl in its own
  keyboard screen (the first screenshot), so the default follows it.
- The remake additionally defaults to mouse **Back** and gamepad **East**
  (Xbox B / PlayStation Circle). These are host choices, not original PC defaults.
  Left Ctrl is reserved for Speedbreaker; Right Ctrl remains a downshift key.
  A normalized wheel East button works through the same gamepad binding, and
  device-specific buttons/axes/pedals can be rebound and saved.
- The controls list's "Speedbreaker" row changes this action. It used to change `Boost` (Shift), which was wrong:
  Boost is the free camera's "fly fast" and is not the original's Speedbreaker.

## 2. The meter [decomp, PC constants verified]

`scenes/world/drive/speedbreaker.rs` holds charge from zero to one and the active
flag. A reset can restore a full meter and always deactivates the breaker.

- The action toggles on a button press. Releasing it does not deactivate the
  breaker. A second press toggles it off.
- Activation needs positive charge and at least 30 mph on the speedometer.
  Falling below 30 mph deactivates it. The original also disallows activation
  during loading, staging, animation, a cinematic or multiplayer.
- While active, charge drains at 0.1 per **real second** (ten seconds full to
  empty), then deactivates at zero. The infinite-breaker cheat skips depletion.
- While inactive, normal recharge occurs only strictly **above 100 mph** and
  adds `simulation_delta / 25`. These are PC globals, not vehicle attributes.
- A separate drift recharge adds `simulation_delta * 0.5 * drift_strength`
  when inactive, speed is strictly above 35 mph and absolute slip/yaw is
  strictly above 30 degrees. Charge is capped at one.
  This producer runs once per successful player physics tick, after the drift
  strength updates, using physical chassis speed and body slip rather than the
  wheel-based speedometer. The exact PC thresholds are in
  [final audit corrections](vehicle-final-audit.md).
- Nitrous recharge is denied while the player breaker is active.

The original values were read from the PC executable and checked against the
meter routines. Mod option keys are separate configuration inputs and do not
identify AttribSys fields.

The meter processes button edges once per loaded render frame, with the input
flag `DriveInput::speedbreaker` and the speedometer speed. Recharge uses that
frame's simulation delta; depletion uses its real delta. Frames that produce
no physics tick still consume real-time charge. The garage and unavailable
local road data freeze both meter and clock progression.

## 3. Global time and vehicle effects [decomp]

The single-player time manager targets a simulation-speed multiplier of 0.25
while active. It moves the current multiplier toward 0.25 by two per real
second, taking 0.375 seconds to enter from normal speed. On exit it moves toward
one by 0.5 per real second, taking 1.5 seconds from quarter speed. World physics
uses this simulation clock; charge depletion still uses the real clock.
The multiplier transition flag remains set through the return to normal speed.
Separate cinematic time overrides and multiplayer mode are outside this meter.

The remake integrates the linear ramp over each frame, then feeds the resulting
simulation delta to the shared 60 Hz accumulator. Player, AI, props, contacts,
police timers, rain, scenery animation and day progression use this clock.
Chase-camera frame input is scaled before accumulation across fixed ticks.
The generic GameSpeed setting remains a separate multiplier. Screenshot batches
use six fixed-step durations as a synthetic real frame, so scripts advance
deterministically through the same shared clock without wall-time dependence.
These integration choices have synthetic frame-rate and transition coverage;
they have not been compared against a live PC driving capture.
The final PC scheduler audit found an architectural difference: the original
updates the multiplier once per 60 Hz real-time tick, then advances physics by
that tick's duration times the final multiplier. The remake's frame integral
and fixed simulation-time accumulator are not equivalent during Speedbreaker.

The PC begin/end event also sets the player's collision mass override to twice
the original mass while active and clears the override on exit. The reference
suspension applies two extra downward g, reduces aerodynamic drag to 25 percent,
increases front-wheel grip by `1 + 0.75 * abs(steering_input)`, and moves the
steering target toward `steering_input * 60` degrees. These are additional
vehicle effects; a changed clock alone does not establish their parity.

## 4. The HUD bar [built; not checked on screen]

The speedometer's gauge is in `HUD_SingleRace.fng`. From the package's names (found with Ghidra, from the strings
`Speedbreaker_Meter_Multi_Image`, `Speedbreaker_Meter_Group`, `SPEEDBREAKER_METER_ICON_GROUP`, and their FEng name hashes
from `blackbox_feng::fe_hash_upper`):

| object | name | hash | role |
|---|---|---|---|
| icon group | `SPEEDBREAKER_METER_ICON_GROUP` | `0x11448B5C` | the stopwatch group (group `0xE9C1E`) |
| meter group | `Speedbreaker_Meter_Group` | `0x8991BD54` | the group (group `0xEC1BD`) |
| meter bar | `Speedbreaker_Meter_Multi_Image` | `0x5DF2F5B4` | the MultiImage that rotates (`0xE3778`, `Meter_Backing2.tga`) |

`hud/bind.rs` binds the bar by `SPEEDBREAKER_BAR` and rotates it by the meter, like the nitrous bar
(`SPEEDBREAKER_BAR_EMPTY_DEG`, which is assumed to equal the nitrous bar's 175°). The meter reaches the HUD through
`HudState::speedbreaker`, which the drive scene fills from `Drive::breaker`.

Not checked: the bar turns the right way and the right amount on screen, and the icon group shows and hides as the
original does (the bar is only driven, not shown or hidden). The next step is a run with a car, and a look at the bar.

The race timer ("00:00.00", `0xE1AA7`) is a different timer, at the top right of the screen.

## 5. Remaining comparison

- Compare live PC activation, depletion and clock transitions with the remake.
- Validate remaining collision mass/inertia paths and scheduler integration.
- Compare audio treatment and camera/visual effects.
- Check HUD bar geometry, direction and icon visibility against the original.
