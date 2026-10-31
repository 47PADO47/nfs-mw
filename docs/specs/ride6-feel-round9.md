# RIDE 6 bikes, round 9: the stick at its end, the load on a leaned tyre, the wheelie, the kerb (clean-room spec)

Round 8 left a stick of 1.0 failing on a quarter of the bikes and worked around it with `STICK_SHARE = 0.8`. This round found why, from the
RIDE 6 executable (build `5.6.1-1697004`, image-relative addresses, written in our own words; no decompiled text is kept in the repository), and
checked three more reports of the players: the stick thrown from side to side, the touchdown after a wheelie, a kerb under the front tyre.
Continues [ride6-feel-round8.md](ride6-feel-round8.md) (its section 7 is closed by sections 1 to 3 here) and
[ride6-lean-controller.md](ride6-lean-controller.md). Tags: **[B]** read from the binary, **[D]** data, **[M]** a choice of this project's model.
Provenance: [ride6-physics-B.md](../provenance/ride6-physics-B.md), "Round 9".

## 1. The controller's steering geometry [B]

**Handlebar angle.** The lean controller (`0x1450c9320`) turns the force it wants at the front tyre into a slip angle with the tyre's inverse
lateral curve (unchanged), then into a handlebar angle. The earlier spec (ride6-lean-controller.md, step 3) said the wanted heading is the
direction of travel turned by the slip *about the steering axis* and projected "along the steering axis". The code does something else:

1. The velocity is that of the point one front-wheel radius below the hub **along the chassis' up axis** (`0x145157eb0`), not that of the tyre's own
   patch. Its part along the **ground normal** `N` (the front contact's) is removed and the rest normalised: `v`.
2. The wanted heading is `h = v - (N x v) s`, in the plane of the ground (our slip sign: `s > 0` when the patch moves to the right).
3. `h` and `N` are rotated into the chassis frame; the handlebar angle is the heading, in the chassis' horizontal plane, of `h` projected onto that plane
   **along `N`** (`0x145158890`: `h' = h - (h_up / N_up) N`, angle `atan2(h'_side, h'_forward)`).

For a bike leaned by `phi` on flat ground `h' = forward + (s / cos phi) side`: the handlebar needs `s / cos(phi)` for a slip `s` *on the ground*. The
model projected along the steering axis, which gives `s`, so at 50 degrees of lean the front wheel got 64 percent of the slip it was asked for
(`libs/blackbox-vehicle/src/bike/lean.rs`, `handlebar_angle(velocity, normal, rotation, slip)`; the velocity point is `Bike::front_velocity`).

**The wanted force.** The first estimate `-tau / h_cg` (`0x1450c8f70`) is divided by `max(cos(lean), 0.05)`, `lean` being the angle of the chassis' right
axis from the ground plane at the front contact (`0x1450c9320`), when the front wheel touches the road and moves faster than 1 m/s; otherwise the
estimate stands. The earlier spec read the divisor as a length. The binary leaves the sign alone (`LeanInput::front_arm`, `lean::lean_arm`,
`choices::MIN_LEAN_ARM`).

## 2. The load on the tyre [B]

The wheel's load (`+0x154`, written by `0x145169380` from the force assembly of `0x145140b30`) is not the suspension force over the cosine of the
axis and the ground normal. The suspension force is turned to point along the ground normal **seen from the side of the bike** (the normal without its
component across the chassis, normalised: `n_s`), scaled so that its part along the fork axis stays the spring's, `F_s = S n_s / c` with
`c = |n_s . axis|` held at 0.5 or more (the code clamps the dot product at -0.5, `0x145140b30` after `0x14514cc40`), and the load is the part of that force
along the ground normal, less the wheel's weight terms. The tangential part of `F_s` is taken out of the tyre force again at the contact, so the chassis
gets the normal part and the tyre's own forces only. Result, with `S` the axial force the model already worked out (spring, damper, coupling, wheel
inertia):

```
load = S x cos(roll) / max(cos(angle between the axis and n_s), 0.5)        cos(roll) = |n_s| = sqrt(1 - (N . right)^2)
```

The model had `S / cos(angle between the axis and N)`, which grows with the lean (2.4 times the right value at 50 degrees, as the round 8 doc
suspected: "2500 N on a 1800 N bike"); the fork now carries `1 / cos^2(lean)` more at a lean for the same weight, as it does on a real bike where the tyre's
lateral force carries the rest of the axial force (`wheel_step.rs`, `load_factor`; the contact keeps the axis for it, `wheel::Contact::axis`).
The raw load is what the tyre model takes (disassembly of the call sites, `0x14516efe9` to `0x14516f045`): the 60-sample average of the wheel data
(`+0x158`) is not used there.

## 3. Results of sections 1 and 2: the stick at 1.0

`bike_stick_limit` (round 8, section 1: a step of the stick held 10 s at a speed that a throttle controller keeps; `RIDE6_STICKS=1.0`), the 288
bikes of the install, bikes that hold it (the `0.2` per m/s controller of round 8, default; the same with a gentler `0.1` and with a constant
throttle of 0.3 in the lower rows):

| speed | round 8 | handlebar geometry | + force over cos(lean) | + load (this round) | gain 0.1 | throttle 0.3 | throttle 0 |
|---|---|---|---|---|---|---|---|
| 8 m/s | 47 | 288 | 288 | 288 | 288 | 288 | 288 |
| 12 m/s | 288 | 288 | 288 | 287 | 287 | 288 | 288 |
| 16 m/s | n.m. | n.m. | n.m. | n.m. | 287 | 287 | 287 |
| 20 m/s | 282 | 281 | 284 | 287 | 287 | 287 | 287 |
| 25 m/s | n.m. | n.m. | n.m. | n.m. | 287 | 285 | 285 |
| 30 m/s | 239 | 233 | 271 | 285 | 286 | 284 | 284 |
| 40 m/s | 22 | 61 | 211 | 267 | 270 | 283 | 277 |
| 50 m/s | n.m. | n.m. | n.m. | n.m. | n.m. (gain 0.05: 284) | 287 | n.m. |

(`n.m.` not measured.) What fails now: at 8 to 30 m/s one to three bikes: the Italjet Dragster 700, whose fall limit equals its lean limit (the wheel's
`LateralGripLimit` is 1.0 and nothing is left above the lean it asks for), and a few others; at 40 m/s the 1000 cm3 sports bikes weave at about 1.5 Hz
(the front wheel leaves the road once a cycle) when the throttle moves: every one of the 21 that fell with the round 8 controller held with a gain of
0.1 and with a constant throttle, so what is left is the throttle's torque spikes (a gear change at lean puts 500 N more on the rear tyre) setting
the weave off, not the lean loop alone. A full throttle held in the lean at the limit is not safe (88 to 95 percent of the bikes, 253 to 275 of 288 at 8
to 40 m/s; at a stick of 0.8, 277 to 288): the rear tyre has nothing left for the drive, the traction control's window being the model's own
(round 8, section 6).

Keyboard manoeuvres at a stick of 1.0 (`bike_sweep_extra`, `RIDE6_STICK=1.0`): a slalom at 12, 20, 30 m/s, a corner with the brake (lever 0.5), a slow
turn at 4 m/s and a start with the stick held: 4 bikes of 288 fall (the Italjet, an Aprilia RS 250 two-stroke, a Honda X-ADV, a Yamaha R7 on one
manoeuvre each), 5 with the lever at 1.0. `BRAKE_LEAN_RELIEF` stays: without it 277 of 288 fall in that corner at a stick of 1.0 (1 at 0.8), so the
reference's own relief is still not found.

## 4. The turning circle, and the share of the stick

A full stick asks for the lean limit, `atan(LateralGripPeak) x LateralGripTolerance` (times the lateral-g tuning, 1 to 0.9 over 0 to 1.1 g), capped by
`MaxLeanAngle` (70 degrees, never reached): **the reference's full-stick radius is `v^2 / (g tan(lean limit))`**, and `tan` of the limit is the grip
(0.88 to 1.2 for a road bike, the lean limit 0.72 to 0.9 rad). Measured on the model at a stick of 1.0 (round 8's `bike_sweep` corners, flat road):

| bike | lean limit | radius at 10 m/s | 20 m/s | 30 m/s | `v^2/(g tan limit)` at 30 m/s | the same at 0.8 of the stick |
|---|---|---|---|---|---|---|
| KTM 450 SMR | 0.87 rad (50 deg) | 9 m | 33 m | 84 m | 77 m | 109 m (the stick of round 8) |
| Yamaha YZF-R1M | 0.86 rad | 9 to 12 m | 34 m | 92 m | 77 m | 110 m |
| Honda X-ADV | 0.90 rad | 10 m | 31 m | n.m. | 69 m | 100 m |

(The measured radius is over the last 3 s and includes the speed the corner costs; the lean limit's own radius is the right column.) So the wide turn of
the report was the 0.8 share: it cost 30 to 40 percent of the radius at every speed. A half stick gives 210 m at 30 m/s and 6 times the full stick's at
10 m/s: the stick maps linearly to the lean target (through its filter, the time scale of the stick and the target's own second-order filter: 0.7 at a
full stick to the right and 0.425 at -1 to the left: the lean comes in slower at a full stick). The share is now `choices::stick_share(speed)` (a choice): a full stick up to 30 m/s, falling to 0.9 at 40 m/s, where section 3 still fails (a stick of 0.9 was held by all 288 bikes at 30, 40 and 50 m/s, 0.95 by 287 at 40); `nfsmw`'s `drive/sim/bike.rs` reads it from the
bike's speed (`command(input, speed)`). The animation's `controls.steering` still takes the full stick. The function can go once the weave at 40 m/s is understood.

## 5. The stick thrown from side to side [M, measured]

`bike_square_stick` (`libs/blackbox-modding/src/content/bike_ride_hazards.rs`, `RIDE6_CARS`): a square wave of the stick (+A, -A) at 0.5, 1 and 2 Hz,
A = 0.5 and 1.0, at 20, 30 and 40 m/s, the throttle holding the speed, 10 s, 288 bikes. The stick moves at the data's rates (25 per second away from the
middle, 6 per second towards it: the crossing takes 1/6 s to the middle). **0 of 288 bikes fall in any of the 18 configurations.** Before sections 1 and 2
the lean could not follow a reversal (the front wheel got 64 percent of the slip at the end of a lean, and the load swung with the lean). In the game the pad adds
what this harness lacks: the stick's analogue path through every amplitude, and the weight lever (the stick's second axis); the report "crashes laterally
almost instantly" came with a build that had none of this round's changes and the 0.8 share.

## 6. The touchdown after a wheelie [M]

`bike_wheelie_landing`: a wheelie held to a pitch (20 or 35 degrees, or as far as the wheelie assist lets the bike go) by full throttle and the weight back, then
let down (throttle 0, weight forward), the stick held at 0, 0.1 or 0.3 (`RIDE6_DURING=1`: through the wheelie too, `RIDE6_START` the speed at the start).
The fall tests do not misfire at touchdown (the angular speed test, the heading test and the lean test all stay quiet in every case that stays upright),
but with the stick held through a wheelie the bike **rolled over while still on its rear wheel**: the lean help acts at the front tyre and is zero with the front
wheel off the road, so nothing resisted the roll (the lean ran from 0.1 to 1.05 rad in 1.3 s with the front wheel up). From 12 m/s, stick 0.3: 19 bikes of 288
fell at 20 degrees and 49 at 35 degrees; from 20 m/s: 3.

The reference's own balance on one wheel was not found: `0x1450c3ae0` ends with a blend of the point where the lean help's force acts (the front contact,
or a point under the centre of gravity: `LeanHelpNeutral*`, constructor defaults, no bike sets them) but the force itself is zero without the front contact.
The model now has **the rider balance the bike on its rear wheel** (`Bike::apply_wheelie_balance`, `choices::BALANCE_REACH`, a choice): with the front wheel
off the road and the rear on it, the roll torque the lean controller asks for (`plan.torque`) is applied to the body, limited to the rider's mass times
gravity times 0.25 m (the largest side offset any bike's rider data states; most state none). Result: 0 of 288 fall after the wheelie at a stick of 0.1 or
0.3 from 12 and from 20 m/s, at 20 and 35 degrees. A stick of 1.0 held through a wheelie is a lean at the limit with full throttle (section 3), not a
wheelie: 127 to 256 of 288 fall.

## 7. A kerb under the front tyre [M, measured]

`bike_curb`: a step across the road (10 and 15 cm, vertical or with a bevel of 3 and 10 cm whose face has its own, tilted normal, as a collision mesh gives
it), met at 0, 20 and 45 degrees to the straight, at 5, 10, 15, 25 and 35 m/s, 4 s, 288 bikes, the vertical rays of the tyres as the game casts them
(`StepGround`, with the lean controller, the fork and the fall tests as in play). **0 falls in the 180 configurations**, and the worst angular speed over
the kerb is 0.6 rad/s (the fork takes the step at the data's damper limit of 5 m/s: `max_speed`). So the library does not throw a bike over a low kerb, and
the fall tests (heading error, angular speed, terrain term) do not see the step. The falls the player sees must therefore come from the host: the walls and steep
faces of `drive/walls.rs` (probes on the body box's outline, a contact impulse written back into the bike's angular velocity by `BikeRide::pull`, and from
there into the angular-speed test at 4.4 rad/s) or the surfaces beside the road. That path was not reproduced here (the places tried in the world had
traffic, toll booths and no kerb); it needs a world test with a collision pack that has a kerb. Reproduced and changed in round 10
(`ride6-feel-round10.md`): a kerb as geometry is never a wall; the falls are the contacts with the road edge's invisible walls.

## 8. In-game check (offscreen, muted)

`nfsmw view-world --drive KTM450 --at 985,4430 --muted --drive-script "12:throttle=0;4:throttle=1;3:throttle=0.5,steer=1"` with a screenshot delay of 18 s:
the bike on the toll plaza's road leans into a right-hand curve at about 80 km/h with the rider leaning with it (`C:\r6\shots\lean-corner-right.png`). The
script clock runs during the world's load, so the script starts with a 12 s idle segment. A steer of -1 for 5 s from the same start runs into the booths.

## 9. What stays open

- The 1000 cm3 weave at 40 m/s with a moving throttle (7 percent of the bikes with the round 8 controller, 6 with a gentle one, 2 to 4 with a steady throttle).
  The model's handlebar filter (a pole at 5.4 to 6.7 rad/s from `HandlebarK`, `HandlebarDamp`) is the binary's (`0x1450f9020`, `0x1450f9120`); multiplying it by 4 removes
  15 of the 21 falls, so its lag is the weakest link; the reference's own steps (`MaxDeltaTime`, `MaxSubsteps`) are still not found.
- The tangential part of the suspension force acts in the binary at the fork's attachment point and the correction at the contact, a roll couple of
  `F_t x height` (about 300 N m per wheel at 50 degrees); the model puts both at the contact. Not modelled.
- The front camber in the binary is the chassis' lean from the ground normal (`0x145150050`), not the tilted wheel's; the model uses the wheel's. Not changed.
- The reference's relief of the brakes in a lean (round 8, section 4) and the lean-based throttle cut of its ECU.
- Falls at kerbs and sidewalks in the game (section 7): needs the world.
- Pitch above 20 degrees is limited by the wheelie assist on some bikes (the Aprilia RS 250 reaches 12 degrees): the sweep counts "reached" per row.

## 10. Tools

`bike_ride_real.rs` (`RIDE6_KNOBS=a,b`, now shared with the sweeps, `bike_tyre_curve_real`, `want` and `rFy` in the trace), `bike_ride_sweep.rs`
(`bike_stick_falls`: why and when each bike falls; `RIDE6_THROTTLE=0.3`, `RIDE6_GAIN=0.1`, `RIDE6_STICK=1.0` for `bike_sweep_extra`),
`bike_ride_hazards.rs` (sections 5 to 7). Tests of the round: `bike/round9_tests.rs`, `lean_tests.rs` (the handlebar geometry and the arm).
