# Tyre force
Sources:
- Pacejka, *Tyre and Vehicle Dynamics*, 3rd ed., Elsevier 2012 (published book, no license; its brush-tyre chapter): the brush
  model's cubic force rise, "theoretical slip" `kappa/(1+kappa)` and `tan(alpha)/(1+kappa)`, pneumatic trail: textbook equations
  and facts, re-derived and written in our own notation.
- Milliken and Milliken, *Race Car Vehicle Dynamics*, SAE 1995 (published book): load sensitivity of the friction coefficient,
  the friction ellipse, camber thrust: understanding.
- Genta, *Motor Vehicle Dynamics: Modeling and Simulation*, World Scientific 1997, and Gillespie, *Fundamentals of Vehicle
  Dynamics*, SAE 1992 (published books): slip definitions, relaxation, aligning moment: understanding.
- rustyAC, https://github.com/thesupersupersigma/rustyAC, `GPL-3.0-or-later` (a port of Assetto Corsa's binary), only
  `docs/port/sctm.md`, `docs/port/tyre_step.md`, `crates/rustyac-physics/src/tyre/` and `src/data/tyres_ini.rs`: understanding
  of what the reference game's tyre does (a normalised brush-like curve with a cubic rise and a hyperbolic fall-off to a
  sliding level, one combined slip magnitude split by direction cosines, a power-law load sensitivity referenced to a fixed
  load, a peak slip that grows with load, grip loss with sliding speed, a pneumatic trail that vanishes near the peak) and
  facts (the numeric constants marked `[U]` below). No code, comment or structure was copied; the equations and the
  pseudocode below are ours.
- This repository's `vehicle-suspension-tires.md`, `vehicle-tire-profiles.md`, `vehicle-calibration.md` and
  `ride6-tyre-mf94.md`: axes, units, wheel order and the host's `SurfaceGrip`: facts about our own engine.

Written by: spec author session (Claude), 2026-11-20   Implemented by: (pending)

Part of [the AC handling specs](README.md). Other pieces: [wheel dynamics](wheel-dynamics.md) (the state around this
function), [parameter mapping](parameter-mapping.md) (the values of the constants), [scenarios](scenarios.md) and
[targets](targets.md).

## 1. Purpose and scope

A **pure function** that turns the slip of one tyre and its vertical load into three outputs: the longitudinal force, the
lateral force and the self-aligning torque. It has the feel of a simulation tyre: a slip curve with a peak and a fall-off,
a friction coefficient that drops with load, one shared grip budget for the two directions, and a tyre that gets lighter in
the hands after the peak.

Inside scope: the curve, load sensitivity, combined slip, surface and user grip scaling, speed sensitivity, camber (an input
that is 0 until the suspension hook exists), aligning torque.

Out of scope (see section 7): tyre temperature, pressure and wear (`tyre-thermal.md`, stage 5), the wheel's spin and the
relaxation lag ([wheel-dynamics.md](wheel-dynamics.md)), rolling resistance (there too, it is a wheel torque), vertical
dynamics, rim contact, flat spots, wet weather.

## 2. Inputs and outputs

Physics space: x right, y up, z forward; the wheel frame has the same axes turned by the steering angle. Wheel order FL FR RL
RR. SI units; angles in radians in this file (the host reports the slip angle in turns, [wheel-dynamics.md](wheel-dynamics.md)
converts).

`tyre_force(p, radius, fz, sx, sy, gamma, slide_speed, speed, surf_lat, surf_drive, grip) -> {fx, fy, mz, n}`

| Input | Unit | Meaning |
|---|---|---|
| `p` | | the parameters of section 5 for this axle |
| `radius` | m | wheel radius, only for the contact patch length of the aligning torque |
| `fz` | N | vertical load, `>= 0`; `fz <= 0` gives all zero |
| `sx` | 1 | **longitudinal theoretical slip** (section 4.1), positive when the patch is driven forward |
| `sy` | 1 | **lateral theoretical slip**, positive when the patch slides towards +x (right) |
| `gamma` | rad | camber in the wheel frame's own sign: positive when the top of the wheel leans towards +x (right). The suspension spec reports camber **outward positive** (a left wheel's top leaning left is positive), so the mod converts: `gamma = +camber_out` on the right wheels (indices 1 and 3), `-camber_out` on the left wheels (0 and 2). It is 0 in stage 2. From stage 3 it is the value the mod's own suspension code computed (the tyre query carries no camber) |
| `slide_speed` | m/s | speed of the rubber over the road, `hypot(wheel speed minus ground speed, lateral speed)` |
| `speed` | m/s | patch speed, only to fade the aligning torque in at walking pace |
| `surf_lat`, `surf_drive` | 1 | the host's surface multipliers (`SurfaceGrip.lateral`, `.drive`) |
| `grip` | 1 | the total grip multiplier of this call: the player's setting `grip_gain` (1 = as tuned) times `tyre_condition` times the thermal factor `grip_scale(T_eff, P, w)` of [tyre-thermal.md](tyre-thermal.md) (1 until stage 5). It scales the peak friction **and** moves the peak slip (section 4.3) |

| Output | Unit | Meaning |
|---|---|---|
| `fx` | N | force on the car along the wheel's forward axis (+z), positive when `sx > 0` |
| `fy` | N | force on the car along the wheel's right axis (+x), **opposes** the slide: `fy < 0` when `sy > 0` |
| `mz` | N m | self-aligning torque about the vertical axis; positive turns the wheel's heading towards +x (right). A patch sliding to the right (`sy > 0`) gives `mz > 0`: the tyre tries to line the wheel up with where it is going |
| `n` | 1 | normalised combined slip: 0 at rest, **1 at the peak of the grip curve**, above 1 sliding |

**Relation to the usual slip quantities.** For a wheel rolling with ground speed `V > 0` along its axis, wheel speed
`R w`, lateral speed `Vy`:

```
kappa = (R w - V) / V            slip ratio (-1 braking to lock, positive driving)
alpha = atan(Vy / V)             slip angle (positive when the patch slides to +x)
sx    = kappa / (1 + kappa)      for kappa > -1          sy = tan(alpha) / (1 + kappa)
```

(`sx` is the slip speed over the wheel's own speed, `sy` the lateral speed over the wheel's own speed: Pacejka's theoretical
slip). [wheel-dynamics.md](wheel-dynamics.md) section 4.1 defines `sx`, `sy` directly from speeds with a floor so that they
have no singularity at standstill or at wheel lock; `kappa` and `alpha` are never needed.

## 3. State

None: the function is pure and allocation-free. The relaxed slips, the wheel speed and the last `n` live in
[wheel-dynamics.md](wheel-dynamics.md).

## 4. Equations

### 4.1 Load and grip scaling

```
fr    = fz / fz0                                    load ratio (fz0 is a constant of the tyre, section 5)
lf_y  = min(fr ^ (exp_y - 1), load_factor_max)      friction coefficient factor, load sensitivity (exp < 1: mu falls with load)
lf_x  = min(fr ^ (exp_x - 1), load_factor_max)
u_y   = clamp(grip * surf_lat,   0, 10)             total grip multiplier of each direction
u_x   = clamp(grip * surf_drive, 0, 10)
sv    = 1 / (1 + speed_sens * slide_speed)          grip lost with the speed of the rubber over the road
```

The peak force of a tyre therefore grows as `fz ^ exp` (for `exp_y = 0.85` doubling the load gives 1.8 times the force).
`load_factor_max` only stops `mu` from growing without bound at a few newtons of load.

### 4.2 Camber

```
sy_eff = sy - camber_gain * sin(gamma)              camber thrust: a lean towards +x pushes the force towards +x
gc     = -gamma * sgn_s(sy_eff)                     camber that leans into the force direction is positive (sgn_s: x / sqrt(x^2 + 1e-6))
cam    = clamp(1 + camber_d1 * gc - camber_d2 * gc^2, 0.5, 1.5)    peak grip gain up to an optimum camber, then loss
```

With the constants of section 5 the lateral peak gains about 3 % at 3 degrees of lean into the force and is back at 1 at
6 degrees; leaning the other way loses grip. Longitudinal grip has no camber term.

### 4.3 Peak friction and peak slip

```
mu_y = mu_y0 * u_y * lf_y * cam * sv
mu_x = mu_x0 * u_x * lf_x * sv          and  mu_x = mu_x * brake_gain  when sx < 0
s0   = tan(peak_angle0)                  (peak_angle0 in radians, converted from the TOML degrees)
flexf = clamp(1 + flex * (fr - 1), 0.3, 3)            the peak slip grows with load
s_py = s0              * flexf * clamp(1 + peak_follow * (u_y - 1), 0.3, 2)
s_px = s0 / cx_ratio   * flexf * clamp(1 + peak_follow * (u_x - 1), 0.3, 2)
```

`s_py`, `s_px` are the lateral and longitudinal slips at which the pure curve peaks. The coupling `peak_follow = 0.75` (a fixed constant, section 5) lets the peak
move a little with the grip multiplier: less grip (`u < 1`) peaks at a smaller slip, more grip at a larger one.
The longitudinal peak is at `1 / cx_ratio` of the lateral slip: tyres are stiffer longitudinally.

### 4.4 Normalised combined slip

```
hx = sx / s_px          hy = sy_eff / s_py                        slips in units of their own peak
n  = ( |hx|^p + |hy|^p )^(1/p)       p = comb_exp (2: ellipse)    one number for the whole contact
if n < 1e-9: return all zero (rolling without slip)
w  = hx^2 / (hx^2 + hy^2)            share of the longitudinal slip in the direction of sliding
```

### 4.5 The curve

One normalised curve `f(n)` with `f(0) = 0`, `f(1) = 1` (the peak), a sliding level far past the peak:

```
n <= 1:   f = 1 - (1 - n)^3                                          cubic rise (brush tyre, Pacejka)
n >  1:   f = a + (1 - a) / (1 + q (n - 1))                           hyperbolic fall-off to the level a
a = fall_y + w (fall_x - fall_y)          q = q_y + w (q_x - q_y)
```

The rise has slope `3` at `n = 0` (so the initial slope of the force is `3 mu Fz / s_p`, the tyre's cornering or longitudinal
stiffness) and slope `0` at the peak. The fall-off starts with slope `-(1 - a) q` (a visible peak, not a plateau) and tends to
`a`: `a` is the share of the peak that is left when the tyre slides completely, `q` how quickly it gets there. Both are blended
by `w`, so a pure-lateral slide (`w = 0`) uses the lateral values and a locked or spinning wheel (`w = 1`) the longitudinal
ones.

### 4.6 Forces

```
fx =  mu_x * fz * f(n) * hx / n
fy = -mu_y * fz * f(n) * hy / n
```

`hx / n` and `hy / n` are the direction cosines in normalised slip space: at the peak (`n = 1`) the force vector lies on the
super-ellipse `|fx / (mu_x fz)|^p + |fy / (mu_y fz)|^p = 1` (an ellipse for `p = 2`), and past the peak it shrinks along the same
direction. A tyre braking at the peak of the longitudinal curve has no lateral capacity left; a tyre cornering at the peak
has no longitudinal one.

**How the two directions differ** (all values are per tyre preset, [parameter-mapping.md](parameter-mapping.md)):

| | slip angle (lateral, `y`) | slip ratio (longitudinal, `x`) |
|---|---|---|
| peak slip | `s_py = tan(peak_angle0)` times load factor (about 7 to 9 degrees) | `s_py / cx_ratio` (about 0.07 to 0.13 of theoretical slip) |
| peak friction | `mu_y0` | `mu_x0 = mu_x_ratio * mu_y0` (1.05 to 1.12 times), and `brake_gain` when braking |
| load sensitivity | `exp_y` | `exp_x` (a little closer to 1: longitudinal grip is less load sensitive) |
| sliding level `a` | `fall_y` (about 0.85) | `fall_x` (about 0.82): a locked or spinning tyre holds less than a drifting one |
| fall-off speed `q` | `q_y` (about 1.2) | `q_x` (about 1.4): the lock-up side is sharper |
| camber | thrust and grip term | none |

### 4.7 Self-aligning torque

```
d      = min(fz / k_vert, 0.3 radius)             tyre deflection, k_vert = k_vert_per_width * tyre width
patch  = 2 sqrt(max(2 radius d - d^2, 0))         contact patch length (the chord of a circle pressed in by d)
t0     = trail_ratio * patch                      pneumatic trail of a tyre with no slip
x      = clamp(n / trail_n_end, 0, 1)             progress along the trail curve
trail  = t0 * (1 - (1 + trail_under) * (3 x^2 - 2 x^3)) * clamp(speed, 0, 1)
mz     = -trail * fy
```

The trail leaves `t0` with zero slope, falls along an S-shaped curve (the cubic `3x^2 - 2x^3`) and ends `trail_under * t0` **below
zero** once `n` reaches `trail_n_end`; it stays there for larger `n`. In numbers, with the constants of section 5: `t0` at
`n = 0`, `0.5 t0` at `n = 0.587`, about `0.014 t0` at the peak (`n = 1`), zero at `n = 1.0175`, and `-0.1 t0` for `n >= 1.25`
(the torque changes sign just after the peak: the steering goes light, then pushes the other way). The `clamp(speed, 0, 1)` fades
the torque in below 1 m/s of patch speed. The torque is a property of the tyre only; whether it reaches a steering wheel is the
steering spec's business.

### 4.8 Pseudocode

```
fn tyre_force(p, radius, fz, sx, sy, gamma, slide_speed, speed, surf_lat, surf_drive, grip) -> Out:
    if not (fz > 0):                       return ZERO
    (u_x, u_y, sv, lf_x, lf_y)  = section 4.1
    (sy_eff, cam)               = section 4.2
    (mu_x, mu_y, s_px, s_py)    = section 4.3
    hx = sx / s_px ; hy = sy_eff / s_py
    n  = norm_p(hx, hy)
    if n < 1e-9:                           return ZERO
    f  = curve(n, blended a, q)            section 4.5
    fx =  mu_x * fz * f * hx / n
    fy = -mu_y * fz * f * hy / n
    return Out { fx, fy, mz: aligning(fy, n, fz, radius, speed), n }
```

## 5. Constants

Origin codes: **[T]** textbook, **[U]** a number the reference game's tyre is understood to use (a fact, not an expression),
**[D]** a design value of ours (tunable, with the range the TOML loader accepts), **[N]** from this repository. Per-compound
values of the `[D]` rows are in [parameter-mapping.md](parameter-mapping.md); the column "default" is the `sport` preset.

| Key | Unit | Default | Range | Origin and reason |
|---|---|---|---|---|
| `fz0` | N | 2000 | 500 to 6000 | [U] the reference load at which `mu0` is stated; a constant, not tuned per car |
| `mu_y0` | 1 | 1.20 | 0.6 to 2.2 | [D] peak lateral friction at `fz0`; road tyre 1.0 to 1.2, semi-slick 1.3, slick 1.5 to 1.7 |
| `mu_x_ratio` | 1 | 1.10 | 0.8 to 1.4 | [D] `mu_x0 / mu_y0`; the longitudinal peak is a little higher |
| `exp_y`, `exp_x` | 1 | 0.85, 0.88 | 0.6 to 1.0 | [T] load exponent; 1 = no sensitivity. Real tyres 0.8 to 0.95 |
| `load_factor_max` | 1 | 1.6 | 1.0 to 3.0 | [D] cap of the load factor at very light load |
| `peak_angle_deg` | deg | 8.0 | 4 to 14 | [D] lateral peak slip angle at `fz0` (the axle bias of the mapping splits it front and rear) |
| `flex` | 1 | 0.20 | 0 to 0.6 | [U] shape: the peak slip rises with load; [D] value |
| `cx_ratio` | 1 | 1.4 | 0.8 to 3 | [D] longitudinal over lateral stiffness at the same load |
| `fall_y`, `fall_x` | 1 | 0.85, 0.82 | 0.4 to 1.0 | [D] share of the peak left when sliding; the mapping derives them from the game's dynamic-over-static grip |
| `q_y`, `q_x` | 1 | 1.2, 1.4 | 0.1 to 6 | [D] fall-off speed in units of `n` |
| `comb_exp` | 1 | 2.0 | 1.2 to 3 | [T] norm exponent: 2 is the friction ellipse |
| `brake_gain` | 1 | 1.0 | 0.8 to 1.3 | [D] extra peak when braking (1 = none) |
| `speed_sens` | s/m | 0.0012 | 0 to 0.02 | [U] shape, [D] value: grip falls by `1 / (1 + k v)` with sliding speed |
| `camber_gain` | 1 | 0.10 | 0 to 0.5 | [D] camber thrust as equivalent slip per radian of lean |
| `camber_d1`, `camber_d2` | 1/rad, 1/rad^2 | 1.15, 11.0 | 0 to 4, 0 to 40 | [D] optimum 3 degrees (`d1 / 2 d2`), gain 3 % there |
| `trail_ratio` | 1 | 0.12 | 0.05 to 0.25 | [U] pneumatic trail at zero slip over the patch length |
| `k_vert_per_width` | N/m per m | 1.1e6 | 0.5e6 to 2e6 | [D] vertical stiffness per metre of tyre width (a 0.225 m tyre: 250 kN/m) |
| `trail_n_end` | 1 | 1.25 | fixed | [U] normalised slip at which the trail has finished falling |
| `trail_under` | 1 | 0.10 | fixed | [U] the final trail, as a negative share of `t0` |
| `peak_follow` | 1 | 0.75 | fixed | [U] how much of the grip change moves the peak slip (section 4.3) |
| speed fade | m/s | 1 | fixed | [U] patch speed below which the aligning torque is faded out (4.7) |

`mu_x0 = mu_x_ratio * mu_y0` is derived, never read.

## 6. Test vectors

Computed with a short Python script that implements only sections 4 and 5 (double precision). Common inputs unless
stated: `radius = 0.33`, the `sport` defaults of section 5 with tyre width 0.225 (`k_vert = 247500`), `camber = 0`,
`surf_lat = surf_drive = grip = 1`, `slide_speed = 0`, `speed = 20`. Tolerance: relative `1e-4`, absolute `1e-3` on forces
(f32 implementations are expected to meet it), `1e-3` on `mz` and `n` relative `1e-4`.

`s0 = tan(8 deg) = 0.1405408347`. At 4000 N: `fr = 2`, `flexf = 1.2`, `s_py = 0.1686490016`, `s_px = 0.1204635726`.

| # | Case | Inputs (`fz, sx, sy`, other) | `fx` | `fy` | `mz` | `n` |
|---|---|---|---|---|---|---|
| T1 | zero load | 0, 0.05, 0.05 | 0 | 0 | 0 | 0 |
| T2 | zero slip | 4000, 0, 0 | 0 | 0 | 0 | 0 |
| T3 | lateral, pre-peak | 4000, 0, 0.05; slide 1 | 0 | -2816.267 | 58.172 | 0.296 |
| T3m | mirror of T3 | 4000, 0, -0.05; slide 1 | 0 | +2816.267 | -58.172 | 0.296 |
| T4 | lateral, at the peak | 4000, 0, 0.1686490016 | 0 | -4326.002 | 1.525 | 1.000 |
| T5 | lateral, far past | 4000, 0, 0.6 | 0 | -3836.567 | -9.393 | 3.558 |
| T6a | drive, pre-peak | 4000, 0.05, 0 | 3886.208 | 0 | 0 | 0.415 |
| T6b | drive, past the peak | 4000, 0.15, 0 | 4635.104 | 0 | 0 | 1.245 |
| T6c | brake, pre-peak (mirror of T6a) | 4000, -0.05, 0 | -3886.208 | 0 | 0 | 0.415 |
| T6d | brake, wheel locked | 4000, -5, 0 | -3999.199 | 0 | 0 | 41.506 |
| T6e | `brake_gain = 1.1`: T6a and T6c | 4000, +0.05 and -0.05 | 3886.208 and -4274.829 | 0 | 0 | |
| T7 | combined | 4000, 0.06, 0.08 | 3411.249 | -2892.681 | 26.015 | 0.688 |
| T9 | surface `surf_lat = 0.6`, `surf_drive = 0.8` | as T7 | 2539.467 | -1961.155 | 5.514 | 0.896 |
| T11 | trail faded in at `speed = 0.5` | 4000, 0, 0.02 | 0 | -1363.754 | 16.229 | 0.119 |
| T12 | sliding speed 30 m/s | 4000, 0, 0.3; slide 30 | 0 | -3873.087 | -9.482 | 1.779 |
| T13 | `comb_exp = 3` | as T7 | 3719.075 | -3153.712 | 35.971 | 0.613 |
| T14 | `grip = 0.8` | as T7 | 2795.081 | -2370.181 | 12.412 | 0.809 |

Further vectors:

- **T8 load sensitivity**, lateral at the peak of each load (`sy = s_py(fz)`, `n = 1`): `fz = 2000`: `sy = 0.1405408`, `fy = -2400.000`
  (`mu = 1.20000`); `fz = 4000`: `sy = 0.1686490`, `fy = -4326.002` (`mu = 1.08150`); `fz = 6000`: `sy = 0.1967572`,
  `fy = -6106.106` (`mu = 1.01768`). The ratio `mu(4000) / mu(2000) = 2^-0.15 = 0.90125`.
- **T10 camber**, `fz = 4000`, `sx = 0`: `(gamma, sy)` = `(0, -0.03)`: `fy = +1922.272`, `n = 0.17788`;
  `(+0.05, -0.03)` (leaning into the force): `fy = +2238.145`, `n = 0.20752`; `(-0.05, -0.03)` (leaning away): `fy = +1512.503`,
  `n = 0.14825`; `(+0.05, +0.03)`: `fy = -1512.503`, `n = 0.14825` (the mirror of the previous one).
- **Curve** `f(n)` with `a = 0.85`, `q = 1.2`: `n = 0, 0.5, 1, 2, 5` give `0, 0.875000, 1.000000, 0.918182, 0.875862`.
- **Contact patch**: `fz = 4000`, `radius = 0.33`, `k_vert = 247500`: `d = 0.0161616`, `patch = 0.2040144 m`, `t0 = 0.0244817 m`.
- **Conversions** (section 2): `kappa = 0.1, alpha = 0.1 rad` gives `sx = 0.090909`, `sy = 0.091213`;
  `kappa = -0.2, alpha = 0.05 rad` gives `sx = -0.250000`, `sy = 0.062552`.

Properties every implementation must satisfy (property tests, any parameter set within range):

1. `fz <= 0` or `n < 1e-9` gives exactly zero; `fy` has the sign of `-sy_eff`, `fx` the sign of `sx`.
2. Odd symmetry: with `gamma = 0` and `brake_gain = 1`, `tyre_force(-sx, -sy)` is `(-fx, -fy, -mz, n)` (`mz` flips with `fy`).
3. At `n = 1` and `p = 2`: `(fx / (mu_x fz))^2 + (fy / (mu_y fz))^2 = 1` (within `1e-6`).
4. `|F|` never exceeds `max(mu_x, mu_y) * fz` times `1.001` for any slip, and is continuous in every input.
5. With `exp = 1` and `flex = 0` the lateral peak force is proportional to the load.

## 7. Limits

- No thermal, pressure or wear state: the grip of a cold tyre equals a warm one. `tyre-thermal.md` adds it at stage 5 as
  an extra multiplier on `mu` and, by `peak_follow`, on `s_p` through the `grip` input.
- One curve family per tyre: no separate rise shape per compound, no data-driven lookup curves, no asymmetric lateral curve
  (left and right slip behave alike except through camber).
- Camber is not an input of the host's tyre query: **it is 0 in stage 2**, and from stage 3 the mod passes in the value its own
  suspension code computed on the previous tick. The camber numbers are design values, untested against any reference.
- The aligning torque follows `fy` only: no `mx` (overturning), no longitudinal contribution, no pneumatic trail change with
  camber, no residual torque.
- The friction ellipse is shared in normalised slip, not in force: a tyre far past its peak in one direction keeps a sliding
  force vector that is aligned with the slide direction, which is physically right, but the transition between the two
  directions at large combined slip is a model, not a measurement.
- Surface multipliers scale friction and move the peak slip, nothing else: no surface-specific curve (gravel, snow).
- Known to differ from the reference game: the exact shape of the fall-off tail, the longitudinal behaviour at very large
  wheel spin, and everything thermal. `targets.md` says which scalars were checked against it (none, so far).
