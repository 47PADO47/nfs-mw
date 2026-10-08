# Vehicle engine, drivetrain, brakes and driver input

How NFS: Most Wanted turns pedals into wheel torque: the engine's torque curve and revs, throttle and rev
limiter, clutch and automatic gearbox, forced induction, nitrous, the differentials, brake torque, and how
raw controls become throttle, brake and steering numbers. Tyre forces, suspension and the rigid body are out
of scope (they belong to `vehicle-physics.md`); this spec stops where torque is handed to a wheel and starts
where controls are read.

- **Sources read:**
  - [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube build), under
    `src/Speed/Indep/Src`: `Physics/Behaviors/EngineRacer.cpp` (engine, clutch, gearbox, NOS, induction, the
    drag-race subclass), `Physics/PhysicsInfo.cpp` (torque lookup, shift points, speedometer, induction and
    NOS helpers), `Physics/Behaviors/PInput.cpp` (player input), `Physics/Behaviors/SuspensionRacer.cpp`
    (only the parts that consume drive torque, brake torque and steering: `DoDriveForces`,
    `Differential::CalcSplit`, `TuneWheelParams`, `Tire::CheckForBrakeLock`, `Tire::UpdateLoaded`,
    `CalculateMaxSteering`, `CalculateSteeringSpeed`, `DoHumanSteering`), `Physics/Behaviors/Chassis.cpp`
    (control read-out), `Interfaces/Simables/{IEngine,ITransmission,IInductable}.h`,
    `Generated/AttribSys/Classes/{engine,transmission,induction,nos,brakes,acceltrans,tires}.h`,
    `Sim/Util.h`, `Misc/Table.cpp`, `Tools/Inc/ConversionUtil.hpp`. Read for understanding; no code copied.
- **Data inputs:** AttribSys `GLOBAL/attributes.bin` classes `engine`, `transmission`, `induction`, `nos`,
  `brakes` (and `tires`, `chassis`, `pvehicle` for the links between them). `acceltrans` is read here only to
  record that it is audio data. Field layouts are in [cardata.md](../formats/cardata.md) once an AttribSys
  reader exists; the field names below are the class definitions' own.

Evidence tags as in the [docs README](../README.md#evidence-tags). **Everything here is [decomp] unless
marked**: nothing was measured in the running game. See "How to check it" for what to measure. Where the
decompilation is a GameCube build and the PC build may differ, the text says so.

## Conventions

- Units in the **formulas** are SI-like as the game uses them internally: RPM for engine speed in the
  AttribSys data, rad/s for the simulated angular velocities, N·m for torque, m/s for speed, seconds for
  time. AttribSys torque is in **ft·lb**; speed limits are in **mph**; wheel/tyre sizes are inch and mm.
- Conversions [decomp]: `rps = rpm / 9.5492958`; `rpm = rps * 9.549296`; `N·m = ft·lb * 1.3558`;
  `hp = ft·lb * rpm / 5252`; `hp(from N·m) = N·m * 0.7376 * rpm / 5252`; `mph = m/s * 2.2369`;
  inch = 0.0254 m.
- `ramp(x, a, b) = clamp((x - a) / (b - a), 0, 1)`; `lerp(a, b, t) = a + (b - a) t`.
- **Gear ids**: 0 reverse, 1 neutral, 2 first, 3 second, ... `GEAR_RATIO[i]` is indexed by gear id, so
  entry 0 is the reverse ratio, entry 1 (neutral) is 0, entry 2 is first gear. Top gear id =
  `len(GEAR_RATIO) - 1`. [decomp]
- **Wheel indices**: 0 front left, 1 front right, 2 rear right, 3 rear left (as in
  [car-assembly.md](car-assembly.md)). A car is "front driven" if `TORQUE_SPLIT > 0` and "rear driven" if
  `TORQUE_SPLIT < 1` (both for an all-wheel-drive split strictly between 0 and 1).
- **Table lookup** `table(x; min, max)` used by the steering curves: N evenly spaced samples between `min`
  and `max`, linear interpolation, clamped to the end values outside the range.
- **Curve lookup** `curve(points, x)` for AttribSys arrays with `n` entries: the first entry is at
  `lo`, the last at `hi`; `pos = (n - 1) (x - lo) / (hi - lo)`, `i = floor(pos)`, `t = pos - i`, result
  `lerp(a[i], a[min(i + 1, n - 1)], t)`.
- Update rate: the simulation runs a fixed step `dT`; the 60 Hz window length in the steering averagers
  suggests 60 Hz [decomp; the step itself is in the physics scheduler, not specified here].

## 1. Inputs consumed per tick

The chassis reads the driver's controls once per tick and hands them to every subsystem [decomp]:

| Control | Range | Meaning |
|---|---|---|
| `fGas` | 0..1 | throttle pedal |
| `fBrake` | 0..1 | brake pedal |
| `fHandBrake` | 0..1 | handbrake |
| `fSteering` | -1..1 | steering, + = right |
| `fNOS` | bool | nitrous held |
| (derived) gear | id | from the transmission |

Override when the engine is blown or the car is destroyed: `gas = 0`, `brake = 1`, `handbrake = 1`
[decomp].

**Update order inside the vehicle tick** (what matters for results) [decomp]:

1. Input behaviour produces the controls (§8, other file).
2. `EngineRacer` runs: throttle, NOS, speed limiter, induction, shifting, then the torque integration (§2-§7).
   It reads wheel angular velocities written by the **previous** tick's chassis step, and writes
   `drive_torque` and sometimes wheel angular velocities.
3. The chassis reads `drive_torque`, splits it across axles and wheels (§6), applies brake torque (§9), then
   steers and integrates the tyres.

The spec therefore describes a one-tick feedback loop: engine speed follows the wheel speeds of the tick
before.

## 2. Engine state and torque

### Fields read (`engine`)

| Field | Unit | Use |
|---|---|---|
| `TORQUE[]` (up to 9 entries) | ft·lb | torque curve, evenly spaced in RPM, see below |
| `IDLE` | rpm | idle speed and the curve's first sample |
| `RED_LINE` | rpm | rev limiter; last usable curve point |
| `MAX_RPM` | rpm | the curve's *last sample* position; also the limiter when `UseRevLimiter` is false |
| `FLYWHEEL_MASS` | (mass, unitless in the data) | engine inertia |
| `ENGINE_BRAKING[]` (up to 3) | fraction 0..1 | closed-throttle drag as a share of the torque at that RPM |
| `SPEED_LIMITER[2]` | mph | `[0]` = speed where the governor starts, `[1]` = width of the cut-off ramp |

### Torque curve

```
torque_ftlb(rpm):
    rpm = clamp(rpm, IDLE, RED_LINE)
    if len(TORQUE) <= 1: return 0
    return curve(TORQUE, rpm; lo = IDLE, hi = MAX_RPM)        # lerp between neighbours
```

Note the asymmetry [decomp; open question Q1]: the samples are spaced between `IDLE` and **`MAX_RPM`** but
the lookup argument is clamped at `RED_LINE`, so the part of the curve between `RED_LINE` and `MAX_RPM` is
never reached.

Final engine torque each tick, in N·m:

```
engine_torque(rpm) = torque_ftlb(rpm) * 1.3558
                   * (1 + induction_boost)                    # §5, 0 when naturally aspirated
                   * nos_boost                                # §4, 1 when off; skipped for remote (network) cars
```

The drag-race subclass multiplies further by a shift boost and a heat penalty (§10).

Reported horse power: `hp = engine_torque(rpm) * throttle * 0.7376 * rpm / 5252`.
Idle-power report: `torque_ftlb(IDLE) * IDLE / 5252`.

### Engine inertia

```
inertia = (FLYWHEEL_MASS * 0.025 + 0.25) * (1.0 if gear != neutral else 0.35)
```

Units: this is the divisor for torque in `d(omega)/dt = torque / inertia` (§3); effectively kg·m².

### Engine braking (closed throttle)

```
braking_torque(torque, rpm):
    if len(ENGINE_BRAKING) > 1:
        load = curve(ENGINE_BRAKING, clamp(rpm, IDLE, RED_LINE); lo = IDLE, hi = MAX_RPM)
        return -torque * clamp(load, 0, 1)
    else:
        return -torque * ENGINE_BRAKING[0]
```

`torque` is the full engine torque of the current RPM (including induction and NOS multipliers). The
negative sign is later flipped when the driven wheels spin the wrong way (§3).

### Cached values

At spawn and on `Reset` the game computes once [decomp]: the shift points (§7.1), the peak torque and its
RPM, and the peak power `mMaxHP`. The helpers that scan the curve for the peak (`MaxInductedTorque`,
`MaxInductedPower`) are not in the sources read here: take them as "sample the curve, including the full
induction boost, over idle..redline and keep the maximum" and verify (Q2).

## 3. Engine speed, clutch and the torque loop

State per car: `omega` (engine angular velocity, rad/s), `omega_trans` (what the engine would turn at if
locked to the wheels), `rpm_display` (smoothed, what the tachometer shows), `clutch` (state and timer),
`gear`, `shift_timer`, `throttle`.

Reset state: `omega = idle_rad`, `omega_trans = 0`, clutch engaged, gear = first, throttle 0.

### Clutch

Three states: ENGAGED, ENGAGING (timer counts down), DISENGAGED. Clutch factor returned each tick:

```
ENGAGED     -> 1.0
ENGAGING    -> 1 - 0.75 * ramp(timer, 0, engage_time)       # goes 0.25 -> 1.0 as timer runs out
DISENGAGED  -> 0.25
```

`Disengage()` only acts from ENGAGED. `Engage(t)` only acts from DISENGAGED: becomes ENGAGING with timer = t
(ENGAGED at once if `t <= 0`). When the timer reaches 0 in ENGAGING the state becomes ENGAGED.

Per-tick clutch commands [decomp]:

- Neutral: disengage.
- First or reverse: engage with **0.05 s** if the engine or transmission speed exceeds
  `idle_rad + rpm_to_rad(800)` (reverse: threshold `idle + 2400 rpm`) or `throttle >= 0.1`, otherwise
  disengage (a launch clutch that slips at idle).
- Any other gear: engage with **0.25 s**.
- A gear change disengages (§7.2); the next tick re-engages it per the rules above.

### One tick of the torque loop

Preconditions: needs the input and suspension behaviours and exactly four wheels, otherwise the tick is
skipped [decomp].

```
min_w  = rad(IDLE)
max_w  = rad(rev_limit)         # rev_limit = RED_LINE normally; MAX_RPM for a drag car outside staging
dir    = -1 if gear == reverse else +1
ratio  = GEAR_RATIO[gear] * FINAL_GEAR * dir
rpm    = rpm_of(omega)

# automatic-transmission style torque converter, optional
if TORQUE_CONVERTER > 0:
    conv = TORQUE_CONVERTER * throttle * (1 - ramp(rpm, IDLE, peak_torque_rpm))
    if shifting: conv *= clutch_factor
    ratio *= 1 + conv

if ratio == 0 and gear != neutral: stop this tick

engine_t  = engine_torque(rpm)
braking_t = braking_torque(engine_t, rpm)
(perfect launch override, §10)

omega_trans_old = omega_trans
omega_trans     = min_w + diff_w_locked * ratio * (max_w - min_w) / max_w
trans_accel     = (omega_trans - omega_trans_old) / dT

# if the wheels are driving the engine faster than the engine drives them, braking torque
# changes sign so it still opposes the motion
if gear != neutral and clutch ENGAGED and braking_t * axle_w * dir > 0: braking_t = -braking_t

total_t = engine_t * throttle + braking_t * (1 - throttle)
engine_braking_flag = total_t < 0
wheels_ratio = max(0.25, wheels_on_ground / 4)
```

Here `diff_w_locked` is the "locked" average driven-wheel angular velocity and `axle_w` the "free" one
(§6.4). Then, for any gear but neutral:

- **ENGAGED**: `drive_t = total_t`; `road_t = -total_t * wheels_ratio`. A second step adds a correction so the engine and the wheels agree on acceleration:

  ```
  ae       = (total_t + road_t) / inertia
  diff     = ae - trans_accel
  response = 1/inertia
  if front driven: response += TORQUE_SPLIT       * 0.1 * ratio^2 * 0.5
  if rear driven:  response += (1 - TORQUE_SPLIT) * 0.1 * ratio^2 * 0.5
  r = diff / response
  drive_t += min(r, 0)
  road_t  -= r * wheels_ratio
  ```


- **ENGAGING** (slipping): `d = clamp(omega - omega_trans, -300, 300)`; stiffness 20, halved if the RPM
  difference just changed sign and both its magnitudes exceed 2000 rpm. `clutch_t = d * stiffness *
  clutch_factor`; `drive_t += clutch_t`; `road_t -= clutch_t * wheels_ratio`.
- **DISENGAGED**: no coupling.

When ENGAGED with at least one wheel on the ground [decomp]:

- If `throttle > 0.2`, drive-wheel slip (in the direction of travel) `> 0.1`, `gear <= first` and
  `omega_trans < omega`: lock the engine to the wheels: `omega_trans = omega` and set the driven wheels to
  `omega / ratio` (differential writeback §6.4).
- Otherwise `road_t += clamp((omega_trans - omega) * 20, -|total_t|, +|total_t|)`.

In first or reverse a clutch-play correction runs [decomp]: if `road_t < total_t` and
`total_t + road_t < 0`, look up `play = ClutchPlay(1000 * (total_t + road_t))` in the table below and add
`play * (total_t + road_t) * CLUTCH_SLIP` to `road_t`.

| x (N·m·1000) | -10 | -7.5 | -3.5 | -0.3 | -0.05 |
|---|---|---|---|---|---|
| factor | 1.0 | 0.96 | 0.925 | 0.875 | 0.0 |

(piecewise linear, clamped to the end values).

Launch drag: in first gear or reverse, engaged, throttle > 0 and `road_t * total_t < 0`:

```
slip  = CLUTCH_SLIP * throttle * (1 - throttle * ramp(rpm, IDLE, peak_torque_rpm))
road_t *= (1 - slip)^2
```

Integration and limits [decomp]:

```
alpha  = (total_t + road_t) / inertia
omega  = clamp(omega + alpha * dT, min_w, max_w)
if ratio != 0: free-wheel limit: for any driven wheel off the ground,
        its angular velocity is clamped to +/- (max_w / ratio), and zeroed if the signs disagree
if omega_trans > max_w and ratio != 0:
        if drive_t * ratio > 0: drive_t = 0           # limiter cuts drive
        omega_trans = max_w; driven wheels set to max_w / ratio
drive_t *= 1 + 0.5 * catchup_cheat   (only when drive_t > 0 and an AI catch-up cheat exists; 0 for humans)
drive_torque = drive_t * ratio * GEAR_EFFICIENCY[gear]        # N·m to the chassis
rpm_display  = smooth(rpm_display, rpm_of(omega))
```

Constants [decomp]: `ClutchStiffness = 20`, `ClutchLimiter = 300` (rad/s), clutch engage time 0.25 s
(0.05 s for first/reverse), idle-clutch RPM 800, seize RPM 2000.

An engine **sabotage** (used for a scripted breakdown) multiplies `omega` by
`1 + 0.5 sin(12 * t_remaining)` each tick until the timer ends and the engine blows. Blown: throttle is 0.

### Smoothed RPM (tachometer, audio)

```
max_decel = -( 15 if (shifting and gear > first) or gear == neutral else 2.5 ) * 1000 / inertia   # rpm/s
new = rpm_of(omega)
if (new - old) / dT < max_decel: new = max(old + max_decel * dT, new)
rpm_display = 0.55 * new + 0.45 * old
```

### Throttle path

`throttle = fGas` (0 if blown). Then the speed governor (§3.1) may scale it. Throttle is stored per tick and
used in torque, induction, NOS, shift logic.

#### 3.1 Speed limiter (ECU)

If gear > neutral and `SPEED_LIMITER[0] > 0` and `SPEED_LIMITER[1] > 0`:

```
v   = speedometer()                       # §7.4, m/s
lim = mph_to_ms(SPEED_LIMITER[0]); band = mph_to_ms(SPEED_LIMITER[1])
if v > lim: throttle *= 1 - clamp((v - lim) / band, 0, 1)
```

The speedometer function (§7.4) separately clamps its result to `SPEED_LIMITER[0]`.

## 6. Transmission data and torque to the wheels

### Fields read (`transmission`)

| Field | Unit | Use |
|---|---|---|
| `GEAR_RATIO[]` (up to 9) | ratio | indexed by gear id (reverse, neutral = 0, first, ...) |
| `GEAR_EFFICIENCY[]` (up to 9) | 0..1 | per gear driveline efficiency |
| `FINAL_GEAR` | ratio | final drive |
| `TORQUE_SPLIT` | 0..1 | fraction of drive torque to the **front** axle |
| `DIFFERENTIAL[3]` | 0..1 | lock factor of front diff `[0]`, rear diff `[1]`, centre diff `[2]` |
| `TORQUE_CONVERTER` | factor | automatic launch multiplication (§3) |
| `CLUTCH_SLIP` | factor 0..1 | clutch-play and launch drag (§3) |
| `SHIFT_SPEED` | s per unit ratio | shift delay (§7.2) |
| `OPTIMAL_SHIFT` | rpm per unit ratio | perfect-shift window in drag races (§10) |

Total ratio at the wheels = `GEAR_RATIO[gear] * FINAL_GEAR` (sign flipped for reverse).

### 6.1 Axle share

`drive_torque` (N·m at the wheels, including gear ratio and efficiency, §3) is split [decomp]:

- Centre diff with `DIFFERENTIAL[2] > 0`: `bias = TORQUE_SPLIT`; use the lock formula below with
  `has_traction[front] = wheel 0 or 1 on ground`, `has_traction[rear] = wheel 2 or 3 on ground`,
  `omega[front] = w0 + w1`, `omega[rear] = w2 + w3`.
- Otherwise `front = TORQUE_SPLIT`, `rear = 1 - TORQUE_SPLIT`.

### 6.2 Differential lock formula (`CalcSplit`)

For a pair `(a, b)` with `bias` (share given to `a` when open), lock factor `f`:

```
if either side has no traction, or locked flag, or f <= 0:
    split = (bias, 1 - bias)
else:
    av_a = omega_a * (1 - bias) ; av_b = omega_b * bias ; c = |av_a + av_b|
    if c > epsilon:
        split_a = (1 - f) * bias       + f * |av_b| / c
        split_b = (1 - f) * (1 - bias) + f * |av_a| / c
    else split = (bias, 1 - bias)
clamp both to 0..1
```

So `f = 1` sends torque to the slower wheel; `f = 0` is an open split. [decomp]

### 6.3 Per-axle wheel torque

For each axle with `|axle_torque| > epsilon` (`axle_torque = drive_torque * share`):

- Left/right bias 0.5; `f = DIFFERENTIAL[axle]`.
- Burnout state (§10) forces a locked diff with a shifted bias and cuts the traction of one wheel.
- Otherwise a **traction control** scales torque and traction when the car both steers and slips sideways:
  `d_lat = mean wheel lateral speed - body lateral speed`; if `d_lat * steer > 0` and
  `axle_torque * mean_slip > 0`:
  `lim = ramp(mean_slip - body_forward_speed, 1, 20) * ramp(|steer * atan2_deg(d_lat, |d_fwd|)|, 1, 16)`;
  the outer wheel torque scale is `1 - lim`, the inner `1 - lim/2`, and the other wheel's traction is
  multiplied by `1 + lim`.
- Torque is applied only to wheels on the ground:
  `wheel_torque = axle_torque * split_i * traction_control_i`.

### 6.4 Wheel speed read-back and write-back

`free` driven-wheel speed (used for the sign of braking torque): average of the driven axles' wheel
speeds, taking the larger magnitude when both axles drive. `locked` speed: the same but forced non-negative
(non-positive in reverse); this feeds `omega_trans`. The write-back (used for the lock-up and limiter) shifts
all driven wheels by the difference to the wanted value; below **40 mph**, or when both wheels of an axle are
off the ground, the pair is first averaged (a locked diff) [decomp].

## 7. Gearbox logic

### 7.1 Shift points (computed at spawn and on reset)

For each gear `j` from first to the one below top (`g1 = ratio[j]`, `g2 = ratio[j+1]`):

```
rpm = (REDLINE + IDLE) / 2
search upward in steps of 50 rpm while rpm < REDLINE:
    cur  = torque_ftlb(rpm)
    next = torque_ftlb(rpm * g2/g1) * g2/g1             # torque at the wheels after the shift
    stop at the first rpm where next > cur
shift_up[j]  = that rpm, or REDLINE - 100 if the search ran to the redline
shift_down[j+1] = shift_up[j] * g2/g1                    # 0 if g1 ~ 0
shift_up[top] = REDLINE
```

The decompiled call multiplies by an induction term whose argument order looks swapped; the net effect is
that the induction multiplier evaluates to exactly 1 (Q3), so shift points use the *naturally aspirated*
curve. If there are more gears than ten, no shift points are computed.

### 7.2 Gear change

`shift(gear)` is accepted when the gear is within reverse..top and differs from the current one. Effects:

```
delay = SHIFT_SPEED * GEAR_RATIO[new_gear]
shift_timer = delay * 0.25 if new_gear < current else delay        # downshifts are 4x faster
gear = new_gear ; clutch.disengage()
```

`shift_timer` counts down by `dT`; "shifting" is `shift_timer > 0`. Torque keeps flowing in a shift only
through the slipping clutch (§3). A player shift raises a game event (used by UI/sound).

### 7.3 Automatic shifting

Runs once per tick when the controls say "automatic" (§8.4, other file) [decomp]. Skipped in reverse, while shifting,
while staging, and for 1.25 s after a manual "sport" shift (the timer drains faster with the throttle:
`rate = 2 - gas`). Neutral becomes first at once.

Shift potential each tick (skipped while the clutch is not ENGAGED or staging, then none):

```
potential(gear, rpm = rpm of omega_trans):
    up   = shift_up[gear] ; down = shift_down[gear]
    if gear > first and ratio[gear-1] > 0:
        lower_up = shift_up[gear-1] * ratio[gear] / ratio[gear-1] - 200
        coast    = lerp(IDLE, down, 0.65)
        down     = min(lerp(coast, down, throttle), lower_up)        # downshifts happen earlier off-throttle
    if rpm >= up and gear < top: UP
    elif rpm <= down and gear > first: DOWN
    else NONE
```

Then, from that potential:

- `DOWN`: step down while the full potential above (throttle and coast included, not the raw table: a lift-off in
  sixth fell to third) of that gear at `rpm * ratio[new]/ratio[cur]` is still `DOWN` (a lower gear has the larger
  ratio; reversed, every downshift falls to first) [decomp: `EngineRacer::AutoShift`]. Shift to it (automatic flag).
- `UP`: only if all four wheels are on the ground with wheel slip below 4; shift one gear up.

### 7.4 Speedometer and top speed

```
speed(rpm, gear) = rad(clutch_rpm) * avg_wheel_radius          # m/s, then min'd with the limiter
  clutch_rpm = (rpm - IDLE) / |GEAR_RATIO[gear] * FINAL_GEAR| / (REDLINE - IDLE) * REDLINE
  avg_wheel_radius = 0.5 * (front_diameter + rear_diameter) / 2
  diameter(axle) = inch_to_m(RIM_SIZE[axle]) + SECTION_WIDTH[axle] * 0.001 * 2 * ASPECT_RATIO[axle] * 0.01
  (tires class: RIM_SIZE inch, SECTION_WIDTH mm, ASPECT_RATIO %, axle 0 front, 1 rear)
  result = min(result, mph_to_ms(SPEED_LIMITER[0]))     # if SPEED_LIMITER[0] > 0
```

`GetSpeedometer()` uses `rpm = rpm_of(omega_trans)` in the current gear. `max speed` = speed at redline in
the top gear, clamped to the limiter. Note the `rpm - IDLE` offset: engine RPM is modelled as
`idle + k * wheel_speed`, not proportional to it (`omega_trans = min_w + diff_w * ratio * (max_w - min_w) /
max_w`).

### 7.5 Manual and tiptronic

Manual shifting (drag races and a menu option): the player's shift request calls `shift(gear + 1/-1)` directly,
and nothing else changes gear ([vehicle-manual-shifting.md](vehicle-manual-shifting.md): limiter, neutral, over-revs).
In automatic mode the shift buttons call "sport shift": accepted when the gear differs, is above neutral, not already
shifting, and the request does not contradict the current potential; it sets the 1.25 s hold above [decomp].

### 7.6 Teleport / respawn

`MatchSpeed(v)`: reset everything, pick a gear (reverse if `v < 0`, else the highest gear whose own up-shift
RPM is not exceeded) and set `omega = omega_trans = guessed rpm`, clutch engaged. Guessed rpm:
`clamp(rpm_of(min_w + (|v|/avg_radius) * ratio_total * (max_w - min_w) / max_w), IDLE, REDLINE)` [decomp].

## Constants

Hard-coded in the game rather than in AttribSys [decomp]:

- Torque conversion `1.3558`; hp `5252`; RPM <-> rad/s `9.5492958`.
- Clutch: engage 0.25 s; first/reverse engage 0.05 s; disengaged factor 0.25; stiffness 20; limiter 300;
  idle-clutch RPM 800 (reverse x3); seize RPM 2000.
- Engine inertia `FLYWHEEL_MASS * 0.025 + 0.25`, neutral multiplier 0.35.
- Smoothed RPM: decel 2.5 (15 while shifting/neutral) x 1000 / inertia; blend 0.55 new / 0.45 old.
- Shift: downshift delay x0.25; sport hold 1.25 s; shift-up search step 50 rpm; fall-back `REDLINE - 100`;
  coast factor 0.65; downshift margin 200 rpm; traction check wheel slip < 4.
- Differential: lock below 40 mph; response weight 0.1 (wheel); free-wheel clamp.

## How to check it

Measured in the original game (original install, PC build) to confirm the implementation:

1. **Torque curve**: log RPM and acceleration in each gear on a flat road for the same car as the data;
   check that the torque at `RED_LINE` is the last curve sample (Q1) and that nothing beyond it is used.
2. **Idle and limiter**: hold the car stationary in neutral and check idle RPM; floor the throttle in
   neutral and check the limiter (smoothed RPM) at `RED_LINE`. Hit the limiter in top gear and check the
   cut is a drive cut, not a throttle cut.
3. **Shift points**: with an automatic car at full throttle, record the up-shift RPMs and compare with §7.1.
   Check the "coast" down-shift RPMs at 0 throttle.
4. **Shift time**: time a manual upshift and downshift from throttle-off to throttle-on torque; compare with
   `SHIFT_SPEED * GEAR_RATIO[new]` and the 0.25 factor.
5. **Speed limiter**: a car with a limiter should taper off through the `SPEED_LIMITER[1]` band, not stop.
6. **Differentials**: lift a driven wheel (or use a ramp) and check how torque reallocates; check the 40 mph
   lock threshold by driving a figure eight at low speed.

Checks for induction, nitrous, brakes, steering and input are in
[vehicle-input-induction-brakes.md](vehicle-input-induction-brakes.md).

Open questions Q1-Q8 are listed in [provenance/vehicle-engine-drivetrain.md](../provenance/vehicle-engine-drivetrain.md).
