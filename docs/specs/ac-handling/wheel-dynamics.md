# Wheel dynamics, relaxation and the substep rule
Sources:
- Pacejka, *Tyre and Vehicle Dynamics*, 3rd ed., Elsevier 2012 (published book, no license): relaxation length and the first-order
  lag of the contact patch deflection, wheel spin equation, slip definitions: textbook equations and understanding.
- Genta, *Motor Vehicle Dynamics*, World Scientific 1997 (published book): wheel torque balance, numerical stiffness of the
  wheel-slip loop: understanding.
- Hairer and Wanner, *Solving Ordinary Differential Equations II*, Springer 1996 (published book): linearly implicit (Rosenbrock
  type) Euler step for stiff systems: textbook method, applied here in our own notation.
- rustyAC, https://github.com/thesupersupersigma/rustyAC, `GPL-3.0-or-later`, only `docs/port/tyre_step.md` and
  `crates/rustyac-physics/src/tyre/vanilla_tyre.rs`: understanding of how the reference game's wheel behaves (slip from sliding
  speed below about 1 m/s, a relaxation length that is longer when the tyre grips and shorter when it slides and grows with
  load, a floor on the lag rate, a wheel that locks when the brake torque exceeds the tyre torque) and facts (the ratios
  marked `[U]`). No code, comment or structure was copied; the scheme and every equation here are ours.
- This repository's `vehicle-suspension-tires.md` section 3, `docs/physics-timing.md` and `docs/decisions/0016-handling-hooks.md`:
  the host's tick, units and the hook contract.

Written by: spec author session (Claude), 2026-11-20   Implemented by: (pending)

Part of [the AC handling specs](README.md). The force at a given slip is [tyre-force.md](tyre-force.md); the constants of both
are set by [parameter-mapping.md](parameter-mapping.md).

## 1. Purpose and scope

What the mod does in **one call of the `tyre` hook for one wheel** ([ADR 0016](../../decisions/0016-handling-hooks.md)): it takes
what the host knows at the start of a tick (load, ground speed under the patch, torques at the hub), integrates the wheel's
spin and the lag of the slip states in `N` substeps, and returns the tick's **mean** forces, the new wheel spin and the
numbers the effects use. The host body, drivetrain and contacts stay at the tick rate; only the stiff part lives here.

In scope: slip quantities with no singularity at rest or at lock, the relaxation lag, the wheel torque balance with brake
and rolling resistance, the airborne wheel, the low-speed force limit, the substep count and what is held across a tick.
Out of scope: the force curve ([tyre-force.md](tyre-force.md)), heat, pressure and wear, suspension, driver aids.

## 2. Inputs and outputs

All SI. Physics space: x right, y up, z forward; the patch velocity is in the **wheel frame** (the steering angle already
applied by the host). Wheel order FL FR RL RR.

The names are those of `BbTyreQuery` and `BbTyreResult` in the stage-1 page [api-handling.md](../../modding/api-handling.md);
the symbols are the ones the equations use.

| Input (`BbTyreQuery`) | Unit | Meaning |
|---|---|---|
| `wheel_index` | 0 to 3 | FL, FR, RL, RR; 0 and 1 use the front axle's parameter set, 2 and 3 the rear's, each wheel has its own state |
| `role` | 0 | 0 = the player's car; anything else is `not handled` |
| `dt` | s | the host tick: 1/60 (default), 1/240 (Speedbreaker), 1/144 (offline mode) |
| `vertical_load_n` | N | load of this tick, from the host's spring; `>= 0` |
| `patch_lateral_velocity` (`Vy`), `patch_forward_velocity` (`Vx`) | m/s | velocity of the contact patch over the ground in the wheel frame: `Vx` along the wheel's forward axis (+z), `Vy` to the right (+x) |
| `body_speed` | m/s | speed of the whole body; not used in stage 2 (the patch velocity carries it) |
| `wheel_spin` | rad/s | spin at the start of the tick, positive when rolling forward |
| `drive_torque_nm` | N m | torque into the hub, signed, positive accelerates forward rolling |
| `brake_torque_nm` | N m | brake torque magnitude at the hub, `>= 0`; it includes any handbrake contribution (the host adds the two), opposes the rotation and cannot reverse it |
| `corner_mass_kg` | kg | the part of the car's mass this wheel carries: the quarter of the sprung mass, a constant of the car (not `Fz / g`, which moves with load transfer). Used only by the momentum limit of 4.6 |
| `wheel_radius_m`, `wheel_inertia_kgm2` | m, kg m^2 | the host's wheel (the inertia is multiplied by `inertia_scale`, section 5) |
| `surface_lateral`, `surface_drive`, `surface_rolling` | 1 | the host's `SurfaceGrip` multipliers of the surface under the patch (`surf_lat`, `surf_drive`, `grip_rolling` below) |
| `tyre_condition` | 1 | 1 for a sound tyre, 0.3 when blown: multiplies the user's grip scale (`grip = grip_scale * tyre_condition`) |
| `ambient_temperature_c`, `road_temperature_c` | deg C | held, unused until the thermal spec |
| `vehicle_revision`, `surface_tag` | | a change of the revision re-seeds the state (section 3); the tag is unused |
| `loaded` | bool | `false` = airborne: spin decays, no force |

| Output (`BbTyreResult`) | Unit | Meaning |
|---|---|---|
| `longitudinal_force_n` | N | mean longitudinal force over the tick, along +z of the wheel frame |
| `lateral_force_n` | N | mean lateral force over the tick, along +x of the wheel frame (opposes `Vy`) |
| `aligning_torque_nm` | N m | mean self-aligning torque over the tick, positive turns the heading towards +x |
| `wheel_spin` | rad/s | spin at the **end** of the tick; the host writes it back and the drivetrain reads it |
| `traction` | 1 | 1 gripping (normalised combined slip `n <= 1`), `1/n` beyond; 0 for an airborne wheel (as the built-in model) |
| `slip` | m/s | wheel speed minus ground speed at the end of the tick, `R w - Vx` (+ when the wheel spins faster) |
| `slip_angle` | turns | `atan(sy_r) / (2 pi)`, 1.0 = 360 degrees, the game's unit; 0 for an airborne wheel |
| status | | `BB_TYRE_HANDLED`, or `BB_TYRE_NOT_HANDLED` to make the host run its own model for this wheel this tick |

`handled = false` when: `role != 0`; no parameter set for the car; `dt <= 0` or `dt > 0.1`; `wheel_radius_m < 0.05` or
`wheel_inertia_kgm2 <= 0`; `corner_mass_kg <= 0`; any input is NaN or infinite; `vertical_load_n < 0`. A `not handled` call leaves the output
fields at zero and marks the wheel's state `fresh` (section 3), so it re-seeds when handled again.

**Contract points and the choices made** (checked against [api-handling.md](../../modding/api-handling.md) and ADR 0016):

1. **`corner_mass_kg` is a query field the host is adding.** Its name and offset are fixed in the interface page, not here; this
   spec consumes it as defined in the table above. A host that does not send it (the field reads 0) gives `not handled`, so the
   mod never runs on a guessed mass.
2. **The patch velocity is that of the start of the tick** (the state the host holds when it calls the hook, before it integrates
   the body). It is held for the whole tick (4.9).
3. **`wheel_inertia_kgm2` is the host's constant (10)**, which may stand for more than a road wheel. A real road wheel is about
   1.2 to 2.5, so the mod scales it (`inertia_scale = 0.3`, giving 3). Whether the drivetrain's reflected inertia is on the host
   side is unknown; the value is a tuning key.
4. **Camber is not an input of the host's tyre query:** it is 0 in stage 2 ([tyre-force.md](tyre-force.md) section 7). **There is no
   steering-lock or steering-angle input either** and none is needed here: the patch velocity already arrives in the wheel frame.
   (Where a later spec needs a lock, [steering.md](steering.md), it is a preset value, not a host number.)
5. The host also limits the lateral force so a tyre cannot reverse the patch's sideways motion within one tick. The mod's own
   limit (4.6) stays, because it also bounds the longitudinal force at rest and the host's limit is not specified for it; two
   limits are harmless (the smaller wins).
6. An airborne call carries zero load, patch velocities and surface multipliers and only the torques (consistent with 4.7, which
   uses none of them). `wheel_spin` is the host's every tick (consistent with section 3).
7. **`slip_angle` is in turns** (1.0 = 360 degrees); the equations of this file and of [tyre-force.md](tyre-force.md) use radians
   and the one place that converts is the output row above (`atan(sy_r) / (2 pi)`), the sign being that of `sy_r`: positive when the
   patch slides to the right. The host's `slip` is metres per second.
8. **The thermal grip factor** ([tyre-thermal.md](tyre-thermal.md)) multiplies the `grip` argument of `tyre_force` (below); it is 1
   until stage 5.

## 3. State

Per wheel, persisting between ticks, owned by the mod (4 wheels, role 0 only):

| Field | Initial value | Meaning |
|---|---|---|
| `sx_r`, `sy_r` | the instantaneous slips of the first call (4.1) | relaxed longitudinal and lateral slip |
| `n_prev` | 0 | the normalised combined slip of the last substep (chooses the relaxation length) |
| `fresh` | true | re-seed on the next handled call |

The wheel's spin is **not** state: it is read from the call at the start of every tick (the drivetrain may have changed it).

`fresh` becomes true on the first call, after an airborne tick, after a `not handled` call, when the mod is switched on, when
`vehicle_revision` changes (the car changed), and when the user changes the preset. A fresh wheel sets `sx_r`, `sy_r` to the slips of 4.1 at the
start-of-tick spin, `n_prev = 0`, then clears `fresh`. An airborne tick sets `sx_r = sy_r = n_prev = 0` and `fresh = true`.

## 4. Equations

Notation: `R` radius, `I = wheel_inertia_kgm2 * inertia_scale`, `Fz` load, `h = dt / N` the substep. `tyre_force` is
[tyre-force.md](tyre-force.md) with the arguments `(radius, Fz, sx, sy, 0, slide_speed, speed, surf_lat, surf_drive, grip = 1)`;
`slide_speed = hypot(R w - Vx, Vy)` and `speed = hypot(Vx, Vy)` of the instant the force belongs to (`Vx`, `Vy` are the held patch
velocities, `w` the spin of that instant); `grip = grip_gain * tyre_condition * grip_scale(T_eff, P, w_wear)` where `grip_gain` is the
player's setting (1 by default) and the last factor is the thermal one (1 until stage 5); `surf_lat`, `surf_drive` are the host's
`surface_lateral`, `surface_drive`.

### 4.1 Slips without singularities

```
rw  = R * w
Wx  = max(|rw|, v_low_x)          Wy = max(|rw|, v_low_y)          v_low_x = 1.0 m/s, v_low_y = 0.4 m/s
sx_i = (rw - Vx) / Wx             sy_i = Vy / Wy                   instantaneous theoretical slips
```

For a wheel that rolls (`|rw| = |Vx| >= 1`) these are `kappa / (1 + kappa)` and `tan(alpha) / (1 + kappa)` of
[tyre-force.md](tyre-force.md) section 2. Because the denominators are the **wheel's own speed** and never the ground
speed: (a) a locked wheel at 30 m/s has `sx_i = -30` (very large slip, the sliding level), not infinity; (b) a standing
wheel has `sx_i = 0` and `sy_i = 0`; (c) a car creeping sideways at 0.05 m/s has `sy_i = 0.125`, a lateral slip stiff enough
to hold it; (d) driving backwards is symmetric: nothing depends on the sign of `Vx`, so the same equations give the mirror
result (vector W9). The floors are the only low-speed treatment; there is no "if speed < x" branch and no force is ever
divided by the ground speed. The lateral floor is smaller than the longitudinal one so that a standing car holds sideways
better than it holds against creeping along the road, as the reference game does.

### 4.2 Relaxation of the slips

The tyre force follows the slip with a lag over a **distance** (the relaxation length), so that a step of steering builds the
force in a fraction of a wheel turn and a slow roll does not chatter:

```
fr   = Fz / fz0
Lg   = relax_len * max(1 + relax_load_slope * (fr - 1), 0.3)          grip relaxation length at this load
L    = Lg * (1 + (relax_slide_ratio - 1) * clamp(n_prev, 0, 1))      shorter when the tyre slides (ratio 0.5)
lam  = max( max(hypot(Vx, Vy), |rw|) / L , 1 / relax_tau_max )       rate, 1/s
a    = h * lam / (1 + h * lam)                                         backward Euler factor of one substep
sx_r' = sx_r + a * (sx_target - sx_r)       sy_r' = sy_r + a * (sy_i - sy_r)
```

`sx_target` is the instantaneous `sx_i` evaluated with the **new** wheel spin (4.3). The rate has a floor,
`1 / relax_tau_max = 12.5 / s` (a 80 ms time constant), so that a standing wheel still builds its force in finite time; the
floor is a time, not a distance, and does not depend on `dt`. Using the larger of the patch speed and the rolling speed keeps
a spinning wheel at standstill responsive (the rubber flows through the patch at the wheel speed). Example (`relax_len = 0.34`,
`relax_load_slope = 0.3`): `Fz = 4000, n = 0`: `L = 0.442 m`; `n = 1`: `0.221`; `Fz = 500`: `0.2635`; `Fz = 0`: `0.238`.

### 4.3 One substep: linearly implicit wheel and slip

Unknown: the new spin `w'` and the new relaxed longitudinal slip. With `F0 = fx(sx_r, sy_r)` of `tyre_force` at the substep
start and `S = max(0, dfx / dsx)` its slope in `sx` (a central difference with step `1e-3`: `(fx(sx + 1e-3) - fx(sx - 1e-3)) / 2e-3`, with `sy`, `slide_speed`, `speed` and every other argument of the two calls equal to those of `F0`), the
linearisation of the backward Euler equations `I (w' - w) / h = T - R fx(sx_r')` and `sx_r' - sx_r = a (sx_i(w') - sx_r)` gives

```
dw   = h * ( T_drive - R * F0.fx - R * S * a * (sx_i - sx_r) ) / ( I + h * R^2 * S * a / Wx )
w_nb = w + dw                                       the spin before the brake
```

`sx_i` here is the instantaneous slip at the **start** of the substep (4.1); `Wx` is also the start value (frozen
coefficients). The denominator is at least `I`, so the step cannot blow up whatever the tyre stiffness `S` is: this is what
makes the substep **unconditionally stable** in the wheel-slip loop (section 4.9).

### 4.4 Brake and rolling resistance

Both oppose the rotation and cannot reverse it, so they are applied as a clamp after the free step:

```
t_rr = R * Fz * ( rr_c0 + rr_c2 * rw^2 ) * grip_rolling       rolling resistance torque, uses rw of the substep start
db   = h * ( brake_torque + t_rr ) / I                          the spin the two can remove in one substep
w'   = 0                       if |w_nb| <= db                  the wheel locks (or stays locked)
w'   = w_nb - sign(w_nb) * db  otherwise
```

A wheel with `w = 0` and a drive torque below the brake and rolling torque stays at 0. A locked wheel is released when the
tyre torque `R |fx|` of the sliding patch plus the drive torque exceeds the brake torque, with no hysteresis. A negative
(engine braking) `drive_torque` goes into `T_drive`; it is not a brake.

### 4.5 End of the substep

```
sx_i1 = (R w' - Vx) / Wx                        instantaneous slip with the new spin (Wx frozen)
sx_r' = sx_r + a (sx_i1 - sx_r)                 sy_r' = sy_r + a (sy_i - sy_r)
Fend  = tyre_force(R, Fz, sx_r', sy_r', 0, hypot(R w' - Vx, Vy), speed, ...)
n_prev = Fend.n
```

### 4.6 The tick: mean force and the low-speed limit

The tick's force is the **trapezoid mean** of the substep end forces, with `F_0` the force of the relaxed state at the start
of the tick (its slide speed from the start spin):

```
F_mean = ( F_0 / 2 + F_1 + ... + F_{N-1} + F_N / 2 ) / N        for fx, fy and mz
```

The trapezoid makes the mean second-order accurate in `h`, which is what lets 10 substeps match 96 to within 2 % of the peak
force on the first tick of a brake or a launch (4.9). Then the **momentum limit**, with `mc = corner_mass_kg` (the query field)
and `kc = stick_cap = 0.5`:

```
cap_y = kc * mc * |Vy| / dt          if |fy_mean| > cap_y:  fy_mean = sign(fy_mean) * cap_y        (always, it opposes Vy)
cap_x = kc * mc * |Vx| / dt          if fx_mean * Vx < 0 and |fx_mean| > cap_x:  fx_mean = sign(fx_mean) * cap_x
```

The longitudinal limit applies only to a force that **opposes** the motion, so that a launch from `Vx = 0` is never held
back. The limit stops a stiff low-speed tyre from reversing a creeping car within one 60 Hz tick (a force of `m |V| / dt`
would stop it exactly; half of it cannot overshoot). It is inactive above about 0.35 m/s for forces up to the tyre's peak. The wheel's own
spin integration does not see the limit. `aligning` is not limited.

### 4.7 The airborne wheel

`loaded = false` or `Fz <= 0`: no tyre force, no relaxation. Per substep, with `c = air_drag_nms` (N m per rad/s):

```
w_nb = ( w + h * T_drive / I ) / ( 1 + h * c / I )
w'   = 0 if |w_nb| <= h * brake_torque / I else w_nb - sign(w_nb) * h * brake_torque / I
```

Outputs: forces and `aligning` 0, `traction` 0, `slip` 0, `slip_angle` 0, `wheel_spin = w'` after `N` substeps. The state is
marked `fresh` with `sx_r = sy_r = 0`. A wheel that touches down starts from an unloaded patch and builds its force over the
relaxation length.

### 4.8 The call, in order

```
fn tyre_call(call) -> Result:
    if not valid(call) or role != 0 or no params: mark fresh; return NOT_HANDLED
    N = substep_count(dt); h = dt / N
    if not loaded or Fz <= 0:
        for each of N substeps: w = airborne_step(w, h)               4.7
        mark fresh; return { 0, 0, 0, w, traction 0, slip 0, slip_angle 0, handled }
    if fresh: seed sx_r, sy_r from 4.1 at the start spin; n_prev = 0; fresh = false
    F_0 = tyre_force at (sx_r, sy_r) with the start spin
    sum = F_0 / 2
    for k in 1..N:
        compute Wx, Wy, sx_i, sy_i, L, lam, a                          4.1, 4.2
        F0, S -> dw -> w_nb                                             4.3
        w = brake_clamp(w_nb)                                           4.4
        update sx_r, sy_r, n_prev, F_k                                  4.5
        sum += (k == N) ? F_k / 2 : F_k
    mean = sum / N; apply the momentum limit                            4.6
    return { mean.fx, mean.fy, mean.mz, w, traction = (n_prev <= 1 ? 1 : 1 / n_prev),
             slip = R w - Vx, slip_angle = atan(sy_r) / (2 pi), handled }
```

### 4.9 The substep rule

**Held for the whole tick** (zero-order hold at the values the host passed): `Fz`, `Vx`, `Vy`, `drive_torque`,
`brake_torque`, the radius and inertia, the three surface multipliers, the temperatures, `loaded` and `corner_mass_kg`. **Evolving inside the tick** (the mod's own state): the wheel spin, `sx_r`, `sy_r`, `n_prev`, and the
quantities derived from them (`Wx`, `Wy`, `L`, `lam`, `S`, the force). **Interpolated: nothing.** The host's patch velocity is
not extrapolated, because the host integrates the body *after* the call with our mean force; extrapolating the velocity here
would count the body's reaction twice. The price (the car does not slow under its own braking force within a tick) is paid
by the momentum limit of 4.6 at low speed and is negligible above a few m/s. **Returned**: the trapezoid mean of the forces
and torques; the spin, `slip`, `slip_angle` and `traction` at the **end**.

**Number of substeps**: `N = clamp(ceil(dt * 600 - 1e-4), 1, 24)`, `h = dt / N`, so about 600 steps per second:

| `dt` | 1/60 | 1/144 | 1/240 | 1/30 | 1/10 | 1/1000 |
|---|---|---|---|---|---|---|
| `N` | 10 | 5 | 3 | 20 | 24 (capped) | 1 |

A setting `substeps` (0 = automatic, 1 to 24) overrides it for tests.

**Why this number.** *Stability* is not what sets it: the wheel-slip loop of 4.3 is solved linearly implicitly, so it is stable
for any `h` whatever the tyre stiffness, and the slip lag of 4.2 is an exact backward Euler step, stable for any rate. (An
explicit step would need `h < 2 I Wx / (R^2 S)`: with the launch vector `S = 1.28e5 N`, `Wx = 1`, `I = 3`, the stiff mode is
`R^2 S / (I Wx) = 4.6e3 / s` and `h < 0.43 ms`, about 40 substeps per 60 Hz tick; with the host's unscaled inertia of 10
the bound is 3.3 times looser.) What stays explicit is the frozen coefficients (`Wx`, `L`, `S`), the brake clamp and the end-of-substep
force, so the error is first order in `h` and **accuracy** sets `N`: at `h = 1/600 s` the mean force of the first tick of a
brake, launch or cornering step is within 2.5 % of the peak force `mu Fz` of the 96-substep result, at every host `dt`
(vectors W10). The cost is `4 wheels x 10 substeps x 4 force evaluations` per car per tick, 160 evaluations, a few
microseconds.

## 5. Constants

| Key | Unit | Default | Range | Origin and reason |
|---|---|---|---|---|
| `v_low_x`, `v_low_y` | m/s | 1.0, 0.4 | 0.2 to 3, 0.1 to 2 | [D] slip floors (4.1); the lateral one is smaller as in the reference game's stiffer standstill hold [U] |
| `relax_len` | m | 0.34 | 0.05 to 1.5 | [D] relaxation length at `fz0` while gripping; road tyres 0.2 to 0.7 [T] |
| `relax_slide_ratio` | 1 | 0.5 | 0.2 to 1 | [U] sliding relaxation length over gripping |
| `relax_load_slope` | 1 | 0.3 | 0 to 1 | [U] relative growth of the length per `Fz / fz0` |
| `relax_tau_max` | s | 0.08 | 0.02 to 0.3 | [U] shape (a floor on the lag rate), [D] value: the longest lag of a standing wheel |
| `stick_cap` | 1 | 0.5 | 0.1 to 1 | [D] share of `m |V| / dt` the force may reach; 1 stops the car in one tick, above 2 overshoots |
| `rr_c0` | 1 | 0.012 | 0 to 0.05 | [D] rolling resistance coefficient of a road tyre |
| `rr_c2` | s^2/m^2 | 1.5e-6 | 0 to 1e-5 | [D] its growth with speed squared (0.020 at 73 m/s) |
| `air_drag_nms` | N m s | 0.05 | 0 to 1 | [D] bearing and air drag of a free wheel |
| `inertia_scale` | 1 | 0.3 | 0.05 to 2 | [D] multiplies the host's wheel inertia (10), giving 3 kg m^2; see contract point 3 |
| `substeps` | 1 | 0 (auto) | 0 to 24 | the section 4.9 rule |
| `600` (substeps per second), `1e-3` (slope step), `0.3` (`Lg` floor) | | fixed | | [D] |

## 6. Test vectors

Python double-precision script implementing only sections 4 and 5 of this file and of `tyre-force.md`. Common inputs:
`R = 0.33`, host inertia 10 (so `I = 3`), `dt = 1/60` (`N = 10`) unless stated, the `sport` tyre of `tyre-force.md` (tyre width
0.225), surfaces and `grip` 1, no camber, `corner_mass_kg = Fz / 9.81` (so the momentum limit has the same numbers as for a car at rest),
fresh state at the first call. Tolerance: forces relative `1e-3` or `0.5 N`;
spin `1e-3 rad/s`; `slip` `1e-3 m/s`; `traction` `1e-3`; `slip_angle` `1e-4`.

**One substep with every intermediate (`h = 1/600`, fresh state, `I = 3`)**:

| | brake | launch | cornering with drive |
|---|---|---|---|
| inputs `Fz, Vx, Vy, w, T_drive, T_brake` | 4000, 30, 0, 90.9091, 0, 2500 | 4500, 0, 0, 0, 1800, 0 | 4000, 20, 3, 60.6061, 600, 0 |
| `Wx`, `Wy` | 30, 30 | 1, 0.4 | 20, 20 |
| `sx_i`, `sy_i` | 0, 0 | 0, 0 | 0, 0.15 |
| `L`, `lam`, `a` | 0.442, 67.8733, 0.101626 | 0.4675, 12.5, 0.020408 | 0.442, 45.7551, 0.070855 |
| `F0.fx`, `F0.fy` | 0, 0 | 0, 0 | 0, -4304.6562 |
| `S` | 119995.69 | 127819.09 | 45121.19 |
| numerator, denominator of `dw` | 0, 3.073778 | 3.000000, 3.473452 | 1.000000, 3.029013 |
| `w_nb` | 90.909091 | 0.863694 | 60.936201 |
| `t_rr`, `db` | 17.6220, 1.398679 | 17.8200, 0.009900 | 16.6320, 0.009240 |
| `w'` | 89.510412 | 0.853794 | 60.926961 |
| `sx_r'`, `sy_r'` | -0.001564, 0 | 0.005750, 0 | 0.000375, 0.15 |
| `F_end`: `fx`, `fy`, `mz`, `n` | -186.6388, 0, 0, 0.01298 | 707.1845, 0, 0, 0.04582 | 16.9286, -4304.6210, 12.8340, 0.88943 |

**Whole ticks** (`wheel_tick`, state carried between ticks of a case):

| # | Case and tick | `force_long` | `force_lat` | `aligning` | `wheel_spin` | `traction` | `slip` | `slip_angle` |
|---|---|---|---|---|---|---|---|---|
| W1 | free rolling, `Fz 4000, Vx 30, w = Vx/R` | -17.535 | 0 | 0 | 90.845 | 1.000 | -0.021 | 0 |
| W5 | standstill, `Fz 4000`, nothing else | 0 | 0 | 0 | 0 | 1.000 | 0 | 0 |
| W2 | launch (`Fz 4500, V 0, T_drive 1800`), tick 1 | 3733.630 | 0 | 0 | 2.396 | 0.972 | 0.791 | 0 |
| | tick 2 | 4987.318 | 0 | 0 | 3.102 | 0.462 | 1.024 | 0 |
| | tick 6 | 4544.176 | 0 | 0 | 8.912 | 0.181 | 2.941 | 0 |
| W3 | brake (`Fz 4000, Vx 30, w = Vx/R, T_brake 2500`), tick 1 | -2059.031 | 0 | 0 | 80.872 | 1.000 | -3.312 | 0 |
| | tick 2 | -4678.079 | 0 | 0 | 75.536 | 0.755 | -5.073 | 0 |
| | tick 3 | -4399.293 | 0 | 0 | 69.643 | 0.469 | -7.018 | 0 |
| | tick 6 | -4057.166 | 0 | 0 | 50.447 | 0.172 | -13.353 | 0 |
| | tick 12 | -3881.672 | 0 | 0 | 9.935 | 0.020 | -26.721 | 0 |
| W4 | lock-up (as W3, `T_brake 6000`), tick 1 | -3173.431 | 0 | 0 | 63.562 | 0.539 | -9.024 | 0 |
| | tick 2 | -4108.696 | 0 | 0 | 37.702 | 0.136 | -17.558 | 0 |
| | tick 3 | -3931.278 | 0 | 0 | 11.497 | 0.036 | -26.206 | 0 |
| | tick 8 (locked, `w = 0` held) | -3848.027 | 0 | 0 | 0 | 0.004 | -30.000 | 0 |
| W6 | creeping sideways, `Fz 4000, V 0, Vy 0.01` (cap 122.324) | 0 | -122.324 | | 0 | 1.000 | 0 | 0.004 |
| | `Vy 0.05` (cap 611.621) | 0 | -611.621 | | 0 | 1.000 | 0 | 0.020 |
| | `Vy 0.3` (cap 3669.725) | 0 | -3669.725 | | 0 | 0.225 | 0 | 0.102 |
| W7a | airborne, `w 60`, no torque | 0 | 0 | 0 | 59.983 | 0 | 0 | 0 |
| W7b | airborne, `w 60`, `T_brake 300` | | | | 58.317 | | | |
| W7c | airborne, `w 0.5`, `T_brake 300` (locks) | | | | 0 | | | |
| W8 | cornering `Fz 4000, Vx 20, Vy 3, w = Vx/R`, tick 1 | -11.067 | -4304.666 | 12.815 | 60.535 | 1.000 | -0.023 | 0.024 |
| | tick 2 | -41.845 | -4304.616 | 12.742 | 60.520 | 1.000 | -0.028 | 0.024 |
| | tick 5 | -51.944 | -4304.577 | 12.711 | 60.539 | 1.000 | -0.022 | 0.024 |
| | tick 30 | -51.226 | -4304.580 | 12.713 | 60.537 | 1.000 | -0.023 | 0.024 |
| W9 | reversing: W3 with `Vx = -30`, `w = Vx/R`, tick 1 | +2059.031 | 0 | 0 | -80.872 | 1.000 | +3.312 | 0 |
| W9b | reversing launch, `T_drive = -1800`, tick 1 | -3733.630 | 0 | 0 | -2.396 | 0.972 | -0.791 | 0 |

Notes: the W1 force is the rolling resistance (the wheel decelerates until a slip of `-0.021 m/s` produces it); W6 is capped
in every row, as `kc m |Vy| / dt` is smaller than the force the stiff tyre would give; the lateral force of W8 is the mean of
the relaxed lateral slip (initialised to the instantaneous `0.15`) and stays at `mu(4000) = 1.076` of the load, the tyre not
yet at its peak (`n = 0.889`).

**W10 substep convergence** at `dt = 1/60`. Mean `force_long` and end `wheel_spin` for the number of substeps `N`, against
`N = 96` (state carried from a fresh wheel for `k` ticks). Acceptance (the unit test): at the automatic `N`, the mean force
differs from the `N = 96` result by at most **2.5 % of the peak force** (`mu_x0 Fz`, 120 N at 4000 to 4500 N), going from `N`
to `2N` changes it by at most **1.5 %** of the peak, and `R * wheel_spin` differs by at most **0.4 m/s**.

| Case (`Fz`, `Vx`, `Vy`, `w`, `T_drive`, `T_brake`) | `N` = 1 | 2 | 5 | **10** | 20 | 40 | `N` = 96 (ref) |
|---|---|---|---|---|---|---|---|
| brake onset, tick 1 (4000, 30, 0, 90.909, 0, 2500): `fx` | -2335.19 | -2430.68 | -2156.01 | **-2059.03** | -2018.09 | -2000.43 | -1991.14 |
| `wheel_spin` | 76.9223 | 79.8653 | 80.8737 | **80.8715** | 80.7624 | 80.6748 | 80.6125 |
| launch, tick 1 (4500, 0, 0, 0, 1800, 0): `fx` | 1652.25 | 2746.15 | 3582.04 | **3733.63** | 3742.48 | 3727.15 | 3712.60 |
| `wheel_spin` | 0.5987 | 1.0454 | 1.8260 | **2.3962** | 2.7415 | 2.9283 | 3.0389 |
| launch, tick 6: `fx` | 5348.45 | 4749.23 | 4555.33 | **4544.18** | 4541.27 | 4540.38 | 4540.03 |
| `wheel_spin` | 0.4131 | 3.0551 | 7.5632 | **8.9115** | 9.5138 | 9.7873 | 9.9344 |
| cornering, tick 1 (4000, 20, 3, 60.606, 0, 0): `fx` | -14.88 | -12.09 | -11.40 | **-11.07** | -10.86 | -10.75 | -10.68 |
| `fy` (all within 0.06 N of -4304.67) | | | | -4304.67 | | | -4304.67 |
| lock-up, tick 3 (T_brake 6000): `fx` | -3941.84 | -3940.90 | -3934.85 | **-3931.28** | -3929.01 | -3927.73 | -3926.94 |
| `wheel_spin` | 6.0069 | 10.5973 | 11.6338 | **11.4967** | 11.2830 | 11.1306 | 11.0274 |

At the other host rates the automatic `N` is also inside the 2.5 % bound: `dt = 1/144, N = 5`, brake onset tick 1:
`fx = -626.93` (96 substeps: `-525.60`), `wheel_spin = 85.5887` (`85.4885`); `dt = 1/240, N = 3`: `fx = -289.91` (`-205.38`),
`wheel_spin = 87.5415` (`87.5080`).

Further properties (property tests): the outputs are finite for any finite inputs in range, including `Fz` jumping between
0 and 9000 N, `|w| <= 250`, `|V| <= 60`, torques of 15 kN m, any `dt` of the table, any inertia from 1.2 to 30; the mirror
(`Vx`, `w`, `T_drive` negated) negates `force_long` and `wheel_spin` and leaves the lateral force and the aligning torque
unchanged for the same `Vy` (to `1e-4` relative);
a standing wheel with no torque stays at `w = 0` with zero force; a wheel under a brake torque above its tyre torque reaches
`w = 0` and stays there.

## 7. Limits

- The body does not react inside the tick: the patch velocity is held ([4.9](#49-the-substep-rule)). At very low speed the
  momentum limit stands in for that; it is an approximation of one wheel against a quarter of the mass.
- The momentum limit stands one wheel against a quarter of the mass, with the host's `corner_mass_kg`; it is least accurate for a wheel nearly off the ground, where the real wheel carries little of the car.
- No hub or suspension motion: stage 3 integrates the hub in the suspension hook, with its own substep rule ([suspension.md](suspension.md) section 4), not this one.
- The relaxation acts on the slips, not on the contact patch's deflection; combined with the pure-function force curve the
  transient response is first order. High-frequency effects (tyre belt modes, shimmy) are absent.
- No ABS, traction control or brake bias here ([aids.md](aids.md)); the brake and drive torques arrive from the host.
- The wheel spin is one rigid inertia: no drivetrain compliance, no differential inside the mod.
- Known to differ from the reference game: the exact low-speed lag behaviour (its floor on the lag rate is per step, ours per
  second), and everything thermal.
