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
KTM 450 the example is taken from), a full stick held for 10 s at 10 to 60 m/s on flat ground, medians. The harness feeds the stick as 1.0.
The harness, per bike and speed: the radius and lateral acceleration of the last 3 s, the lean against its limit,
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
| `lateral_grip` | the lateral force of both tyres, peak, stiffness and camber thrust, multiplied once in `Bike::new` (`Mf94::with_lateral_scale`); the lean limit (`atan` of the grip), the fall limit, the lean help's cap and the host's skid threshold read the scaled tyre | 1 | 1 (the lever works, see 3.1; it was not needed) |
| `lean_limit` | the factor on the lean the stick can ask for | 1 | 0.7 (52 to 36.5 degrees) |
| `lean_rate` | the factor on the time scale of the target filter | 1 | 1.38 (was 3, [section 10](#10-the-lean-in-slowed-playtest-round-2)) |
| `time_scale_speed_floor` | the least of the data's speed curve of that time scale (the data slows the lean-in to a third at 330 km/h) | 0 (the data's curve) | 1 (none) |
| `mirror_stick_curve` | the data's stick curve of the time scale read at the quicker side for both directions (round 13) | false (signed) | true |
| `roll_spring` | the frequency of the roll spring-damper, damping ratio kept | 1 | 1.28 (was 2) |
| `stick_response`, `handlebar_response` | the frequency of the stick's and the handlebar's filters | 1, 1 | 1.28, 1.085 (were 2, 1.25) |
| `lean_help`, `lean_help_limit` | the reference's lean help (a side force at the front contact towards the one the controller wants, 0.16 of the gap above 10 km/h, capped at the rest-load peak) closes this many times more of the gap and may give this many times more | 1, 1 | 6.25 (the whole gap), 3 |
| `fall_angular_speed` | the factor on the fall test's angular speed limit (a quick flip is the rider's motion, not a tumble) | 1 | 3 |
| `cornering_assist` | a side force at the centre of gravity (in g at full lean), towards the lean the rider asks for, no roll torque; comes in over `assist_speed` 2 to 8 m/s | 0 | 1.1 |
| `stick_reach` | the share of a pad stick's travel that reads as the whole stick (the host's input layer; was `choices::STEER_REACH`) | 1 | 0.9 |
| `authority`, `authority_speed` | the factor on the cornering assist at the slow and at the fast end of a speed range (a smooth curve over the logarithm of the speed): a tighter circle when slow, a wider one when fast ([section 11](#11-the-turning-circle-over-the-speed-playtest-round-2)) | [1, 1] | [1.5, 0.6] over 5 to 70 m/s |
| `turn` | the turn law of round 3 ([ride6-bike-turn-radius.md](ride6-bike-turn-radius.md)): the assist as the part of the circle's acceleration the lean does not give, with the circle a full stick turns named by one table over the speed (tight when slow, wide when fast), a path that follows the stick at a low speed and a quicker lean-in there; while set it replaces `cornering_assist`, `assist_speed` and `authority` | none | `TurnLaw::GAME` |
| `wheelie_steer_drop` | the most the stick lowers the front wheel of a wheelie (the pitch assist keeps its hold in a lean, [section 12](#12-steering-in-a-wheelie-playtest-round-2)) | none (the data's assist) | 7 degrees |

`choices::TIME_SCALE_SPEED_FLOOR` and `choices::STEER_REACH` of rounds 12 and 13 are gone: they are the fields `time_scale_speed_floor` (0.65 then, 1 now)
and `stick_reach` (0.9, unchanged).

### 3.1 The cornering assist

The lateral acceleration of the turn is `g tan(lean) + assist x (target lean / limit)`: the tyres hold the bike against its gravity at a lean of 36 degrees
(0.6 g of grip is enough) and the assist is the rest of a 1.6 to 1.7 g turn. It acts on the **target** lean, which is ahead of the bike's own lean
while it rolls, so the path turns as soon as the stick is moved (a path flip of 0.35 to 0.5 s) and the bike's lean follows (0.65 s). It is a force
at the centre of gravity, so it adds no roll torque and does not change the balance of the bike; the stability help's wanted yaw rate adds it
(`(g tan(lean) + assist) / v`, `lean_help.rs`) so the two do not fight. It needs the rear wheel on the road and is off in a burnout and a
fall. It did not look at the surface (on gravel or in the rain the bike still turned at 1.6 g); it does now, scaled by the grip the bike believes in ([ride6-bike-rough-ground.md](ride6-bike-rough-ground.md)).

Why a force and not the tyre's grip: raising `lateral_grip` to 1.15 gives 1.19 g only (the lean goes to 56 degrees, the rider's offset and the tyre's
patch take the rest), and a lean of 60 degrees or more is beyond the fall limit of the data (57 degrees); 1.6 g is `tan(58 degrees)` of the line from the
patches to the centre of gravity. The assist is the cheapest lever that does not lean the bike over its fall limit, and the lean limit of 0.8 leaves
a margin of 20 degrees to it, which is what keeps the quick flips from falling (0.8 gave 18 falls in 1050 runs at 60 to 80 m/s, 0.7 gives 2 in 9800).

### 3.2 What the other levers do

`lean_help` and `lean_help_limit` give the roll the front tyre cannot give in time (the front wheel hops in a quick flip and its handlebar lags);
the help is a force at the front contact, so it keeps the coupling of lean and yaw that a direct roll torque lost (a roll servo and a roll
torque were tried and rejected: with a torque on the body the bike leaned and did not turn, and fell in 3 of 8 bikes even in a steady turn at
10 m/s [M]). `fall_angular_speed` stops the flip being read as a tumble: a roll of 15 rad/s still falls (test).

## 4. After [M]

The same 16 bikes and the same run, `ArcadeSteering::default()`, on the tree merged with the skid and fall work of the integration branch:

| speed m/s | radius m | lateral g | lean deg | heading error deg | front / rear tyre use | worst slide f / r (1 = force peak) | flip t80 s | flip t95 s | path flip t80 s |
|---|---|---|---|---|---|---|---|---|---|
| 10 | 7 | 1.61 | 34.7 | 4.8 | 0.24 / 0.43 | 0.11 / 0.14 | 0.63 | 0.92 | 0.50 |
| 15 | 14 | 1.67 | 36.1 | 1.7 | 0.19 / 0.45 | 0.07 / 0.14 | 0.65 | 0.92 | 0.48 |
| 20 | 24 | 1.68 | 36.6 | 0.4 | 0.30 / 0.46 | 0.10 / 0.15 | 0.67 | 0.92 | 0.45 |
| 30 | 53 | 1.68 | 36.6 | 0.8 | 0.39 / 0.46 | 0.12 / 0.17 | 0.67 | 0.88 | 0.42 |
| 40 | 93 | 1.68 | 36.6 | 1.3 | 0.42 / 0.46 | 0.13 / 0.20 | 0.67 | 0.87 | 0.38 |
| 60 | 205 | 1.67 | 36.5 | 1.7 | 0.44 / 0.46 | 0.13 / 0.29 | 0.67 | 0.90 | 0.37 |

(before, same columns: worst slide 0.31 to 0.46, flip t80 1.30 to 1.65 s; the logs: `C:\r6work\steer-after-audit2.log`.) Against the targets of the brief: radius 25 m at 20 m/s
(target 25), 53 at 30 (50), 93 at 40 (90), 205 at 60 (200): **met** (1.6 to 1.7 g is the middle of the 1.4 to 1.8 asked). The turn-in does not get worse with the
speed (t80 0.63 to 0.67 s from 10 to 60 m/s; before 1.30 to 1.65 s) and the heading error stays at 0.4 to 4.8 degrees. The tyres use under half of their peak and the
worst slide of any bike of the 16 is 0.29 of the force peak, far from the host's skid threshold of 1.05 (`bike_slide.rs`): **the squeal, the smoke and the marks stay quiet in a
full-stick arcade turn** (and quieter than before: 0.46). **The flip is not 0.4 s but 0.65 s** to 80 percent of the steady lean (0.9 s to 95 percent): the
path (the yaw rate) reaches 80 percent of the other side in 0.35 to 0.5 s. 0.4 s for the lean is impossible without leaving the road (section 2, item 4); 0.65 s is what the same physics gives
with the lean limit cut to 0.7 (a 73-degree flip instead of 99), the quick controller and the lean help of section 3.

## 5. Falls [M]

`bike_steer_sweep` on the merged tree: **98 imported bikes** (every third of the 293 manifests of the full import), 10 s each at 8, 12, 16, 20, 30, 40, 50, 60, 70 and 80 m/s: held sticks
of 1.0, 0.8, 0.6, 0.45 and 0.3, a slalom of full sticks (1.2 s a side), the stick flicked every 0.35 s, and a flip every 0.6 s: 9800 runs.

| manoeuvre | falls before (round 13 steering, 21 bikes) | falls after (98 bikes) |
|---|---|---|
| held 1.0, 0.8, 0.6, 0.45, 0.3 | 0 | 0 |
| flick, flip every 0.6 s | 0 | 0 |
| slalom of full sticks | 0 | 2: the Italjet Dragster 200 scooter at 70 and 80 m/s (a 200 cm3 scooter does not reach 70 m/s) |

The audit of 16 bikes: before, the Honda X-ADV scooter falls in a held turn at 30 and 40 m/s and in a flip at 30, 40 and 60; after, none of the 16 falls at any of the six speeds.
The three Indian Challenger RR cruisers (alone, the speed held or the throttle fixed at 0, 0.5 and 1) fall in no manoeuvre, before or after, in this harness: the "falls at a stick of
0.6 above 100 km/h" of the playtest is not reproduced here, so it comes from the host (the wall and the rider's contacts, which the harness does not have) and not from the lean
controller. Tuning that did fall in the sweep (and was rejected): lean limit 0.8 with the same controller, 18 falls at 60 to 80 m/s; lean help 6.25 with a spring of 1.6, 4 falls.

The existing tests (`cargo test -p blackbox-vehicle`, the host's bike tests) keep their numbers: those written for rounds 12 and 13 ride with the steering of
those rounds (`round13()` in `turn_tests.rs`: `ArcadeSteering { time_scale_speed_floor: 0.65, mirror_stick_curve: true, ..REFERENCE }`), the rest with
`BikeSpec::example()`, which carries `REFERENCE`. New tests, `arcade_tests.rs`: the reference is neutral; 1.4 to 1.9 g at 20 and 40 m/s against under
1.3 g; a half stick turns half and a left stick as a right one; the flip is 20 percent quicker than round 13's and does not slow with the speed; a bike thrown over or tumbling still falls;
the grip scales the tyres and the limits together.

## 6. In the scene

The offscreen runs (`view-world --drive BMW_S-1000-RR-2019 --at 985,4430 --muted --camera-mode superfar` with a drive script, the same executable with
`NFSMW_BIKE_STEERING=reference` and `arcade`; frames under `C:\r6work\steer-shots`) start in a tunnel next to a toll plaza: the bike meets a wall within seconds of a stick in both
settings and the two runs do not stay on the same path, so the frames show that the setting reaches the bike (the reference bike ends at 28 km/h after the wall, the arcade bike holds
94 km/h round the tunnel's bend) and **not** a measured trajectory; the numbers of this spec are the closed loop's ([M]), not the scene's.

## 7. Verification

`cargo test --release -p blackbox-vehicle` (821 tests, new: `arcade_tests.rs`, `arcade.rs`), `cargo test --release -p nfsmw --bin nfsmw bike`,
`cargo clippy -p blackbox-vehicle -p blackbox-modding -p nfsmw -- -D warnings`, `cargo xtask check`. The harness: `RIDE6_CARS=<folder> RIDE6_FILTER=a,b RIDE6_EVERY=n
[RIDE6_ARCADE=reference|before|grip=..,lean=..,assist=..] cargo test --release -p blackbox-modding --features host --lib bike_steer_audit|bike_steer_sweep|bike_steer_trace -- --ignored --nocapture`.

## 8. Open items

- Flip time of the lean 0.65 s, not 0.4 s: needs the wheels to stay on the road while the body rolls above `sqrt(g / h)`; a kinematic roll about the
  centre of gravity (the body's roll set directly with a hold-down force) would give 0.4 s; not done (it would also have to drive the yaw).
- The cornering assist looks at the surface since [ride6-bike-rough-ground.md](ride6-bike-rough-ground.md) (it was an open item here). Jumps: [ride6-bike-jumps.md](ride6-bike-jumps.md).
- The bikes with the lowest grip (the adventure bikes: base limit 49 degrees, 0.77 g) turn at the same 1.6 to 1.7 g as the others; the Italjet scooter falls in a slalom at 70 m/s and more.
- The in-scene check is not a trajectory (section 6); a drive-script log of the car's position would give one.
- The Challenger fall of the playtest is not reproduced by the harness (section 5).
- The setting has no menu row (the settings pattern needs a row and a translation in every language).
- The rider's lean animation reads the lean over `max_lean`: with a lean of 36 degrees the rider hangs 30 percent less than before.

## 9. How to switch it off

- In the game: `bike_steering = "reference"` in `config.toml` (or in an alias's settings), `NFSMW_BIKE_STEERING=reference`, or the console command
  `set bike_steering reference`; it reaches the bike loaded next. `arcade` is the default.
- In the library: `spec.steering.arcade = ArcadeSteering::REFERENCE` before `Bike::new`. A game that builds its bikes from a manifest layer
  chooses there; `BikeSpec::example()` carries `REFERENCE`.

## 10. The lean-in slowed (playtest round 2)

Playtest: "make leaning left/right a bit slower, something in the middle between now and original delay but a bit more to the original." [C]

**What was done.** The lean-in of this game is the sum of four filters in series (section 2, item 3): the stick's, the controller's target filter, the roll spring and the handlebar's. Each of
them is a second-order filter whose frequency is scaled by one lever (`stick_response`, `lean_rate`, `roll_spring`, `handlebar_response`). The first try moved each lever so that its stage time was 67.5
percent of the way back to the original (harmonic interpolation, `1 / lever = 0.325 / arcade + 0.675 / 1`: 1.28, 1.19, 1.19, 1.07); **measured, that was too slow** (the chain is not additive: the lean help and the
target filter's sensitivity to the size of the step shorten the arcade time less than they shorten the original's): 79 percent of the way. Two more tries (1.6, 1.4, 1.4, 1.12: 50 percent; 1.35, 1.25, 1.25, 1.08:
72 percent) bracket the goal; the values kept are

| lever | original | before (round 1) | now |
|---|---|---|---|
| `lean_rate` | 1 | 3 | **1.38** |
| `roll_spring` | 1 | 2 | **1.28** |
| `stick_response` | 1 | 2 | **1.28** |
| `handlebar_response` | 1 | 1.25 | **1.085** |

Not touched: `lean_limit`, `time_scale_speed_floor` (1: the lean-in is not slowed with the speed, which is the other half of "too much time to lean"), `mirror_stick_curve`, `lean_help` and its
limit, `fall_angular_speed`, `cornering_assist`.

**Measured** [M; `arcade_tests.rs::the_lean_in_is_two_thirds_of_the_way_from_the_arcade_time_to_the_originals`, `--nocapture`]: a stick stepped from 0 to 1 while riding straight at 60 km/h (held by the throttle),
the time to 90 percent of the steady lean (the lean after 10 s of the held stick), the example bike with a superbike's power, stick to the right:

| 0 to 90 percent of the lean at 60 km/h | time | share of the way from before to the original |
|---|---|---|
| before (round 1: 3, 2, 2, 1.25) | 0.50 s | 0 |
| **new (1.38, 1.28, 1.28, 1.085)** | **0.80 s** | **64 percent** (the test accepts 55 to 80; the time moves in steps of 1/60 s) |
| original (`ArcadeSteering::REFERENCE`) | 0.97 s | 100 percent |

The flip of the stick from side to side to 80 percent of the other lean (same test file, 20 m/s): before 0.60 s, **new 0.92 s**, original 1.22 s. On the imported bikes (`bike_steer_audit`, every 12th bike, 25 bikes,
medians; the log is `C:\r6work\feel-audit-*.log`) the flip to 95 percent of the other side takes 0.65 to 0.67 s before, **1.00 to 1.07 s new**, 1.35 to 1.65 s in the original, and the yaw rate reaches 80 percent of the other
side in 0.38 to 0.50 s before, **0.60 to 0.75 s new**, 0.97 to 1.22 s original, at 20 to 200 km/h: slower, nearer the original than the middle, and still not slowing with the speed.
The cornering assist follows the *target* lean (section 3.1), so the path turns a little later too; the circle itself is section 11. A slalom or a flip does not fall any imported bike of the 25 at 20 to 200 km/h in the audit
(`0 turn, 0 flip` at every speed).

## 11. The turning circle over the speed (playtest round 2)

Playtest: "turning should be harder (less radius than now) when going slow and easier when going fast (more radius than now, progressive)." Read as: **at low speed a tighter circle than now, at high speed a
wider one, with a smooth curve in the speed.** [C]

**The lever.** The cornering assist (section 3.1) supplies most of the lateral acceleration: `a = g tan(lean) + assist x g x (target lean / limit) x fade(speed)`, `g tan(lean)` being 0.7 g at the 36-degree
limit and the assist 1.1 g at full. A circle is `v^2 / a`, so the speed dependence is moved into the assist: it is multiplied by `ArcadeSteering::authority_factor(speed)`, one named, documented
pair of fields: `authority = [slow, fast]` and `authority_speed = [v0, v1]`. The factor is `slow` at or under `v0`, `fast` at or over `v1`, and between them
`slow + (fast - slow) x smoothstep(t)` with `t` the position of `ln(speed)` between `ln(v0)` and `ln(v1)`: a smooth curve (no step, no kink at the ends: the test steps 0.1 m/s from 0 to 90 m/s and
finds no change over 1.5 percent) that moves where the speed doubles from a little, which is where a rider feels it (40 to 80 km/h changes more than 160 to 200). The assist is a force at the
centre of gravity, so the lean limit, the fall limit and the tyre's use are untouched: the stick turns a tighter circle slowly **without** leaning further.

**The numbers.** `authority = [1.5, 0.6]` over `[5, 70]` m/s (18 to 252 km/h): 1.5 times the assist at a town pace, 0.6 at a motorway speed and above; the factor passes 1 (the assist as it
was) at about 18 m/s (65 km/h). Chosen so that a full stick still asks for more than the lean alone at every speed (0.7 g plus 0.6 of the assist is above 1.3 g), so a bend is never impossible, and so that
the radius change is felt but not extreme: about 15 to 20 percent tighter under 40 km/h and 25 percent wider at 200 km/h.

**Measured** [M]. `arcade_tests.rs::the_turning_circle_is_tighter_when_slow_and_wider_when_fast_progressively` (the example bike with a superbike's power, a heavy one and a light one, a full stick held 8 s, the
circle of the last 2 s; the three bikes agree to 0.01 in the ratio, the first is shown):

| speed | factor | radius before | radius new | new / before |
|---|---|---|---|---|
| 20 km/h | 1.50 | 1.8 m | 1.5 m | 0.84 |
| 40 km/h | 1.30 | 7.3 m | 6.3 m | 0.86 |
| 80 km/h | 0.96 | 29.0 m | 29.9 m | 1.03 |
| 120 km/h | 0.77 | 65.3 m | 75.7 m | 1.16 |
| 200 km/h | 0.62 | 181 m | 233 m | 1.29 |

And on 25 imported bikes (`bike_steer_audit`, every 12th bike, medians, a full stick held; "before" is `RIDE6_ARCADE=previous`, the original `reference`):

| speed | radius before | **new** | original | lateral g before | **new** | original |
|---|---|---|---|---|---|---|
| 20 km/h | 5 m | **4 m** | 5 m (4 of 25 turn over) | 1.00 | **1.84** | 0.71 |
| 40 km/h | 8 m | **7 m** | 13 m | 1.65 | **1.94** | 1.01 |
| 80 km/h | 30 m | **30 m** | 46 m | 1.68 | **1.65** | 1.07 |
| 120 km/h | 66 m | **76 m** | 102 m | 1.68 | **1.45** | 1.06 |
| 200 km/h | 179 m | **234 m** | 265 m | 1.68 | **1.28** | 1.05 |

The new circle is tighter than before under about 60 km/h (12 to 20 percent), the same at 80 km/h (the factor crosses 1 at 65 km/h) and wider above (15 percent at 120, 30 percent at 200 km/h), the curve
smooth between. The lean stays at 33 to 37 degrees at every speed (the assist is a force at the centre of gravity), no bike of the 25 falls in the audit.

The tests (`arcade_tests.rs`, `arcade.rs`): the ratio new/now is under 0.97 at 20 and 40 km/h, over 1.03 at 120 and 200 km/h and never falls with the speed, for the example bike, a heavy one (mass x1.35,
torque x1.3) and a light one (mass x0.85); the radius grows with the speed in the new, the old and the original steering; the factor is 1 at every speed in `REFERENCE`, never rises with the speed and has no
step. A low speed is also where the tyres are least loaded, so the lateral grip is not the limit; the harness (`bike_steer_audit` and `bike_steer_sweep`, `RIDE6_ARCADE=previous|slow=1.5,fast=0.6`) is the check on the imported bikes.

**Switching it off.** `bike_steering = reference` gives `authority = [1, 1]` (and everything else of the original) back; `REFERENCE` is exactly neutral (`the_reference_steering_is_neutral...` rides the same
circle with every field at its neutral value).

## 12. Steering in a wheelie (playtest round 2)

Playtest: "if I'm wheeling and go right/left the bike should be brought down only a little bit, not all the way." [C]

**What lowers the front** [M]. In a wheelie the pitch assist (`pitch_assist.rs`) is faded out by the lean: `lean_help = 1 - ramp(|lean| / limit, [0.2, 1])` of the data (`PitchParams::lean_help`), the one term of
the assist that depends on the stick. With the stick over the assist lets go. In the harness (`wheelie_steer_tests.rs`, the example bike with 2.2 times the torque, full stick for 3 s after the wheelie settled):

| wheelie | before the stick | lowest with the stick (data's assist) | with the floor (7 degrees) |
|---|---|---|---|
| the lever (manual), stick 1 / -1 / 0.5 | 42.5 degrees | 32.2 / 32.2 / 32.6 | the same (the floor is not reached) |
| the throttle's (the game's assist), stick 1 | 32.2 degrees | **falls to the road (-1.8)** with the dip and the shift of this round's assist | **22.3** (10 lost); stick 0.5: 29.6 |

So the manual wheelie loses about 10 degrees by itself, and the throttle's wheelie in a lean fell all the way once it had the gear-change dip: the playtest's report.

**The change** (`ArcadeSteering::wheelie_steer_drop`, `Some(7 degrees)` in this game, `None` in `REFERENCE`): a **floor** under a steered wheelie. The bike remembers the pitch its wheelie had before the lean (`TrickState::wheelie_ref`:
followed while the front wheel is up and the lean is under a quarter of its limit, held while it leans, 0 on the road) and, while the front wheel is up and the bike leans, the pitch assist **lifts the nose whenever it is
under that pitch less 7 degrees** (a time-optimal lift towards the floor, `STEER_LIFT_GAIN` 6 times the data's torque because a leaned bike has lost most of the engine's lift; 4 and 10 gave the same result within a degree), through the data's own limits (no lift when the rear wheel is up, the speed ranges), and **never lowers** it. It is one-sided on purpose: a first version that aimed the assist at a lower target
made the throttle's wheelie fall (the allowed angle follows the nose down), and 10 degrees held the nose 13 to 14 degrees down because the lift settles a few degrees under its floor, so the floor is 7. The result is a
**bounded loss of about 10 degrees** with the wheelie still up, at a full stick.

**Tests**: `wheelie_steer_tests.rs` (the lever's and the throttle's wheelie at stick 1, -1 and 0.5: under 12 degrees lost, the front wheel stays up, and never lower than with the data's assist), `pitch_assist_tests.rs` (the data's assist is 0 at a
full lean; with the floor a nose under it is lifted and one over it is left alone, also above the target; the data's assist returns with the front wheel on the road).

## 13. Round 3: the turn law

Playtest round 3 ("less steering radius when slow, more when fast") replaced the speed authority of section 11 by the turn law: the assist is the part of the
lateral acceleration that the lean does not give, towards a circle that one table names at each speed (2.2 m at 15 km/h, 4.5 at 30, 15 at 50, 40 at 80, 90 at 120, 225 at 180),
the path follows the stick at a low speed and the lean-in is quicker there. See [ride6-bike-turn-radius.md](ride6-bike-turn-radius.md). Section 11's `authority`
stays for a steering without a turn law (`turn: None`); `bike_steering = reference` is unchanged.

## 14. Round 4: the fast turns and the brakes

Round 4 ("bikes should brake way more and also going faster than 100 kph turn way more") retuned the turn law's table above 100 km/h (about half the radii,
2.4 g of assist at speed, see [ride6-bike-turn-radius.md](ride6-bike-turn-radius.md), section 9) and added lever 11, the brake help `ArcadeSteering::brake` (a
force at the centre of gravity that makes up the deceleration the levers aim for, and a hold against the nose-over; [ride6-bike-braking.md](ride6-bike-braking.md)).
`ArcadeBrake::REFERENCE` is neutral and is part of `ArcadeSteering::REFERENCE`.
