# Vehicle induction, nitrous, brakes and driver input

Second half of the engine and drivetrain spec: forced induction, nitrous, brake torque, driver input
shaping (throttle, brake, steering, handbrake, auto-reverse) and special cases. Companion of
[vehicle-engine-drivetrain.md](vehicle-engine-drivetrain.md), which holds the conventions (units, gear ids,
wheel order, table lookups), the engine, clutch, torque loop, drivetrain split and gearbox logic. Section
numbers are shared across the two files (this file: 4, 5, 8, 9, 10). References like "§3" point to the other
file.

- **Sources read:** as in [vehicle-engine-drivetrain.md](vehicle-engine-drivetrain.md) (dbalatoni13/nfsmw,
  CC0-1.0, decompiled): `EngineRacer.cpp`, `PhysicsInfo.cpp`, `PInput.cpp`, `SuspensionRacer.cpp`,
  `Chassis.cpp`, `Generated/AttribSys/Classes/{induction,nos,brakes,tires}.h`. Read for understanding; no
  code copied.
- **Data inputs:** AttribSys classes `induction`, `nos`, `brakes`, `tires`.

Evidence tags as in the [docs README](../README.md#evidence-tags); everything is [decomp] unless marked.


## 4. Nitrous (`nos`)

| Field | Unit | Use |
|---|---|---|
| `NOS_CAPACITY` | seconds of full burn at the default tuning | duration of a full tank |
| `TORQUE_BOOST` | fraction | extra torque multiplier minus one |
| `FLOW_RATE` | (display) | exposed only; no effect on the maths seen |
| `NOS_DISENGAGE` | seconds | delay after releasing before recharge may start |
| `RECHARGE_MIN`, `RECHARGE_MAX` | seconds for a full tank | slowest and fastest recharge |
| `RECHARGE_MIN_SPEED`, `RECHARGE_MAX_SPEED` | mph | speeds mapping onto min..max recharge |

The car "has NOS" when `NOS_CAPACITY > 0` and `TORQUE_BOOST > 0`. State: `nos_capacity` in 0..1 (starts 1 if
it has NOS), `nos_engaged` in 0..1, `nos_boost` multiplier (1 = off).

Tuning slider `t_nos` in [-1, 1] (from the Performance Tuning screen): `torque_boost' = TORQUE_BOOST *
(1 + 0.25 t_nos)`; `capacity' = NOS_CAPACITY * (1 - 0.25 t_nos)` [decomp].

Per tick:

```
engaged = held button
if gear < first or throttle <= 0 or blown: engaged = false
if (speed_mph < 10 and not already engaged) or (speed_mph < 5 and already engaged): engaged = false

if capacity' > 0:
    if engaged and nos_capacity > 0:
        nos_capacity -= dT / capacity'          # network (remote) cars and "infinite NOS" cheat: no drain
        nos_capacity  = max(nos_capacity, 0)    # AI catch-up cheat can scale drain (down to 0.5x)
        nos_boost     = 1 + torque_boost'
        nos_engaged   = 1
    elif nos_engaged > 0 and NOS_DISENGAGE > 0:
        nos_engaged -= dT / NOS_DISENGAGE
        nos_boost    = 1
    elif nos_capacity < 1 and recharge_rate > 0:
        nos_capacity += dT / recharge_rate      # AI catch-up cheat up to 2x
        nos_engaged   = 0 ; nos_boost = 1
    else: nos_engaged = 0 ; nos_boost = 1
else: nos_capacity = 0 ; nos_engaged = 0 ; nos_boost = 1

recharge_rate (only if the player is allowed to recharge, or the car is not a player's):
    if speed_mph >= RECHARGE_MIN_SPEED and gear >= first:
        recharge_rate = lerp(RECHARGE_MIN, RECHARGE_MAX, ramp(speed_mph, RECHARGE_MIN_SPEED, RECHARGE_MAX_SPEED))
    else 0
```

"Engaged" for other systems means `nos_engaged >= 1`. The chassis multiplies tyre traction by `nos_boost`
(so nitrous adds grip in the model, §10 note). [decomp]

## 5. Forced induction (`induction`)

| Field | Unit | Use |
|---|---|---|
| `LOW_BOOST` | torque fraction | boost at the start of the boosting range |
| `HIGH_BOOST` | torque fraction | boost at redline |
| `SPOOL` | 0..1 of idle..redline | normalised RPM at which a turbo starts to work; 0 means supercharger |
| `SPOOL_TIME_UP`, `SPOOL_TIME_DOWN` | seconds | time to go 0..1 and back |
| `VACUUM` | torque fraction (negative) | drag below the boost threshold |
| `PSI` | psi | gauge value at full boost (display) |

Type: if `HIGH_BOOST > 0` or `LOW_BOOST > 0` then **turbo** if `SPOOL > 0`, else **supercharger**; otherwise
none. For none: spool, boost and psi stay 0.

Tuning slider `t_ind` in [-1, 1] [decomp]:

```
spool_n = SPOOL ; if SPOOL > 0 and tuned:
        range = SPOOL * 0.25 if t_ind < 0 else (1 - SPOOL) * 0.25
        spool_n = SPOOL + range * t_ind
spool_rpm = spool_n * (RED_LINE - IDLE) + IDLE
low  = LOW_BOOST  - LOW_BOOST  * t_ind * 0.25
high = HIGH_BOOST + HIGH_BOOST * t_ind * 0.25
```

Spool state per tick:

```
desired = ramp(throttle, 0, 0.5)
if shifting: desired = 0
if turbo and rpm < spool_rpm: desired = 0
move `spool` toward `desired` at 1/SPOOL_TIME_UP per second (up) or 1/SPOOL_TIME_DOWN (down);
a time <= epsilon means jump instantly.
spool = clamp(spool, 0, 1)
```

Boost (the torque multiplier is `1 + boost`):

```
if rpm >= spool_rpm:
    r      = ramp(rpm, spool_rpm, RED_LINE)
    boost  = r * high + (1 - r) * low
    target_psi = spool * PSI * ramp(boost, 0, max(high, low))
elif VACUUM < 0:
    d      = ramp(rpm, IDLE, spool_rpm)
    boost  = d * VACUUM
    target_psi = d * -PSI * ramp(-boost, 0, max(high, low))
else boost = 0 ; target_psi = 0
induction_boost = boost * spool
psi moves toward target_psi at 20 psi/s
```

A supercharger (`SPOOL = 0`) has `spool_rpm = IDLE`, so it boosts across the whole range. [decomp]

## 8. Driver input (player)

### 8.1 Raw actions to controls

Input comes as game actions with an analogue value 0..1 (gas, brake, steer-left, steer-right, handbrake) or
a button (shift up/down, NOS, speed-breaker, reset, camera). A blocked input source clears the queue.

```
steering = steer_right - steer_left              # each 0..1, so -1..1
gas, brake: values above (1 - DEAD) snap to 1, values below DEAD snap to 0       # DEAD = pad dead zone
handbrake: > 0 sets fHandBrake to the value, else 0
if forced-stop (script or cutscene): all controls cleared, brake = 1 (and velocity zeroed if bit 2 set)
```

`PAD_DEAD_ZONE` is an extern global in the sources read, with no value there (Q4).

### 8.2 Automatic reverse and brake-to-stop

Reverse engages through the brake button when the car is nearly stopped, as in modern racers:

```
v = forward speed (m/s)
if gear != reverse:
    if v < 2.5 and brake > 0 and gas == 0: shift to reverse
else if v > -5 and (brake == 0 or gas > 0): shift to first
if gear == reverse: swap pedals: gas := brake_button ; brake := gas_button
```

Auto-brake when neither pedal is pressed: forward gears and `v < 8`:
`brake = 1 - 0.75 * ramp(v, 0, 8)`; in reverse: `brake = 1 - 0.75 * ramp(-v, 0, 10)` (speeds in m/s)
[decomp]. If the handbrake is pulled the pedal brake is cleared.

### 8.3 Shift requests

`desired = clamp(gear + dir, 1, top)`; ignored from reverse (gear 0), ignored if equal, and while staging a
drag start no shifting above second gear. Manual: `shift(desired)`; automatic: sport shift (§7.5). Note that
`clamp(.., 1, ..)` lets a downshift from first reach neutral.

### 8.4 Automatic vs manual

Automatic is on for AI-controlled "human" cars (autopilot), for any non-drag car when the player's setting
is automatic, and always off for drag-style events and for the manual setting [decomp].

### 8.5 Steering (consumed by the chassis, not the input behaviour)

Steering shaping lives in the chassis step [decomp, `SuspensionRacer`]. For a human-steered car
(gamepad):

```
v = forward speed in the car frame (m/s, local z); u = fSteering
max_deg = SteeringRange(v)                                  # 10 samples over 0..160, degrees
    SteeringRangeData = [40, 20, 10, 5.5, 4.5, 3.25, 2.9, 2.9, 2.9, 2.9]  over v = 0..160 (the table's x unit is not confirmed; the lookup is fed the car-frame forward speed in m/s, Q6)
tb      = 1 - (gas + 1 - (brake + ebrake)/2) / 2                          # 0 when on throttle, up to 0.5/1 on brakes
max_deg *= (1.45 * tb * SteeringSpeed(v) + 1)                             # braking widens the angle
SteeringSpeedData = [1, 1, 1, 0.56, 0.5, 0.35, 0.3, 0.3, 0.3, 0.3]
max_deg *= RangeCoeff(|avg_recent_input|)  with RangeData = [1, 1, 1.1, 1.2, 1.25, 1.35] over 0..1
max_deg *= 1 + 0.2 * t_steering                                            # tuning slider
post-collision damping: for 1 s after a hard hit, scale by speed_coeff*(1-c)+c, c from
        [(0,0.2),(0.2,0.5),(0.5,0.7),(0.7,1.0)] over elapsed time, speed_coeff = 1 - min(1, v / (170 mph*0.7))
countersteer: if steering into the slide, max_deg >= rear slip angle (deg), capped at 45
hard turn held (avg input >= 0.5): max_deg = max(max_deg, last_max)
max_deg = min(max_deg, 45)                                                 # ABSOLUTE_MAX_STEERING
target = clamp(u * max_deg * tires.STEERING, -45, 45)
rate   = 180 * SteeringSpeed(v) * InputSpeedCoeff(|du/dt|) * InputCoeff(|avg_input|) * tires.STEERING
         # InputSpeedData = [1,1.05,1.1,1.5,2.2,3.1] over 0..10 ; InputData = [1,1.05,1.1,1.2,1.3,1.4] over 0..1
angle  = clamp(target, prev - rate*dT, prev + rate*dT)                     # degrees; steering wheel = no rate limit
```

The averages use 0.55 s and 0.15 s windows at 60 Hz. The input value used for the average and coefficient
lookups is first run through a 21-point remap table that flattens small stick deflection (default
"medium": `-1, -0.736, -0.542, -0.4, -0.292, -0.214, -0.16, -0.123, -0.078, -0.036, 0, ...` mirrored; the
other sets are in the source and are less aggressive near the centre). Racing wheels use a separate range
table (`[45, 15, 11, 8, 7, 7, ...]`) or full range if speed-insensitive. Speed-breaker (the slow-motion
"game breaker" effect) lerps the angle toward `steer * 60` degrees. AI steering is `45 deg * tires.STEERING
* input` (no shaping). Left/right wheel angles come from an Ackermann calculation (not specified here:
take geometry from `ecar` wheel positions). [decomp]

## 9. Brakes (`brakes`)

### Fields read

| Field | Unit | Use |
|---|---|---|
| `BRAKES[2]` | ft·lb | maximum brake torque of the front `[0]` and rear `[1]` axle |
| `BRAKE_LOCK[2]` | factor | fraction of that torque that can lock the wheel |
| `EBRAKE` | ft·lb | handbrake torque (rear wheels only) |

### Per wheel each tick [decomp]

```
brake_spec  = BRAKES[axle]  * 1.3558 * 4        # global scale BrakingTorque = 4
ebrake_spec = EBRAKE        * 1.3558 * 10       # EBrakingTorque = 10
brake_value (per wheel) = brake_pedal, biased by the tuning slider t_br:
        front * (1 + 0.5 t_br) , rear * (1 - 0.5 t_br)
ebrake_value = handbrake on rear wheels only; +0.5 on a hard handbrake turn
              (handbrake > 0.2, slip angle > 0.3 rad, speed < 80 mph)
if gas > 0.8 and brake > 0.5 and |speed| < 10 mph on a driven wheel (a launch / brake-torque stand):
        brake_value = 0.05 * |speed_mph|
blown tire: brake_value = 1, ebrake = 0, traction x 0.3
torque_brake = brake_value * brake_spec + ebrake_value * ebrake_spec        # opposes wheel rotation
```

- **Lock-up**: `lock_spec = BRAKE_LOCK[axle] * BRAKES[axle] * 1.3558 * 4`; available torque `T = (value*lock_spec
  + ebrake_value*ebrake_spec) * 1.2`. The wheel locks (angular velocity forced to 0, and held) when
  `T > ground_force * wheel_radius + |wheel_av| * 100` and `T > 1`.
- **Applied torque**: normally `-sign(av) * torque_brake`. When the wheel speed over ground is below 1 m/s
  the torque is `-value * wheel_load * v / radius` instead (a viscous stop that cannot reverse the car), and
  the handbrake additionally cancels the wheel's drive torque proportionally.
- Wheel moment of inertia 10 kg·m², rolling friction 2 (units unknown) [decomp].

## 10. Special cases and cross-system couplings

- **Burnout / fishtail** (first gear, full throttle, speed < 20 mph, slip > 0.5): sets a state that locks the
  diff and cuts one wheel's traction by a table of `slip -> factor` (`0:1, 5:0.8, 9:0.9, 12.6:0.833,
  17.1:0.72, 25:0.65`, divided by 1.4 for the grip), alternating sides for 2 s per step. Out of scope for
  the engine, specified with the tyres.
- **Staging** (drag): tyre traction x 0.25; shifting limited to first/neutral; rev limit stays at redline only
  while staging (outside it a drag car may rev to `MAX_RPM`).
- **Perfect launch**: when the vehicle's perfect-launch value is positive and the throttle is down outside
  staging, the engine is forced to throttle 1, torque = the cached peak torque * NOS boost, no engine braking.
  The launch window for the UI: `range = (REDLINE - IDLE) * 0.25`, centred so its top is at most
  `REDLINE - 500` (first gear only).
- **Drag shift quality** (player only, drag cars): potential PERFECT/MISS/GOOD/UP/DOWN. With
  `perfect_point = shift_up[g] - OPTIMAL_SHIFT * GEAR_RATIO[g+1]`: rpm at or over redline = MISS (-1); in
  `[perfect_point, redline)` bonus `= lerp(1, 0.5, ramp(rpm, perfect_point, redline))` = PERFECT; within 1.5x
  the window below = GOOD. A perfect shift gives a torque boost `lerp(1, 2, boost/3 * time_left)` for 3 s,
  `boost = bonus * 0.75`. Engine heat: gains 0.45/s while above redline, loses 0.1/s otherwise, blows at 1;
  torque `* (1 - 0.75 * ramp(heat, 0, 0.6))` from heat above 0 and gear above first (not in first).
- **Engine blow / repair**: blown = zero throttle, brakes on; repaired = normal.
- **Traffic and spline cars** have their own simplified engines (not read).
- **`acceltrans`** (`AccelFromIdle_*`): audio-only transient settings for the engine sound at acceleration
  from idle (three times in ms, a peak RPM, a peak volume). No gameplay effect found. [decomp]

## How to check it

1. **Induction and NOS**: record boost and tank with time at full throttle, release and re-press; compare
   spool times, the 10 mph NOS minimum and the disengage delay.
2. **Brakes**: measure stopping distance on a straight and the lock-up point; check front/rear bias with
   the tuning slider at both ends.
3. **Steering**: log wheel angle against speed with a held stick; compare with the §8.5 tables and the
   rate limit.
4. **Auto-reverse**: stop on a hill, press brake only, see when reverse engages.
5. **Drag shifts**: time a perfect shift window against the §10 formula.

## Constants

Hard-coded in the game rather than in AttribSys [decomp]:

- NOS: minimum 10 mph (5 once engaged); tuning x0.25. Induction: psi slew 20 /s; spool throttle ramp 0..0.5;
  tuning x0.25.
- Brakes: `BrakingTorque = 4`, `EBrakingTorque = 10`, static-to-dynamic 1.2, lock av factor 100.
- Steering: `ABSOLUTE_MAX_STEERING = 45` deg, base rate 180 deg/s, brake range multiplier 1.45, tables in §8.5.
- Auto-reverse: engage below 2.5 m/s, release above -5 m/s; auto-brake ramps over 8 (forward) and 10
  (reverse) m/s, strength 0.75.
