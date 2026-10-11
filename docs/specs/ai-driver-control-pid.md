# AI driver control: the PID controllers (racers, cops, autopilot)

The controller used by every AI car except traffic: a velocity-form PID for gas and brake, and an adaptive
PID for steering. Second part of [ai-driver-control.md](ai-driver-control.md) (composition, schedule, the
simple controller, reversing, stuck recovery, the output interface). The speed that the throttle controller
is asked to hold is chosen by the action: [ai-driver-speed-skill.md](ai-driver-speed-skill.md). For the tag
meanings, see [evidence tags](../README.md#evidence-tags).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src`: `AI/Common/AIVehicleRacecar.cpp` (the PID controller class, constants and tables),
  `AI/AdaptivePIDController.h`, `AI/Common/AdaptivePIDController.cpp`, `Misc/Table.hpp`, `Misc/Table.cpp`
  (`PidError`, `AverageWindow`, `Table`, `Graph`), `AI/Common/AIVehicle.cpp` (the base controller it falls back
  to). Read for understanding; no code copied.
- **Data inputs:** none from AttribSys except the suspension's `max steering` (derived from `tires.STEERING`)
  and, for the choice of drag tuning, the driver style.

All maths is planar (y dropped) unless stated. Angles in radians unless marked degrees. "Tick" is one 1/60 s
simulation step; the controller steps every tick with `dT = 1/60 * time scale`.

## 1. Building blocks

### 1.1 Lookup helpers [decomp]

- `Table(data[n], min, max)`: evenly spaced samples over `[min, max]`; the lookup is linear between neighbours
  and clamps outside. `index = (x - min) * (n - 1) / (max - min)`.
- `Graph(points)`: piecewise-linear over `(x, y)` points with increasing `x`; clamps to the first or last `y`
  outside.
- `ramp(v, a, b) = clamp((v - a) / (b - a), 0, 1)`.

### 1.2 `PidError(n_integral, n_derivative, frequency)` [decomp]

A small history of one error signal. `record(e, t)` (t = seconds since the last record):

```
prev = cur;  cur = e;  delta = cur - prev
area = t * (prev + delta/2)           # trapezoid between the two samples (0 if integral is switched off)
slope = delta / t                      # 0 if the derivative is switched off
push t into a ring of n_integral times, area into a ring of n_integral areas, slope into a ring of n_derivative
error()      = cur
integral()   = (sum of areas in ring) * (samples in ring) / (frequency * sum of times in ring)
derivative() = mean of the slopes in the ring (over the samples recorded so far, at most n_derivative)
```

With constant `t` the integral is the sum of the last `n` errors divided by `frequency`, not multiplied by
time. This is not a true time integral; keep the formula. The rings start empty (mean over the samples
present; integral 0 when none). The "switched off" flags are false in every use here.

### 1.3 The adaptive controller [decomp]

`AdaptivePIDController` holds, per term `P`, `I`, `D`: a coefficient `C` (starts at 0), a term value `T`, an
adaptation gain `gamma`, a tuning threshold, a coefficient range, a term clamp (default +-99999, never changed
here). Output:

```
output = C_P * clamp(T_P) + C_I * clamp(T_I) + C_D * clamp(T_D)
```

`update(model_behaviour, actual_behaviour, dT)` is one step of model-reference adaptive control with the
**MIT rule**:

```
model_error = actual_behaviour - model_behaviour
k = floor(world_clock_seconds / time_slice) mod 3        # one term per time slice, in turn: P, I, D, P, ...
cd = 0
if |T_k| >= threshold:
    s = sensitivity(k)
    cd = -gamma_k * model_error * s                      # coefficient derivative
    C_k = clamp(C_k + cd * dT, range_k)
record cd in term k's derivative window, and 0 in the other two terms' windows
record (model_error - last_model_error) / max(dT, 0.001) in the model-error derivative window
last_model_error = model_error
sensitivity(k) = clamp(mean(model_error_derivative_window) / mean(window_k), -1000, 1000)   if |mean(window_k)| > 1e-9
               = 0 if |mean(model_error_derivative_window)| < 0.001, else +1 if it is positive, else -1
```

The windows are time windows of 0.1 s fed once per update (5 slots at the nominal 45 Hz; entries older than
0.1 s are dropped). `world_clock_seconds` is the world timer (wall-clock-like), so the term in turn is not
tied to the tick count. Other adaptation rules (normalised MIT, sign rules, Lyapunov) exist in the class and
are not used. `force coefficient(term, value)` overwrites a coefficient without clamping. [decomp]

## 2. Throttle and brake (`on gas/brake`)

Run when drive flag 2 is set. One signed state `tb` in -1..1 persists between ticks (reset to 0 when the
behaviour is reset); positive is gas, negative is brake. Gas, brake, handbrake and vertical steering are
zeroed first.

```
if staging (drag or race grid):               tb = 0.8
elif not reversing_speed and steering_behind: tb = 1;  handbrake = 1          # handbrake turn
else:
    reversing = gearbox in reverse
    cur = signed forward speed;  want = drive_speed
    err = cur - want
    speed_error.record(err, dT)                       # PidError(4, 4, 30)
    if want < 0.5:                 tb = -1            # stop
    elif reversing:                tb = (cur > 1) ? -1 : +1
    elif cur < -1:                 tb = -1            # rolling backwards in a forward gear
    else:                          # the controller proper
        I = clamp(speed_error.integral(), -5, 5)
        D = clamp(speed_error.derivative(), -10, 10)
        tb += -0.4 * err - 0.01 * I - 0.1 * D        # velocity form: adds to the previous value
tb = clamp(tb, -1, 1)
gas = clamp(tb, 0, 1);  brake = clamp(-tb, 0, 1)
```

Gains `P = 0.4`, `I = 0.01`, `D = 0.1`; limits for the integral +-5 and derivative +-10; max gas 1, max brake 1,
staging gas 0.8 [decomp]. Because the formula *adds* the PID value each tick, the real controller is
integrating: a speed error of 1 m/s moves `tb` by 0.4 per tick (24 per second), so the pedal swings between
full gas and full brake within a few ticks when the error is a few m/s, and settles with a small ripple;
`tb` never accumulates beyond +-1. The brake and gas are never both non-zero. The adaptive throttle controller
(`AdaptivePIDControllerSimple`, with its error-model table of `(-20 -> 100, -10 -> 10, -5 -> 7, -2 -> 3,
0 -> 0, 2 -> 0, 10 -> -3, 20 -> -10, 30 -> -100)` and coefficient limits) is constructed but switched off by
a constant, and its update is empty; do not implement it. [decomp]

Differences from the simple controller (traffic): continuous pedals instead of 0/1, no 2.5 m/s coast band,
the stop test is `want < 0.5` only, and the full-lock handbrake turn sets gas to the stored state.

## 3. Steering (`on steering`)

If the reverse override is active (stuck recovery) use the simple steering of
[ai-driver-control.md](ai-driver-control.md) section 4.1 instead. Otherwise, when drive flag 1 is set and the
car has a steering input and a suspension:

```
steering = 0; vertical steering = 0
speed = planar speed
if drive_speed == 0 and speed < 1: return                         # parked
d = unit(planar(drive_target - position));  f = unit(planar(forward))
body_err = asin(clamp(cross(f, d).y, -1, 1))                      # signed, radians, + = target on the right
body_hist.record(body_err, dT)                                    # PidError(5, 5, 30)
(a second history of the same error measured from the velocity heading is recorded but its weight is 0)
angle_error = body_hist.error()
integral    = clamp(body_hist.integral(),   -0.5, 0.5)
derivative  = clamp(body_hist.derivative(), -10,  10)
controller.set_terms(P = angle_error, I = integral, D = derivative)
growing = angle_error * derivative >= 0
rate = growing ? |derivative| : -|derivative|                     # signed rate of change of |error|, rad/s
model  = HeadingErrorModel(|degrees(angle_error)|)                # deg/s the error "should" change at
actual = degrees(rate)
controller.update(model, actual, dT)
if speed < 10:  force coefficients to the speed tables (below)
angle = controller.output()                                       # radians of steering angle
steer = angle / radians(max_steering_degrees)
steering_behind = false
if gearbox in reverse:  steer = (forward speed < 0) ? (steer < 0 ? +1 : -1) : 0     # full lock, opposed, while backing up
steer = clamp(steer, -1, 1)
write steering
```

Notes [decomp]:

- The error is an **arcsine**, so a target more than 90 degrees to the side folds back (a target straight
  behind reads zero). The reversing rule and stuck recovery handle cars pointing away; the steering PID does
  not full-lock for a target behind (unlike the simple controller, which does).
- While reversing and moving backwards the output is bang-bang: full lock to the side opposite to the sign of
  the computed steer. While in reverse but not yet moving backwards it is 0.
- The oversteer correction term is a stub (0).
- `max_steering_degrees` is `45 * STEERING` for the racer chassis, 45..60 for the cop chassis (see
  [ai-driver-control.md](ai-driver-control.md) section 4.1).

### 3.1 Heading error model

`HeadingErrorModel` is a piecewise-linear graph of `|error|` in degrees to the desired rate of change in
degrees per second [decomp]:

| error (deg) | 0 | 3 | 6 | 9 | 12 | 15 | 18 | 21 | 24 | 27+ |
|---|---|---|---|---|---|---|---|---|---|---|
| rate (deg/s) | 0 | -1 | -2.05 | -3.5 | -4.94 | -6.14 | -7.35 | -8.55 | -9.4 | -10 |

The model therefore says: a larger heading error should shrink faster, up to 10 degrees per second.

### 3.2 Controller settings

Rule MIT; derivative window 0.1 s; time slice 0.1 s (so each coefficient is revisited every 0.3 s); gain
`gamma = 1e-5` for P, I and D; tuning threshold 0.01 for each term; integral term clamp +-0.5, derivative +-10
(applied on the inputs, above). Coefficient ranges [decomp]:

| | P | I | D |
|---|---|---|---|
| racing | 0.4 .. 1.0 | 0.01 .. 0.1 | 0.1 .. 0.4 |
| drag (human car in a drag race) | 0.6 .. 1.2 | 0.05 .. 0.5 | 0.02 .. 0.6 |

(The drag set is chosen at construction when the driver style is drag *and* the driver class is human; an AI
drag opponent uses the racing set.)

Speed-indexed seeds, used while the planar speed is below **10 m/s** (they overwrite the coefficients every
tick; `Table` over speed 0..160 m/s, 10 samples, 17.78 m/s apart; at 10 m/s the factor is 0.5625 of the way to
the second sample) [decomp]:

| index | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |
|---|---|---|---|---|---|---|---|---|---|---|
| P | 0.328 | 0.22 | 0.148 | 0.115 | 0.09 | 0.074 | 0.057 | 0.049 | 0.043 | 0.04 |
| I | 0.21 | 0.244 | 0.267 | 0.29 | 0.305 | 0.321 | 0.328 | 0.336 | 0.341 | 0.344 |
| D | 0 | 0.0758 | 0.0758 | 0.0738 | 0.0604 | 0.0470 | 0.0470 | 0.0403 | 0.0403 | 0.0403 |

Behaviour implied by the numbers:

- Below 10 m/s the gains are the seeds, below the racing P minimum. At 10 m/s or more the coefficients start
  from whatever the last forced values were (zero for a car that never drove slowly) and each is clamped into
  its range the first time its term is tuned, so within 0.3 s the effective gains are at least `0.4 / 0.01 / 0.1`.
- The adaptation step is tiny: `gamma * model_error * sensitivity * dT` with model errors of at most about
  10 deg/s, sensitivities of at most 1000 and `dT` 1/60 gives coefficient changes of at most about 0.002 per
  update, and most updates change much less. I expect the coefficients to stay close to their clamp minima
  in normal driving **[unconfirmed]**; measure before simplifying. The adaptation exists to be implemented
  because it costs little, and the result of the lookups below 10 m/s alone makes the steering well damped.

## 4. Constants (all [decomp] unless stated)

| Name | Value | Use |
|---|---|---|
| throttle P, I, D | 0.4, 0.01, 0.1 | velocity-form PID |
| throttle integral / derivative clamp | 5 / 10 | inputs |
| throttle `PidError` | 4 integral, 4 derivative samples, frequency 30 | |
| steering `PidError` (body, heading) | 5, 5, frequency 30 | |
| steering integral / derivative clamp | 0.5 / 10 | inputs |
| max gas / max brake / staging gas | 1 / 1 / 0.8 | |
| adaptive time slice, derivative window | 0.1 s, 0.1 s | |
| adaptation gain (steering, each term) | 1e-5 | |
| tuning threshold | 0.01 | |
| adaptation only below / seeds below | 10 m/s | |
| model error derivative floor on dT | 0.001 s | |
| sensitivity clamp | +-1000 | |
| reversing turn-in | full lock opposed while speed < 0 | |

## 5. How to check it

- Step response: hold a 20 m/s target and change it to 30: the pedal should go to full gas, then ease off
  within a second; no overshoot beyond a few m/s on flat ground.
- Steering: a racer at 25 m/s approaching a 10 degree heading error should reduce it at roughly the model's
  rate (about 5 degrees per second at 12 degrees) without oscillation; at 5 m/s the gains are the seed values.
- Log `tb` while following a speed that jumps up and down by 5 m/s: the swing time to saturation should be
  about 3 ticks per 5 m/s error.

## 6. Open questions

1. Whether the real adaptation ever leaves the clamp minima in play; needs a recording of the coefficients.
2. The PID error is not recorded on ticks where the car is parked (early return): the history keeps the last
   moving sample. Matches the code; no known effect.
3. The seeds table index base: the table helper is evenly spaced 0..160 per the constructor; confirmed only by
   the declaration.

## 7. Rust implementation notes

- A pure `PidError` ring buffer type and a `SteeringPid`/`ThrottlePid` pair in the driver crate; both take
  `dT` per tick and return plain `f32`s.
- `SteeringPid::step(angle_to_target, speed, dT, max_steer_rad) -> steer` where `angle_to_target` is the
  signed asin value above; the caller supplies `max_steer_rad = (45 deg * tires.steering).to_radians()` from
  the vehicle (a new `Vehicle::max_steering_rad()`, 45 deg for drag style).
- `ThrottlePid { tb: f32, err: PidError }::step(speed, want, reversing, staging, handbrake_turn, dT) -> (gas,
  brake, handbrake)`; keep `tb` between ticks.
- The adaptive controller's "term in turn" uses a clock; use the sim time `t` (not wall time) so replays are
  deterministic, accepting a small difference.
- Unit tests: step responses above; `PidError::integral` against hand-computed values; the seed table lookups
  at 0, 10 and 160 m/s.
