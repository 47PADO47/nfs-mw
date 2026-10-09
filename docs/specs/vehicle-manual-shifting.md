# Manual shifting

How the original lets the player change gear by hand: where the choice is kept, what the manual setting switches
off, and what the engine does when the player asks for a gear that does not fit the road speed. It extends
section 7.5 of [vehicle-engine-drivetrain.md](vehicle-engine-drivetrain.md) and sections 8.3 and 8.4 of
[vehicle-input-induction-brakes.md](vehicle-input-induction-brakes.md); formulas of the torque loop, the clutch and
the shift points are not repeated here.

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), under `src/Speed/Indep/Src`: `Physics/Behaviors/EngineRacer.cpp` (`DoShifting`, `AutoShift`,
  `SportShift`, `OnGearChange`, `DoGearChange`, the torque loop's limits, `FindShiftPotential`; the drag subclass),
  `Physics/Behaviors/PInput.cpp` (`FetchInput`, `DoShifting`, `DoAutoReverse`, `DoAutoBraking`,
  `IsAutomaticShift`), `Physics/Behaviors/PInput.h`, `World/RaceParameters.hpp` (`eTransmissionType`),
  `Frontend/Database/FEDatabase.cpp` (`PlayerSettings::Default`),
  `Frontend/MenuScreens/Safehouse/options/uiOptionWidgets.cpp` (`POTransmission`),
  `Frontend/MenuScreens/Safehouse/quickrace/uiQRCarSelect.cpp` (the transmission prompt). Read for understanding;
  no code copied.
- **Data inputs:** the AttribSys `transmission` fields of the drivetrain spec (`GEAR_RATIO`, `SHIFT_SPEED`,
  `CLUTCH_SLIP`); the player's `Transmission` setting, which is not in the data but in the player settings.

Evidence tags as in the [docs README](../README.md#evidence-tags). **Everything is [decomp]** unless marked; nothing
was measured in the running game. The input bindings (which key or button asks for a shift) are not in the decompiled
sources read: the original takes them from controller configuration attributes. Section 6 is the rewrite's own design.

## 1. The setting

- The player settings hold `Transmission`, an integer. `PlayerSettings::Default` sets it to **0 (automatic)**. The
  enum `eTransmissionType` lists `AUTOMATIC = 0`, `MANUAL = 1`, `SPORT = 2`, `MANUAL_CLUTCH = 3`; the engine code
  only ever compares against `MANUAL`, and nothing in the sources read sets 2 or 3. [decomp]
- The **Gameplay options screen** has one row for it (`POTransmission`): left or right on the pad toggles between 0
  and 1 (any other value is left alone). Title language key `0xD31407E7`; data string `0x8CD532A0` for automatic and
  `0x317D3005` for manual. [decomp]
- The **quick race** car-select screen asks "automatic or manual" before a race (a dialog whose two buttons store 0
  or 1); a flag `TransmissionPromptOn` (default on) enables it, and it is skipped for drag races. [decomp]
- Whether the shifting is automatic is decided every tick by the player's input behaviour (`IsAutomaticShift`):

  ```
  if the car is driven by the autopilot (AI control of a "human" car): automatic
  if the player's Transmission == MANUAL:                              manual
  if the driver style is DRAG:                                         manual
  otherwise:                                                           automatic
  ```

  Every other behaviour (the base input class used by AI and remote cars, traffic, the spline engine) is automatic.

## 2. A shift request

The shift buttons are queued actions; the input behaviour takes at most one per tick (the last one wins, `+1` up,
`-1` down) and calls `DoShifting(dir)`:

```
top     = number of gear ratios - 1
desired = clamp(gear + dir, 1, top)            # 1 is neutral: a downshift from first goes to neutral
if gear == reverse:                 refuse     # the shift buttons do nothing in reverse
if desired == gear:                 refuse     # an upshift in the top gear
if staging in a drag race and desired > 2: refuse
if manual:    Shift(desired)                   # the plain gear change of the drivetrain spec, section 7.2
else:         SportShift(desired)              # the "tiptronic" of section 7.5; refused while a shift runs
```

`Shift(gear)` is `DoGearChange(gear, automatic = false)`: it refuses a gear above the top or below reverse, then
`OnGearChange`: refuses the same gear, sets `shift_timer = SHIFT_SPEED * GEAR_RATIO[gear]` (a quarter of it for a
lower gear) and `gear = new`, and opens the clutch. It then clears the shift potential and, for the player's car,
raises the event `EPlayerShift(status, automatic, from, to)` that the HUD and the sound system observe. [decomp]

In manual mode a request is **accepted while a shift is still running** (`shift_timer > 0`): there is no lock-out
and no check against the shift potential, so the player may change several gears in quick succession, each restarting
the timer with the new gear's delay. Only sport shifting refuses a request while shifting. [decomp]

## 3. What manual mode switches off, and what stays

The only branch is in `EngineRacer::DoShifting`, which runs `AutoShift` only if the input says the box is automatic.
In manual mode therefore:

- **No automatic up or down shifts**, at any rpm. The engine does not shift at the red line and does not downshift
  when the car slows to a stop.
- **Neutral stays neutral.** The automatic box leaves neutral for first at once; manual does not. Neutral is reached
  by shifting down from first. In neutral the clutch is open, the inertia is 0.35 of the in-gear value and the
  engine free-revs to the limiter on full throttle; the drive torque is zero. [decomp]
- **No sport hold**: the 1.25 s timer is only set by `SportShift`.

Unchanged by the setting: auto-reverse (`DoAutoReverse`) and idle auto-braking (`DoAutoBraking`) run for every
player car. From any gear but reverse, at less than 2.5 m/s with the brake pressed and no gas, the car goes to
reverse; in reverse, the brake released or the gas pressed above -5 m/s goes to first. A manual player who stops in
fifth and presses the brake is put in reverse, and in first when they then press the gas. [decomp]

The shift potential (the up, down or none the automatic box would take) is still computed every tick from the
transmission-side rpm whenever the clutch is engaged, and is shown by the HUD's shift light and gear display; manual
mode only ignores it. [decomp]

## 4. The rev limiter and over-revving

The limiter is part of the torque loop and is the same in every gear and in manual mode (drivetrain spec, section 3):

- The engine's own speed is clamped to `[idle, red line]` after each integration. It cannot stall: at the lowest it
  runs at idle, whatever the gear and the road speed. There is no stall rule, and there is no manual clutch pedal
  (`MANUAL_CLUTCH` is in the enum but nothing reads it; the rewrite's optional pedal is in section 6).
- The transmission-side speed `omega_trans = idle + wheels * ratio * (red - idle) / red` is what the engine would
  turn at if locked to the wheels. When it exceeds the red line: a positive drive torque is **cut to zero**,
  `omega_trans` is set to the red line and **the driven wheels are set to the speed that gives the red line in the
  current gear** (`red / ratio`, written back as a shift of all driven wheels; below 40 mph or with both wheels of
  an axle in the air the pair is first averaged).
- Held in a gear on full throttle, the car therefore stops accelerating at the speed of the red line in that gear
  and the tachometer sits at the red line. In the top gear that is the top speed; with the speed limiter set it may
  come first. A manual driver who never shifts up gets exactly this.

**A downshift into a gear that would over-rev is not refused.** Neither `DoShifting` nor `Shift` compares the new
gear's rpm with the red line. The gear changes and from the next tick on the transmission-side speed exceeds the red
line (that check does not depend on the clutch, which is still slipping); the wheel write-back then pulls the driven
wheels down to the red line speed of the new gear in a single tick. The effect is a strong deceleration, harder the larger the mismatch,
through the tyres' longitudinal slip (the wheels are forced slower than the road) and the drive torque cut; the
engine stays at the red line meanwhile. This is how an over-rev downshift feels in the original: engine braking
with no protection. [decomp, from the loop; the feel is not measured]

The speed limiter (ECU) still tapers the throttle in the gears above neutral.

## 5. Shift delay, the clutch and reverse

- Delay: `SHIFT_SPEED * GEAR_RATIO[new]`, a quarter of it for a downshift, the same in both modes. During the delay
  the clutch is open; it then engages over 0.25 s (0.05 s into first or reverse, and only when the engine or road
  speed is above idle plus 800 rpm, or the throttle is at least 0.1, otherwise it stays open). While it slips, torque
  flows through the clutch spring (drivetrain spec, section 3). The induction (turbo) treats a shift as a throttle
  lift (`shifting` input).
- Neutral: the clutch opens and no engine torque reaches the wheels; the car coasts.
- Reverse: entered and left only by the auto-reverse logic of section 3; the shift buttons are ignored in it.
- Upshifting beyond the top gear and downshifting below neutral are refused silently (no event).
- The player's shift raises `EPlayerShift` in both modes with `automatic = false` for a manual or sport shift and
  `true` for the box's own shifts. The sound system reacts to the gear number changing, not to the flag.

## 6. The rewrite's design (not from the original)

- **Setting.** `transmission`: `automatic` (default, as in the original) or `manual`, through the layered settings
  (command line `--transmission`, `NFSMW_TRANSMISSION`, config file `transmission`, console `set transmission`) and
  the Gameplay options row with the original's labels. The quick-race prompt does not exist yet (there is no quick
  race). Drag events do not exist yet, so only the setting decides.
- **Keys.** The original's bindings come from controller config data that was not read. Defaults chosen here: keyboard
  Q shifts down and E shifts up (Shift and Ctrl keep working), pad left bumper down and right bumper up, steering
  wheel paddles as the same two actions through the config keys `paddle_up` and `paddle_down` (the codes of the
  gamepad buttons a wheel's paddles arrive as; the log prints the code of an unnamed button when it is pressed).
  Not tried on a wheel. In the free camera Q and E still move the camera down and up, and on the
  main menu Q quits; those contexts do not drive.
- **Gears by number.** The actions `gear_reverse`, `gear_neutral` and `gear_1` to `gear_7` (no default bindings; wheels
  and keys are bound by the player) ask for a gear id directly, skipping gears and leaving reverse, which the shift
  buttons cannot. Only a manual box listens. Reverse is refused above 2.5 m/s, as a real box refuses it. A gear key
  asks once when it goes down (latched like the shift presses). With the `h_shifter` setting the buttons are a
  selector that holds a gear: the gear whose button is held is asked for every step, no button is neutral (not
  while the console has the keyboard), and the automatic reverse is off, so braking to a stop never picks reverse and
  the pedals are never swapped. A direct request wins over a shift edge and over the automatic reverse in its step.
- **Clutch pedal (optional).** The `manual_clutch` setting (off by default; the original has no clutch pedal) makes the
  `clutch` action (0..1, default key Z; wheel pedals through `pedal:` or `pedal_inv:` sources) hold the clutch open
  while it is pressed past 10%: the engine free-revs, nothing drives the wheels, and the car coasts. Released, the
  gear's own engage rule of section 5 applies again. There is no partial slip, no stalling (section 4) and no grinding:
  shifts never need the pedal. It also works with the automatic box. With the setting off the action is ignored.
- **Edge presses are latched.** The original queues shift actions; the rewrite's fixed 60 Hz step runs fewer times
  than frames above 60 fps, so a press is held until a physics step consumes it, and one step takes at most one
  request (the last wins), as in the original.
- **Held on the limiter in a low gear** (the example car in first, full throttle, no shift), the speed overshoots the
  gear's red-line speed by about 4% within four seconds and then creeps up by a few percent per ten seconds. The
  tire port's wheel-spin reaction term keeps pushing while the limiter pins the wheels each tick; in the top gear and
  above 40 mph the car settles within 1% of its red-line speed. Not investigated against the original.
- **Known difference.** An auto-reverse gear request in the same step cancels a shift request (the original runs
  both, shift first); the order only matters in the step the car stops.

## How to check it

In the original (PC build), with Transmission set to Manual in the Gameplay options:

1. Hold full throttle in first: no upshift, the tachometer stops at the red line, the speed stops growing.
2. Shift down from first: neutral (the gear display shows N); the engine revs freely; shift up: first.
3. At about 120 km/h in fifth, shift down three times quickly: record the speed and rpm trace; compare the
   deceleration with section 4 (no refusal expected, engine at the red line meanwhile).
4. Stop in third with the brake: expect reverse once the car is nearly stopped, then first on the gas.
5. Press the shift button twice within one shift delay: both shifts should happen.
6. Switch to Automatic: the same button presses do the sport shift of section 7.5.

The wheel controls of section 6 have no original to compare with. They are checked by synthetic tests only; a real
wheel (axis codes, pedal direction, paddles, an H-shifter, a clutch pedal) has not been tried.
