# Speedbreaker

Sources: original PC 1.3 executable inspected locally; and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0 declarations and
reference behavior in `LocalPlayer.cpp`, `PInput.cpp`, `QuickGame.h` and
`SuspensionRacer.cpp`. Read for behavior only; no decompiled expression copied.
The action, meter and HUD bar are built. PC meter/time behavior below was
researched on 2026-10-13. Native HUD arithmetic and scripts were checked on
2026-10-14; rendered geometry still needs visual comparison.

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

The remake keeps a shared real-time cadence: 60 Hz by default, with 144 Hz as
an explicit offline comparison setting. Each completed tick updates the physics
multiplier and supplies its final scaled delta to player cars, AI, props, contacts,
police and race timers. Full Speedbreaker therefore gives 60 updates with delta
1/240, or 144 updates with delta 1/576. Rendering FPS does not select this cadence.

A continuous frame ramp remains available for camera/frame presentation. Physical
tick scaling uses the original repeated single-precision endpoint update, rather
than the area under that ramp. The generic GameSpeed/moment scale is separate.
Screenshot batches use a bounded real-time slice; network sessions force the
existing unscaled 60-Hz contract. See [shared physics clock](shared-physics-clock.md).

The PC begin/end event also sets the player's collision mass override to twice
the original mass while active and clears the override on exit. The reference
suspension applies two extra downward g, reduces aerodynamic drag to 25 percent,
increases front-wheel grip by `1 + 0.75 * abs(steering_input)`, and moves the
steering target toward `steering_input * 60` degrees.

Status of these vehicle effects [built; partly verified]:

- Built. `drive/sim/performance.rs` maps the tunables to a `blackbox_vehicle::ManeuverAssist`
  while the breaker is active, and to the neutral assist otherwise. The vehicle step applies each field:
  downforce (`apply_aero`), drag scale (`apply_aero`), tire steering drag (`begin_tires`), front
  grip (`tune_tires`), collision mass (`modify_object_hit`) and steering angle (`update_steering`).
  Unit tests cover each effect and its return to neutral on exit.
- Reference check (CC0 `SuspensionRacer.cpp`, the repo the spec cites). Its GameBreaker path has
  `Tweak_GameBreakerExtraGs = -2.0` (times weight, mass and 9.81), steer drag `Lerp(0.15, 1.0, weight)`,
  aero `drag_pct = 1 - 0.75 * weight`, front grip `weight * abs(steer) * 0.75 + 1` on the front tires, and steering
  `Lerp(newsteer, steer * 60, weight)`. At full weight these match the built values. The built
  gravity uses the PC `GRAVITY` 9.8128, which the reference rounds to 9.81.
- Not matched: the reference ramps the weight down after exit at 1/3 per physics second, so every effect above
  fades over about 0.33 s. The remake switches them off at once. The collision mass is binary in the PC
  path as far as the spec shows, so only the other effects would need the fade.
- PC Ghidra (`speed.exe`): the constants are present in the PC data (`0.25`, `0.75`, `25.0`, `60.0`, `2.0`), and
  `60.0` at `0x8905F8` has code references; the one routine read (`0x4120CF`) is not the steering or event code,
  as far as it was read. The event function was not located, so the per-effect PC mapping is verified only against the reference.

Configuration. Each tunable reads from `NFSMWSpeedFixerSettings.ini` next to the config file, with the mod's
keys and units (see `docs/mods/nfsmwspeedfixer.md`). The reader is `libs/ini-tuning`; the NFS:MW schema and the vanilla
values are `crates/nfsmw/src/settings/speedbreaker.rs`. Missing or malformed values are vanilla. Out-of-range values
clamp. Two keys follow the mod's omission rules: an absent `maxDuration` is unlimited, and passive recharge is off
when both of its keys are absent. Our clamp ranges are our choice where the mod gives none. Unit conversions: km/h to
m/s at the boundary, percent to fraction, and `timeScale` as the divisor of the multiplier (4 gives 0.25). `gravityScale`
is the total gravity multiple, so the extra downforce is `(gravityScale - 1) g`, and `slipGripReduction` is the tire steer-drag
coefficient (15 percent normal, 100 percent active).

## 4. The HUD bar [built; PC arithmetic checked]

The speedometer's gauge is in `HUD_SingleRace.fng`. From the package's names (found with Ghidra, from the strings
`Speedbreaker_Meter_Multi_Image`, `Speedbreaker_Meter_Group`, `SPEEDBREAKER_METER_ICON_GROUP`, and their FEng name hashes
from `blackbox_feng::fe_hash_upper`):

| object | name | hash | role |
|---|---|---|---|
| icon group | `SPEEDBREAKER_METER_ICON_GROUP` | `0x11448B5C` | the stopwatch group (group `0xE9C1E`) |
| meter group | `Speedbreaker_Meter_Group` | `0x8991BD54` | the group (group `0xEC1BD`) |
| meter bar | `Speedbreaker_Meter_Multi_Image` | `0x5DF2F5B4` | the MultiImage that rotates (`0xE3778`, `Meter_Backing2.tga`) |

`hud/speedbreaker.rs` binds and exposes the meter/icon groups. Native HUD update
confirms a 175-degree empty mask, its linear fill's fixed left edge, low-charge
threshold at 0.3 and charge icon scripts. The drag package uses -48 degrees.
Thirteen isolated original-code cases cover charge thresholds and package modes;
FEng accessors are synthetic. The meter reaches the HUD through
`HudState::speedbreaker`, which the drive scene fills from `Drive::breaker`.
Rendered layout equivalence remains separate. Audio and further presentation
contracts are in [speedbreaker-audio.md](speedbreaker-audio.md).

An active-state edge also supplies the original package fade and ACTIVATE
messages; exit supplies fade-in and DEACTIVATE. The package animates the normal
HUD away and brings in the bottom central horizontal charge meter. The charged
circular icon is the inactive HUD indicator, not a replacement for this active
layout. The original camera transition changes both spatial lag and FOV;
see the paired contract in [speedbreaker-audio.md](speedbreaker-audio.md).

The race timer ("00:00.00", `0xE1AA7`) is a different timer, at the top right of the screen.

## 5. Remaining comparison

- Camera and transition timing still differ in gameplay comparisons. The
  corrected scalar and HUD animation clocks do not establish a matched complete
  transition. Chase is a custom camera; the other views also retain native
  follow, obstruction, noise and framing gaps documented in
  [camera-views.md](camera-views.md).
- Compare live PC activation, depletion and clock transitions with the remake.
- Validate remaining collision mass/inertia paths and scheduler integration.
- Compare audio treatment and camera/visual effects.
- Check HUD bar geometry, direction and icon visibility against the original.
