# Air control (clean-room spec)

**This is a modification made exclusively for the NFS:MW rewrite. Neither NFS: Most Wanted (2005) nor RIDE 6 has it.** It lets the player steer
a car or a bike that has no wheel on the ground, as the open-world games of the genre do. It is **off by default** (setting `air_control`) and
acts on the **player's vehicle only**. Tags: **[R]** read in this repository's code, **[C]** a choice of this project (with its reason),
**[T]** covered by a test that passes (built and run at the build slot).

## 1. What the player asked

"let me control the car when in the air (behind a gameplay menu toggle). if enabled i should be able to control the car with joystick or mouse
and keyboard, like gta".

## 2. The setting

| Where | How |
|---|---|
| `config.toml` (and an alias's settings) | `air_control = true` |
| Environment | `NFSMW_AIR_CONTROL=on` (`on`, `off`, `true`, `false`, `1`, `0`) |
| Console | `set air_control on` or `off`, `set air_control` alone to flip it, `get air_control` (`air-control` also works) |
| Menu | Options > Gameplay, the **Air Control** row, in the **Experimental** section under Turn Signals and Cop Health Bars ([menu-sections.md](menu-sections.md), section 5) |

Default off. It is read at every physics step (`scenes/world/bike_assist.rs`, `set_air_control`), so a change reaches a vehicle on the road at
once. A computer driver's vehicle (traffic, police, racers) never gets it.

## 3. The controls

Three new input actions (`air_pitch`, `air_roll`, `air_yaw`, `crates/nfsmw/src/input`), each an axis from -1 to 1, rebindable like the others
(`bind air_pitch key:I`, the `[bindings]` table). **[C]** They share the keys that drive, so nothing new has to be learned and the car's own
keys keep their jobs (the stick reads as a stick only while no wheel is on the ground):

| | pitch (forward = nose down) | roll (right = right side down) | yaw (right = spin right) |
|---|---|---|---|
| Keyboard | W / S, Up / Down | D / A, Right / Left | E / Q |
| Pad | left stick Y (up is forward) | left stick X | right stick X |
| Mouse (captured, or the right button held) | up / down | | right / left |

The mouse is a stick whose deflection is its **speed**, like the helmet camera's look: 700 pixels a second hold it at its end
(`MOUSE_HEAD_LOOK_PIXELS`), up is the nose down as the stick forward is, and it falls back to nothing when the mouse stops, so a flick turns the
vehicle and does not hold it turning. The mouse sensitivity setting applies. A pad player who wants the camera stick back for something else
rebinds `air_yaw`. **[C]** Pitch is "stick forward = nose down" because that is how the genre's games fly their vehicles; the other sign is a
rebinding (`air_pitch = ['axis:LeftStickY:-1']`).

A drive script drives them too (`air_pitch=`, `air_roll=`, `air_yaw=`, [modding/drive-script.md](../modding/drive-script.md)). A network seat does
not send them (`DriveInput::from_vehicle` leaves them at rest).

## 4. What it does

Only with **no wheel on the ground** (a bike: both wheels, as `Bike::wheels_on_ground` counts; a car: `wheels_on_ground() == 0` and no corner of
the body box on the ground either, so a car on its roof or its belly is not flying), the vehicle awake, and a bike's rider on it. With a wheel on
the ground the controls do nothing at all ([T] `the_air_control_does_nothing_on_the_ground_or_when_it_is_off`, and the car's twin).

`blackbox_vehicle::air_control::AirControlSpec::torque` turns the three axes into one torque, in the body's own axes (x right, y up, z forward):

- each axis asks for an **angular speed** `rate` (rad/s) at full input; the angular acceleration that closes the gap is `gain x (rate x input - speed)`
  and is **held to `accel`** (rad/s^2) and multiplied by the body's inertia about that axis, so a heavy body and a light one turn alike and nothing
  can spin up faster than `accel` ([T] `the_angular_acceleration_is_bounded_whatever_the_gap_and_scales_with_the_inertia`);
- an input never pushes an axis that already turns faster than it asks for, the way it asks ([T]);
- with **no input** on an axis the same bounded torque damps its angular speed at `damping` per second: the body settles, it does not tumble for ever
  ([T] `a_full_input_reaches_its_rate_and_holds_it_without_overshoot` lets go and checks it settles);
- the signs: the stick forward is positive about the body's right axis (the nose goes down), the stick right is negative about the forward axis (the right
  side goes down), a spin to the right is positive about the up axis ([T]).

### Named constants (`AirControlSpec::CAR`, `AirControlSpec::BIKE`) [C]

| | pitch rate / accel | roll rate / accel | yaw rate / accel | gain | damping |
|---|---|---|---|---|---|
| Car | 1.1 rad/s / 3.5 rad/s^2 | 1.6 / 5.0 | 1.4 / 3.5 | 6 /s | 0.5 /s |
| Bike | 1.8 / 8.0 | 2.4 / 11.0 | 1.8 / 7.0 | 8 /s | 0.7 /s |

Why: a full stick reaches its rate in 0.3 s (car pitch) to 0.2 s (bike roll), which is quick enough to feel like a control and slow enough not to feel
like a rocket; the car's rates give about a quarter turn in 1.4 s at full input, which is the time a 2 s jump allows for a flip the player chose; a bike
is lighter and quicker in all three axes but the same shape of curve. The damping is weak on purpose (a turn is kept for a second or two when the stick
is let go, as in the genre); the gain is 6 to 8 per second, a first-order lag of 0.12 to 0.17 s that cannot overshoot at 60 Hz ([T] the integration
test). The bounds are per axis and in angular acceleration, so they hold whatever the vehicle's mass.

### Attitude limits (bikes) [C]

A bike falls over beyond its airborne lean (70 degrees) and pitch (65 degrees) limits (`FallSpec`, `FallExtra`). The player's push fades out as the
bike nears **0.7 of them** (whole below half of that, gone at it; 0.85 and a band of 0.3 let the momentum carry a bike past the 65-degree limit and fall, measured), and an axis the player is holding but that has faded to nothing is steered to a
standstill, so the bike stops at the edge and does not drift on past it ([T] `a_held_stick_stops_short_of_the_fall_limits`,
`a_body_turning_into_an_attitude_limit_is_braked_and_not_left_to_drift_on`). A car has no limits: landing on the roof is the player's doing.

## 5. Where it acts

- **A bike**: `Bike::set_air_control(Some(axes))` every step (`sim/bike.rs`, `BikeRide::respond`), applied in the bike's own step with the other
  torques of the rider and the air (`bike/air.rs`, `Bike::apply_air_attitude`). It composes with the arcade steering's levelling in the air
  ([ride6-bike-jumps.md](ride6-bike-jumps.md), section 3): the levelling gives way on an axis by the share the player pushes it, so a stick held fully
  forward is not fought by the help, and a stick let go is levelled again.
- **A car**: `CarSim::apply_air_control` (`sim/step/air.rs`) puts the torque on the body after the driving response of the step, once; like every force
  of the response it moves the body in the next step.
- **Not the AI, not the other seats** (section 3), not a bike's fallen rider, not a vehicle the host has frozen or put to sleep.

## 6. Tests

`libs/blackbox-vehicle`: `air_control/tests.rs` (signs, bounds, inertia scaling, rate and settling, half input, the attitude limits, a body turned over,
bad input) and `bike/air_tests.rs` (a bike in the air pitches, rolls and spins by the stick, does nothing on the ground or with the control off, a held
stick stops short of the fall limits). `crates/nfsmw`: `input/air_input_tests.rs` (keys, sticks, mouse and its gate, the action names),
`scenes/world/drive/sim/tests/air_control.rs` (a car and a bike through the sim, the player and the setting, a car on the road),
`scenes/world/bike_assist.rs`, `settings` (every layer), `devtools/console/settings_cmd_tests.rs`, `frontend/options_sections_tests.rs`.

## 7. Switching it off

`air_control = false` (the default). The reference behaviour of a vehicle in the air is untouched when it is off: the torque is not computed and the
only difference in the code path is a `None` ([T] `the_air_control_is_the_players_and_off_by_default` compares a run with the setting off and one with it on for a computer driver, bit for bit).

## 8. Not verified, open

- The tests pass; the feel of the constants is not play-tested.
- The constants are a first choice; the play test will say whether the car's pitch wants more `accel`.
- No force-feedback or haptic cue; no HUD hint that the control is live.
- The mouse reads only while the cursor is captured or the right button is held (the gate of every mouse look); a game that never captures the
  cursor while driving needs the right button.
