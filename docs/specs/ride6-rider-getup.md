# RIDE 6, the rider's get-up and the fallen bike: who waits for whom (clean-room spec)

A report from play: "fall and get-up work, but by the time the getting-up animation is playing the bike is already reset and ready to
go." This page says why, what the reference does, and what this project does now. It continues
[ride6-rider-fall-animation](ride6-rider-fall-animation.md) (the clips and the state machine) and [ride6-rider-fall](ride6-rider-fall.md)
(the release). Tags: **[B]** read from the binary, **[D]** from the data assets, **[S]** from the class dump, **[M]** a choice of this
project's model.

## 1. What is recovered of the reference

The rider's component has phases **riding, fallen, before get-up, get-up, walk** (and raising the bike), each with a duration
(`BeforeGetUpDuration`, `GetUpDuration`, `RaiseBikeDuration`, `TimeToReachPosition`) and a character-physics configuration name [S, B]. The
bike is raised at the *end* of that sequence: the rider falls, gets up, walks to the bike and lifts it; the bike does not stand by
itself. The durations were recovered in round 12 (the component's constructor defaults, [fall-animation page](ride6-rider-fall-animation.md) section 8): `BeforeGetUpDuration` 1.0 s (plus a 1.0 s delay), `GetUpDuration` 4.0 s, `RaiseBikeDuration` 1.0 s,
`TimeToReachPosition` 0.1 s, `BikeRecoveryTimerDuration` 14 s. The component holds no animation: the standing and walking clips of a fallen rider **do not exist in the install** (searched there; the walk belongs to the rider movement component, whose clip names a blueprint not found sets), so the phases after the get-up clip cannot be played.
What the data does give is the get-up clips and the rule that starts them (a pelvis under 0.5 m/s, within 1 of the ground, lying in one of three bands): the get-up lengths are the clips' own (1.0 s on the back, 1.4 s face down, 2.53 s on the side) [D].

What follows from it, and is all this project relies on: **the bike is stood up after the rider has got up, never before.**

## 2. Why the bike was ready before the rider (root cause)

Two things ran on two clocks:

1. The sim's bike (`drive/sim/bike.rs`, `lie`) stood a fallen bike up on its own: once it had lain **2 s** (`GET_UP_AFTER`) and slid slower than
   **1.5 m/s** (`GET_UP_SPEED`) it was placed upright where it lay (`place_on_ground`), which gives the rider back to it (`restore_rider`) and
   gives the controls back (a fallen bike ignores the throttle and the stick, an upright one does not).
2. The rider (`drive/rider/fall.rs`) only learned of it afterwards: the bike was up, the host called `Fall::stop` every step, and that
   decided whether the rider might return: at once, when the get-up clip had played (at most 8 s), or, if he had not yet begun to get
   up, after 2 s. The wait existed, but it only delayed the *rider's* return. The **bike was already upright and under the player's
   hands**; the rider lay beside it, getting up, for up to 8 s.

The usual fall puts the rider on the ground sliding for several seconds, so the bike (which slows quickly on its box) satisfied its two
conditions *before* the rider's get-up had even begun, never mind ended. There was no second reset path: the car's reset-after-crash
timers do not touch a bike's fall (`reset_collision`, the falling-off-the-map watch in `drive/fall.rs` and the reset button are separate,
see section 5), and the fall-end is not an event on speed: the one stand-up is `lie`.

## 3. What this project does now

**The bike waits for its rider** (`sim/bike/lie.rs`, `sim/bike_fall.rs`):

- `lie` keeps the old two conditions (2 s down, under 1.5 m/s) but they now only make the bike *wait* (`waiting`, read through
  `CarSim::bike_waits_for_rider`). It is stood up only when the host has said the rider is ready (`CarSim::set_rider_ready`, said every
  step; true by default, so a bike with no rider who gets up, and every test of the dynamics, behave as before).
- While it waits the bike stays fallen: the rider is off it, the throttle and the stick are not taken, the rider's mass is out of it.

**The rider says when** (`drive/rider/fall.rs`, `fall_anim.rs`): each physics step `Fall::bike_may_stand_up(waiting, dt)`
runs the bike's wait as a clock (it counts only while the bike waits) and answers through `Tumble::may_return`:

| The rider | The bike may be stood up |
|---|---|
| is not down (never fell, or already back) | yes |
| has no fall animation (falls limp) | yes (nothing to wait for) |
| is in the get-up | when the get-up has played to its end (`Anim::done`: the clip's own length, held at its last frame), or after `GETUP_LIMIT` (8 s) of get-up |
| has not begun to get up | after the bike has waited `TUMBLE_LIMIT` (8 s) |

The step then goes: the bike is stood up in the next physics step, `Fall::stop` sees the bike back (`off` false) and the rider blends back
onto it. The get-up is not cut, the bike is not under the player's hands before it has ended. The blend takes the way he has to go at
`RETURN_SPEED` (3 m/s), at least `RETURN_TIME` (0.25 s) and at most `RETURN_LONGEST` (2 s): the bike is stood up where it lay, which is
metres from where he knelt, and the earlier 0.25 s blend carried him there in a fifth of a second (seen in the offscreen runs of
[the fall page](ride6-rider-fall-animation.md), section 7: 3.5 m, now about 1.0 s). The bike is controllable from the moment it is up.

**A reset by hand skips the wait and the walk.** The reset button (`DriveInput::reset`, `drive/simulation.rs`) and every other placement of the bike
(`place_moving`, the fall off the map, a new spawn) put the bike upright at once and clear `waiting` (`BikeRide::place`); the host sees
`off` false and `Fall::stop` returns the rider to the bike without waiting for the clip, from wherever he is, in `RETURN_TIME`. So pressing R while the
rider is getting up is allowed to skip (the player asked for it).

## 4. Choices

| Choice | Value | Why |
|---|---|---|
| Wait for a rider who has not begun to get up | `TUMBLE_LIMIT` = 8 s (the bike waits, lying still) | was 2 s after the bike was up (the old grace); the wait must outlast a slide, and only a rider stuck between the three bands (never getting still) may be put back mid-tumble. A modelling choice: the raise-bike phases' durations were not recovered |
| Longest get-up | `GETUP_LIMIT` = 8 s from the start of the get-up | the get-up ends with its clip (the source); this only keeps a clip that never ends from holding the bike for good. Unchanged from the old `RETURN_LIMIT` |
| Bike lies before it waits | 2 s and 1.5 m/s | unchanged ([ride6-rider-fall](ride6-rider-fall.md)) |
| Return blend | his way at 3 m/s, 0.25 to 2 s | a modelling choice: the walk clips and phases were not recovered; was a flat 0.25 s (a pop of metres). A reset by hand keeps 0.25 s |

Not modelled (as before, and now for a stated reason): the standing, the walk to the bike and the raising of it. The reference spends 4 s in the get-up phase, a walk and 1 s on the raise; here the
bike stands up where it lies the moment the get-up clip ends and the rider blends onto it. The 4.0 s and 1.0 s are not wired in: with no standing or walking pose they would hold him on hands and knees for up to 3 s more and then slide him to the seat,
which is an invented animation. When a standing pose and a walk exist the timeline is `BeforeGetUp` 1.0, `GetUp` 4.0, walk, `RaiseBike` 1.0 with the bike down until the raise. The reset button keeps skipping all of it.

## 5. Other ways a fallen bike could be reset, and what was checked

- **Reset button** (`input.reset`, subject to `reset_collision_active`): allowed, skips the wait (above).
- **Fell off the map / state not finite** (`check_road`): places the bike again; the same path as the button.
- **Fall-end on speed**: none; `GET_UP_SPEED` only gates `lie`.
- **Car "reset after crash"**: cars only; a bike's fall does not reach it.

## 6. Verified by tests

`nfsmw` (`drive/rider/fall_tests.rs`): the bike is held while the get-up plays and may stand up after about the clip's length (the clip
there is 1 s); with the bike waiting from 1 s, it is not let up before the get-up has played (the reported fault); a bike that does not
wait does not start the clock; a rider who never begins to get up holds the waiting bike for `TUMBLE_LIMIT` and not longer; a limp rider
or one who is not down never holds it; `stop` mid get-up (the reset button) takes the rider back at once; the return takes longer for a rider 4.5 m from the bike than for one next to it, within the limits, and a reset by hand does not. `drive/sim/bike_ride_tests.rs`
and `bike_fall_tests.rs`: a bike that has lain still waits and stays fallen for a long time while the rider is not ready, ignores full
throttle and stick, and is stood up (rider back on it) at the next step once he is ready; placing the bike by hand does not wait.
