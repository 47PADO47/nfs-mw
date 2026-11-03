# RIDE 6 bikes, round 11: the stick flicked with the throttle open, and the turn at speed (clean-room spec)

Two reports of the players: "I press the gas and quickly flick the stick left and right and the bike falls down" and "the turning at higher
speeds is still too wide". Both were reproduced on the 288 imported bikes and traced to the RIDE 6 executable (build `5.6.1-1697004`,
image-relative addresses, written in our own words; no decompiled text is kept in the repository). Continues
[ride6-feel-round9.md](ride6-feel-round9.md) (sections 3 to 6) and [ride6-lean-controller.md](ride6-lean-controller.md). Tags: **[B]** read from the
binary, **[D]** data, **[M]** measured here, **[C]** a choice of this project's model. Provenance:
[ride6-physics-B.md](../provenance/ride6-physics-B.md), "Round 11".

## 1. The flick with the throttle open

### 1.1 Reproduction [M]

`bike_flick` (`libs/blackbox-modding/src/content/bike_ride_flick.rs`, ignored, `RIDE6_CARS`): full throttle from t = 0 (the bike's own axis rate opens it,
3 per second in the data), from 0.3 s the stick moves at `1 / rise` per second to +1, then to -1, and so on (rise 0.1, 0.2, 0.3 s; each side held 0 or
0.25 s more), through the share the game gives the stick by speed (`choices::stick_share`, as `command()` does), flat road, 8 s, starting at 0, 5, 10, 15, 20 and
25 m/s: 36 configurations, all 288 bikes. Falls before this round (round 10 build), of 288:

| start | rise 0.1 hold 0 | 0.1 / 0.25 | 0.2 / 0 | 0.2 / 0.25 | 0.3 / 0 | 0.3 / 0.25 |
|---|---|---|---|---|---|---|
| 0 m/s | 9 | 288 | 276 | 286 | 285 | 252 |
| 5 m/s | 185 | 285 | 281 | 268 | 244 | 6 |
| 10 m/s | 13 | 276 | 169 | 81 | 92 | 9 |
| 15 m/s | 0 | 232 | 32 | 7 | 3 | 12 |
| 20 m/s | 0 | 183 | 22 | 11 | 1 | 8 |
| 25 m/s | 0 | 31 | 0 | 36 | 1 | 7 |

Almost every fall is a low side in 1st gear. Round 9's `bike_square_stick` found 0 falls because its throttle only held the speed; here the throttle
is open and the front wheel comes up.

### 1.2 What happens [M]

A trace (Ducati Panigale V4, KTM 450, Yamaha R1; rise 0.2, from standstill): the throttle opens in 1st gear, the load on the front tyre drops from 640 N to 0 within
0.3 s (about 7 m/s), the front wheel hovers or lifts a few centimetres (a pitch of 0.03 to 0.05 rad), the stick has just moved the lean target and the bike is
rolling at 1.4 rad/s. The lean help acts at the front contact and was **zero with the front wheel up**, and its cap with a light wheel was the grip *at that light
load*; the only thing left, round 9's rider balance, was limited to 172 N m. The roll was not arrested, the lean went on to 1.06 rad in 0.5 s (the
fall limit) and the bike fell, with the controller's target still at +-0.28.

### 1.3 What the reference does [B]

The lean help (`0x1450c3ae0`, "Lean help" of [ride6-chassis-round6.md](ride6-chassis-round6.md)) differs from the earlier reading in two places:

1. **The tyre's limit is worked out at the rest load at least.** With the front contact flag of the wheel set, the load given to the two peak functions is
   `max(load, RestLoad.front)` (`RestLoad` is the handler's rest load, `+0xd40`); the cap on the help force is `LeanHelpLimit x peakY(that load) x
   sqrt(1 - (longitudinal force / peakX)^2)`. The model used the load of the moment: 150 N on a front wheel the throttle has unloaded gave a cap near 0.
2. **Without a front contact the help does not stop.** The share that acts as a force at the contact (`+0xd1c`, the balance) is set to 0 and nothing returns early:
   the force the controller wants (limited by the cap, the actual tyre force being 0) is applied as a **pure couple about the centre of gravity**
   (`(1 - share) x (point - CG) x force`, the point being the one under the front hub, `0x145157eb0`). The load for the cap is then `(1 - s k) x RestLoad`, `s` the
   fade by the wheel's distance from the road (the handler's range at `+0x4e8`; chassis `+0x1a28` or `+0x1a2c`) and `k` at `+0x4f8`; for a wheel a few
   centimetres up `s` is 0 and the cap is the grip at the rest load. Round 9's note "the force itself is zero without the front contact" was right about the *force* and missed the couple.

### 1.4 Change [B]

`Bike::apply_lean_help` (`handling.rs`): the cap uses `max(load, rest load)`; with the front wheel off the road and the rear wheel on it, the help is applied as a couple with
the force share 0 at the point one wheel radius under the front hub along the chassis' up, along the horizontal direction of the bike's right. `apply_wheelie_balance` and its
authority (`choices::BALANCE_REACH`: 0.25 m of the rider's mass, the port's own, 172 N m) are no longer used; the constant stays in `choices.rs`, unused. Not modelled:
the fade of the help with the front wheel's ground distance (20 to 35 cm in the data's `LeanHelpFactorAirborneRange`; `+0x4e8` and `+0x4f8` are constructor defaults that were not read): a
bike with the rear wheel up has no help here, as before.

Which part matters, on the six configurations "hold 0.25" at 0, 10 and 20 m/s, rise 0.1 and 0.3 (falls of 288):

| config | before | load floor only | couple only | both |
|---|---|---|---|---|
| 0 m/s, 0.1 | 288 | 273 | 23 | 0 |
| 0 m/s, 0.3 | 252 | 94 | 3 | 0 |
| 10 m/s, 0.1 | 276 | 125 | 29 | 0 |
| 10 m/s, 0.3 | 9 | 14 | 0 | 0 |
| 20 m/s, 0.1 | 183 | 16 | 18 | 0 |
| 20 m/s, 0.3 | 8 | 8 | 3 | 0 |

Both are needed: without the couple a hovering wheel gets nothing; without the floor the light wheel's cap is too small.

### 1.5 Result [M]

`bike_flick`, all 36 configurations, 288 bikes: **0 falls**. With the thumb's arc (`RIDE6_WEIGHT=arc`: the weight lever pulled back by a quarter of the stick's deflection;
0, 10 and 20 m/s, rise 0.1 and 0.3, both holds): 0 falls. Regressions, same bikes: `bike_square_stick` 0 of 288 in all 18 configurations; `bike_wheelie_landing` 0 of
288 at 20 and 35 degrees, stick 0, 0.1 and 0.3 held through the wheelie, from 12 and 20 m/s; `bike_curb` 0 in all 90 configurations; `bike_sweep_extra` with a stick of 1.0
(slalom, corner with the brake, slow turn, launch): 3 bikes flagged (Honda X-ADV, Italjet Dragster 700, Kawasaki GPZ 900 R, all in the corner with the brake at lever 0.5), against 4 in round 9.
Tests: `bike/round11_tests.rs` (a flicked stick with full throttle on a bike strong enough to lift its front wheel, from 0 to 20 m/s: fails with the round 10 help,
passes now; the couple on the rear wheel; the g-force of the limit), and `drive/sim/bike_ride_tests.rs`
(`a_throttle_held_open_with_the_steering_flicked_from_side_to_side_does_not_throw_the_rider`: the same through the game's input layer, keys W and A/D into
`ActionState`, `DriveInput`, `InputState`, `command()` and `BikeRide::respond`).

## 2. The turn at speed

### 2.1 Measurement [M]

`bike_radius` (`bike_ride_radius.rs`): a full stick (rising in 0.5 s, through `stick_share`), the throttle holding the speed with the proportional controller of the
other sweeps, 12 s, the radius over the last 3 s as `v / yaw rate`. The bikes cannot hold the speed in the turn (the Aprilia RS 250 loses a third of it), so the radius is
given at the nominal speed from the lateral g reached (`R = v^2 / (g a)`):

| bike | quantity | 20 m/s | 30 m/s | 40 m/s | 50 m/s |
|---|---|---|---|---|---|
| KTM 450 | lateral g before / after | 0.92 / 1.00 | 0.92 / 0.99 | 0.83 / 0.98 | 0.78 / 0.97 |
| | radius (m) before / after | 44 / 41 | 100 / 93 | 197 / 166 | 327 / 263 |
| Yamaha R1 (2020) | g | 0.98 / 1.07 | 0.98 / 1.07 | 0.84 / 1.06 | 0.83 / 1.06 |
| | radius | 42 / 38 | 94 / 86 | 194 / 154 | 307 / 240 |
| Honda X-ADV | g | 1.18 / 1.18 | 1.15 / 1.15 | 1.11 / 1.14 | 1.00 / 1.14 |
| | radius | 35 / 35 | 80 / 80 | 147 / 143 | 255 / 224 |
| Aprilia RS 250 | g | 1.04 / 1.09 | 1.04 / 1.07 | 0.91 / 1.08 | 0.88 / 1.07 |
| | radius | 39 / 37 | 88 / 86 | 179 / 151 | 290 / 238 |

The data's lean limit, `atan(grip)`, is 0.93 rad for the KTM (tangent 1.34); the model before leaned to 0.88 at 20 and 30 m/s and to 0.80 at 50 m/s.

### 2.2 Why the model was wider than the data [B, M]

Two things of the model held the radius up, and one thing in the data keeps it above round 9's formula:

1. **The g-force that cuts the lean limit is the acceleration along the bike, not the cornering g.** `LeanLimitGForceTuning` (1 at 0 g to 0.9 at 1.1 g for the supermotard
   handler, 0.825 at 2 g by default) is read by the lean limit (`0x14510b180`) from the chassis' `+0x1858`, which the IMU update (`0x14513c270`) writes as the acceleration
   dotted with the rotated vector (1, 0, 0), over the length of gravity; its neighbour `+0x1850` is the same with (0, 1, 0), negated. The chassis frame is x forward,
   y right, z up (round 9), so **`GForce.Y` is the longitudinal g** and `GForce.X` the lateral one. The model fed the curve with the cornering g (`v x yaw rate / g`): in
   every steady turn at 1 g it took 8 to 9 percent off the limit. The reference cuts the limit when the bike brakes or accelerates hard (a friction circle), which is
   what such a curve is for; a limit that fell with the lean's own result would chase itself. `LeanInput::lateral_g` became `long_g`: the body's acceleration along the bike
   over 9.81 (of the last sub-step, `Bike::acceleration`, kept by `shift_weight`), in absolute value [C] (the default of the translator's absolute-value flag was not read).
2. **The stick was held to 0.9 from 40 m/s** (`choices::stick_share`, rounds 9 and 10), because the 1000 cm3 bikes weaved at that speed when the throttle moved. `bike_stick_limit` with a
   stick of 1.0 and the moving-throttle controller of round 8, 10 s, all 288 bikes: 287 hold at 8, 20, 30, 40 and 50 m/s (round 9: 267 at 40 m/s, 274 to 284 at 50); the one that falls is
   the Italjet Dragster 700, whose fall limit equals its lean limit. The weave went with the lean help of section 1 and the whole limit. `STICK_SHARE_AT_TOP` is now 1.0.
3. **What stays belongs to the geometry.** At a lean of 0.93 rad the lateral g is 1.0 for the KTM, not `tan(0.93) = 1.34`: the contact patch of a wide tyre moves inward by the tube
   radius times the sine of the lean, so the arm of the centre of gravity is `(h - r) sin(lean)` and not `h sin(lean)`, and the supermotard rider stays upright over the bike, 12 cm to the
   outside (`Rider.LateralOffset` with `LeanInfluence` 1, [ride6-rider-body-aero.md](ride6-rider-body-aero.md)). A road bike (R1, no hang-off in the data) reaches 1.06 to 1.07 g, a scooter (X-ADV) 1.14 to 1.18.
   Round 9's `v^2 / (g tan(limit))` therefore understates the radius by 15 to 25 percent for every bike; it is not the reference's. The bikes also slow in the turn (the KTM 450 at 50 m/s ends at 33), which is the drag
   of the lean and the 450's power.

### 2.3 The tunable [C]

`choices::STICK_SHARE_AT_TOP` (1.0), with `STICK_FULL_UNTIL` (30 m/s) and `STICK_REDUCED_AT` (40 m/s), stays as the user-tunable knob: a share under 1.0 widens the turn above 30 m/s (0.9 was the
old value, about 20 percent more radius at 40 m/s). The default is the reference's (the whole stick), which the sweep supports. No other constant was added.

### 2.4 Result [M]

Radius at 30 m/s: KTM 450 100 m to 93 m, R1 94 to 86, RS 250 88 to 86; at 40 m/s 197 to 166, 194 to 154, 179 to 151, X-ADV 147 to 143; at 50 m/s 327 to 263, 307 to 240, 290 to 238, X-ADV 255 to 224.
The gain is 0 to 9 percent at 20 and 30 m/s, 3 to 21 percent at 40 m/s and 12 to 22 percent at 50 m/s, where the old share had cut the lean. The sweeps of section 1.5 and `bike_stick_limit` show no new falls.

## 3. What stays open

- The default of the translator flag that decides whether a braking g (negative) also cuts the lean limit; the model cuts for both signs (a g of 1 takes 9 percent off the limit).
  `BRAKE_LEAN_RELIEF` stays (the reference's relief of the brakes in a lean is still not found).
- The fade of the lean help with the front wheel's ground distance (a wheel more than 20 cm up) and the help of a bike whose rear wheel is up too.
- The acceleration of the IMU is the body's kinematic acceleration of one sub-step here, unfiltered; the reference sums its own terms (`0x1450d6660`). A bump puts a spike into the limit for a
  sub-step; the target filters take it out. Not measured on rough ground.
- The Italjet Dragster 700 falls at a stick of 1.0 at any speed (fall limit equals lean limit) and in the corner with the brake; two more bikes fall in that corner (section 1.5).
- The in-game check was not run; the host test covers the chain from the keys to `BikeRide`.

## 4. Tools

`bike_ride_flick.rs` (`RIDE6_SPEEDS`, `RIDE6_RISES`, `RIDE6_HOLDS`, `RIDE6_WEIGHT=arc`, `RIDE6_FALLS=1`; `RIDE6_VERBOSE=1` with `RIDE6_FROM`, `RIDE6_EVERY`),
`bike_ride_radius.rs` (`RIDE6_SPEEDS`, `RIDE6_STICK`), and the sweeps of rounds 8 and 9 for the regressions.
