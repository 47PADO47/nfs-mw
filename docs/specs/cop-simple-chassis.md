# Police simple chassis: tires, brakes and drive

The original gives every police car the simple chassis (`SuspensionSimple`), not the racer chassis the player's car and
the AI racers drive with. It shares the racer chassis' springs, dampers, anti-roll, centre of gravity, aerodynamics and
`YAW_SPEED` yaw scaling, but its tires, brakes, drive and stability rules are its own and much simpler: no racing slip
solver, no drift, no yaw-control assist. This spec covers those parts; the centre of gravity, the 45 to 60 degree
steering range and the first-landing yaw halving are in [npc-chassis-contact-state.md](npc-chassis-contact-state.md).

Sources: the CC0 reference decompilation (`Physics/Behaviors/SuspensionSimple.cpp`, `Physics/Behaviors/Chassis.cpp`),
read for understanding only, and the retail PC executable, whose loaded-tire routine and constants were checked against
it value for value **[PC binary]** (private notes). Evidence tags as in the [docs README](../README.md#evidence-tags).

Angles are in turns (1.0 = 360 degrees) unless marked degrees, as the tire code keeps them. `axle` is 0 at the front.

## 1. Per step

1. The speed-dependent scales of the shared chassis (`max_slip`, lateral `grip_scale`, `traction_scale`, spec
   [vehicle-suspension-tires.md](vehicle-suspension-tires.md) section 3.7) start every tire's step: each tire's traction
   boost is the traction scale, its grip boost the grip scale, its slip limit the max slip, and it may not slip.
2. **Oversteer correction** on the two rear tires, for every driver but a remote one: `oversteer = |body slip angle -
   0.25 * yaw rate|` (the yaw rate in radians per second taken as turns), `grip = ramp(oversteer, 0, 35 deg) *
   ramp(speed, 0, 20 m/s) * 0.15`; each rear tire's traction boost is set to `1 + grip`. **[decomp]**
3. **Drive** (only when the gearbox gives a drive torque): the torque splits by `TORQUE_SPLIT`, half to each grounded
   wheel of an axle. A driven grounded wheel may slip when the car is staging or in reverse, neutral, first or second gear
   with more than half throttle; its traction boost is multiplied by the nitrous boost (and by 0.25 while staging).
   **[decomp + PC binary]**
4. **Brakes:** every tire takes the brake pedal, the two rear tires the handbrake. **[decomp]**
5. **Steering drag reduction** is 0.5 for a police car (an AI racer interpolates 0.23 to 0.5 by its catch-up). **[decomp]**
6. The wheel loop of the shared chassis (spring and damper load, contact, the lateral force clamped to what stops the
   wheel's sideways speed in one step with a quarter of the mass, the drive force along the ground, `YAW_SPEED` scaling
   of the yaw torque) runs each tire's loaded or free rule below.

## 2. A loaded tire [decomp + PC binary]

Inputs: lateral and forward contact speeds `lat`, `fwd`, body speed, load `L` (clamped at 0), the step `dt`, the drag
reduction `k`; the tire's specs `GRIP_SCALE`, `STATIC_GRIP`, `DYNAMIC_GRIP` of its axle, the brake torques, its radius
`r`, its wheel speed `w` (rad/s) and the surface's lateral, drive and rolling grip.

```
if the tire carried no load last step and is not locked: w = fwd / r
fwd_acc = (fwd - last fwd) / dt
traction = 1;  locked = handbrake > 0 and rear
torque = drive torque
if brake + handbrake > 0:
    if |fwd| < 1:  torque += -(brake + handbrake) * L * fwd / r             # stops the wheel at walking pace
    else:          torque += (w <= 0 ? +1 : -1) * (brake * BRAKES[axle] + handbrake * EBRAKE) * 1.3558 * 10
slip_angle = atan2(lat, |fwd|)                                               # turns
slip = (locked or may slip) ? w * r - fwd : 0
slipping = may slip and |slip| > max_slip;  ratio = slipping ? max_slip / |slip| : 1;  traction *= ratio
skid = sqrt(slip^2 + lat^2)
Fy = grip_boost * ramp(|slip_angle|, 0, 12 deg) * 1.54 * L * GRIP_SCALE * traction_boost, against the slip angle's sign
if locked and skid > 1:
    Fy = -1.5 * lat * L * traction_boost * DYNAMIC_GRIP / skid
    Fx = slip * L * traction_boost * DYNAMIC_GRIP / (1.5 * skid)
elif slipping:
    pilot = 0.85 + 0.15 * ramp(body speed, 30 mph, 50 mph)
    Fx = limit(DYNAMIC_GRIP * traction_boost * L * pilot * slip / skid, torque / r)
else:
    Fx = torque / r
if may slip and not slipping and not locked:  Fx += 10 * (wheel_acc * r - fwd_acc) / r      # the wheel's inertia
limit = L * STATIC_GRIP * traction_boost * (2.5 * brake + 1)
if Fx^2 + Fy^2 > limit^2:  scale Fx, Fy and traction by limit / |F|; ratio *= (that scale)^2
if |fwd| > 1:  Fx -= sin(slip_angle) * Fy * k / GRIP_SCALE                   # cornering drag
else:          Fy *= min(|lat|, 1)
if not braking and rear:  Fx *= surface drive grip;  Fy *= surface lateral grip
wheel speed:
    locked:                          wheel_acc = 0, w = 0
    may slip and ratio < 1:          t = ((torque - Fx * r) + last t) / 2;  e = t - 1.5 * w * surface rolling grip
                                     wheel_acc = lerp(0.1 * e - ratio * slip / (r * dt), fwd_acc / r, ratio);  w += wheel_acc * dt
    otherwise:                       w = fwd / r
```

`limit(a, b)` keeps `a` within `|b|`. The lateral force is therefore linear in the slip angle up to 12 degrees and then
flat, capped by the friction circle; there is no load sensitivity curve and no slip-angle peak.

## 3. A tire in the air [decomp]

No load, slip, slip angle or forces. A locked rear tire (handbrake) stops; any other wheel speeds up by the torque over the
wheel inertia times the step.

## 4. The rewrite

`blackbox_vehicle` has the rule as `Tire::update_simple` and `update_simple_free` (`tires/simple.rs`) and runs it, with
the per-step rules of section 1 (`vehicle/simple_tires.rs`), when the simple chassis is selected with `simple_tires` on;
the tire's wheel speed is then its own (the gearbox reads it and does not write it back, as for the rolling chassis of
traffic). Unit and whole-car tests cover the slip-angle ramp, the friction circle, the locked rear skid, the walking-pace
fade, braking to a stop and a steering pulse settling at speed.

**On for every police car.** `CarSim::configure_police` turns `simple_tires` on for cops and patrol cars (it was off
until a play comparison on 2026-11-05; the paragraph below records why it was held back). In that heat 5 chase against
the stock recording: sideways slides of a second or more 0 percent of cop time (stock 0, racer tire 0.16 to 0.76), cops
behind the player at 0.86 of its speed (stock 0.86, racer tire 0.80), hard braking behind the player 6 percent (stock 4,
racer tire 11), top speed at the 99th percentile 99.5 m/s (stock 91). With it on, the rewrite's heat 5 cop loses
the installed catch-up checks: on the straight installed road it holds about 77 m/s in third gear where it reaches
100 m/s on the racer tire, and falls back to the 400 m retire distance. On a flat straight at full throttle both tires
give the same speed, gear and drive force; the loss comes from the pursuit's speed control. The simple tire puts the whole
drive torque on the road, so under the speed governor's request (which grows at the matched acceleration, about
2 m/s² here) the car surges at 5 to 14 m/s², overshoots, and the throttle controller brakes; the simple brakes are 2.5
times the racer tire's, and the governor lowers its request while the car decelerates, so the cop settles low. With the
brake torque at the racer tire's scale it still loses (96.5 m/s at most). The original runs the same governor, throttle
controller and tire, so a part of that loop still differs.

**Steering gains [ours].** A police car's steering controller ([ai-driver-control-pid.md](ai-driver-control-pid.md)
section 3) holds its coefficients at the lower ends of their ranges at speed instead of adapting them
(`Driver::police`); below 10 m/s the speed tables apply as before. The original's controller, its tables, its MIT rule,
its sensitivity and its term rotation match the PC executable, and its cops show an effective steering gain that stays
flat over a cop's life (a fit of the steering input to the heading error in a stock heat 5 recording: 0.68 to 0.83 at
every age) and equal to the rewrite's at the range minima. The rewrite's adaptation instead climbed (0.7 to about 1.3 by
40 s; P from 0.40 to 0.63 and D from 0.10 to 0.21 in a matched run), and the cause of that difference is not found. On a
45 s heat 5 highway stretch cut from the stock recording, the stock cops change their yaw direction 1.76 times a second
(20 Hz samples); the rewrite's cop did 2.20 times with adaptation and 1.97 with the gains held (racer tire; 1.83 with
the simple tire as well).

**Per-tick evidence (stock 60 Hz recording) [ours].** Replaying the throttle controller of
[ai-driver-control-pid.md](ai-driver-control-pid.md) section 2 on an original cop's logged speed and request, tick by
tick, reproduces its logged pedals (mean difference 0.001 over 371 ticks above 20 m/s): the controller is the same.
What differs is the car's answer to the pedal. On a straight at 40 to 75 m/s the original cop decelerates about 76 m/s²
in the tick after a brake above half (the simple tire's brake rule of section 2 gives the same order); the rewrite's cop
on the racer tire, about 26 m/s². Full throttle gives about 11 m/s² in both. With the weaker brake the same controller
needs longer, deeper brake dips to correct an overshoot (brake while accelerating to the request: 13 to 17 percent of
the time at a median 0.33 to 0.43 in the rewrite, 6.6 percent at 0.11 in the original). The racer tire also lets a cop
spin: sideways slides of a second or more take 0.14 to 0.76 percent of the rewrite's cop time, against about none in
the original, whose tire keeps its lateral force past 12 degrees.

The stock cops also slide more than the rewrite's: a body slip above 40 m/s of 0.89 degrees at the median and 3.7 at the
90th percentile, against 0.6 and 2.4, which fits their softer tire.

## 5. Open questions

- Why the rewrite's steering adaptation drifts upward while the original's, with the same rule and constants, does not.
- What keeps a cop on the simple tire from holding its speed in the rewrite's speed loop (section 4). In the matched
  city run (`installed_physical_catchup`) the simple-tire cop keeps pace until a contact with the player, then trails in
  the close action and misses the late-engagement check; in play the cops keep pace (section 4). The brake dips while
  accelerating to the request stay deeper than the original's (median 0.27 against 0.11).
- **Known failing acceptance checks.** With the simple tire, four installed checks fail that pass on the racer tire:
  `installed_catchup` (both cases), `installed_physical_catchup` and `installed_route` (a cop stays in sight too long
  for the lane-search stage). In the catch-up case the governor's request grows about 2 m/s² (the matched target's
  acceleration table) and sits 1 to 2 m/s above the cop's speed; the cop accelerates 9 m/s² at full throttle without
  nitrous (the original 8.5 at 70 to 80 m/s), and the throttle controller dips the brake every few ticks (0.3 to 0.9;
  the original's cops at 70 to 80 m/s near their request: 10 percent of ticks at a median 0.08, gaining 2.6 m/s² on
  average), so the cop holds about 76 to 80 m/s. A model car with the measured full-throttle and full-brake responses
  and the same controller also dips 0.17 to 0.43, so the original's speed near its request must answer the pedal
  more smoothly in a way not yet found. The rewrite's cops also burn nitrous far more often than the original's on this
  run (506 of 2100 ticks; the stock recording shows nitrous in under 2 percent of full-throttle ticks).
- Which chassis the original gives an AI racer; the rewrite gives racers the simple chassis' steering range with the racer
  tire, and leaves that to the race work.
