# RIDE 6 bikes: the arcade steering of this game (clean-room spec)

**This is a modification made exclusively for the NFS:MW rewrite. It is not the behaviour of RIDE 6.** The reference game's bikes turn like
real bikes; this game's roads need bikes that turn like an arcade game's. Every number of the change is in one struct,
`blackbox_vehicle::bike::ArcadeSteering` (`libs/blackbox-vehicle/src/bike/arcade.rs`); `ArcadeSteering::REFERENCE` (also `disabled()`) is neutral
in every field and gives the reference behaviour back exactly, so another game that reuses the library, or a player who wants RIDE 6's feel,
switches it off ([section 9](#9-how-to-switch-it-off)).

Continues [ride6-feel-round12.md](ride6-feel-round12.md), [ride6-feel-round13.md](ride6-feel-round13.md) and
[ride6-lean-controller.md](ride6-lean-controller.md). Tags: **[M]** measured in the model's own closed loop (the harness
`libs/blackbox-modding/src/content/bike_ride_steer.rs`, ignored tests on the imported bikes), **[D]** data (the 288 imported manifests),
**[R]** read in this repository's code, **[C]** a choice of this project.

## 1. What the player reported

"Driving with bikes is impossible. Too much time to lean from one side to the other and too much large turn radius. The only way I can do a
curve on the road is by going 20 km/h. I turn all the way and it seems like the bike is only leaned, but still goes forward outside the curve
into the wall. The more I speed the worse it is." Rounds 12 and 13 had made left as quick as right and held the speed curve of the lean-in
at 0.65; it was still far too slow.

## 2. Where "leaned but goes straight" comes from [M]

16 imported bikes (superbikes, nakeds, a cruiser-like naked, an adventure bike, a scooter, the supermotos CRF450SM, KTM 690 SMC R, TM SMX and the
KTM 450 the example is taken from), a full stick held for 10 s at 10 to 60 m/s on flat ground, medians. The stick is `STEER_REACH`-free here
(the harness feeds 1.0). The harness, per bike and speed: the radius and lateral acceleration of the last 3 s, the lean against its limit,
the angle between the velocity and the heading, the front slip angle and handlebar, the share of its peak each tyre uses, and the time a flip
of the stick takes (the stick held left for 5 s, then right: the seconds to 80 and 95 percent of the steady lean).

| speed m/s | radius m | lateral g | lean deg | lean limit deg | base limit deg | heading error deg | front slip deg | handlebar deg | front / rear tyre use | flip t80 s | flip t95 s |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 10 | 10 | 0.98 | 50.7 | 49.7 | 52.1 | 1.2 | 2.6 | 14.4 | 0.75 / 0.77 | 1.30 | 1.75 |
| 15 | 21 | 1.04 | 52.2 | 51.3 | 52.1 | 1.6 | 2.8 | 7.0 | 0.80 / 0.81 | 1.37 | 1.82 |
| 20 | 37 | 1.07 | 52.7 | 51.9 | 52.1 | 2.3 | 2.9 | 3.9 | 0.83 / 0.83 | 1.42 | 1.88 |
| 30 | 83 | 1.06 | 52.6 | 51.9 | 52.1 | 2.7 | 2.9 | 1.3 | 0.83 / 0.82 | 1.45 | 1.93 |
| 40 | 145 | 1.05 | 52.5 | 51.8 | 52.1 | 3.2 | 2.9 | 0.3 | 0.82 / 0.81 | 1.52 | 2.02 |
| 60 | 286 | 1.05 | 52.5 | 51.8 | 52.1 | 3.8 | 3.4 | 1.3 | 0.82 / 0.81 | 1.65 | 2.23 |

(the example bike's table is in [ride6-feel-round11.md](ride6-feel-round11.md); the log of this run: `C:\r6work\steer-before-audit.log`, not committed.)

What the table says, and what it rules out:

1. **The lean limit is not cut** (51.8 against a base of 52.1 degrees: no skid cut, no g cut), **the tyres do not saturate** (0.82 of their peak), the
   heading error is 1 to 4 degrees and the handlebar is far from its lock (0.3 to 14 degrees of 18 to 42). The bike does what it is asked: it is
   asked for too little.
2. **The turn is the physics of a real bike.** A steady turn has a lateral acceleration of `g tan(lean)` where the lean is that of the line from
   the contact patches to the centre of gravity; the tyre grip (mu 1.28, `atan` 52 degrees) caps the lean, and the wide tyres' patch and the
   rider's offset take 6 degrees off, so **1.05 g at every speed and a radius of `v^2 / 10.3`**: 37 m at 20 m/s, 145 m at 40, 286 m at 60. A road
   bend of 100 m at 30 m/s asks 0.9 g and at 40 m/s 1.6 g. The player turns "all the way", the bike leans to its limit, and the radius is
   too wide: the more speed, the more the radius grows with `v^2`. That is the whole "the more I speed the worse".
3. **The flip takes 1.3 to 1.65 s to 80 percent**, and grows with the speed even with the floor of round 13: the chain of the controller has five
   slow stages in series, each an overdamped second-order filter with one slow pole [R, D]: the stick filter (K 1000, z 2.5: 0.15 s), the target
   filter (K 1000, z 2.25: 0.135 s over the time scale, 0.4 to 0.5 for a full stick and 0.65 or less at speed), the roll spring (K 1000, z 3.75: 0.23 s), the
   handlebar filter (K 1000, z 3: 0.185 s) and the front tyre's force, which has to roll the whole bike.
4. **A bike cannot flip much faster without leaving the road [M].** In the closed loop a flip at 40 m/s with the lean help and the
   controller made 3 times quicker had both wheels in the air for 0.1 to 0.5 s in the middle of the flip: a body rolled about its contact line
   leaves the road when `omega^2 h > g`, that is above `sqrt(g / h)` = 3.5 rad/s with the centre of gravity at 0.8 m, and a flip of 99 degrees
   (left limit to 80 percent of the right) needs a mean of 4 rad/s in 0.4 s. The reference's own fall test says the same: a body angular speed above
   `FallSpec::angular_speed` (4.4 rad/s for the Panigale) is a fall. A flip of 0.4 s from 52 degrees to 80 percent of 52 degrees is therefore
   **not possible with the tyres and the rigid body of the model**; see section 5 for what is done instead.

## 3. The levers, from the most physical to the least

Each field of `ArcadeSteering`; the neutral value is `REFERENCE`'s.

| field | what it does | reference | this game |
|---|---|---|---|
| `lateral_grip` | the lateral force of both tyres, peak, stiffness and camber thrust, multiplied once in `Bike::new` (`Mf94::with_lateral_scale`); the lean limit (`atan` of the grip), the fall limit and the lean help's cap read the scaled tyre | 1 | 1 (the lever works, see below; it was not needed) |
| `lean_limit` | the factor on the lean the stick can ask for | 1 | 0.8 (52 to 42 degrees) |
| `lean_rate` | the factor on the time scale of the target filter | 1 | 3 |
| `time_scale_speed_floor` | the least of the data's speed curve of that time scale (the data slows the lean-in to a third at 330 km/h) | 0 (the data's curve) | 1 (none) |
| `mirror_stick_curve` | the data's stick curve of the time scale read at the quicker side for both directions (round 13) | false (signed) | true |
| `roll_spring` | the frequency of the roll spring-damper, damping ratio kept | 1 | 2 |
| `stick_response`, `handlebar_response` | the frequency of the stick's and the handlebar's filters | 1, 1 | 2, 1.25 |
| `lean_help`, `lean_help_limit` | the reference's lean help (a side force at the front contact towards the one the controller wants, 0.16 of the gap above 10 km/h, capped at the rest-load peak) closes this many times more of the gap and may give this many times more | 1, 1 | 6.25 (the whole gap), 3 |
| `fall_angular_speed` | the factor on the fall test's angular speed limit (a quick flip is the rider's motion, not a tumble) | 1 | 3 |
| `cornering_assist` | a side force at the centre of gravity (in g at full lean), towards the lean the rider asks for, no roll torque; comes in over `assist_speed` 2 to 8 m/s | 0 | 0.9 |
| `stick_reach` | the share of a pad stick's travel that reads as the whole stick (the host's input layer; was `choices::STEER_REACH`) | 1 | 0.9 |

`choices::TIME_SCALE_SPEED_FLOOR` and `choices::STEER_REACH` of rounds 12 and 13 are gone: they are the fields `time_scale_speed_floor` (0.65 then, 1 now)
and `stick_reach` (0.9, unchanged).

### 3.1 The cornering assist

The lateral acceleration of the turn is `g tan(lean) + assist x (target lean / limit)`: the tyres hold the bike against its gravity at a lean of 42 degrees
(0.7 g of grip is enough) and the assist is the rest of a 1.6 g turn. It acts on the **target** lean, which is ahead of the bike's own lean
while it rolls, so the path turns as soon as the stick is moved (a path flip of 0.35 to 0.5 s) and the bike's lean follows (0.65 s). It is a force
at the centre of gravity, so it adds no roll torque and does not change the balance of the bike; the stability help's wanted yaw rate adds it
(`(g tan(lean) + assist) / v`, `lean_help.rs`) so the two do not fight. It needs the rear wheel on the road and is off in a burnout and a
fall. It does not look at the surface: on gravel or in the rain the bike still turns at 1.6 g (open item).

Why a force and not the tyre's grip: raising `lateral_grip` to 1.15 gives 1.19 g only (the lean goes to 56 degrees, the rider's offset and the tyre's
patch take the rest), and a lean of 60 degrees or more is beyond the fall limit of the data (57 degrees); 1.6 g is `tan(58 degrees)` of the line from the
patches to the centre of gravity. The assist is the cheapest lever that does not lean the bike over its fall limit, and the lean limit of 0.8 leaves
a margin of 15 degrees to it, which is what keeps the quick flips from falling.

### 3.2 What the other levers do

`lean_help` and `lean_help_limit` give the roll the front tyre cannot give in time (the front wheel hops in a quick flip and its handlebar lags);
the help is a force at the front contact, so it keeps the coupling of lean and yaw that a direct roll torque lost (a roll servo and a roll
torque were tried and rejected: with a torque on the body the bike leaned and did not turn, and fell in 3 of 8 bikes even in a steady turn at
10 m/s [M]). `fall_angular_speed` stops the flip being read as a tumble: a roll of 15 rad/s still falls (test).

## 4. After [M]

The same 16 bikes and the same run, `ArcadeSteering::default()`:

| speed m/s | radius m | lateral g | lean deg | heading error deg | front / rear tyre use | flip t80 s | flip t95 s | path flip t80 s |
|---|---|---|---|---|---|---|---|---|
| 10 | 7 | 1.53 | 39.6 | 4.1 | 0.17 / 0.52 | 0.63 | 0.92 | 0.53 |
| 15 | 15 | 1.59 | 41.2 | 1.3 | 0.29 / 0.55 | 0.65 | 0.93 | 0.50 |
| 20 | 25 | 1.61 | 41.9 | 0.4 | 0.42 / 0.56 | 0.67 | 0.92 | 0.47 |
| 30 | 55 | 1.61 | 41.8 | 1.3 | 0.50 / 0.56 | 0.67 | 0.88 | 0.43 |
| 40 | 97 | 1.60 | 41.7 | 1.7 | 0.52 / 0.56 | 0.67 | 0.90 | 0.38 |
| 60 | 212 | 1.60 | 41.6 | 2.2 | 0.54 / 0.55 | 0.67 | 1.07 | 0.35 |

Against the targets of the brief: radius 25 m at 20 m/s (target 25), 55 at 30 (50), 97 at 40 (90), 212 at 60 (200): **met** (1.6 g is the middle of the
1.4 to 1.8 asked). The turn-in does not get worse with the speed (t80 0.63 to 0.67 s from 10 to 60 m/s; before 1.30 to 1.65 s) and the heading error stays
at 0.4 to 4 degrees. The tyres use half of their peak, so a full-stick turn does not skid (no squeal or marks from the skid thresholds of the
host, which read the tyre's own peak). **The flip is not 0.4 s but 0.65 s** to 80 percent of the steady lean (0.9 s to 95 percent): the path (the yaw
rate) reaches 80 percent of the other side in 0.35 to 0.5 s. 0.4 s for the lean is impossible without leaving the road (section 2, item 4); 0.65 s is what the same
physics gives with the lean limit cut to 0.8 (an 84-degree flip instead of 99), the quick controller and the lean help of section 3.

## 5. Falls [M]

`bike_steer_sweep`: 21 bikes (every 14th of the 288), 10 s each at 8, 12, 16, 20, 30, 40, 50, 60, 70 and 80 m/s: a held full stick, a held 0.8 stick, a
slalom of full sticks (1.2 s a side), the stick flicked every 0.35 s, and a flip every 0.6 s.

| manoeuvre | falls before | falls after |
|---|---|---|
| held 1.0, held 0.8, slalom, flick, flip | 0 of 1050 | see the log below |

(The audit of 16 bikes, `bike_steer_audit`: before, the Honda X-ADV scooter falls in a held turn at 30 and 40 m/s and in a flip at 30, 40 and 60;
after, it falls in a flip at 60 m/s only (both wheels in the air at 26 degrees of heading error): the scooter has a short wheelbase and a low rest-load
grip; it is the one bike of the 16 that does.)

The existing tests (`cargo test -p blackbox-vehicle`, the host's bike tests) keep their numbers: those written for rounds 12 and 13 ride with the steering of
those rounds (`round13()` in `turn_tests.rs`: `ArcadeSteering { time_scale_speed_floor: 0.65, mirror_stick_curve: true, ..REFERENCE }`), the rest with
`BikeSpec::example()`, which carries `REFERENCE`. New tests, `arcade_tests.rs`: the reference is neutral; 1.4 to 1.9 g at 20 and 40 m/s against under
1.3 g; a half stick turns half and a left stick as a right one; the flip is 20 percent quicker than round 13's and does not slow with the speed; a bike thrown over or tumbling still falls;
the grip scales the tyres and the limits together.

## 6. In the scene

(filled in from the offscreen runs; see section 7.)

## 7. Verification

(see the report of the run.)

## 8. Open items

- Flip time of the lean 0.65 s, not 0.4 s: needs the wheels to stay on the road while the body rolls above `sqrt(g / h)`; a kinematic roll about the
  centre of gravity (the body's roll set directly with a hold-down force) would give 0.4 s; not done (it would also have to drive the yaw).
- The cornering assist does not look at the surface (gravel, rain): scale it by the rear contact's lateral grip once the wheel state carries it.
- The bikes with the lowest grip (the adventure bikes: base limit 49 degrees, 0.77 g) turn at the same 1.6 g as the others; the scooter falls in a flip at 60 m/s.
- The setting has no menu row (the settings pattern needs a row and a translation in every language).
- The rider's lean animation reads the lean over `max_lean`: with a lean of 42 degrees the rider hangs 40 percent less than before.

## 9. How to switch it off

- In the game: `bike_steering = "reference"` in `config.toml` (or in an alias's settings), `NFSMW_BIKE_STEERING=reference`, or the console command
  `set bike_steering reference`; it reaches the bike loaded next. `arcade` is the default.
- In the library: `spec.steering.arcade = ArcadeSteering::REFERENCE` before `Bike::new`. A game that builds its bikes from a manifest layer
  chooses there; `BikeSpec::example()` carries `REFERENCE`.
