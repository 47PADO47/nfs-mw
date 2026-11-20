# Suspension: spring, damper, anti-roll bar, camber, roll centre, hub
Sources: Milliken and Milliken, "Race Car Vehicle Dynamics" (SAE, 1995), public literature, understanding (roll centre, geometric load transfer, bump camber, anti-roll bars)
Sources: Gillespie, "Fundamentals of Vehicle Dynamics" (SAE, 1992), public literature, understanding (ride and roll rates, quarter-car model)
Sources: Dixon, "The Shock Absorber Handbook" (Wiley, 2nd ed. 2007), public literature, understanding (bump and rebound rates, knee, damper force curve)
Sources: rustyAC, https://github.com/thesupersupersigma/rustyAC, `crates/rustyac-physics/src/car/` and `docs/port/`, GPL-3.0-or-later (per its LICENSING.md; the repo also carries MIT/Apache files for unrelated parts), understanding of behaviour and facts (two-rate damper with knee speeds and its defaults, bump stop rate 500000 N/m, anti-roll bar as a spring on the height difference of the two hubs); no code, comments or structure taken
Sources: this repository, [vehicle-suspension-tires.md](../vehicle-suspension-tires.md) (what the host does today), [ride6-suspension.md](../ride6-suspension.md) (our own clean-room statement of the two-slope damper), MIT OR Apache-2.0, facts
Written by: spec author session (Claude, stage 0 part 2), 2026-11-20; revised by the spec maintainer session, 2026-11-20 to the built hook   Implemented by: not yet

Part of [the AC handling specs](README.md), [ADR 0016](../../decisions/0016-handling-hooks.md). Implements the `suspension` hook
(`bb.handling` 1.1, as built: [api-handling-suspension.md](../../modding/api-handling-suspension.md)). Axes: x right, y up, z forward. Wheel order FL FR RL RR (0..3). SI units, angles in radians.

## 1. Purpose and scope

Replace the host's per-corner vertical force (`spring_force`: progressive spring, one-rate damper, anti-roll bar) by a model with
the parts that decide how a car feels in roll, pitch and over bumps, and give the tyre model a camber and a wheel load that
include the unsprung mass.

| In scope | Out of scope |
|---|---|
| linear spring plus an optional quadratic term, bump stop and droop stop | anti-dive, anti-squat, bump steer, toe, compliance steer |
| damper with separate bump and rebound rates and a fast/slow knee on each | adjustable or semi-active dampers, damper blow-off |
| anti-roll bar (a spring on the travel difference of one axle) | heave (third) springs, linkage motion ratio curves (a constant wheel rate is used) |
| camber from wheel travel, from body roll and from steering | camber from the road slope or from body pitch |
| roll centre height, as a rule for where the lateral force reaches the body | roll axis inclination, roll centre migration with travel |
| an unsprung hub mass integrated in substeps inside the mod | tyre lateral or longitudinal compliance, wheel hop of the brakes |

**Split of state.** The host keeps one rigid sprung body (position, orientation, velocities) at the tick rate and keeps the
ground probe (`compression` below). The mod keeps four hub states and integrates them in substeps. The mod never moves the
body: it returns forces, the host applies them.

## 2. Inputs and outputs

**Host contract (revised to the built hook).** The hook is called **once per axle** per tick: the front axle (`axle_index` 0), then
the rear (1), each time with both wheels in the query (`wheels[0]` left, `wheels[1]` right) and both in the result. The host probes
the ground under all four wheels first and has answered both axles before it asks any tyre, so the camber and the load this call
returns reach the tyre query of **the same tick** (`camber_rad` is a field of the tyre query since 1.1, outward positive, 0 for a
wheel this hook did not answer). The two wheels of an axle are integrated **together** (lockstep): the anti-roll bar couples them
inside a substep. *The first draft assumed otherwise* in six places, all rewritten below: one call per tick for the four wheels;
a camber that travelled inside the mod to the tyre code of the next tick; a compression speed the mod had to derive; `rest_travel`
as the host's ride height; a lateral force the mod took from its own tyre hook; and a `hub_travel` output (there is none).

Per axle, from the query: `role` (0, else not handled), `vehicle_revision` (a change reseeds, section 3), `dt`, and

| Name | Unit | Meaning |
|---|---|---|
| `c_rest` (`rest_travel_m`) | m | the compression of a wheel of this axle standing at rest on flat ground: the **spring sag** at which the vehicle's own spring carries `static_load_n` (about 0.026 m on the example car). It is **not** the host's design ride height (which can exceed the whole travel); it depends on the car, so it is constant for a `vehicle_revision` |
| `c_max` (`max_travel_m`) | m | the largest compression. The road input is bounded `-c_rest <= u <= c_max - c_rest`; the model uses it for nothing else |
| `m_corner` (`corner_mass_kg`) | kg | the sprung mass of one wheel (a quarter of the vehicle); `m_u` is clamped to at most `0.3 m_corner` |

Per wheel `i` of the axle, from `wheels[i]`:

| Name | Unit | Meaning |
|---|---|---|
| `c` (`compression_m`) | m | how far the wheel is pushed up from full droop, `0..c_max`, `+` towards the body; this tick's value; `0` for a wheel not touching |
| `loaded` | bool | the wheel presses on the ground (the host's own test); `c` is `0` when it does not |
| `c'` (`compression_speed_mps`) | m/s | the rate of change of the compression over the last tick, `+` = compressing |
| `steer` (`steer_angle_rad`) | rad | steering angle of this wheel (`0` on the rear axle), `+` = right |
| `a_up` (`body_accel_up_mps2`) | m/s^2 | acceleration of the body point above the wheel along the body up axis, kinematic (`0` at rest, about `-9.81` in free fall), over the last tick; `0` on the first tick after the hook is installed and after the car is placed |
| `Fy` (`tyre_lateral_force_n`) | N | the lateral force the tyre of this wheel put on the body in the last tick, along body x, `+` = right, as the tyre hook (or the built-in tyre) answered after the host's own limit; `0` when airborne and on the first tick. So the roll centre rule works with or without the mod's own tyre hook |
| `load_est` (`static_load_n`) | N | the vertical load the host estimates for this wheel at rest: the weight the axle carries, shared by its two wheels (it includes the unsprung weight); used only to (re)seed |
| `dt` | s | the tick (the same for both axles) |

Parameters (section 5): per axle, from the preset and the per-car overrides; `track` comes from the vehicle `spec`.

Per wheel, to the host (`BbSuspensionWheelResult`):

| Name | Unit | Meaning |
|---|---|---|
| `body_force` (`body_force_n`) | N | force on the sprung body along the body up axis at the wheel (`+` = up), the mean over the tick. Replaces the host's built-in spring force and may be negative (a droop stop pulls the body down) |
| `tyre_load` (`tyre_load_n`) | N | vertical force between road and tyre, `>= 0`, the mean over the tick. It **becomes the tyre's vertical load** of this tick without further scaling (the host's slope correction of the load is not applied) |
| `camber` (`camber_rad`) | rad | inclination of the wheel to the road, **outward positive** on either side (top leaning away from the car centre line). The host passes it to the tyre query unchanged; the mod's tyre code converts to the tyre force sign: `gamma = +camber` on the right wheels, `-camber` on the left ([tyre-force.md](tyre-force.md) section 2). The thermal spec takes it outward positive as it is |

There is **no hub travel output** (the host does not ask for one). The host refuses an answer with a non-finite field, a
`body_force` beyond 1e6 N, a `tyre_load` outside 0 to 1e6 N or a `camber` beyond 0.6 rad (a fault); it takes both wheels of an
answer or neither; on a handled axle its own anti-roll bar and damper do not run; when **no** wheel of the car is loaded it
applies no wheel force to the body at all, so the pull of a droop stop (V10) acts only while at least one wheel is loaded.

`handled = false` (return NOT_HANDLED: the host's own spring runs for **both** wheels of the axle this tick) when: `role != 0`;
the vehicle has no parameter set; `dt <= 0` or `dt > 0.1 * (1 + 1e-6)`; any input is NaN or infinite; `static_load_n <= m_u g`;
or the failure rule of section 4 fires. Declining is not a fault for the host.

## 3. State

Per wheel: `x` (hub travel from rest, m, `+` = compression), `v` (its rate, m/s), `W_s` (static sprung corner load, N), `d0` (static
tyre deflection, m), `was_loaded` (bool). There is no stored road input: `u_prev = u - u' dt` is rebuilt from the query each tick
(section 4). Initial values and the **reseed** rule:

```
reseed(i):               # first call, a change of vehicle_revision or preset, a toggle, a teleport, a failure
    x = 0 ; v = 0
    W_s  = load_est - m_u * g
    d0   = load_est / k_t
    this tick: u' = 0                 # no road velocity on a seeding tick
```

`W_s` is the spring preload: at `x = 0` the spring carries exactly the sprung weight of the corner, so a car at rest on flat
ground has no net vertical force on any hub or on the body. A wheel with `loaded = false` keeps `was_loaded = false`; on the first
loaded tick after that, `u' = 0` (the compression of an airborne wheel is a convention, `0`, not a measurement, so the jump to the
first real value is not road velocity; the hub already carries its own motion).

## 4. Equations

Notation: `x` and `v` are the hub travel and rate (compression `+`), `sigma = -1` for a left wheel and `+1` for a right wheel,
`g = 9.81`. All parameters are those of the wheel's axle.

**Road input.** `u = c - c_rest` (m, `+` = the road moved up relative to the body; `0` for a car at rest, whatever the ride-height
tuning, because `c_rest` is the sag at rest). `u' = c'` (the host's rate over the last tick; `0` on a seeding or touchdown tick,
section 3), and the road input at the start of the tick is `u_prev = u - u' dt`.

**Spring.** `F_s(x) = W_s + k_w * x + k2 * x * |x|`.

**Damper.** `v > 0` is bump (compression), `v < 0` rebound. Two slopes, continuous at the knee:

```
v >= 0:  F_d =  c_bs * min(v, v_kb) + c_bf * max(v - v_kb, 0)
v <  0:  u = -v
         F_d = -( c_rs * min(u, v_kr) + c_rf * max(u - v_kr, 0) )
```

`F_d` is the force on the body (`+` = up): bump pushes the body up, rebound pulls it down. `c_bs`, `c_rs` are the slow
rates (below the knee), `c_bf`, `c_rf` the fast rates; typically `c_bf < c_bs` and `c_rf < c_rs`. Set `c_bf = c_bs` for a
single-rate damper.

**Stops.** `k_bs` is the stop rate. The bump stop engages `x_up` above rest, the droop (rebound) stop `x_dn` below rest:

```
F_stop(x) = k_bs * (x - x_up)        if x >  x_up
          = k_bs * (x + x_dn)        if x < -x_dn          (negative: it pulls the body down)
          = 0                        otherwise
```

**Anti-roll bar.** On one axle, with `x_L` and `x_R` the two hub travels of the same substep:
`F_arb,L = k_arb * (x_L - x_R)` and `F_arb,R = -F_arb,L` (force on the body at each wheel; the hubs get the opposite force).

**Roll centre (geometric load transfer).** The host applies the tyre's lateral force to the body at the wheel's contact point.
A roll centre at height `h_rc` above that point means the part of the lateral force that travels through the links must not roll
the body. The rule: add one equal and opposite vertical pair per axle, acting through the links (between hub and body):

```
f_axle = Fy_L + Fy_R                       # N, along body x, from last tick
V_i    = -sigma_i * h_rc * f_axle / track  # N, on the body at wheel i (+ = up)
```

A left turn has `f_axle < 0`; the right (outer) wheel gets `V > 0` and the left wheel `V < 0`. `V` is added to the link force
below, so it appears in the hub equation and therefore in `tyre_load`: it is load transfer that bypasses the springs. `h_rc` is
measured above the point where the host applies tyre forces; `h_rc = 0` removes the term. The pair sums to zero, so it adds no
net vertical force.

**Link force** between hub and body (positive pushes the body up and the hub down):

```
F_link_i = F_s(x_i) + F_d(v_i) + F_stop(x_i) + F_arb,i + V_i
```

**Tyre vertical force.**

```
loaded:    F_t = max(0, k_t * (d0 + u - x) + c_t * (u' - v))
otherwise: F_t = 0
```

**Hub.** The hub is a point mass `m_u` that moves with the body (`a_up`) plus its relative motion `x''`:

```
m_u * (a_up + x'') = F_t - F_link - m_u * g
=>  x'' = (F_t - F_link - m_u * g) / m_u - a_up
```

At rest (`x = v = 0`, `u = 0`, `a_up = 0`, `F_link = W_s`, `F_t = k_t*d0 = W_s + m_u*g`) the right side is exactly zero.

**Camber.** With `x_L`, `x_R` the end-of-tick hub travels of the axle and `phi = (x_L - x_R) / track` the body roll seen from the
axle (`+` = left side lower):

```
gamma_i = gamma0 - g_t * x_i  -  sigma_i * phi  +  sigma_i * kappa * steer_i
gamma_i = clamp(gamma_i, -0.30, +0.30)
```

The terms are: static camber (`gamma0 < 0` is top inward), bump camber (compression adds negative camber), the wheel following the
body roll relative to the road, and caster-induced camber (steering right leans both wheel tops to the right, so the outer left
wheel gets negative and the inner right wheel positive camber). `kappa = 0` on a non-steering axle.

**Substeps.** This hook has its own substep rule, chosen for the stiffness of the hub (not the 600 steps per second of the wheel spin in [wheel-dynamics.md](wheel-dynamics.md)): `n = clamp(ceil(dt / 0.0025), 1, 16)`, `h = dt / n` (60 Hz gives 7, 144 Hz gives 3, 240 Hz gives 2). Semi-implicit
(symplectic) Euler, both wheels of the axle in lockstep:

```
for k in 1..=n:
    for each wheel i:                         # forces from the state at the start of the substep
        u_k   = u_i - u'_i * dt * (n - k) / n             # road input interpolated over the tick (u_prev at k = 0, u at k = n)
        F_t   = tyre force(u_k, u' , x_i, v_i)
        F_link_i = as above, with F_arb from the x of BOTH wheels of this substep start
    for each wheel i:
        a   = (F_t - F_link_i - m_u*g)/m_u - a_up_i
        v_i += a * h
        x_i += v_i * h                         # uses the updated v
        sum_link_i += F_link_i ; sum_t_i += F_t
body_force_i = sum_link_i / n          # mean over the tick: the host applies it for dt
tyre_load_i  = sum_t_i / n
```

Stability: the stiffest mode is the tyre on the hub, `w = sqrt(k_t / m_u)` (about 81 rad/s); `w * h < 0.2` for every allowed `h`.
`a_up` is held constant over the tick (it is the host's value over the last tick, so the loop through the body has one tick of
delay); the lateral force `Fy` of the roll centre rule is also the last tick's.

**Failure handling.** If any output is not finite, outside the host's ranges (section 2), `|x| > 0.5`, or `|v| > 20`, the mod
**declines** the axle (NOT_HANDLED): the host runs its own spring for both wheels this tick and the mod reseeds both hubs on the
next call. A decline is free; the mod never sends a refused answer, because the host counts that as a fault (five faults
quarantine the hook).

**Pseudocode of one call** (one axle, both wheels):

```
fn suspension_axle(query) -> Option<result>:
    if role != 0 or no parameter set or bad dt or any non-finite input: return None        # NOT_HANDLED
    for i in {left, right}:
        if needs_reseed(i): reseed(i)                       # includes a vehicle_revision change
        u_i  = c_i - c_rest
        u'_i = (reseeded or not was_loaded_i) ? 0 : c'_i
    f_axle = Fy_L + Fy_R
    V_i    = -sigma_i * h_rc * f_axle / track
    run the substep loop above
    camber_i = from the end-of-tick x_L, x_R and steer_i
    was_loaded_i = loaded_i
    if any non-finite or out of range: mark both hubs for reseed; return None
    return body_force_i, tyre_load_i, camber_i          # per wheel; no hub travel
```

## 5. Constants

Defaults for a 1400 kg sports saloon, 55 % front, used by the test vectors (front-left wheel unless stated). All are **design
values** (typical published ranges for a road car: ride frequency about 1.5 Hz, damping ratio 0.3 in bump and 0.6 in rebound,
bump camber 0.3 to 0.6 rad/m, roll centre 0.03 to 0.15 m), except where marked *reference*. The presets of
`parameter-mapping.md` replace them per car class.

| Symbol | Front | Rear | Unit | Range | Reason |
|---|---|---|---|---|---|
| `m_u` unsprung mass | 38 | 38 | kg | 15..60 | design: hub, wheel, tyre, half the links |
| `k_w` wheel rate | 30000 | 28000 | N/m | 10000..200000 | design: 1.5 Hz ride frequency for the corner mass |
| `k2` quadratic spring term | 0 | 0 | N/m^2 | 0..5e6 | design: off by default |
| `k_t` tyre vertical rate | 250000 | 250000 | N/m | 150000..350000 | design: road tyre |
| `c_t` tyre vertical damping | 200 | 200 | N s/m | 0..800 | design |
| `c_bs`, `c_bf` bump slow, fast | 1900, 700 | 1600, 600 | N s/m | 200..12000 | design: zeta 0.3, fast = 0.37 x slow |
| `c_rs`, `c_rf` rebound slow, fast | 3800, 1400 | 3200, 1200 | N s/m | 200..16000 | design: zeta 0.6, fast = 0.37 x slow |
| `v_kb`, `v_kr` knee speeds | 0.15, 0.20 | 0.15, 0.20 | m/s | 0.05..0.5 | *reference* order of magnitude (0.2 m/s is the reference default) |
| `x_up` bump stop gap | 0.060 | 0.060 | m | 0.01..0.15 | design |
| `x_dn` droop stop gap | 0.070 | 0.070 | m | 0.01..0.15 | design |
| `k_bs` stop rate | 500000 | 500000 | N/m | 1e5..2e6 | *reference* (500000 N/m in the reference) |
| `k_arb` anti-roll bar | 20000 | 12000 | N/m | 0..80000 | design: about 60 % of the front roll stiffness from the bar |
| `g_t` bump camber gain | 0.40 | 0.30 | rad/m | 0..1 | design |
| `gamma0` static camber | -0.035 | -0.026 | rad | -0.10..0.05 | design (-2.0 and -1.5 deg) |
| `kappa` caster camber | 0.10 | 0 | rad/rad | 0..0.3 | design (about 6 deg caster) |
| `h_rc` roll centre above force point | 0.06 | 0.12 | m | -0.1..0.4 | design |
| `track` | 1.52 | 1.52 | m | 1.2..2.0 | vehicle `spec` |
| `c_rest` | 0.026 | 0.026 | m | from the query | the sag at rest; a query constant, not a preset value (the vectors use 0.026, the example car) |
| `W_s` | 3367 | 2754 | N | from `load_est` | seeded; the vectors use 3367 at the front |
| `n` substep size | 0.0025 | | s | 0.001..0.005 | design: `w*h < 0.2` |

## 6. Test vectors

Computed by a script that implements only sections 3 and 4 (float64). Tolerance: **1e-4 relative** for the closed-form lines
(V1 to V8) and **1e-2 relative** for the dynamic lines (V9 to V13), which pass through the stop thresholds. Front-left wheel,
default front constants, `load_est = 3739.78` (that is `W_s + m_u*g`), `dt = 1/60`, `steer = 0`, `a_up = 0`, `f_axle = 0`
unless stated; the cases are given in the road input `u`, and the query that carries it is `compression_m = c_rest + u` with
`c_rest = 0.026` and `compression_speed_mps = (u - u_prev) / dt` (V15); the dynamic cases V9 to V13 are preceded by one rest tick
(the reseed) with `u = 0`.

| # | Case | Result |
|---|---|---|
| V1 | one tick at rest, `u = 0` both wheels | `body_force = 3367.0`, `tyre_load = 3739.78`, `x = 0`, `camber = -0.035` |
| V2 | `F_s` at `x = +0.02`, `-0.02` | 3967.0, 2767.0 |
| V3 | `k2 = 2e6`, `F_s` at `x = +0.03`, `-0.03` | 6067.0, 667.0 |
| V4 | `F_d` at `v = 0.05, 0.15, 0.5` | 95.0, 285.0, 530.0 |
| V4b | `F_d` at `v = -0.05, -0.20, -0.60` | -190.0, -760.0, -1320.0 |
| V5 | `F_stop` at `x = 0.03, 0.07, -0.05, -0.09` | 0, 5000, 0, -10000 |
| V6 | `F_arb,L` for `x_L = 0.02, x_R = -0.01` (right is the negative) | 600.0 |
| V7 | `camber` L and R for `x_L = 0.03, x_R = -0.02, steer = 0.2` | -0.034105, -0.039895 |
| V7b | camber at rest (`x = 0`, `steer = 0`); right wheel at `x = 0`, `steer = 4.0` (the clamp) | -0.035; +0.30 (unclamped it would be +0.365) |
| V8 | `V_L`, `V_R` for `f_axle = -6000`, `h_rc = 0.06`, `track = 1.52` | -236.842, +236.842 |
| V9 | road step on the left wheel only: `u_L = 0.02`, `u_R = 0`, both loaded, from rest | see V9 table |
| V10 | airborne, `loaded = 0`, `compression_m = 0` (`u = -c_rest = -0.026`, unused: `F_t = 0`), `a_up = -9.81`, both wheels | see V10 table |
| V11 | road step on both wheels `u = 0.08` (reaches the bump stop) | see V11 table |
| V12 | steady lateral force `f_axle = -6000`, 30 ticks, `u = 0` | L: `body_force 3181.97`, `tyre_load 3554.75`, `x 0.0007`; R: `3552.03`, `3924.81`, `-0.0007` |
| V13 | the V9 step over the same 1/6 s at 60 Hz (10 ticks) and at 144 Hz (24 ticks) | left `x`: 0.0167 and 0.0167; `body_force` 4176.9 and 4185.7 (within 0.5 %): the result converges with the tick |
| V14 | `n` for `dt = 1/60, 1/144, 1/240, 0.1` | 7, 3, 2, 16 |
| V15 | query to road input (the host's `rest_travel_m` and `compression_*`): `c_rest = 0.026`, `compression_m = 0.046`, `compression_speed_mps = 1.2`, `dt = 1/60` | `u = 0.020`, `u' = 1.2`, `u_prev = 0.000` (the first tick of V9's left wheel); `compression_m = 0.026`, speed 0: `u = 0` (a car at rest, whatever its ride-height tuning) |
| V16 | touchdown: tick A airborne (`loaded = 0`, `compression_m = 0`), tick B loaded with `compression_m = 0.026`, `compression_speed_mps = 1.56` | tick B uses `u' = 0` (not 1.56), `u = 0`; the V1 equations hold from the hub state of tick A |
| V17 | one axle call, both wheels at rest, `static_load_n = 3739.78`, host query then result | result `wheels[0]`, `wheels[1]` identical: `body_force_n 3367.0`, `tyre_load_n 3739.78`, `camber_rad -0.035` (outward positive on both sides) |

V9, left wheel (`x` in m, `v` in m/s, forces in N); the right wheel follows through the anti-roll bar:

| tick | `x_L` | `v_L` | `body_force_L` | `tyre_load_L` | `x_R` | `body_force_R` |
|---|---|---|---|---|---|---|
| 1 | 0.0076 | 0.9294 | 3821.88 | 6313.64 | 0.0001 | 3334.89 |
| 2 | 0.0236 | 0.6743 | 4991.79 | 4783.04 | 0.0008 | 3152.16 |
| 5 | 0.0152 | 0.0658 | 3709.38 | 4937.23 | 0.0011 | 2978.25 |
| 10 | 0.0167 | 0.0023 | 4176.85 | 4563.86 | 0.0011 | 3077.93 |

V10, left wheel, hub falls to the droop stop (`F_t = 0` throughout):

| tick | `x_L` | `v_L` | `body_force_L` |
|---|---|---|---|
| 1 | -0.0103 | -0.9569 | 2181.82 |
| 5 | -0.0774 | 0.1711 | -2702.27 (the droop stop pulls the body down) |
| 30 | -0.0724 | 0.0000 | 0.0002 (spring and stop balance in the weightless hub) |

V11, both wheels, left wheel shown (the hub rings between tyre and bump stop):

| tick | `x_L` | `v_L` | `body_force_L` | `tyre_load_L` |
|---|---|---|---|---|
| 1 | 0.0316 | 3.9014 | 4686.65 | 13954.53 |
| 2 | 0.0918 | 1.0996 | 13317.26 | 7301.95 |
| 3 | 0.0534 | -2.4272 | 12102.26 | 4434.11 |
| 4 | 0.0503 | 1.2049 | 3424.15 | 12078.07 |
| 5 | 0.0747 | 0.6470 | 8808.63 | 7909.40 |

Properties every implementation must also satisfy (tests with random inputs, not vectors): both wheels of an axle are answered or
neither; `rest_travel_m` shifts `compression_m` and changes nothing else (the same `u` gives the same result for any sag); the pair
`V_L + V_R = 0`; with
`h_rc = 0`, `k_arb = 0` and a symmetric road input the two wheels give identical outputs; `tyre_load >= 0`; with
`loaded = false`, `tyre_load = 0`; one tick at rest returns the V1 values for any `dt` from 1/240 to 1/30.

## 7. Limits

- Vertical motion only. No pitch-dependent camber, no toe change, no bump steer; anti-dive and anti-squat are not modelled (the
  host's longitudinal force still reaches the body at the contact point, so a car dives and squats like the built-in one).
- The wheel rate is a constant (plus the optional quadratic term): no motion-ratio curve, no progressive linkage.
- The hub follows the body's vertical acceleration from the **previous** tick; the loop body to hub has one tick of delay. A
  harsh impact is therefore a little too stiff for one tick. The two axles are answered before any tyre, so camber and load reach
  the tyre of the same tick, but the roll-centre force `Fy` is the last tick's.
- The host's compression is its own clamped ride-height measure (`0..max_travel_m`): a wheel at the end of its travel reads the
  same compression however hard it is pushed, and an airborne wheel reads 0; the road input at the ends is therefore a
  saturated version of the real one. The host's slope correction of the load is not applied to `tyre_load`, so on a slope the
  load is the model's, not the host's.
- The brake lamp and the effects of the host read the mod's tyre results; nothing in this hook affects them.
- The roll centre rule assumes the host applies tyre forces at the contact point of each wheel and that the body is one rigid
  box. Roll centre migration and the jacking of the front and rear axle roll axis are not modelled.
- The anti-roll bar is linear with no free play and no friction.
- The dampers are linear on each side of the knee; no temperature fade, no blow-off, no hysteresis.
- Camber is clamped to +-0.30 rad and is not corrected for road banking: on a banked road the camber the tyre sees is that of a
  flat road.
- Tyre vertical force is linear (no tyre hysteresis, no load-dependent rate, no sidewall bottoming).
- Wheel hop at the stops is audible in the numbers (V11); the model is not tuned for crashes or for landing from a jump.

## 8. Changes (revision of 2026-11-20, for the implementer's tests)

| Item | Before | Now |
|---|---|---|
| call | one call per tick, four wheels | one call per axle (front then rear), both wheels in the query and the result; both axles answered before any tyre |
| `rest_travel` | the host's ride height for the axle | `rest_travel_m`: the compression at rest (sag, 0.026 m on the example car), a query field; `u = compression_m - rest_travel_m` |
| compression speed | derived by the mod from `u_prev` | `compression_speed_mps` from the host; `u_prev = u - u' dt`; state `u_prev` dropped; `was_loaded` added; `u' = 0` on seeding and touchdown ticks |
| lateral force `Fy` | from the mod's own tyre hook | `tyre_lateral_force_n` of the query (also with the built-in tyre) |
| `load_est` | not named | `static_load_n` of each wheel (includes unsprung weight) |
| camber delivery | to the tyre code of the next tick (the tyre query had none) | to the tyre query of the same tick as `camber_rad` (outward positive; the mod converts the sign) |
| outputs | `body_force`, `tyre_load`, `camber`, `hub_travel` | `body_force_n`, `tyre_load_n`, `camber_rad`; no `hub_travel` |
| failure | a fault (five quarantine) | decline both wheels and reseed (free); the mod never sends a refused answer |
| V10 input | `c = 0`, `u = -0.127` | `compression_m = 0`, `u = -0.026` (unused, `F_t = 0`); results unchanged |
| V1 to V14 results | | unchanged (same maths, same `u` histories); V9 to V13 are preceded by a reseed tick |
| new vectors | | V15 (query to road input), V16 (touchdown), V17 (axle call) |
