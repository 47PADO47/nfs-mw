# RIDE 6 bikes: the turn radius over the speed, round 3 (this game's own tuning)

**This is a modification made exclusively for the NFS:MW rewrite. It is not the behaviour of RIDE 6.** It continues
[ride6-arcade-steering.md](ride6-arcade-steering.md) (sections 10 and 11: the lean-in slowed and the speed authority) and replaces the
speed authority's assist with the **turn law**, `blackbox_vehicle::bike::TurnLaw` (`libs/blackbox-vehicle/src/bike/turn_law.rs`), one
field of `ArcadeSteering` (`turn`). `bike_steering = reference` (`ArcadeSteering::REFERENCE`, `turn: None`) still gives the original
behaviour exactly. Tags: **[R]** read in this repository's code, **[C]** a choice of this project, **[M]** measured in the model,
**[U]** not measured.

## 1. What the player reported

Playtest, round 3: "still: less steering radius when slow, more when fast (with bike)". Round 2 (sections 10 and 11 of the arcade
steering spec: `authority = [1.5, 0.6]` over 5 to 70 m/s, radius 1.8 to 1.5 m at 20 km/h and 181 to 233 m at 200 km/h on the example
bike) did not satisfy the player: at a low speed (walking pace to 40 km/h) the bike still cannot turn tight, at a high speed it still
turns too tight. The brief gave the circle wanted for a full stick: **15 km/h at most 3 m, 30 km/h at most 6 m, 50 km/h about 15 m,
80 km/h about 40 m, 120 km/h about 90 m, 180 km/h about 220 m or wider**, a quick lean-in at a low speed, falls at 0.

## 2. Why round 2 did not give it [R]

A full stick held for 8 s ended, in round 2, on a circle at or inside the low-speed numbers of the brief on the harness
(`ride6-arcade-steering.md`, section 11: 1.5 m at 20 km/h on the example bike, 4 m on the median imported bike). So the steady circle was
not what the player felt at a low speed; the code gives four reasons that act on the first seconds of a turn and on the bike that is
leaned less:

1. **The assist follows the controller's target lean, which is slow.** `arcade_forces.rs` took `plan.target / limit`; the target is the stick
   through its filter, the target filter, the roll spring's reference and the time scale of round 2 (`lean_rate` 1.38 instead of 3,
   `ride6-arcade-steering.md`, section 10). The yaw rate reached 80 percent of the other side after 0.60 to 0.75 s on the imported bikes
   (round 2, section 10), 0.38 to 0.50 s before. A slow bend through a chicane at 30 km/h covers 5 to 6 m in that time.
2. **The assist faded in over 2 to 8 m/s** (`assist_speed`, 7 to 29 km/h): at 15 km/h (4.2 m/s) the bike had 37 percent of its assist.
3. **The lean limit is blended in with the speed**: 1 degree at a standstill, whole at 30 km/h (`LeanSpec::stopped_limit`,
   `stopped_speed`). A slow bike leans 18 degrees at 15 km/h, so the lean gives it 0.33 g and the turn rests on the assist.
4. **The circle depended on the bike**: the assist was a fixed multiple of the lean, so the radius was whatever the lean limit, the
   grip and the mass made of it. A bike with a lower lean limit turned wider, and the speed authority could only scale all of it.

At a high speed the same law gave `1.1 g x 0.6` of assist above 252 km/h and `x 0.77` at 120 km/h: a circle of 76 m at 120 km/h and
233 m at 200 km/h against the 90 m and 220 m or more asked, and 30 m at 80 km/h against 40.

## 3. The turn law

The assist is a side force at the centre of gravity (`arcade_forces.rs`, no roll torque: the roll is prescribed by the lean controller).
Under the law it is **the part of the lateral acceleration that the lean does not give**:

```text
share   = target_share + (stick_share - target_share) x (1 - ramp(speed, quick))     the stick's share at a low speed, the target's at a high one
wanted  = share x speed^2 / radius(speed)                                            the acceleration of the circle the table names
expect  = g x tan(target lean)                                                       what the lean the controller aims at gives through the tyres
assist  = (wanted - expect) x ramp(speed, fade)   [0 against the stick, at most max_assist g]
```

In a steady turn the lean is its target, so lean and assist add up to `wanted`: the circle is the table's, whatever the bike. [R: the
lean part was measured at `g tan(lean)` to 2 percent in round 1, `ride6-arcade-steering.md`, section 3.1: 0.74 g at 36.6 degrees
plus 0.96 g of assist gave the 1.68 g measured.] The stability help's wanted yaw rate (`(g tan(lean) + assist) / speed`,
`lean_help.rs`) is `wanted / speed` too, so the yaw rate and the force agree. Because `expect` is the **target** lean and not the bike's
own, the assist does not change if the bike's roll lags: in a transient the total is `wanted` plus the (small) shortfall of the lean.

**The table** (`turn_law::RADIUS`, km/h to metres, joined by straight lines in `ln(speed)` and `ln(radius)`: a power law between two
points, never falling with the speed, no step; past the last point the last segment's exponent goes on; below the first point its radius
is held):

| km/h | radius m | asked | exponent to the next | `v^2 / r` in g |
|---|---|---|---|---|
| 6 | 1.8 | (walking pace: the handlebar's own lock turns the bike) | 0.06 | 0.16 |
| 15 | 1.9 | at most 3 | 1.21 | 0.93 |
| 30 | 4.4 | at most 6 | 2.27 | 1.61 |
| 50 | 14 | about 15 | 2.23 | 1.40 |
| 80 | 40 | about 40 | 2.03 | 1.26 |
| 120 | 91 | about 90 | 2.39 | 1.24 |
| 180 | 240 | about 220 or wider | 2.39 (beyond) | 1.06 |

[C] Why these numbers: the low-speed radii are put under the asked limits (2.2 against 3, 4.5 against 6) so that "clearly tighter" holds
with a margin and the 20 and 25 km/h in between (3.0 and 3.7 m) are tight too. From 50 km/h up they are the asked ones. The
acceleration the table asks never passes 1.57 g (at 30 km/h), under the 1.65 to 1.68 g of rounds 1 and 2, so no bike's tyres, fall limit or
skid cut are asked for more than they gave. 1.1 to 1.3 g from 50 km/h up is what makes the radius `0.08 v^2` (a radius of 15, 40, 90 and
225 m at 14, 22, 33 and 50 m/s).

**What shapes the first seconds** (the other fields of `TurnLaw`; defaults `TurnLaw::GAME`):

| field | value | what and why [C] |
|---|---|---|
| `fade` | 0.3 to 1.5 m/s | the assist comes in over walking pace (1 to 5 km/h), not 7 to 29 km/h; below it the handlebar's direct mapping (`blend_range`, 2 to 4 m/s) is what turns the bike |
| `quick` | 4 to 12 m/s (14 to 43 km/h) | the share the assist follows goes from the **stick's** (the hand's filter, 0.15 s) at 14 km/h and under to the lean target's at 43 km/h and over: a slow turn needs little lean, so the path does not wait for it; a fast flip keeps the target's pace (a quick path with a slow lean at speed is what the falls of the sweeps were about) |
| `trim_gain`, `trim_reach` | 3 per second, 4 m/s^2 | the trim of section 4 |
| `max_assist` | 1.7 g | the assist never gives more than round 2's most (1.1 g x 1.5): the stick's first instant can ask for `wanted - expect` of 2 g and more while the lean is still on the other side |
| `slow_lean_rate` | 2.2 at 4 m/s and under, 1 at 12 m/s and over | the time scale of the controller's target filter, the one stage of the four that carries a lever of its own (`lean_rate`): 1.38 x 2.2 = 3.0, round 1's value, at a low speed, where the lean needed is small and the slowing of round 2 only hurt |

Not touched: `lean_limit`, `roll_spring`, `stick_response`, `handlebar_response`, the lean help, `time_scale_speed_floor`: above 43 km/h the
lean-in is as round 2 left it, and so is the flip time of the lean (0.9 to 1.0 s to 95 percent).

## 4. Before and after [M]

`bike_turn_table` on 25 of the 293 installed bikes (every 12th of the folder `config\mods\ride6-bikes\cars` of the game's profile), a full stick held for 10 s after a second of
straight running, the speed held by the throttle and, over the speed, both brakes; the radius is the mean speed over the mean yaw rate of the last 3 s, the time is the
curvature (yaw rate over speed) of the path reaching 80 percent of its steady value. "Round 2" is the same run with `RIDE6_ARCADE=round2` (the authority of section 11 of the arcade
steering spec, no turn law). Medians, then the range over the bikes.

| km/h asked | asked | table | **round 2** radius m | round 2 t80 s | **new** radius m (range) | new t80 s | falls |
|---|---|---|---|---|---|---|---|
| 15 | at most 3 | 1.9 | 4.0 | 1.02 | **2.5** (2.4 to 3.1) | 0.75 | 0 |
| 30 | at most 6 | 4.4 | 4.4 | 0.58 | **4.9** (4.7 to 5.0) | 0.42 | 0 |
| 50 | about 15 | 14 | 10.8 | 0.57 | **14.4** (13.6 to 14.6) | 0.45 | 0 |
| 80 | about 40 | 40 | 30.1 | 0.60 | **39.7** (37.8 to 41.1) | 0.48 | 0 |
| 120 | about 90 | 91 | 75.4 | 0.62 | **88.8** (85.5 to 92.0) | 0.58 | 0 |
| 180 | 220 or wider | 240 | 185 | 0.62 | **228.5** (207.4 to 236.2) | 0.77 | 0 |

(The mean speed of the runs was 19, 32, 51, 80, 119 and 176 km/h; the radius is a little tighter than the table's where the bike was faster than asked. 0 falls in the turn and
0 in a flip of the stick every 0.8 s for 6 s, at every speed, on all 25 bikes. One bike of the 25 turns 3.1 m at 15 km/h, the median is 2.5 m. The logs: `C:
6work\still-turn-table.log`.)

The slow turns are tighter or as tight and the path turns sooner (t80 0.42 s at 30 km/h against 0.58, 0.75 s at 15 km/h against 1.02); the fast turns are 20 to 25 percent wider than round 2, as asked.
On the example bike of `arcade_tests.rs` (3 bikes: the example, a heavy and a light one) the closed loop gives within -25 to +35 percent of the table at 50, 80, 120 km/h and 0.9 of it or more at 180.

**The trim.** The first version of the law (the assist alone, section 3) came out 10 to 55 percent wider than the table in the closed loop, more at a low speed: the bike's
tyres resist a side force at the centre of gravity until they have turned the bike (the heading lags the velocity), by a margin that depends on the bike and the speed. An integral
term (`TurnLaw::trim_step`: `trim_gain` 3 per second on the shortfall of `speed x yaw rate` about the **road normal** from the circle's acceleration, the shortfall counted at most
`trim_reach` 4 m/s^2 per step so a flip does not wind it up, the trim held so that the assist stays between 0 and `max_assist`) makes up the rest; it dies away with the stick and is zero
under walking pace. Measuring the yaw rate about the leaned bike's own up made the circle 20 percent too tight (`cos` of the lean): the road normal is the right axis.

## 5. Tests

`blackbox-vehicle` (`turn_law_tests.rs`, `arcade_tests.rs`):

| test | what it says |
|---|---|
| `the_table_is_ascending...` | the table's numbers (at most 3 m at 15 km/h, at most 6 m at 30, 15, 40, 90, at least 220 m at 180) and ascending in speed and radius |
| `the_radius_never_falls_with_the_speed_and_has_no_step` | the **radius table is monotonic**: 0 to 120 m/s in steps of 0.05 m/s, never falls, never steps more than 2 percent |
| `the_radius_passes_through_the_points...` | the points, the exponent beyond the last, the held first radius, an empty table |
| `a_steady_lean_leaves_exactly_the_rest...` | lean plus assist is the circle's acceleration at each of the six speeds |
| `the_assist_follows_the_stick_and_never_works_against_it` | symmetry, proportion, nothing against the stick, nothing without it, a flip's assist held to the cap |
| `the_assist_is_faded_out_below_walking_pace`, `a_low_speed_path_follows_the_stick...`, `the_lean_in_is_quicker_at_a_low_speed...` | the three shaping fields |
| `the_circles_acceleration_stays_under...` | no point of the table asks 1.7 g or more |
| `the_turning_circle_follows_the_turn_laws_table_on_three_bikes_and_never_falls` | closed loop, the example bike, a heavy one and a light one: at most 3 m at 15 km/h, at most 6 m at 30, within -25 to +35 percent of the table at 50, 80 and 120, at least 95 percent of it at 180, growing with the speed, no fall |
| `the_slow_turn_is_tighter_than_the_originals...` | the original steering turns more than 1.2 times wider at 15 and 30 km/h |
| `a_full_stick_turns_the_path_quickly_at_a_low_speed` | the yaw rate is at 80 percent within 0.8 s at 15 and 30 km/h |
| `the_lean_in_is_quicker_at_a_low_speed_than_without_the_turn_laws_rate` | the lean's 0 to 90 percent at 3 m/s is shorter with `slow_lean_rate` |
| `a_flip_of_the_stick_at_a_low_speed_does_not_fall` | 3, 4.2, 6, 8.3 and 11 m/s |
| the round 2 tests of `arcade_tests.rs` | kept: the reference is neutral, half stick turns half, left as right, the lean-in is 64 percent of the way to the original at 60 km/h and does not slow with the speed, the flip is quicker than the original; the 1.4 to 1.95 g and 1.15 to 1.85 g bands moved to 1.1 to 1.5 g and 1.0 to 1.4 g for the new law (1.27 g at 72 km/h, 1.2 g at 144 km/h) |

The audit on the imported bikes is `bike_turn_table` (`libs/blackbox-modding/src/content/bike_ride_turn_table.rs`, an ignored test):

```text
RIDE6_CARS=<folder> RIDE6_EVERY=12 [RIDE6_ARCADE=reference|round2|previous|turn=0|slowlean=1|maxassist=1.4] \
  cargo test --release -p blackbox-modding --features host --lib bike_turn_table -- --ignored --nocapture
```

It prints per speed (default 15, 30, 50, 80, 120, 180 km/h) the table's radius, the median and the range of the measured radius (the speed over the
yaw rate of the last 3 s of a held full stick after the lean has built up), the median seconds to 80 percent of the yaw rate after the stick is stepped
on, and the falls of a held turn and of a flip every 0.8 s. The older `bike_steer_audit` and `bike_steer_sweep` (`bike_ride_steer.rs`) run with the
new law too; `RIDE6_ARCADE=round2` gives them round 2's steering.

## 6. How to switch it off

- `bike_steering = reference` (config, `NFSMW_BIKE_STEERING=reference`, console `set bike_steering reference`): `turn: None`, everything of the original.
- In the library: `ArcadeSteering { turn: None, ..ArcadeSteering::default() }` gives round 2's assist (`cornering_assist`, `authority`, `assist_speed`);
  `TurnLaw { slow_lean_rate: 1.0, ..TurnLaw::GAME }` keeps the law with round 2's lean-in; a different `radius` table is the one tuning knob.

## 7. Falls

Round 2's sweep had 0 falls on every manoeuvre but two on one scooter at 70 and 80 m/s (`ride6-arcade-steering.md`, section 5). The law asks
less lateral acceleration than round 2 at every speed (never over 1.57 g against 1.65 to 1.68), so the tyres and the fall limit are asked for less;
the new thing is the quick path at a low speed (the stick's share) and the cap of 1.7 g, which a flip at under 43 km/h meets. The test
`a_flip_of_the_stick_at_a_low_speed_does_not_fall` and the `bike_turn_table` falls column are the checks; both show 0 falls (section 4).

## 8. What is not verified

- The `nfsmw` bike tests pass (166); the one failure of that filter, `viewer::camera::bike::fov_tests`, is the camera's and was not touched. The three `air_tests` of `blackbox-vehicle` fail on `integ/bike-2012-sweep` without this branch too.
- At walking pace (under 7 km/h) the circle is the handlebar's own lock (`blend_range`, direct mapping): the data's `max_angle` of 18 to 42
  degrees gives 4.3 to 1.6 m with a wheelbase of 1.4 m. The table's 1.8 m under 6 km/h is only reached with the assist (fade 0.3 to 1.5 m/s). A
  yaw-rate servo for the first metres per second (a handlebar steer-angle model: `psi' = v / r(v)` with no lean) is the next step if the harness shows
  walking pace too wide; it was left out because the stability help is already such a servo above 10 km/h and an untested one below could chatter.
- The assist does not look at the surface (gravel, rain), as before.
