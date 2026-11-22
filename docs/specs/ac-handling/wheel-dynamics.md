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

Written by: spec author session (Claude), 2026-11-20; revised by the spec maintainer session, 2026-11-20 (inertia default, vectors)   Implemented by: (pending)

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
| `camber_rad` | rad | camber of this wheel this tick, outward positive (section 4 converts it to the wheel frame's sign); 0 until the suspension hook is on |
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

`handled = false` when: `role != 0`; no parameter set for the car; `dt <= 0` or `dt > 0.1 * (1 + 1e-6)` (the host sends `dt` as `f32`, so
0.1 s arrives as 0.100000001 and must still be handled); `wheel_radius_m < 0.05` or
`wheel_inertia_kgm2 <= 0`; `corner_mass_kg <= 0`; any input is NaN or infinite; `vertical_load_n < 0`. A `not handled` call leaves the output
fields at zero and marks the wheel's state `fresh` (section 3), so it re-seeds when handled again.

**Contract points and the choices made** (checked against [api-handling.md](../../modding/api-handling.md) and ADR 0016):

1. **`corner_mass_kg` is a query field the host is adding.** Its name and offset are fixed in the interface page, not here; this
   spec consumes it as defined in the table above. A host that does not send it (the field reads 0) gives `not handled`, so the
   mod never runs on a guessed mass.
2. **The patch velocity is that of the start of the tick** (the state the host holds when it calls the hook, before it integrates
   the body). It is held for the whole tick (4.9).
3. **`wheel_inertia_kgm2` is the host's constant (10) and the mod uses it unscaled: the default `inertia_scale` is 1.** A bare
   road wheel is only 1.2 to 2.5 kg m^2, but the host's 10 stands for the wheel *and the driveline it is coupled to* (an engine of
   0.2 kg m^2 reflects `0.2 * (3.6 * 3.5)^2 = 32 kg m^2` to the driven axle in first gear [T]). The reason it is not scaled down is
   the host's drivetrain loop: the host computes the next tick's drive torque from the spin this hook returns, so wheel and
   powertrain form a feedback loop with one tick of delay. Driven on the real host (the real loader, a real vehicle with one gear
   and one tyre, 60 Hz), a wheel of `I = 3` (`inertia_scale 0.3`) **limit-cycles** that loop: the drive torque swings tick by tick
   between about +1800 and -1500 N m, the driven wheels' spin changes sign and the car crawls at about 2 m/s under full throttle;
   the same at 0.35, 0.4 and 0.45; **0.5 and above is stable** (observed). A delayed feedback of gain `K dt / I` (K the loop's
   stiffness in N m s/rad) is stable only below 2, so the lighter the wheel the stiffer the loop gets: the observed threshold, an
   effective inertia between 4.5 and 5 kg m^2, puts `K` near 600 N m s/rad (our reasoning, not a measurement). The consequence
   for the specification: the **default is 1.0**; `inertia_scale` stays a documented key (section 5) with a **stable range of 0.5 to 2
   on the real host** (the product of the host's constant and the key is what matters, not the key; the threshold was not
   measured at other tick rates and the reasoning above says it can only fall with `dt`, so 1.0 is safe at 144 and 240 Hz). A value
   below 0.5 is accepted by the loader (the planar harness of [scenarios.md](scenarios.md) runs 0.3 and has no drivetrain loop) but
   logs one warning. A host that integrated the wheel and the driveline together implicitly would lift the lower bound; that is
   the host's change, not the mod's.
4. **Camber is an input since `bb.handling` 1.1** (`camber_rad`, a query field, outward positive on either side, the suspension
   hook's answer of the same tick, 0 when none answered or on an older host); the earlier text said the query had none. It is 0 in
   stage 2 ([tyre-force.md](tyre-force.md) section 7) and the vectors below use none. **There is no
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
[tyre-force.md](tyre-force.md) with the arguments `(radius, Fz, sx, sy, gamma, slide_speed, speed, surf_lat, surf_drive, grip = 1)`,
`gamma` being the wheel-frame camber of this tick (`+camber_rad` on wheels 1 and 3, `-camber_rad` on 0 and 2; 0 when the query's
`camber_rad` is 0), held for the whole tick;
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
Fend  = tyre_force(R, Fz, sx_r', sy_r', gamma, hypot(R w' - Vx, Vy), speed, ...)
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
explicit step would need `h < 2 I Wx / (R^2 S)`: with the launch vector `S = 1.28e5 N`, `Wx = 1` and the default `I = 10`, the
stiff mode is `R^2 S / (I Wx) = 1.4e3 / s` and `h < 1.4 ms`, about 12 substeps per 60 Hz tick; with the light wheel of
`inertia_scale 0.3` (`I = 3`) it is `4.6e3 / s`, `h < 0.43 ms` and about 40 substeps.) What stays explicit is the frozen coefficients (`Wx`, `L`, `S`), the brake clamp and the end-of-substep
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
| `inertia_scale` | 1 | 1.0 | 0.05 to 2 (stable on the real host: 0.5 to 2; a value below 0.5 logs a warning) | [D] multiplies the host's wheel inertia (10); 1.0 because the host's drivetrain limit-cycles below 0.5, see contract point 3 |
| `substeps` | 1 | 0 (auto) | 0 to 24 | the section 4.9 rule |
| `600` (substeps per second), `1e-3` (slope step), `0.3` (`Lg` floor) | | fixed | | [D] |

## 6. Test vectors

Python double-precision script implementing only sections 4 and 5 of this file and of `tyre-force.md`. Common inputs:
`R = 0.33`, host inertia 10 with the default `inertia_scale = 1` (so `I = 10`), `dt = 1/60` (`N = 10`) unless stated, the
`sport` tyre of `tyre-force.md` (tyre width 0.225), surfaces and `grip` 1, no camber, `corner_mass_kg = Fz / 9.81` (so the
momentum limit has the same numbers as for a car at rest) except in the airborne rows W7, whose load is 0 and which send
`corner_mass_kg = 350` (a zero mass would make section 2 decline the call), fresh state at the first call. Tolerance: forces
relative `1e-3` or `0.5 N`; spin `1e-3 rad/s`; `slip` `1e-3 m/s`; `traction` `1e-3`; `slip_angle` `1e-5` turns absolute (the
tables give it to six decimals).

**One substep with every intermediate (`h = 1/600`, fresh state, `I = 10`)**:

| | brake | launch | cornering with drive |
|---|---|---|---|
| inputs `Fz, Vx, Vy, w, T_drive, T_brake` | 4000, 30, 0, 90.9091, 0, 2500 | 4500, 0, 0, 0, 1800, 0 | 4000, 20, 3, 60.6061, 600, 0 |
| `Wx`, `Wy` | 30, 30 | 1, 0.4 | 20, 20 |
| `sx_i`, `sy_i` | 0, 0 | 0, 0 | 0, 0.15 |
| `L`, `lam`, `a` | 0.442, 67.8733, 0.101626 | 0.4675, 12.5, 0.020408 | 0.442, 45.7551, 0.070855 |
| `F0.fx`, `F0.fy` | 0, 0 | 0, 0 | 0, -4304.6562 |
| `S` | 119995.69 | 127819.09 | 45121.19 |
| numerator, denominator of `dw` | 0, 10.073778 | 3.000000, 10.473452 | 1.000000, 10.029013 |
| `w_nb` | 90.909091 | 0.286439 | 60.705771 |
| `t_rr`, `db` | 17.6220, 0.419604 | 17.8200, 0.002970 | 16.6320, 0.002772 |
| `w'` | 90.489487 | 0.283469 | 60.702999 |
| `sx_r'`, `sy_r'` | -0.000469, 0 | 0.001909, 0 | 0.000113, 0.15 |
| `F_end`: `fx`, `fy`, `mz`, `n` | -56.5260, 0, 0, 0.00389 | 242.2207, 0, 0, 0.01521 | 5.1139, -4304.6530, 12.8347, 0.88942 |

**Whole ticks** (`wheel_tick`, state carried between ticks of a case):

| # | Case and tick | `force_long` | `force_lat` | `aligning` | `wheel_spin` | `traction` | `slip` | `slip_angle` |
|---|---|---|---|---|---|---|---|---|
| W1 | free rolling, `Fz 4000, Vx 30, w = Vx/R` | -5.903 | 0 | 0 | 90.883 | 1.000 | -0.0086 | 0 |
| W5 | standstill, `Fz 4000`, nothing else | 0 | 0 | 0 | 0 | 1.000 | 0 | 0 |
| W2 | launch (`Fz 4500, V 0, T_drive 1800`), tick 1 | 2452.414 | 0 | 0 | 1.468 | 1.000 | 0.485 | 0 |
| | tick 2 | 5256.930 | 0 | 0 | 1.520 | 0.857 | 0.501 | 0 |
| | tick 6 | 4639.845 | 0 | 0 | 2.834 | 0.267 | 0.935 | 0 |
| W3 | brake (`Fz 4000, Vx 30, w = Vx/R, T_brake 2500`), tick 1 | -792.789 | 0 | 0 | 87.176 | 1.000 | -1.232 | 0 |
| | tick 2 | -3031.551 | 0 | 0 | 84.685 | 1.000 | -2.054 | 0 |
| | tick 3 | -4358.273 | 0 | 0 | 82.900 | 1.000 | -2.643 | 0 |
| | tick 6 | -4674.815 | 0 | 0 | 78.171 | 0.792 | -4.203 | 0 |
| | tick 12 | -4215.627 | 0 | 0 | 67.380 | 0.363 | -7.764 | 0 |
| W4 | lock-up (as W3, `T_brake 6000`), tick 1 | -1709.085 | 0 | 0 | 81.876 | 1.000 | -2.981 | 0 |
| | tick 2 | -4588.763 | 0 | 0 | 74.407 | 0.739 | -5.446 | 0 |
| | tick 3 | -4354.903 | 0 | 0 | 66.783 | 0.408 | -7.962 | 0 |
| | tick 8 | -3940.644 | 0 | 0 | 27.748 | 0.063 | -20.843 | 0 |
| | tick 12 (locked, `w = 0` from here on) | -3850.839 | 0 | 0 | 0 | 0.004 | -30.000 | 0 |
| W6 | creeping sideways, `Fz 4000, V 0, Vy 0.01` (cap 122.324) | 0 | -122.324 | | 0 | 1.000 | 0 | 0.003978 |
| | `Vy 0.05` (cap 611.621) | 0 | -611.621 | | 0 | 1.000 | 0 | 0.019792 |
| | `Vy 0.3` (cap 3669.725) | 0 | -3669.725 | | 0 | 0.225 | 0 | 0.102416 |
| W7a | airborne, `w 60`, no torque | 0 | 0 | 0 | 59.995 | 0 | 0 | 0 |
| W7b | airborne, `w 60`, `T_brake 300` | | | | 59.495 | | | |
| W7c | airborne, `w 0.5`, `T_brake 300` (locks) | | | | 0 | | | |
| W8 | cornering `Fz 4000, Vx 20, Vy 3, w = Vx/R`, tick 1 | -3.577 | -4304.663 | 12.828 | 60.580 | 1.000 | -0.0085 | 0.023701 |
| | tick 2 | -16.273 | -4304.677 | 12.801 | 60.562 | 1.000 | -0.0146 | 0.023708 |
| | tick 5 | -44.353 | -4304.616 | 12.732 | 60.541 | 1.000 | -0.0216 | 0.023720 |
| | tick 30 | -50.644 | -4304.584 | 12.715 | 60.538 | 1.000 | -0.0224 | 0.023723 |
| W9 | reversing: W3 with `Vx = -30`, `w = Vx/R`, tick 1 | +792.789 | 0 | 0 | -87.176 | 1.000 | +1.232 | 0 |
| W9b | reversing launch, `T_drive = -1800`, tick 1 | -2452.414 | 0 | 0 | -1.468 | 1.000 | -0.485 | 0 |

Notes: the W1 force is the rolling resistance (the wheel decelerates until a slip of `-0.0086 m/s` produces it); W6 is capped
in every row, as `kc m |Vy| / dt` is smaller than the force the stiff tyre would give; the lateral force of W8 is the mean of
the relaxed lateral slip (initialised to the instantaneous `0.15`) and stays at `mu(4000) = 1.076` of the load, the tyre not
yet at its peak (`n = 0.889`). The heavy wheel of W4 locks late (tick 12): its spin is still 27.7 rad/s at tick 8.

**The `inertia_scale` key at 0.3** (`I = 3`, the light wheel of the earlier draft; the key must still be honoured and the planar
harness may run it, section 5): W2 tick 1: `force_long` 3733.630, `wheel_spin` 2.396, `traction` 0.972, `slip` 0.791; W3 tick 1:
-2059.031, 80.872, 1.000, -3.312; W4 tick 1: -3173.431, 63.562, 0.539, -9.024; W4 tick 8 (already locked, `w = 0`): -3848.027,
0, 0.004, -30.000. All other columns as in the table above.

**W10 substep convergence** at `dt = 1/60`, `I = 10`. Mean `force_long` and end `wheel_spin` for the number of substeps `N`,
against `N = 96` (state carried from a fresh wheel for `k` ticks). Acceptance (the unit test): at the automatic `N`, the mean
force differs from the `N = 96` result by at most **2.5 % of the peak force** (`mu_x0 Fz`, about 120 N at 4000 to 4500 N), going
from `N` to `2N` changes it by at most **1.5 %** of the peak, and `R * wheel_spin` differs by at most **0.4 m/s**. (The worst
cases at `N = 10`: 62.8 N, 27.5 N and 0.11 m/s.)

| Case (`Fz`, `Vx`, `Vy`, `w`, `T_drive`, `T_brake`) | `N` = 1 | 2 | 5 | **10** | 20 | 40 | `N` = 96 (ref) |
|---|---|---|---|---|---|---|---|
| brake onset, tick 1 (4000, 30, 0, 90.909, 0, 2500): `fx` | -1199.17 | -1003.07 | -846.06 | **-792.79** | -766.66 | -753.80 | -746.37 |
| `wheel_spin` | 86.7131 | 87.0892 | 87.1896 | **87.1755** | 87.1542 | 87.1391 | 87.1288 |
| launch, tick 1 (4500, 0, 0, 0, 1800, 0): `fx` | 1596.09 | 2203.26 | 2456.14 | **2452.41** | 2424.93 | 2404.07 | 2389.63 |
| `wheel_spin` | 0.5703 | 0.9008 | 1.2905 | **1.4684** | 1.5641 | 1.6130 | 1.6417 |
| launch, tick 6: `fx` | 5385.19 | 4901.57 | 4681.26 | **4639.85** | 4623.39 | 4616.10 | 4612.14 |
| `wheel_spin` | 0.5323 | 1.3843 | 2.4610 | **2.8338** | 3.0144 | 3.1024 | 3.1527 |
| cornering, tick 1 (4000, 20, 3, 60.606, 0, 0): `fx` | -4.46 | -3.83 | -3.70 | **-3.58** | -3.49 | -3.44 | -3.40 |
| `fy` (all within 0.01 N of -4304.66) | | | | -4304.66 | | | -4304.66 |
| lock-up, tick 3 (T_brake 6000): `fx` | -4418.05 | -4387.30 | -4364.55 | **-4354.90** | -4349.39 | -4346.41 | -4344.60 |
| `wheel_spin` | 65.9910 | 66.7368 | 66.8547 | **66.7829** | 66.7161 | 66.6736 | 66.6458 |

At the other host rates the automatic `N` is also inside the 2.5 % bound: `dt = 1/144, N = 5`, brake onset tick 1:
`fx = -199.08` (96 substeps: `-163.01`), `wheel_spin = 89.2093` (`89.1986`); `dt = 1/240, N = 3`: `fx = -89.18` (`-62.07`),
`wheel_spin = 89.8720` (`89.8687`).

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
- The host's drivetrain reads the returned spin one tick late: the wheel must keep the effective inertia of contract point 3
  (at least about 5 kg m^2 at 60 Hz) or the host's drive torque limit-cycles. The mod cannot see that loop.
- Known to differ from the reference game: the exact low-speed lag behaviour (its floor on the lag rate is per step, ours per
  second), and everything thermal.

## Changes (revision of 2026-11-20, for the implementer's tests)

| Item | Before | Now |
|---|---|---|
| default `inertia_scale` | 0.3 (`I = 3`) | **1.0** (`I = 10`); range 0.05 to 2 kept, stable 0.5 to 2 on the host, warning below 0.5 |
| common inputs of section 6 | `I = 3` | `I = 10`; airborne rows W7 send `corner_mass_kg = 350` |
| one-substep table, W1 to W9b, W10 | computed at `I = 3` | all recomputed at `I = 10` (section 6); the `I = 3` values of W2, W3, W4 kept as the `inertia_scale 0.3` check |
| `slip_angle` | 3 decimals, tolerance `1e-4` (contradictory) | 6 decimals, tolerance `1e-5` absolute |
| W4 locks at | tick 8 | tick 12 (at `I = 10`; tick 4 at `I = 3`) |
| `dt` guard | `dt > 0.1` | `dt > 0.1 * (1 + 1e-6)` |
| W10 acceptance | 2.5 %, 1.5 %, 0.4 m/s | unchanged and met (worst 62.8 N, 27.5 N, 0.11 m/s) |
