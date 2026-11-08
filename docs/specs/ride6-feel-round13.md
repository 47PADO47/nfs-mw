# RIDE 6 bikes, round 13: left as quick as right, and the stick at speed (clean-room spec)

Two reports of the players after round 12: "left turns are slower" and "when going fast with the bikes the faster the less steering".
Continues [ride6-feel-round12.md](ride6-feel-round12.md) (section 2, the turning analysis; section 1, the crash sound, is kept as it is) and
[ride6-lean-controller.md](ride6-lean-controller.md). Tags: **[M]** measured outside the model (a script, not in the repository, that follows
the controller's own filters at the game's 1/480 s sub-step: stick filter, time scale, target filter, lean spring, the lean following its
prescription exactly; limit 52 degrees, 80 percent of it as the target angle), **[D]** data (the 288 imported manifests, read with a script),
**[R]** read in this repository's code, **[C]** a choice of this project. **Nothing of this round was compiled or run** (the agent's rule:
the lead builds once at the end): every test named here is unverified, and the in-game check was not made.

> **Superseded in this game by [ride6-arcade-steering.md](ride6-arcade-steering.md):** the floor of 0.65 (section 4) and the stick's reach (section 6) are now fields of
> `ArcadeSteering` (`time_scale_speed_floor` 1.0, `stick_reach` 0.9) with a quicker controller and a steady turn of 1.6 g; `choices::TIME_SCALE_SPEED_FLOOR` and
> `choices::STEER_REACH` are gone. The analysis below still describes the reference game's chain (`ArcadeSteering::REFERENCE`).

## 1. What the players report, and what was asked

1. Left turns lean in slower than right ones: make them the same.
2. At 40 to 90 m/s a full stick should still reach the lean the tyres allow in a sensible time: find every speed-dependent attenuation from the
   stick to the controller, check the 1 g lean cap against RIDE 6's data (maximum lean, tyre friction, lean help, tyre peak at speed) and remove
   any guessed cap the data does not support.
3. Keep or remove `STEER_REACH` (0.9) with a reason.

## 2. The chain from the stick to the lean, and which stage depends on the speed [R, D]

| stage | where | depends on the speed? | verdict |
|---|---|---|---|
| pad dead zone (15 percent, rescaled), sensitivity | `crates/nfsmw/src/input/bindings.rs`, `axis_response` | no (the car's steering table and rate limit are not applied to a bike) | none |
| the stick's reach, the lever shape | `drive/sim/bike.rs`, `command` | **was: `choices::stick_share(speed)`**, 1.0 at any speed since round 11 | the stage is removed (no effect, one speed-dependent stage fewer) |
| the stick's rates (rise 25 per second, fall 6) | `bike/input_axes.rs` | no | none |
| stick filter K 1000, z 2.5 | `lean.rs`, `plan` | no | none |
| lean limit `atan(grip at the rest load, camber 0) x LateralGripTolerance` | `lean.rs`, `lean_limit`; `handling.rs` | no: the tyre's peak has no speed term (section 5) | none |
| limit cut by the acceleration along the bike (`LeanLimitGForceTuning`: 268 bikes 1.0 at 0 g to 0.825 at 2 g, 18 bikes 0.9 at 1.1 g) | `lean_limit` | indirectly: coasting at 80 m/s the drag slows a bike by about 0.7 g, so 6 percent | the data's; kept |
| limit cut by a skid (heading error from 8 degrees, full at 25 or 30, exponent 4) | `lean_limit` | only in a slide | none |
| limit moved by the road's bank, scaled by a speed curve (0 at 10 km/h, 1 at 50) | `lean_limit` | full above 14 m/s, a bank only | none |
| **time scale of the target filter: `LeanTimeScaleSpeedTuning`** | `lean.rs`, `plan` | **yes: 1 at 40 km/h falling to 0.333 at 330 km/h for 270 bikes (the other 18: 0.65 at 200 km/h, flat above)** | **held at 0.65 (section 4)** |
| **time scale: `LeanTimeScaleInputTuning`, a curve of the *signed* stick** | `lean.rs`, `plan` | no, but 0.4 at -1 and 0.5 at +1 (207 bikes), 0.28 and 0.45 (61), 0.425 and 0.7 (20) | **read at the quicker side for both (section 3)** |
| target filter K 1000, z 2.25; lean spring K 1000, z 3.5 to 3.75 | `lean.rs` | no | none |
| roll torque to a front force, slip from the force, handlebar from the slip | `lean.rs` | no: the slip for a force and the handlebar angle for a slip do not contain the speed; the lock is 18 to 42 degrees | none |
| handlebar filter K 1000, z 3 | `handling.rs` | no | none |
| lean help (factor 0.16 flat above 10 km/h, cap `help_limit x peak(max(load, rest load))`) | `handling.rs`, `apply_lean_help` | no: the cap is at the rest load, so the front wheel the drive force unloads at speed does not weaken it | none |
| gyroscopic roll torque `cos(lean) (I_f w_f + I_r w_r) yaw rate` | `handling.rs` | in a steady turn at a given g, `w` goes with the speed and the yaw rate against it: about `I g tan(lean) / r` = 70 N m against the 1400 N m of the lean's gravity | none |

So the only speed-dependent attenuation of the stick is the time scale's speed curve. It does not shrink the lean the stick reaches (that is the
limit's, section 5) but the time to reach it, and together with the signed stick curve it is what a player feels: at 80 m/s a full stick to the left
took 1.8 to 2.3 s for the 0.28 curve (table in section 4), 2 s of straight road is 160 m.

## 3. Left as quick as right [D, C]

The curve over the stick of the time scale is `FRuntimeFloatCurve` over -1 to 1; every bike's is lower at -1 than at +1, so the target filter ran 10 to
40 percent slower to the left (the round 12 numbers). The reading of the binary (signed or absolute) was not checked: the RIDE 6 executable was not
open in the disassembler, and the data states the two sides differently on purpose or by accident. The rule here, `curve::factor_either_side`: the curve
is read at `|stick|` and at `-|stick|` and the greater value is the time scale of **both** directions. So a left stick is exactly the mirror of a
right one, never slower than what the data gives its quicker side, and for every bike of the data the quicker side is the right one (so right
turns are unchanged and left turns come in as the right did). A mod's curve that is quicker at -1 is mirrored the same way.

Tests (`bike/turn_tests.rs`, `bike/curve.rs`): the plan's target, limit and roll acceleration for +stick and -stick are mirrors over 240 steps for six
shapes of the stick curve, two speed tables, three speeds and three sticks; the open-loop time to 80 percent of the limit is equal left and right
(to a millisecond) over the same shapes at 10, 20, 40, 60 and 80 m/s; the 0.28 / 0.45 curve's left is the time of a curve that is 0.45 both ways.

## 4. The speed curve of the time scale held at 0.65 [D, M, C]

The time to 80 percent of the limit after a stick thrown to its end (right / left, seconds), over the three families of the data (A: 207 bikes, stick
0.4 / 0.5, speed curve to 0.333; B: 61 bikes, 0.28 / 0.45, speed curve to 0.333; C: the 18 bikes with the speed curve to 0.65 and 0.425 / 0.7), before
this round (the data's curves) and after (quicker side, floor 0.65) [M]:

| family | | 10 m/s | 20 m/s | 40 m/s | 60 m/s | 80 m/s |
|---|---|---|---|---|---|---|
| A | before | 0.98 / 1.08 | 1.01 / 1.12 | 1.11 / 1.24 | 1.26 / 1.43 | 1.53 / 1.77 |
| A | after | 0.98 | 1.01 | 1.11 | 1.20 | 1.20 |
| B | before | 1.01 / 1.29 | 1.05 / 1.35 | 1.16 / 1.52 | 1.32 / 1.80 | 1.63 / 2.31 |
| B | after | 1.01 | 1.05 | 1.16 | 1.26 | 1.26 |
| C | before | 0.90 / 1.10 | 0.92 / 1.13 | 0.98 / 1.23 | 1.04 / 1.33 | 1.04 / 1.33 |
| C | after | 0.90 | 0.92 | 0.98 | 1.04 | 1.04 |

(the time scale is the product of the speed factor and the stick factor: at 80 m/s a full stick to the left in family B had 0.43 x 0.28 = 0.12, and
the target filter's slow time constant, 0.135 s at a scale of 1, is then 1.1 s.)

Why 0.65: the tyre's roll authority has no speed term (section 5) and the front force the lean help may add is capped at the rest load, so nothing in
the physics asks for a slower lean at speed; the slowing is the reference's pace for its rider. The closed loop was verified only to 50 m/s (rounds 8 to
11, 287 of 288 bikes hold a full stick there), where the curve is at 0.68; 0.65 is the value the 18 bikes of family C keep from 200 km/h, so it is a
number of the data. Under 53 m/s nothing changes (every bike), above it the lean-in time stays that of 53 m/s: 1.0 to 1.3 s to 80 percent of the limit
at any speed. `choices::TIME_SCALE_SPEED_FLOOR`, 0 gives the data's curve back; a table that starts under the floor is not raised (`lean::speed_scale`).
It is a reduction of a slowing, not a speeding up: no bike leans in faster than it did at 53 m/s.

Tests: `the_lean_in_time_grows_little_with_speed_whatever_the_data_tables` (open loop, within 30 percent of the time at 20 m/s and under 1.5 s at 40,
60 and 80 m/s, over the shapes), `a_full_stick_leans_the_bike_in_about_the_same_time_at_20_and_at_80_ms_in_the_game_loop` (closed loop, a superbike-geared
example, stick +1 and -1, 20 to 80 m/s: within 40 percent and a tenth of a second, 40 degrees under 2.2 s).

## 5. The 1 g cap against the data [D, R]

Over the 288 manifests (rest load of the front wheel, the model's own `a1`, `a2` of the front tyre):

| quantity | value |
|---|---|
| the lean limit, `atan(mu)` with mu the peak lateral coefficient at the rest load and camber 0 (times `LateralGripTolerance`, 1 for every bike) | 48.6 to 53.3 degrees, mean 52.1 (mu 1.13 to 1.34, mean 1.28) |
| `MaxLeanAngle`, the upper bound of the limit | 70 degrees for 286 bikes, 55 for 2; never reached |
| the fall limit, `atan(mu) x LateralGripLimit` (1.1 for 269 bikes, 1.2 for 18, 1.0 for 1) | 52 to 64 degrees, typically 57 |
| the block-fall assist's hold of the target (0.975 x 0.99 of the fall limit above 40 km/h) | typically 55 degrees |
| the tyre's peak lateral force over the load, camber 0 / 30 / 50 / 60 degrees (front, all bikes, mean) | 1.28 / 1.35 / 1.39 / 1.41 (the range 1.13 to 1.51); the rear the same |
| the same at 0.5, 1, 2 and 3 times the rest load | 1.29 / 1.28 / 1.27 / 1.26 |
| the speed term of the tyre formula | none: the speed enters the tyre only through the rolling resistance and the slip's smoothing under 1 m/s |

So **no cap is guessed**: the stick can ask for the limit, the limit is the tyre's grip as the reference reads it, and the data's own ceilings sit above
it (fall 57, block-fall hold 55, maximum 70). The choices of this project on the lean path are the stick's reach (section 6, helps), the
brake relief (braking only), the wheelie floor (a wheelie only) and the floor of section 4 (loosens); the lean limit itself, `LATERAL_GRIP_TOLERANCE`, is the data's 1.

What reaches 1.0 to 1.07 g and not the 1.3 of `tan(52 degrees)` is the geometry the data states (round 11, section 2.2): the wide tyre's patch moves in
with the lean and the supermoto rider stays upright 12 cm to the outside. A bike that leans 55 to 60 degrees and corners at 1.4 to 1.7 g has a tyre
that holds 1.4 to 1.7: the data's peaks are 1.34 to 1.51 at the lean's camber, so 1.4 g is the most any bike of the data can have even with a
patch under the centre of gravity, and the chassis lean for it (57 degrees and over) is beyond the fall limit and the block-fall hold of the data. Going there
means a project choice beyond the data (`LATERAL_GRIP_TOLERANCE` 1.05 would give 55 degrees, under the 57 of the fall limit, with a 2 degree margin for the
loop's weave at the limit); it is not done, since the weave of the lean loop at the limit decided the margin in rounds 8 and 11 and nothing can be run
this round.

The radius a full stick holds, `v^2 / (g tan(lean))` with the lean 52 degrees: 32 m at 20 m/s, 127 m at 40, 287 m at 60, 510 m at 80; at the measured
1.0 g (round 11) 41, 163, 367 and 652 m. A corner of 100 m at 40 m/s asks 1.6 g, more than any tyre of the data has: a bike has to brake for it.

## 6. `STEER_REACH` stays at 0.9 [C]

It is not speed-dependent (a key is 1.0 and unchanged, a pad's stick is rescaled by the same factor at any speed), so it is not the reason for "the faster
the less steering". It exists for the pad: the lean target is linear in the stick and the radius goes with `1 / tan(lean)`, so a stick at 0.9 leans to 46.8
degrees (`tan` 1.07 against 1.28) and widens the turn by 20 percent, at 0.95 by 9 percent. A thumb rarely holds the end of a pad's travel exactly, and the
weight lever has the same `REACH` 0.9 (`input/weight_axis.rs`). With the quicker side read for both directions the stick curve of the time scale is at 0.73
for a stick of 0.9 against 0.70 at 1.0 (example bike), so reaching the end earlier costs no lean-in time. Kept.

## 7. What did not change

The crash-sound fixes of round 12 (section 1 there): the double's ground contacts cleared at the start of every step, the "tipped" landing never set for a
two-wheeler, `CarSim::up_dot`. The lean limit, the lean help, the fall tests, the filters' constants.

## 8. To check in the game

A bike at 60 m/s needs a long straight: accelerate with the first segment, then a slalom of full sticks (the drive script,
[drive-script.md](../modding/drive-script.md); `--muted` keeps the engine quiet; the screenshots are offscreen; `--camera-mode chase` shows the lean):

```
nfsmw view-world --drive ducati_panigale-v4-2018 --at 985,4430 --muted --camera-mode chase \
  --drive-script "25:throttle=1;1:throttle=0.7,steer=1;1:throttle=0.7,steer=-1;1:throttle=0.7,steer=1;1:throttle=0.7,steer=-1;2:throttle=0.7" \
  --screenshot out/slalom.png --screenshot-delay 25 --screenshot-interval 1 --screenshot-count 5
```

(`--drive` takes a car folder or a unique prefix, here the imported Panigale V4; the position `--at` must be a road with a straight of 1.5 km, the one of the drive-script page is a
placeholder; the bike runs off the road in the slalom, which does not matter for the lean). A left and a right frame of the same second should show the same lean.
The numbers on the imported bikes (the harness of round 11, ignored without the folder; no cargo this round, the lead runs it):

```
RIDE6_CARS=%APPDATA%\nfsmw\config\mods\ride6-bikes\cars RIDE6_FILTER=ktm450,ducati_panigale-v4-2018 RIDE6_SPEEDS=20,40,60,80 \
  cargo test -p blackbox-modding --features host --lib bike_radius -- --ignored --nocapture
```

## 9. Not verified, open

- Everything above that is a measurement was made outside the model on the filters alone (the closed loop was not run): whether the front tyre and the lean help
  carry the faster lean-in at 60 to 90 m/s without the weave of round 8 is the thing to look at first. If a bike weaves there, raise the floor towards the
  data's curve (it is one constant).
- The closed-loop tests ride the example with a superbike's gearing; their bounds (0.7 to 1.6 times the radius of the lean, 0.85 of the 20 m/s lean) are
  guesses of the same kind as round 12's.
- Signed or absolute reading of the stick curve in the binary (section 3); the quicker-side rule is the safe reading of either.
- A speed-sensitive stick curve (the pad's stick is linear in the lean at any speed) if the players still find a partial stick too coarse at 40 m/s and above.
