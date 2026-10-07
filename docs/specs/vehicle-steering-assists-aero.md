# Vehicle assists, drive split, aero and steering

Second half of the chassis spec: continues [vehicle-suspension-tires.md](vehicle-suspension-tires.md)
(conventions, per-step order, springs, tire model are there; section numbers continue from it). Covers
the traction/stability assists, drive torque split, burnout and drift, aerodynamics, steering and
Ackermann geometry, airborne stabilisers, sleep damping, player tunings, and the full list of AttribSys
fields read by the whole chassis.

- **Sources read, data inputs, evidence tags:** as in
  [vehicle-suspension-tires.md](vehicle-suspension-tires.md). Open questions are listed there.

## 4. Assists and engine coupling

### 4.1 `TuneWheelParams` (per step) [decomp]

Runs inside the wheel-forces step, after steering and before the wheel loop.

```
yaw_control state (steering.yaw_control, 0.5..1):
    ebrake >= 0.5:  yc -= 20*dT, floor 0.5       else  yc += 1*dT, cap 1
yc_eff = yc * (1 - drift.value)
brake_by_axle = {brake, brake};  if tuning BRAKES = b: front *= (1 + 0.5b), rear *= (1 - 0.5b)
yaw_limit = YAW_CONTROL curve (tires class) sampled by speed / max_speedometer
            (linear interpolation among the 1 to 4 entries of the array); drag race: 0.1;
            +2.5 if the player's stability control setting is off
```

Per tire:

- `brake`: normally `brake_by_axle[axle]`; but with `gas > 0.8`, `brake > 0.5` and `|speed| < 10 mph` on a
  *driven* wheel (the launch-control burnout), the brake is `|speed_mph| * 0.05` instead.
- `ebrake`: rear wheels only (+0.5 extra when `ebrake > 0.2`, `slipangle > 0.3 rad` and `speed < 80 mph`),
  front wheels 0.
- **Yaw friction boost (stability assist) on rear wheels:**
  ```
  boost = 1 + |grade|                         grade = forward vector's y component
  unless (ebrake > 0.5 and |slip| < 20 deg):  boost += min(|car_slip_rad| * yaw_limit * speed_factor, 0.35)
  speed_factor = speed / 30 (m/s)             (0 at 0, 1 at 30 m/s, extrapolates above)
  traction_boost *= yc_eff * (boost - 1) + 1
  ```
  So while a rear slides sideways, its friction limit rises (up to +35%) in proportion to the body slip
  angle: a built-in oversteer catcher, weakened by the handbrake and by drift state. Front wheels get
  nothing.
- (speed-break) front wheels: `lateral_boost = 1 + level*|steer|*0.75`, also multiplied into
  `traction_boost`.
- `traction_boost *= nos_boost * shift_boost` (nitrous and perfect-shift drag boosts).
- Handling tuning h: `traction_circle = (1 + 0.2h, 1 - 0.2h)` (lateral up, longitudinal down for h > 0).
- Blown tire: `ebrake = 0`, `brake = 1`, `traction_boost *= 0.3`.
- If `traction` stays below 1 for a long burnout: see § 4.3.
- Track the largest `slip` over the four tires and which one (for the burnout state).

After the loop: burnout update (first gear, `gas >= 1`, not drag race) or reset; staging
(`flags` bit 0) multiplies every tire's `traction_boost` by 0.25; then `DoDrifting` (§ 4.4).

### 4.2 Drive torque split (`DoDriveForces`) [decomp]

Input: `drive_torque` from the transmission (N m at the wheels, already gear-ratio multiplied; the
transmission spec owns that). Return early if it is 0.

```
center diff: factor = DIFFERENTIAL[2]; bias = TORQUE_SPLIT   (0 = rear drive, 1 = front drive)
   if factor > 0: speed-sensitive split from front/rear mean wheel speed (as the axle diff below,
                  with "has_traction" = any wheel of the axle grounded)
   else:          split = (TORQUE_SPLIT, 1 - TORQUE_SPLIT)       (front, rear)
for each axle a in {front, rear}:
    axle_torque = drive_torque * split[a]; skip if ~0
    diff.factor = DIFFERENTIAL[a], bias 0.5
    fwd_slip = sum over axle tires of split[a]*slip*0.5 ; lat_slip likewise with lateral_speed
    burnout state bit 0 set: right/odd wheel loses grip: boost[1] = burnout_traction, diff bias = traction/2, locked
    burnout state bit 1 set: mirror image
    else traction control (below)
    torque_i = axle_torque * diff_split[i] * traction_control[i], applied only if that wheel is grounded,
               and traction_boost *= traction_boost_i (default 1)
```

Differential split [decomp]: if either wheel is off the ground, the diff is locked, or `factor <= 0`, the
split is `(bias, 1-bias)`. Otherwise with wheel speeds `w0, w1`:
`a0 = w0*(1-bias)`, `a1 = w1*bias`, `s = |a0 + a1|`; if `s > eps`:
`split0 = (1-factor)*bias + factor*|a1|/s`, `split1 = (1-factor)*(1-bias) + factor*|a0|/s`, both clamped to
[0, 1]; else `(bias, 1 - bias)`. (`DIFFERENTIAL` is thus 0 for an open split at the bias and rises toward
a speed-sensitive limited-slip.)

Traction control (not a brake: it moves torque and grip) [decomp]:

```
delta_lat = lat_slip - local_vel.x ; delta_fwd = fwd_slip - local_vel.z
only if delta_lat * steer_input > 0 and axle_torque * fwd_slip > 0:
    lim = Ramp(delta_fwd, 1, 20)
    if lim > 0:
        angle = |steer_input * atan2_deg(delta_lat, |delta_fwd|)|
        lim *= Ramp(angle, 1, 16)
        if lim > 0:
            outer = side opposite to delta_lat's sign
            torque to the slipping-lateral side wheel *= (1 - lim), to the other *= (1 - lim/2)
            traction_boost of the wheel on the cornering inner side *= (1 + lim)
```

(Signs of the sides follow `delta_lat > 0` -> wheel 1 loses torque, wheel 0 gets the boost.)

### 4.3 Burnout / fishtail state [decomp]

State machine with `state` (integer counter), `time`, `allow_time`, `traction`. Updated once per step.

```
active (state != 0):
    if speed > 5 mph and |body_slip_rad| > 0.5:  reset
    elif max_slip < 0.5:  allow_time += dT ; if > 1 s: reset
    else:  allow_time = 0 ; time -= dT ; if time < 0:  state -= 1 ; time = 2 s
inactive and speed < 20 mph and max_slip > 0.5:
    coeff = Graph{(0,1),(5,0.8),(9,0.9),(12.6,0.833),(17.1,0.72),(25,0.65)}(max_slip)
    traction = coeff / 1.4 ; cut = 1.5 - coeff
    state = int(6 * cut + (max_slip_wheel & 1)) ; time = 2 s ; allow_time = 0
```

`state` feeds the drive split (bit 0 / bit 1 pick which side loses grip, so an alternating fishtail when
`state` counts down). Conditions: first gear and throttle fully pressed, outside drag races; otherwise reset.

### 4.4 Drift [decomp]

`drift.value` in [0, 1] and `drift.state` in {out, enter, in, exit}. Reset (and nothing else) when staging
or drag race. Each step: `value += 8*dT` in states in/enter, `-2*dT` in out/exit; clamp. Value <= 0 -> out;
>= 1 -> in. Transition rules (slip angles are body slip, `speed` in m/s):

- when state was > enter (in/exit): `in` if (yaw rate opposes slip, `|slip| <= 0.25 turn`, speed > 30 mph,
  average steering does not match slip direction, `|slip| > 12 deg`) or (`gas*steer*slip > 12 deg` and
  speed > 30 mph); `enter` if `gas*|slip| > 12 deg`; else `exit`.
- else `enter` if speed > 30 mph and (handbrake > 0.5 or `|slip| > 12 deg`).

While `value > 0`:
- (speed-break) charge it while drifting over 35 mph and 30 degrees of slip;
- if yaw rate opposes slip and at least 2 wheels grounded: apply yaw damping torque about up
  `value * (-yaw_rate) * stab(|slip|) * 4 * inertia.y`, where `stab` is the graph
  `{0:0, 15 deg:0.1, 30 deg:0.45, 45 deg:0.85, 60 deg:0.95, 89 deg:1.15, 90 deg:0}` (slip given in radians);
- rear tires get `drift_friction = RearTable(driftcoeff * value)`, with
  `driftcoeff = 0.5*|slip_rad| + 4*countersteer + 0.5*|yaw_rate|` and
  `countersteer = |steer_input|` if the steering has the same sign as the slip, else 0, and
  `RearTable` the 10-entry table over [0, 1]: `1.1, 0.95, 0.87, 0.77, 0.67, 0.6, 0.51, 0.43, 0.37, 0.34`.
  This lowers the static friction limit of the rear (`max_f` in § 3.4 step 10), making drifts sustainable.

## 5. Aerodynamics (`DoAerodynamics`) [decomp]

Applied as two body forces each step.

**Drag** (skip when the drag percentage is 0):

```
drag = speed * drag_pct * DRAG_COEFFICIENT                     speed-proportional coefficient
drag += drag * (2 - 1) * (1 - gas)                              off-throttle doubles it
drag += drag * 0.25 * aero_tuning                              aero_tuning in [-1, 1]
F_drag = -linear_vel * drag                                     opposes motion; |F| = coef * v^2 * ...
applied at the cog; if ground_effect >= 0.5 the point is lowered by 0.1 m * (1 - gas)
```

So drag force grows with the *square* of speed (`speed` in the coefficient times `linear_vel`), and
lifting off the throttle doubles the drag (the "off-throttle" slowdown). `drag_pct = 1 - 0.75*speedbreak_level`.

**Downforce** (skip when `aero_pct` is 0; drag-race driver uses 1.5, otherwise 1):

```
upness  = body_up.y clamped >= 0; if ground_effect >= 0.5: upness = 1
move_dir = linear_vel/speed (if speed > 1e-4) else body forward
forwardness = max(0.4, max(dot(move_dir, forward), 0) ^ 0.5)         falls off when sliding sideways
downforce = aero_pct * upness * forwardness * speed * 2 * AERO_COEFFICIENT * 1000        newtons
if no wheel on ground: downforce *= 0.8
downforce *= 1 + 0.25 * aero_tuning
force = body_up * (-downforce)  (pushes the car down along its own up axis)
point: cog by default; if ground_effect != 0: z = (front_z - rear_z) * AERO_CG*0.01 + rear_z,
       where front_z, rear_z are the z of wheel 0 and wheel 2 arms (so AERO_CG = % from rear to front)
       y = cog.y, x = 0
```

Downforce is linear in speed, not quadratic [decomp]. `AERO_CG` in percent of the wheelbase measured from
the rear axle toward the front [inferred]. No lift, no wind, no drafting effect is in this class.

## 6. Steering and Ackermann geometry

### 6.1 Human steering (`DoHumanSteering`) [decomp]

Output: front inside-wheel target angle in degrees (then converted to turns). All degrees below.

```
STEER_MAX = 45 deg  (ABSOLUTE_MAX_STEERING)
range(v)  = Table over v = local forward speed (m/s) 0..160, 10 points:
            [40, 20, 10, 5.5, 4.5, 3.25, 2.9, 2.9, 2.9, 2.9]       (gamepad / keyboard; wheel-controller
            "speed sensitive" uses [45, 15, 11, 8, 7, 7, 7, 7, 7, 7]; "insensitive" uses 45 flat)
speedcoef(v) = Table over 0..160 m/s: [1, 1, 1, 0.56, 0.5, 0.35, 0.3, 0.3, 0.3, 0.3]
tb = 1 - (gas + 1 - (brake + ebrake)*0.5) * 0.5               0 at full throttle, 0.5 coasting, 0.75 braking, 1 braking+ebrake
max = range(v) * (1.45 * tb * speedcoef(v) + 1)
max *= RangeCoef(|avg_input|)   with Table over |input| in 0..1 of [1, 1.05, 1.1, 1.2, 1.3, 1.4]
max *= 1 + 0.2 * steering_tuning
post-collision (timer in (0,1]): max *= speed_term*(1 - blend) + blend   (terms below)
counter-steer: if steer_input > 0 and rear tire 3 slip angle > 0, max = clamp(max(max, slip3_deg), .., 45); symmetric with tire 2 for steer_input < 0
elif |avg_input| >= 0.5: max = max(max, last_max)
max = min(max, 45); remember as last_max
target = steer_input * max * STEERING          (STEERING from the tires class), clamped to +/-45
gamepad only: rate-limit target to prev +/- rate*dT, rate = 180 deg/s * STEERING * speedcoef(v) * InSpeedCoef * InputCoef
              (InSpeedCoef: 6-point table over |d steer_input/dt| 0..10 of [1,1.05,1.1,1.5,2.2,3.1], averaged over 0.15 s;
               InputCoef: 6-point table over |avg_input| of [1,1.05,1.1,1.2,1.3,1.4])
store target as prev; (speed-break) blend toward steer_input * 60 deg
```

Post-collision (timer `T` set by a hard hit, from 0 to 1 s): `secs = 1 - T`; if `0 < secs < 1`:
`speed_term = 1 - min(1, fwd_speed / (170 mph*0.7))`, `blend = Graph{(0,0.2),(0.2,0.5),(0.5,0.7),(0.7,1)}(secs)`,
`max *= speed_term*(1-blend) + blend`. A collision with impulse > 10 sets `T = max(T, Ramp(impulse, 10, 40))`.

Averages: `avg_input` is a mean of the *remapped* steering input over the last 0.55 s of game time. The
input remap is a 21-point table over [-1, 1] whose centre is flat (derivative ~3.4 per unit near 0):
values `-1, -0.712, -0.453, -0.303, -0.216, -0.148, -0.116, -0.08, -0.061, -0.034, 0, ...` mirrored. Note
the remapped value is only used for `last_input` and the averages; the target angle itself uses the raw
`steer_input` [decomp: `newsteer` is formed before the remap]. Open question 8.

AI cars: `max = 45 deg` (times `STEERING` unless drag race) and `target = max * steer_input`.

### 6.2 Ackermann split (`ComputeAckerman`) [decomp]

Input `s` = target angle in turns (negative = left). Output: direction vector and angle (radians) for the
left and right front wheels.

```
inside = |s| * 2 pi ; if inside > pi: inside -= 2 pi   (so values > 180 deg wrap)
L = WHEEL_BASE ;  T = TRACK_WIDTH.front
outside = L * inside / (T * inside + L)               (angles in radians; small-angle form of
                                                       cot(out) = cot(in) + T/L)
turning right:  right = +inside,  left = +outside
turning left:   left = -inside,   right = -outside
wheel forward direction = rotate(body_matrix) of (sin(angle), 0, cos(angle))
```

Note the sign for left turns follows from the input sign. The same direction vector is the `f` used in
§ 2.5 and the longitudinal axis of the wheel; rear wheels use the body forward vector. The stored
`steering.wheels[0], [1]` (left, right, radians) drive the visual steering angle and `GetWheelSteer`.
Tire slip angle therefore includes the steer angle through the wheel's own velocity decomposition.

### 6.3 Wall steer (`DoWallSteer`) [decomp]

When touching a wall: records one of two signed values at the contact:

- nose-in: contact in the front 25% of the length (z beyond 0.75*half length), closing speed^2 < 56.25,
  body speed^2 < 6.25 and normal facing against the forward vector -> `wall_nose = sign(n.x) - n.x`.
- side: reversing (`vehicle speed < 0`) and a wall beside the car ahead of / behind 0.75*half length.

Applied while more than 2 wheels are grounded: yaw assist = `steer_input * gas * 0.125 rad/s` times
`|wall|` added to the angular velocity about the body up axis (nose-in, only if throttle is on and the
sign of the assist matches `wall`), or the opposite sign when reversing along a wall.

## 7. Other behaviours in this class

### 7.1 Understeer / oversteer factors [decomp]

`understeer = min(|avg(skid_f)| / speed, 1)` if moving forward, speed > 1 and steering opposes the lateral
skid; `oversteer = min(|avg(skid_r)| / speed, 1)`. For UI and effects only.

### 7.2 Queries exposed to other systems [decomp]

Per-wheel traction, load, slip speed, tolerated slip, lateral speed (`GetWheelSkid`), slip angle, angular
velocity (0 if brake-locked; the rolling rate `road_speed/radius` if sliding on the ground; else `AV`),
compression, on-ground flag, world position, road surface, wheels on ground count, steer angle, max steer.
`GuessCompression(downforce)`: for negative downforce (a push down) `-(downforce*0.25)/spring[axle]` m else 0.
`GetRideHeight(i)` = `RIDE_HEIGHT[axle]` in metres + ride-height tuning in inches.

### 7.3 Airborne stabilisers (`DoJumpStabilizer`) [decomp]

When no wheel is grounded: add a constant extra downward force `mass * 3 m/s^2`, rising by
`(1 - Ramp(alt, 0, 6 m)) * 10 * Ramp(speed, 0, 20 m/s)` m/s^2 if airborne > 1 s, flatter than 16 degrees
from the ground normal, higher than 2 m at some point, falling, below 6 m ("landing gravity"). With fewer
than 2 wheels grounded and body up.y > 0.8: damp angular momentum (not about up) by
`Ramp(speed, 0, 5) * g(|w|)` with `g` the graph `{0:0, 0.4:0.15, 2:5}`. With no wheel grounded and flat
ground below (normal.y > 0.9, up.y > 0.1, alignment > 0.8, altitude > 0): pitch and roll torques of
`mass * Ramp(speed,0,5) * (1 - Ramp(alt,0,15)) * {40 pitch, 20 roll} * (dot - 1)` align the body to the
ground normal. These keep jumps controllable.

### 7.4 Sleep damping (`DoSleep`) [decomp]

Not while staging. If speed < 0.5 m/s, all four wheels grounded, brake or handbrake pressed, no gas, small
spin (< 0.25 rad/s) and no collision this step: damp the body by `1 - speed`, zero all wheel `AV` ("all
asleep"). Else if speed < 1 and spin < 0.25 and no gas: scale lateral velocity and yaw rate by `speed`
and the lateral force and yaw torque by `1 - speed` ("lateral asleep"), so a creeping car does not drift
sideways forever.

## 8. Player tunings (integer sliders per category)

Seven values `STEERING, HANDLING, BRAKES, RIDEHEIGHT, AERODYNAMICS, NOS, INDUCTION`, each a number whose
range is set by the upgrade code (`LowerLimit/UpperLimit`, outside this spec) [decomp]. Effects here:

| Tuning | Effect |
|---|---|
| STEERING `s` | max steering range `x (1 + 0.2 s)` |
| HANDLING `h` | `traction_circle = (1 + 0.2h, 1 - 0.2h)` |
| BRAKES `b` | front brake x `(1 + 0.5b)`, rear x `(1 - 0.5b)` |
| RIDEHEIGHT `r` | added to `RIDE_HEIGHT` (inches), both axles; changes cog height and spring compression |
| AERODYNAMICS `a` | drag x `(1 + 0.25a)`, downforce x `(1 + 0.25a)` |

## Fields read

All via AttribSys (units from the conversions in the code unless marked).

| Class | Field | Units / shape | Used in |
|---|---|---|---|
| `chassis` | `SPRING_STIFFNESS` (AxlePair) | lb/in per wheel | § 2.1 `spring`, `GuessCompression` |
| | `SPRING_PROGRESSION` | 1/m multiplier | § 2.4 |
| | `SHOCK_STIFFNESS`, `SHOCK_EXT_STIFFNESS` | lb*s/in (compress / extend) | § 2.4 |
| | `SHOCK_VALVING`, `SHOCK_DIGRESSION` | in (as m/s), 0..1 | § 2.4 |
| | `SHOCK_BLOWOUT` | x weight | § 2.4 |
| | `SWAYBAR_STIFFNESS` | lb/in on the compression difference | § 2.4 |
| | `TRAVEL` | in | § 2.3 |
| | `RIDE_HEIGHT` | in | § 2.3, § 1.1, § 7.2 |
| | `TRACK_WIDTH` | m | § 1.3, § 6.2 (front) |
| | `WHEEL_BASE`, `FRONT_AXLE` | m | § 1.1, 1.3, 6.2 |
| | `FRONT_WEIGHT_BIAS` | % on the front axle | § 1.1 |
| | `ROLL_CENTER` | in | § 1.1 |
| | `AERO_COEFFICIENT`, `AERO_CG`, `DRAG_COEFFICIENT` | see § 5 | § 5 |
| | `RENDER_MOTION` | visual only | § 2.6 |
| `tires` | `RIM_SIZE` (in), `SECTION_WIDTH` (mm), `ASPECT_RATIO` (%) per axle | | § 1.3 |
| | `GRIP_SCALE` per axle | multiplier of the lateral curve, also divides steer drag | § 3.3, 3.4 |
| | `STATIC_GRIP`, `DYNAMIC_GRIP` per axle | friction coefficients (limit / sliding) | § 3.4 |
| | `STEERING` | multiplier of max steering angle | § 6.1 |
| | `YAW_CONTROL[1..4]` | stability-assist strength vs speed | § 4.1 |
| | `YAW_SPEED` | multiplier on yaw torque | § 2.6 |
| `brakes` | `BRAKES` per axle | ft*lb | § 3.4 |
| | `BRAKE_LOCK` per axle | multiplier for lock threshold | § 3.4a |
| | `EBRAKE` | ft*lb (rear wheels only) | § 3.4 |
| `transmission` | `DIFFERENTIAL[3]` (front, rear, centre), `TORQUE_SPLIT` | 0..1 | § 4.2 |
| `simsurface` | `LATERAL_GRIP`, `DRIVE_GRIP`, `ROLLING_RESISTANCE` | multipliers | § 3.4, 3.9 |
