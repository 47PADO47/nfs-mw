# RIDE 6 bikes, round 8: what makes them rideable (clean-room spec)

The user's report on the pushed state: "I lose grip all the time, turning is very wide, the bikes flip over almost instantly". This round
measured the model on every bike of the install, found the failures and fixed what the recovered facts or a documented choice could fix.
Continues [ride6-chassis-round7.md](ride6-chassis-round7.md) and [ride6-powertrain-round7.md](ride6-powertrain-round7.md). Sources: the RIDE 6
executable, build `5.6.1-1697004` (addresses in [provenance B](../provenance/ride6-physics-B.md), "Round 8"), the SDK dump's field names, the
resolved data of the 319 vehicles. Tags: **[B]** read from the binary, **[D]** data, **[M]** a choice of this project's model.

## 1. How it was measured

`libs/blackbox-modding/src/content/bike_ride_sweep.rs` (ignored tests; `RIDE6_CARS=<folder of the importer's cars> [RIDE6_FILTER=a,b]`, run with
`cargo test -p blackbox-modding --features host --lib bike_sweep -- --ignored --nocapture`) rides every manifest of a folder on flat ground:

- `bike_sweep`: a standing start at full throttle (25 s: time to 100 km/h, top speed, the gears and the rpm of every change, the peak slip),
  at half throttle (12 s) and with a throttle ramp; corners at 10, 20 and 30 m/s (the speed held by a throttle controller) with the stick at 0.3,
  0.5, 1.0 as a step and 1.0 as a ramp (lean against the limit, the radius against `v^2 / (g tan lean)`, the slip angles); braking from
  30 m/s with both brakes, the front alone and the rear alone (distance, pitch, falls, slip).
- `bike_stick_limit`: the largest stick (0.6 to 1.0, `RIDE6_STICKS`) a bike holds for 10 s at 8 to 40 m/s (`RIDE6_SPEEDS`) without falling.
- `bike_sweep_extra`: keyboard-like manoeuvres at the share of the stick the game gives (0.8): a slalom at 12, 20 and 30 m/s, a corner with the
  brake (`RIDE6_LEVER=1.0` for the pedal fully down, 0.5 by default), a slow turn at 4 m/s, a start with the stick held. `RIDE6_TCS_SCALE`
  scales the traction control's window for an experiment; the folder of manifests should carry `[bike.input]` (section 4).
- `bike_ride_real.rs` (`bike_trace_real`, `bike_launch_trace_real`): one bike traced second by second (`RIDE6_TRACE=speed,stick`,
  `RIDE6_SCENARIO=launch|trail`, `RIDE6_KNOBS=nogyro,nohang,stiff,tcs` to switch a term off and see what it did).

288 bikes (the whole install: supermoto, 600, 1000, naked, cruiser/retro, adventure, scooters, trail, 1980s and 1990s 250 and 500 cm3
two-strokes; no electric bike exists in the data).

| 288 bikes | pushed state | now |
|---|---|---|
| time to 100 km/h, mean / worst | 4.2 s / 21.7 s | 3.8 s / 5.6 s |
| bikes that never reach 100 km/h in 25 s | 18 | 0 |
| bikes stuck in a low gear at full throttle (end gear before the top and under 70 m/s) | 50 | 0 |
| half throttle for 12 s: speed, mean / worst | 23 m/s / 4 m/s (60 bikes under 12 m/s) | 36 m/s / 19 m/s |
| top speed | 23 to 372 km/h | 174 to 347 km/h |
| a stick of 1.0 held at 30 m/s for 8 s (a step): bikes that fall | 249 | 69 |
| a stick of 1.0 at 10 m/s | 105 | 5 |
| the largest stick every bike holds for 10 s at 8 to 40 m/s | not measured | 0.8 for 287 bikes (one holds 0.7 at 40 m/s) |
| braking from 30 m/s, both / front only / rear only | n.a. | 47 to 58 m / the same / 100 to 115 m; no fall, pitch under 0.12 rad |
| a lever of 1.0 for 2 s at 25 m/s in a corner (stick 0.8): bikes that fall | 275 (a lever of 0.5) | 5 (round 8, with the keyboard's rates) |
| a start at full throttle for 10 s with the stick at 0.8: bikes that fall | 263 | 17, all after 9 s (above 80 m/s) except one at 1.7 s |
| a slalom at 12, 20 and 30 m/s at 0.8: bikes that fall | 5 | 0 |

## 2. The fall lean limit and the grip the limits use [B]

**Two factors, not one.** The lean limit multiplies `atan(LateralGripPeak)` by the front wheel's `LateralGripTolerance` (wheel data `+0xEC`, 1 for
every bike), as round 6 said. The fall test's running limit `FallLeanLimit` moves towards `atan(LateralGripPeak) x wheel data +0xF0 x 57.2958`
degrees, and `+0xF0` is **`LateralGripLimit`** (1.2 on the KTM front, 1.1 on the R1, 1.0 to 1.3 elsewhere), not the tolerance. Round 6 used the
tolerance (1) for both, so the bike fell at the lean the stick asks for. Now the fall limit is 10 to 30 percent of an angle above it
(`ChassisState::follow_grip` takes `atan(grip) x front.lateral_grip_limit`). The rate (up 0.025, down 0.01 of the gap per step) and the start (65
degrees) are unchanged.

**The grip is the one at the rest load.** The handler's tick (`0x1450e0480`) writes `LateralGripPeak` (and `LongitudinalGripPeak`) every step as the
tyre model's peak coefficient at the front wheel's *rest* load (`RestLoad`, handler `+0xD40`) and camber 0, times the wheel's grip scale. The model
took the grip at the load of the moment (`front.load`), so the lean limit and the fall limit swung with every bounce of the fork (a quarter of the
load in a corner). `Bike::steer_handlebar` now uses the static front load (`front_share x mass x g`).

## 3. The sub-step: 1/480 s [M]

Round 7 divided the step into 1/240 s parts and measured that the bike held the stick at 0.8 to 30 m/s. With the stick at 1.0, or at 0.8 above 30
m/s, the loop still weaved: a yaw rate of 0 to 0.8 rad/s at 1.8 Hz, growing, the front wheel leaving the road once or twice a cycle (a step of
the front load to 0 and to 2.5 times), then a low side after 3 to 6 s. The loop has a handlebar filter of ratio 3 (a pole at 5 rad/s) and a front force
that is worked out one sub-step late. Multiplying the handlebar stiffness by 4 removed the weave (`RIDE6_KNOBS=stiff`), so did a shorter sub-step:

| sub-step | the 20-bike subset, stick 1.0 at 8 to 40 m/s: falls |
|---|---|
| 1/240 s | 15 bikes (a fall limit of the grip angle) |
| 1/480 s | 7 bikes |
| 1/960 s | 6 bikes |

1/480 s is the choice (`choices::MAX_SUBSTEP`, at most 32 sub-steps): with it 287 of the 288 bikes hold a stick of 0.8 for 10 s at every speed
from 8 to 40 m/s (the table of section 1; the 288th holds 0.7 at 40 m/s), and 19 of 20 hold it at 50, 60 and 70 m/s. It costs four times the 60 Hz work of the wheels, about 2 percent of
a core per bike. The game's own `MaxDeltaTime` and `MaxSubsteps` (project settings, not found) are still open. What stays: the bike at the lean limit
itself (stick 1.0) is marginal on a third of the bikes, and the front wheel still hops at the end of the lean range (section 7).

## 4. The rider's axes [D], the share of the stick [M], the brake in a lean [M]

**The axes move at the data's rates [D].** `InputData` of every bike (316 of 319 vehicles; the other two have none) gives the throttle a rise of 3
and a fall of 5 per second, the brake the same (3 and 5), the stick a deprecated rise of 25 and fall of 6 (`bUseDeprecatedRate` is not set, so
the constructor default, true, picks them over `InputRate` 10) and the weight channel 6. The constructor defaults are 10000 per second
(`0x1450a5f20`), "at once". The game fed the model the keyboard's 0 or 1 at once: a key slammed the throttle open and the brake shut. Now
`BikeSpec::input` (`InputRates`, `bike/input_axes.rs`; manifest `[bike.input]`, written by the importer's `physics/input_axes.rs`) moves the
throttle, both brake levers and the stick at those rates over each step (a stick thrown across goes through the middle at the fall rate). The
axes' `Linearity` (throttle 0.75, brake 0.65, one bike's throttle 2.0) was not read: the function that applies it was not found (the setup's
copies at `0x1450e7c30` and `0x1450e9160` only move the struct). A manifest without the table keeps the axes instant (the manifests imported
before this round need a re-import). Of itself it fixes little (a key's third of a second at the throttle or the brake changes no fall of the
sweep alone), but it is the reference's input and it tames a start and a stop.

**The share of the stick [M].** The keyboard gives -1, 0 or 1 and the pad's stick is read as it is. The reference's full stick asks for the
lean limit, the edge of the front tyre's grip, and the model's bike held there falls on a quarter of the bikes within seconds. `nfsmw`'s bike
input (`drive/sim/bike.rs`, `STICK_SHARE`) hands the lean controller 0.8 of the stick: the largest share 287 of the 288 bikes held at every speed
to 40 m/s. The consequence is a lean that tops out at 0.8 of the limit (about 40 degrees at 30 m/s on a sport bike, a radius of about 100 m at
108 km/h; at 10 m/s about 12 m). It is physical, not a tuning: the wide turn of the user's report at speed is the radius `v^2 / (g tan lean)` of a bike
that cannot lean further. The animation's `controls.steering` keeps the full stick.

**The brake in a lean [M]** (`choices::BRAKE_LEAN_RELIEF`, 0.8). The brake levers are multiplied by `1 - 0.8 (lean / limit)^2`. Why: with the
pedal fully down in a corner (a lever of 1.0, joined brakes: the rear lever follows the front one) the bikes decelerate at about 1 g; the front tyre's
force is spent on the brake (the front load is 2000 to 2500 N), the rear wheel unloads to 300 to 400 N and lifts or slides (slip ratio -0.15 to -0.18,
slip angle -0.1 to -0.18 rad, with the rear ABS holding it at the longitudinal peak and the brake reduction taking most of the rear brake), and the
bike falls inward as its speed falls (a lean of 0.7 held at 10 m/s needs more front force than the tyre has; the same fall as a stick of 1.0 at 8
m/s). What the reference does is not recovered: its ABS releases by the pure longitudinal slip, and its front brake reduction is 0 for every bike
read ([ride6-powertrain-round7.md](ride6-powertrain-round7.md), section 1: the front has no throttle tuning). Without the relief 15 of 20 bikes of the
subset fall; with 0.5, 8; with 0.8, 1 (and 5 of 288 with the keyboard's rates). A stop with the bike upright (lean 0) is untouched (47 to 58 m from 30
m/s). It should go if the reference's own relief is found. Engine braking was tested as a cause (friction torque at a tenth: no change).

## 5. The clutch [B + M]

[ride6-engine-drivetrain.md](ride6-engine-drivetrain.md) says: with the engagement at 1 the clutch is locked, the engine turns with the wheel;
otherwise it slips with the torque `e x T_max x sign(difference)`. The model locked a slipping clutch only when the two speeds crossed inside a
step and unlocked it whenever the torque it carried passed `e x T_max`, *also at e = 1*, and slipped with `sign` smoothed over 1 rad/s. With the
engine's inertia (0.015 to 0.04 kg m^2) against the wheel's the result was a limit cycle of three steps: one step of the whole torque, two of
nothing. The drive force was a third of what the engine gave (KTM in second gear: 1 m/s^2 at 10 000 rpm, 100 km/h after 8.1 s, "stuck in gear 2"),
the rear load swung between 500 and 2600 N (the chain term of the suspension coupling follows the drive torque), and on the MT-07 the wheel left the
road at every start. Now:

- an engagement of 1 (the lever and the slipper both 0) is a **rigid joint**: the engine takes the speed of the wheel at once, carries any torque
  (the engine's inertia is added to the wheel's), and never unlocks by torque. Slipper clutch and gear changes open it through the lever as before;
- a partly engaged clutch passes **the torque that would bring the two speeds together at the end of the step, clamped to its capacity**
  (`Powertrain::slipping_torque`): both sides are advanced by what acts on them (engine torque; the road's and the brake's torque on the rear wheel from
  the last step, the wheel's inertia) and by the clutch torque itself. A clutch that can hold the two together does, one that cannot passes its
  capacity. It has no state of its own to chatter. [M]: the reference's own clutch step is `0x14515ac20`; its crossing check is replaced by this
  closed form because the model's wheel is solved implicitly with the tyre and the engine is not.

Result, KTM 450 SMR: 100 km/h in 5.0 s (8.1; 4.2 s before the traction control of section 6), the gear changes at 10.4, 10.3, 10.3 and 10.1 krpm, the half-throttle start no longer stalls (22 m/s at
12 s in first gear: the engine at half throttle has 5 N m at 10 000 rpm because the data's friction torque is 30 N m at the limit [D] and the
generated up-shift point is 94 percent of the limit [B], so the bike stays in first until it gets there). MT-07: 100 km/h in 3.7 s (it never
reached 100 km/h: 26 m/s after 25 s). ZX-4RR: 4.9 s (21 m/s after 25 s, stuck in the first two gears).

**The feathering start.** `0x1450c0290` was read again: `R = MaxRPM x MaxRPMFactor + MaxTorqueRPM x PeakTorqueRPMFactor` (the KTM: 13 785 rpm, above
its rev limit), the controller ends only when the clutch has caught up (slip under 1 percent of `R`) with the wheel-side rpm over 80 percent of `R`
(11 028 rpm), which no first gear reaches, or when the input is 0 because the clutch torque asked for is more than the clutch carries, which it
never is below the peak torque; and `0x1450d7000` changes no gear while the start lasts. So in the reference too the start of a KTM lasts until
something else ends it (the throttle released, `0x1450d7cd0` when the input is exactly 0). The bound at `ClucthReleaseVelocityRange.Y` (50 km/h) stays
as the model's stand-in, now fed by a clutch that locks as soon as its capacity allows. Not found: what the reference does at speed to bring the
input to exactly 0.

## 6. The traction control [M]

`TCSSlipThresholdsDry` (0.09 to 0.2) is read in the model as the slip of the rear wheel's *vector*, `hypot(slip ratio, sin slip angle)` (a rear that
slides under power has lost its grip as much as one that spins), and the window is **half** the data's (`choices::TCS_WINDOW_SCALE`, 0.5). The
reference shapes the cut by the lean (`ThrottleModulationNormalizedLeanRange`, 1 to 1: no shaping) and the yaw rate
(`ThrottleModulationYawRange` 0.08 to 0.17; the quantity was not found), and its ECU element reduces the throttle by slip, wheelie angle and lean (no
bike of the install states a tuning). Without them, a start with the stick held spins the rear to 0.1 to 0.17 and lets it slide (rear slip angle 0.13 to
0.17 rad): a high side after 1.5 to 3 s on 14 of the 20 bikes of the subset, 263 of 288 in the fleet. With the half window: 20 of 288, at the price of 0.5 s
on the mean time to 100 km/h (3.4 s to 3.8 s over the 288). It is the one number of this round that is a fudge; it should go when the yaw shaping is found.

## 7. What stays open (measured)

> Round 9 ([ride6-feel-round9.md](ride6-feel-round9.md)) found the cause of the first item: the handlebar angle was projected along the steering axis instead of the
> ground normal, the wanted force lacked the division by the cosine of the lean, and the tyre load grew with the lean instead of falling.

- **The stick at 1.0** (the game scales a pad's end of travel to 0.8 too): 69 of 288 bikes fall within 8 s at 30 m/s (5 at 10 m/s). Findings: the bike
  held at the lean limit sits at the peak of its tyres (slip angles of 0.07 to 0.09 rad on both, the lean 0.02 rad above its target: the closure's
  offset), where the tyre's cornering stiffness is nil and the loop weaves at 1.5 Hz (a yaw rate of 0.1 to 0.6 rad/s) and the front wheel hops once or
  twice a cycle (a step of the front load to 0, and to 2.5 times). Handlebar stiffness x4, finer sub-steps (1/960) and the fall limit with margin
  each remove part of it; the front hop is not a contact-search failure (24 iterations instead of 6 change nothing). The model takes the tyre load as
  the suspension force over the cosine of the axis against the ground normal, which at 50 degrees of lean gives a total load of 2500 N for a 1800 N
  bike (a lateral friction component of the reaction is attributed to the normal); making the load cosine-free moves 3 of 20 bikes at 30 m/s to a
  stick of 1.0 and changes nothing else, so it is probably also wrong, but it is not the cause. At a stick of 1.0 and 8 m/s the controller's wanted
  front force climbs to 16 kN (the tyre's peak is 1.7 kN): the bike falls inward for want of roll authority, the steer sitting at 0.22 rad, the aim
  saturated at the geometric steer plus the peak slip, the front tyre's force the wrong way; the rotation about the leaned steering axis gives less
  slip on the ground as the lean grows (about its cosine), which would explain it. The reference's controller has the same formula; the difference
  is not found.
- A full-throttle start with the stick held for 10 s: 17 of 288 bikes fall, 16 of them after 9 s, above 80 m/s (290 km/h); one (a 2024 600) at 1.7 s.
- Five bikes still fall in the corner with the pedal down, 0.6 to 0.8 s after the lever is let go (5.6 to 5.8 s).
- `MaxDeltaTime`, `MaxSubsteps`: the project settings were not found; 1/480 s is the model's.
- The clutch is a closed-form stand-in (section 5); the feathering start's end is the release speed (section 5).
- In-game check: the build runs the scripted drives of bikes (`view-world --drive <ID> --at 985,4430 --mods-dir <mods with the bike manifests>`): the
  bike revs, moves, collides and stays upright against walls. The script clock does not follow the screenshots' (the sim runs ahead during the
  world's load: a script of 20 s was over before the first frame, one of 60 s had not started at the 16th second), the default spawn faces a bus, and
  the other spots tried face a wall or a toll booth, so a screenshot of a leaning bike was not obtained; the lean is checked in the sweeps.
