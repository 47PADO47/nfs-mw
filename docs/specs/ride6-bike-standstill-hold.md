# Bikes at a standstill on a slope: the hold, the reverse walk, the engine auto-stop delay

**A modification made exclusively for the NFS:MW rewrite.** The reference game's bike (RIDE 6) rolls on a slope when its rider lets go
of the brake and the throttle; this game's player did not want that. Tags: **[R]** read in this repository's code, **[C]** a choice of this
project, **[U]** not yet measured (the code was written and committed before any build; section 7 says what to run).

## 1. What the player reported

"fix the fact that if im still with a bike up hill it starts walking back. wrong it should stay still and if engine turn off is enabled turn
off engine after x time." The walking back is the rider's reverse walk
([ride6-rider-standstill.md](ride6-rider-standstill.md), section 4, [ride6-rider-styles.md](ride6-rider-styles.md), section 4): the rider's
animation starts when the bike's speed along itself is under -1 km/h (`rider/reverse.rs`, `RetroStart_Speed_Treshold`), and the sim's own reverse
(`libs/blackbox-vehicle/src/bike/reverse.rs`) pushes the bike back after the front brake has been held on a standing bike for `hold_time`.

## 2. Two causes [R]

1. **A bike with no throttle and no brake rolls back.** Gravity along a road of one degree is 0.17 m/s^2, the wheels' rolling resistance
   (`rolling` 0.02, 0.2 m/s^2) holds the bike below that and nothing above. At 10 degrees the bike creeps away at 1.7 m/s^2 and passes -0.28 m/s
   in a fifth of a second: the animation walks.
2. **A brake held with the bike standing is the reverse request.** A rider who stops uphill with the brake in his hand is, by the rule of
   `ReverseState::step`, asking for the reverse: after `hold_time` the brakes are let go and the bike is pushed back. That is the case of
   "if I'm still with a bike up hill": the rider holding the bike is walked back down the hill.

## 3. The standing hold (`hold.rs`)

Below a small speed, with no throttle, on a slope, the bike is **held**: the rider puts his feet down and stands on the road
(his standing clips already play, [ride6-rider-standstill.md](ride6-rider-standstill.md)). In the model, in each sub-step
(`Bike::step_hold`, called after the brake levers are settled in `respond_once`):

- both brake levers are held at `LEVER` (0.5) at least, which locks the wheels against the tyres' own static friction (the parking brake);
- a force cancels gravity along the road, `m g sin(slope)` along the slope at the centre of gravity (no torque), so nothing is left to roll the bike;
- the velocity along the road is damped with `exp(-dt / DAMPING_TIME)`, so a creep from the last step does not stay.

| constant | value | what and why [C] |
|---|---|---|
| `HOLD_SPEED` | 0.8 m/s | speed along the road under which a bike is taken to stand: a rider's walking pace; the walk's animation starts at 0.28 m/s backwards, so a bike creeping away from rest is held long before |
| `LEAVE_SPEED` | 1.2 m/s | a held bike is let go over this: the hysteresis, so a bike at the edge does not flutter; a bike that really rolls is not held, its brakes are the rider's |
| `THROTTLE` | 0.05 | the throttle (after the rider help) at which the hold lets go |
| `MIN_SLOPE` | 1 degree | under it the wheels' own rolling resistance holds the bike (above), so a flat road is unchanged by the hold |
| `MAX_SLOPE`, `SLOPE_FADE` | 30 degrees, 5 degrees | the steepest slope held in full, then the force fades to 0 at 35 degrees: `tan 30 = 0.58`, which a rider's feet and a locked tyre (mu 1.2) hold; nobody holds a bike on a ramp steeper than any road |
| `LEVER` | 0.5 | enough to lock a wheel that no engine drives, little enough to be a hold and not a stoppie |
| `DAMPING_TIME` | 0.15 s | quick enough to stop a creep within a few frames (9 at 60 Hz), slow enough that a bike stopped by the hold from 0.8 m/s does not jerk |

The hold is **off** in a fall, a burnout, with the rider off the bike, with both wheels off the road, while the rider pushes the bike back, at the
throttle and above `LEAVE_SPEED`. It needs no setting: with `bike_steering = reference` the steering is the original but a bike still does not roll
away from its parked rider (the same for a bike of another game that reuses the library: `Bike::held_on_slope` reads the state).

## 4. The reverse walk only on request (`reverse.rs`)

The deliberate reverse of the bikes is the **front brake pulled on the standing bike** (the game's input has no other reverse control for a bike,
`sim/bike/command.rs`: the gearbox is automatic). To tell a rider who holds the bike from one who asks to walk it back, on a slope
(`MIN_SLOPE`, from the wheels' normals) a lever that was **pulled while the bike still rolled on** (at `REQUEST_SPEED` 1 m/s or more) and has not been let go since
(`ReverseState::carried`) is not a request. The rider lets go and pulls the lever again on the standing bike: that is the reverse, with the
same hold time and force as before. On flat ground nothing changed (the held brake still asks, `on_flat_ground_the_held_brake_still_walks...`).
`ReverseState::step` is unchanged for a caller that does not know the slope; `step_on` takes it.

The engine auto-stop of the 2012 content mod uses the same idea for restarting from the brake (a fresh press, not a pedal held since the stop,
`chameleon-engine-state.md`).

## 5. The engine turns off after the player's delay

The setting exists: **2012 engine auto-stop** of the Gameplay settings (id `engine_auto_stop`, in the 2012 content mod, `engine_stop.rs`,
[chameleon-engine-state.md](../formats/chameleon-engine-state.md), [api-engine.md](../modding/api-engine.md)). While the player drives free of any
event and the vehicle stands still (speed under 0.5 m/s either way, no throttle; the brake does not count) the engine switches off, the revs fall to 0, the
engine sound stops, every lamp goes dark (the "ignition off lamp"), and the throttle, or a fresh press of the brake, starts it again after the cranking
(0.6 s) with the start-up sound. The switch is the **player's vehicle's**, a car or a bike: `Drive::engine` is read for both, the bike's throttle is
cut by `DriveInput::engine_stopped`. What it lacked for a bike on a slope was a speed under 0.5 m/s: the creep and the walk of section 2 reset its clock; with
the hold the bike stands, so the clock runs.

New: **a delay row.** `engine_auto_stop_delay` (menu row "Engine auto-stop delay", the same section of the settings as the toggle) picks the seconds the
vehicle must stand still first: **5 (default, the delay the rule always had), 10, 20, 30 or 60 s**. The values are design choices
(`DELAYS` in `engine_stop.rs`; the 2012 data holds no rule, [chameleon-engine-state.md](../formats/chameleon-engine-state.md)); the first is the default so
that a player who had the setting on keeps his five seconds. The row follows the settings pattern of the mod: the choice is kept by the game under
`[mod_values.nfsmw]` as `option_engine_auto_stop_delay`, the labels are messages of the mod's catalogue in every language it has (`lang/en`, `lang/it`:
`nfsmw2012-engine-auto-stop`, `nfsmw2012-engine-auto-stop-delay`, `nfsmw2012-engine-delay-5` to `-60`), and `EngineMachine::set_idle_seconds` applies a change at once
(a longer delay waits the rest of it, a shorter one counts what was already stood). The setting is offered when a 2012 install is set (it lives in
that mod); the brief's "a Gameplay setting with a config key, env and console" is the game's own settings pattern, not used because the existing
setting is a mod setting and a second, parallel setting would have two switches for one engine.

## 6. Tests

| test (`libs/blackbox-vehicle/src/bike/hold_tests.rs`) | what it says |
|---|---|
| `a_bike_with_no_throttle_stays_still_on_a_10_and_a_20_degree_slope_either_way` | 10 and 20 degrees, uphill and downhill, 6 s with no input: under 0.35 m moved, under 0.3 m/s, held |
| `a_bike_on_a_slope_does_not_roll_back_fast_enough_for_the_reverse_walk` | never under -0.28 m/s on 15 degrees |
| `a_flat_road_is_not_held...`, `a_slope_past_the_limit_is_not_held`, `a_bike_that_rolls_is_not_held`, `the_throttle_lets_the_hold_go` | the edges of the hold |
| `a_brake_held_through_the_stop_on_a_slope_does_not_walk_the_bike_back_but_a_fresh_pull_does` | the reverse of section 4 on 10 degrees |
| `on_flat_ground_the_held_brake_still_walks_the_bike_back`, `the_reverse_state_ignores_a_lever_carried...` | flat ground unchanged; the state machine |

`libs/chameleon-engine-state/src/tests.rs`: `the_delay_can_be_changed_while_the_car_stands_and_a_bad_value_is_ignored`; `mods/nfsmw-2012-content/src/engine_stop/tests.rs`:
`the_delay_setting_picks_how_long_a_standing_vehicle_waits` (every delay: no stop 0.3 s early, one stop 0.3 s late) and
`the_delay_values_are_plain_ids_the_first_is_the_old_five_seconds...`.

## 7. Measured [M] and not verified

`cargo test --release -p blackbox-vehicle --lib hold`: the example bike set down at rest on 10 and 20 degrees, uphill and downhill, no input for 6 s: it moves 0.04 to 0.28 m in the first
half second (the settling of the suspension on a bike placed unpitched) and then 0 m, speed under 0.01 m/s, held, no fall; never under -0.28 m/s (the walk's threshold) on 15 degrees after the
settling. The brake held through the stop on 10 degrees never asks for the reverse in 4 s; let go and pulled again it does, and the bike goes back 0.3 m or more; on flat ground the held brake
walks the bike back as before. **At 30 degrees the model's own fall test ends the bike (`Generic`) within half a second whatever the hold does**, so the steepest slope tested is 20 degrees; `MAX_SLOPE` stays 30 degrees
as the hold's own limit. Not looked at: in the game on a real hill, a bike parked against a slope **and** a wall, the surface (ice, gravel: a locked wheel holds less there).
`cargo test -p chameleon-engine-state` and `-p nfsmw-2012-content` (the delay setting) pass.
