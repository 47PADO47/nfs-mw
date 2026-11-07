# RIDE 6 bikes, round 12: the crash sound at speed, and the stick that reaches the end of the lean (clean-room spec)

Two reports of the players after the RIDE 6 bikes were played: "turning with bikes is still hard, better but still hard" and "I hear
constantly the sound of the bikes crashing with the ground when going faster". Continues [ride6-feel-round11.md](ride6-feel-round11.md) and
[ride6-lean-controller.md](ride6-lean-controller.md). Tags: **[M]** measured outside the model (a script that follows the controller's own
filters, see section 2.2), **[D]** data (the 288 imported manifests), **[C]** a choice of this project, **[R]** read in this repository's
code. **Nothing of this round was compiled or run** (the agent's rule: the lead builds once at the end); every test named below is
unverified, and the in-game check was not made.

## 1. The crash sound

### 1.1 The path [R]

A bike is a four-wheel body double in the game's simulation (`drive/sim/bike.rs`, [vehicle-two-wheel-integration.md](vehicle-two-wheel-integration.md)).
Everything the car sound hears of the player's car comes from three places (`drive/sound.rs`, `SoundFeed::after_step`):

1. `Impact::sounds`: the walls and the props against the double's box, and, from `CarSim::end_step`, one `ContactSource::Ground` contact for
   every entry of `Vehicle::body_contacts()` with a positive impulse (a contact of the box with the road: `BottomOut` or `HitGround`, and the
   scrape loops that persistent contacts start). The same list also feeds the ground damage hits (`contact_damage.rs`) and the ground sparks
   (`impact_effects.rs`).
2. `CarSim::take_sound_contacts` (pairs, and the non-player callbacks).
3. The telemetry: the wheels (`wheel_sounds`: slip, skid, load, compression) and `up_dot`, from which the effects mixer makes the road and
   skid loops and the **landings** (`blackbox-carsound/src/effects/landing.rs`, the `BottomOut` stitch samples).

### 1.2 Cause A: the last ground contact of a fall is read in every later step [R]

`Vehicle::body_contacts` is cleared by `Vehicle::begin_body_step` and refilled by `finish_body_step`. A bike's double never runs either while
its rider is on: `BikeRide::begin_body` moves the bike and copies the pose, and `finish_body` calls `finish_body_step` only for a **fallen**
bike (the double lies on its box). So the list holds the contacts of the **last step of the fall** until the next fall, with their
impulse, velocity and normal. After the bike is stood up (`lie.rs`) or placed again, `end_step` read the list as new contacts in every
physics step (60 per second): a ground impact for the sound, a damage hit for the events and sparks for the effects. The sound is a
continuous crash and scrape of the bike on the ground, whatever its speed, until the next fall rewrites the list.

Fix: `Vehicle::clear_body_contacts` (new, `vehicle/contact.rs`), called by `BikeRide::begin_body` at the start of every step, as the car's own
begin does. A fallen bike refills the list in the same step (`finish_body`), so the sound of a bike that is down is unchanged.

Test: `bike_ride_tests::the_ground_contacts_of_the_double_are_those_of_the_step_and_not_of_the_last_fall` and
`bike_sound_tests::a_bike_that_fell_and_stood_up_again_does_not_crash_on_the_ground_in_every_later_step`.

### 1.3 Cause B: the "tipped" landing latches for good on a bike [R]

The effects mixer has a "tipped" state (the spec, [engine-sound-effects.md](engine-sound-effects.md) section 8, "a heavily leaning car (up vector
dotted with world up below 0.8 and no wheel down) landing on any side counts as hard"). It is set when no wheel is on the ground and `up_dot < 0.8`,
and, as ported, it clears only when **exactly one wheel** is on the ground. After it is set every wheel pair that touches the ground again, with
no minimum time in the air, plays a **hard landing**. A bike has two wheels, each reported twice (front as wheels 0 and 1, rear as 2 and 3), so its
count of wheels on the ground is 0, 2 or 4 and never 1: once set, the state never clears.

It is set whenever a bike leans past 37 degrees (`up_dot < 0.8`; a bike under its rider leans to 50 or 60 degrees in every fast corner) and loses
both wheels for one sample (a crest, a seam, a bump at speed, or any fall). From then on every flicker of the wheels' contact with the road (a wheel
that leaves the ground for one step and touches again, which happens more often the faster the bike is, over the road's mesh) is a hard `BottomOut`
landing. That is the "crash with the ground, constantly, when going faster".

The police speech reads the same `up_dot` for "the car is on its side" (`speech_events.rs`); a leaned bike would trigger that too.

Fix:

- `CarInput::two_wheeler` (new, `blackbox-carsound/src/input.rs`; the host sets it from `CarSim::is_bike`, `drive/sound.rs`): `LandingFx` never sets the
  tipped state for it. A car is unchanged (the port of the original's latch stays as it is). A fallen bike is covered by the body double's ground
  contacts (cause A's list, correct while it lies).
- `CarSim::up_dot` returns 1 for a bike that is under its rider, and the real value for a fallen one (sound and police speech).

Test: `effects/tests.rs` (`a_car_that_was_tipped_lands_hard_each_time_a_pair_touches_again`, `a_two_wheeler_leaning_hard_over_a_crest_is_not_tipped_and_a_flickering_road_makes_no_landing`)
and `bike_sound_tests` (`a_bike_leaned_hard_at_speed_is_not_tipped_for_the_sound_and_for_the_police`).

### 1.4 The run at 60 to 90 m/s [test]

`bike_sound_tests::a_bike_at_60_to_90_ms_over_a_rough_road_makes_no_crash_scrape_or_landing_sound`: the example bike placed at 60, 75 and 90 m/s on a
road with 3 mm bumps in both directions (a synthetic `Ground`, since the game's test road has no mesh to ride on in a unit test), 5 s each, through
`CarSim::step` and `SoundFeed::after_step`: no contact in the impact, none in the body double's list, none in the ticks the feed hands to the sound,
the rider never falls, the sound is told "two wheeler" and "not tipped". The carsound-level behaviour (no landing for a flickering road after a
leaned crest) is the `effects/tests.rs` test above, on the mixer itself.

### 1.5 What was looked at and found clean [R]

- Walls: the probes are at the box's mid-height and floors (normal.y above 0.7) are not walls, so a leaned bike's box does not meet the road. A kerb
  or a wall beside the road is met for real (a leaned box reaches 0.75 m to the side of the tyre line at 45 degrees and 0.15 m above the road).
- `bottom`/`BottomScrape` come only from `body_contacts`; the skid loops read `lateral_speed = tan(slip angle) x speed`, which grows with speed (a bike's
  slip angle of a few degrees at 80 m/s is 3 to 5 m/s of "skid"): a tyre squeal that rises with speed, not a crash. Not changed; if it is
  too loud the cure is in the skid tuning, not in this round.

### 1.6 Not verified

Which of A and B the player heard was not established by a run (nothing was run): A needs one fall, B needs one both-wheels-off sample while
leaned past 37 degrees. Both are removed; if the sound is still there, a third source exists and the next step is to log the `CarSoundTick`
contacts and landings per step on a bike.

## 2. Turning

### 2.1 What is fixed by the data and cannot be "fixed"

- **The steady radius is the lean limit's**: `v^2 / (g tan(lean))` plus the geometry of the tyre ([ride6-feel-round11.md](ride6-feel-round11.md), section 2:
  KTM 450 93 m at 30 m/s, 166 m at 40 m/s, about 1 g). The lean limit is `atan` of the tyre's grip, which is the physical limit; going tighter
  needs a bigger limit (the `LateralGripTolerance` of the data is 1) and is not done.
- **The lean-in time** is set by three filters of the data (the stick filter K 1000 z 2.5, the target filter K 1000 z 2.25 stepped by `dt x time scale`, the lean
  spring K 1000 z 3.6): see 2.2.

### 2.2 The time to lean after a step of the stick [M]

A script (not in the repository) that follows `Lean::plan` (stick filter, time scale = speed curve x signed-stick curve, target filter, lean spring) with the lean
following the prescription exactly gives the **least** time for a stick that is thrown to its end (example bike's tables: speed 1 at 10 km/h to 0.65 at
200 km/h, stick 0.425 at -1, 1 at 0, 0.7 at +1; lean limit `atan 1.3` = 52 degrees):

| speed | stick | to 20 degrees | to 40 degrees | to 80 % of the limit |
|---|---|---|---|---|
| 10 m/s | +1 | 0.45 s | 0.80 s | 0.85 s |
| 10 m/s | -1 | 0.54 s | 0.98 s | 1.05 s |
| 20 m/s | +1 | 0.46 s | 0.82 s | 0.87 s |
| 20 m/s | -1 | 0.56 s | 1.01 s | 1.09 s |
| 40 m/s | +1 | 0.49 s | 0.87 s | 0.93 s |
| 40 m/s | -1 | 0.60 s | 1.10 s | 1.18 s |
| 20 m/s | +0.5 | 0.76 s | (not reached) | |

With the manifests of the bikes (the speed curve is 1 at 11 m/s to 0.333 at 92 m/s for 270 of the 288) the times to 40 degrees at 40 m/s are 0.99 s (right)
and 1.11 s (left) for the 207 bikes whose stick curve is 0.5 / 0.4, and 1.04 s and 1.37 s for the 61 whose curve is 0.45 / 0.28; at 60 m/s 1.1 to 1.6 s. So a bike
**takes about a second to lean 40 degrees, and a left turn 10 to 35 percent longer than a right one**, before the physics adds its own lag. That
is the data; a key that is tapped for a tenth of a second leaves the stick at 1/6 s per second (the data's fall rate) and the lean never builds.

`bike/turn_tests.rs` pins the open-loop times (the port of the filters) and the closed-loop ones (generous: 20 degrees under 1.2 s and 40 under 2.2 s at a stick of 0.9
from 10 to 30 m/s, the steady radius within 0.7 and 1.6 times the lean's own).

### 2.3 The stick reaches the end of the lean at 0.9 of its travel [C]

The pad's steering goes through the dead zone (15 percent, rescaled) and `Action::Steer`'s sensitivity as a car's does, and then **linearly** into the lean target.
The lean target is linear in the stick while the radius goes with `1 / tan(lean)`: at 30 m/s a stick of 0.85 asks for 0.85 of the lean limit (tan 0.78 rad
= 0.98 against 1.3 at the end of the example bike's 0.915 rad) and the radius is about a third wider; a corner of 100 m at 30 m/s needs the better part of the
stick. A thumb rarely holds a pad stick at exactly the end of its travel (the weight lever has the same problem and `REACH` 0.9 of
`input/weight_axis.rs`). So `choices::STEER_REACH` (0.9): `sim/bike.rs::command` divides the stick by it and holds it to 1. A key is 1.0 and is unchanged.
It is a choice of this game's input layer; 1.0 gives the reference's reading back. The animation still reads the stick as it is (`controls.steering`).

### 2.4 What was looked at and not changed [R]

- The input path: nothing for a bike sits between the action and `command`: no car steering table (the car's speed-sensitive range and rate limit are not applied),
  the data's rates (rise 25 per second, fall 6) in the library, the stick through `stick_share` (1 at any speed since round 11).
- The `time_scale_input` curve is read at the **signed** stick (section 2.2, the left-right difference). Whether the binary reads it at `|stick|` was not
  checked: the RIDE 6 executable was not open in the disassembler (only `CustomHud.asi`). If it is `|stick|`, a left turn comes in like a right one and the
  data's 0.28 at -1 is never used. Noted in `choices.rs`, no constant.
- The terrain term of the lean limit (a road's bank), the body double's wall probes and the collision help are in the game and not in the library's
  flat-road sweeps; their effect on a cambered, bumpy road was not measured.

### 2.5 Open

- A closed-loop measurement of the lean-in time on a real road in the game (the harness above is on flat ground with the example bike).
- The signed or absolute reading of the stick curve of the time scale.
- A speed-sensitive stick (a car's range narrows at speed; a bike's does not) if the players still find partial sticks too coarse at 40 m/s and above.
