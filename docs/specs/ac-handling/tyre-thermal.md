# Tyre thermal state, pressure, wear and the grip multiplier
Sources: Kelly and Sharp, "Time-optimal control of the race car: influence of a thermodynamic tyre model", Vehicle System Dynamics (2012), public literature, understanding (a lumped surface and core model of tyre temperature)
Sources: Milliken and Milliken, "Race Car Vehicle Dynamics" (SAE, 1995), tyre chapter, public literature, understanding (an optimum temperature window, pressure sensitivity, wear)
Sources: the ideal gas law (public knowledge), facts (tyre pressure rises with absolute gas temperature)
Sources: rustyAC, https://github.com/thesupersupersigma/rustyAC, `crates/rustyac-physics/src/tyre/` (`thermal.rs`, `data.rs`, `vanilla_tyre.rs`) and `docs/port/`, GPL-3.0-or-later (per its LICENSING.md; the repo also carries MIT/Apache files for unrelated parts), understanding of behaviour and facts (grip factor is a temperature curve times 1/(1 + gain*|pressure - ideal|) times a wear curve over a "virtual km" of sliding distance; the temperature that matters is the core plus a quarter of the way to the contact-patch temperature; heat comes from sliding speed times grip times load, and from rolling into the core; pressure rises by a fixed 0.16 psi per degree C of core temperature above 26 C; three stripes across the tread plus a core; typical optimum 80 C, ideal pressure and coefficients are per compound); no code, comments or structure taken
Sources: this repository, [ride6-tyre-thermal.md](../ride6-tyre-thermal.md) (our own clean-room spec of another game's tyre temperature and wear, for the shape of such a spec), MIT OR Apache-2.0, understanding
Written by: spec author session (Claude, stage 0 part 2), 2026-11-20   Implemented by: not yet

Part of [the AC handling specs](README.md), [ADR 0016](../../decisions/0016-handling-hooks.md), stage 5. The multiplier defined here is
consumed by `tyre-force.md`; the other quantities are the mod's own state. Axes: x right, y up, z forward; wheel order FL FR RL RR.
Temperatures in degrees C, pressures in kPa gauge, forces in N, speeds in m/s.

## 1. Purpose and scope

Give each tyre a temperature (three surface stripes across the tread and a core), an inflation pressure that follows the
core temperature, and a wear state; combine the three into one factor that scales the peak friction of the tyre force model, and
give the HUD four numbers.

| In scope | Out of scope |
|---|---|
| surface temperature of three stripes (inner, centre, outer), core temperature, per tyre | brake disc temperature, wheel bearing and rim heat |
| heat from sliding and from rolling; cooling by air, road and the rim; conduction between parts | graining, blistering, flat spots, punctures, dirt, rain and wet surfaces |
| pressure from the core temperature (gas law), pressure effect on grip and rolling heat | pressure changes by leaks or by the player in the race (a setup value at session start only) |
| wear as a function of sliding distance and tyre heat; grip lost with wear | wear affecting cornering stiffness, relaxation length or tyre mass |
| the grip multiplier `grip_scale(temp, pressure, wear)`; a HUD value | a pit-stop tyre change (reseed does it), tyre blankets as a separate rule (use `warm start`) |

## 2. Inputs and outputs

### 2.1 The interface to the tyre force model

```
grip_scale(temp, pressure, wear) -> factor        # pure function, no state; section 4.6
```

`factor` is a dimensionless number in `(0, 1]` that enters the tyre force model through its **`grip` input**: the mod passes
`grip = grip_gain * tyre_condition * factor` ([wheel-dynamics.md](wheel-dynamics.md) section 4), so the factor scales the peak
friction coefficient (`mu`) of the slip curve and, by the fixed coupling `peak_follow = 0.75` of [tyre-force.md](tyre-force.md)
section 4.3, moves the peak slip a little (a factor of 0.8 puts the peak at 0.85 of its slip). `temp` is the effective grip
temperature `T_eff` (section 4.5), `pressure` the gauge pressure in kPa, `wear` the wear in km of virtual sliding distance. The tyre model evaluates it once per tick **at the start of the tick**, from the thermal
state left by the previous tick, so grip lags the heating by one tick. Nothing else of the thermal model reaches the force model,
except the optional `pressure_rr_scale(pressure)` of section 4.4 (a multiplier of the rolling resistance coefficient, so that the
drag the tyre model applies and the heat this model adds use the same law).

### 2.2 The per-tick update

Per wheel, from the tyre hook, after its substeps (values are means over the tick):

| Input | Unit | Meaning |
|---|---|---|
| `dt` | s | the tick (`dt` of the query) |
| `p_slide` | W | mean of `abs(Fx * vsx) + abs(Fy * vsy)` over the tick's substeps (`vsx`, `vsy` are the slip velocities of the contact patch relative to the road). A mean of the product, **not** the product of the means |
| `v_slide` | m/s | mean of `sqrt(vsx^2 + vsy^2)` over the tick |
| `Fz` | N | mean vertical load (the mean of `vertical_load_n`, or `tyre_load` from the suspension hook once it exists) |
| `v` | m/s | body speed, `abs(body_speed)` |
| `gamma` | rad | camber, outward positive (from the suspension stage) |
| `T_amb`, `T_road` | C | `ambient_temperature_c` and `road_temperature_c` of the query; the host has no weather or thermal model and reports 20 and 25, which are therefore the values of a session |
| `loaded` | bool | the tyre is on the ground |

| Output | Unit | Meaning |
|---|---|---|
| (state) | | updated temperatures, pressure and wear |
| `hud_surface` | C | contact-patch mean `T_cp` |
| `hud_core` | C | core temperature `T_c` |
| `hud_pressure` | kPa | pressure `P` |
| `hud_life` | % | remaining tyre life, 100 new, 0 worn out |

The host's tyre result has **no temperature field** (ADR 0016 listed an optional one; the interface page has none): the four HUD
values are the mod's own and are shown through `bb.hud` (plan, stage 5).

## 3. State

Per tyre: `T_s[0..2]` (surface temperatures, 0 = inner stripe, towards the car centre line, 1 = centre, 2 = outer), `T_c` (core and
gas), `w` (km), and the fixed `P_set` (kPa) and `T_set` (C) the tyre was inflated at.

**Initial state (reseed).** At session start, on a preset change, a toggle of the mod, a tyre change and on teleport to the pits:
`T_s[i] = T_c = T_start`, `w = 0`, `T_set = T_amb`, `P_set` = the preset's cold pressure. `T_start = T_amb` (cold) or `T_warm`
(the "warm start" option, as if the tyres were in blankets). Pressure is then `P = P(T_c)` (section 4.4), so a warm start begins with
a pressure above `P_set`.

## 4. Equations

### 4.1 Where the heat goes

```
shares(gamma, P):                                  # fraction of the heat each stripe takes, sums to 1
    a   = clamp(-k_cam * gamma, -1/3, +1/3)        # negative camber loads the inner stripe
    w0  = 1/3 + a ; w1 = 1/3 ; w2 = 1/3 - a
    eta = clamp(k_pr * (P / P_ideal - 1), -0.3, 0.3)     # over-inflated: the centre takes more
    w0 *= (1 - eta) ; w2 *= (1 - eta)
    w1  = 1 - w0 - w2
    return (w0, w1, w2)
```

### 4.2 Heat sources

```
if loaded:
    Q_f = eta_f * p_slide                                        # sliding friction, W; eta_f of it enters the tyre
    Q_r = max(0, c_rr * pressure_rr_scale(P) * Fz * v)           # rolling hysteresis, W
else:
    Q_f = 0 ; Q_r = 0
Q_i = shares_i * (Q_f + rho_s * Q_r)         # to surface stripe i
Q_c = (1 - rho_s) * Q_r                      # to the core
```

### 4.3 Conduction and cooling

```
g_air = g_a0 + g_a1 * v                 # stripe to air, W/K per stripe
g_rd  = loaded ? G_rd0 * sqrt(Fz / Fz_ref) : 0     # stripe to road
g_ca  = g_ca0 + g_ca1 * v               # core to air through the rim and sidewall
q_0 = G_ss*(T_s1 - T_s0)
q_1 = G_ss*(T_s0 - T_s1) + G_ss*(T_s2 - T_s1)
q_2 = G_ss*(T_s1 - T_s2)
dTs_i = ( Q_i + q_i + G_sc*(T_c - T_si) + g_air*(T_amb - T_si) + g_rd*(T_road - T_si) ) / C_s
dTc   = ( Q_c + G_sc * sum_i (T_si - T_c) + g_ca*(T_amb - T_c) ) / C_c
```

Explicit Euler, every derivative taken from the state at the start of the tick:
`T_si += dTs_i * dt` and `T_c += dTc * dt`. Stability needs `dt * (G_total / C) < 0.5` (the fastest rate is about 0.065 per second
at 60 m/s); if `dt > 0.5 s`, split the update into steps of at most 0.5 s. Keep the thermal state in `f64`.

### 4.3b Wear

With `T_cp` the contact-patch mean before the update (section 4.5):

```
m_hot = 1 + k_wt * max(T_cp - T_hot, 0)           # hot rubber wears faster
if loaded: w += (v_slide + eps_roll * v) * dt * 1e-3 * m_hot       # km
```

Wear counts sliding distance plus a small share of the rolled distance.

### 4.4 Pressure

```
P(T_c) = (P_set + P_atm) * (T_c + 273.15) / (T_set + 273.15) - P_atm        # P_atm = 101.325 kPa
pressure_rr_scale(P) = 1 + k_rrp * (P_ideal / max(P, 50) - 1)               # rolling resistance and rolling heat
```

The tyre was inflated at `T_set` to `P_set`; the gas heats with the core. At 190 kPa cold this is 0.94 kPa per degree C (the
reference uses a fixed 0.16 psi = 1.10 kPa per degree C above 26 C). Pressure is not changed by the player during a session.

### 4.5 Which temperature the grip sees

```
T_cp  = shares(gamma, P) . T_s                  # contact-patch mean, weighted like the heat
T_eff = T_c + w_cp * (T_cp - T_c)               # the core with a quarter of the way to the patch
```

The slow core dominates: a tyre whose surface is hot but whose core is cold is not at its best.

### 4.6 The multiplier

```
g_T(T):  d = T - T_opt ; s = (d < 0) ? s_c : s_h
         e = max(|d| - h, 0) / s
         return g_min + (1 - g_min) * exp(-0.5 * e * e)            # 1 inside the plateau, falls to g_min
g_P(P):  return 1 / (1 + k_gp * |P - P_ideal|)                     # in (0, 1], 1 at the ideal pressure
g_W(w):  piecewise linear through the knots (km, factor) of the table in section 5; flat outside the first and last knot
grip_scale(T, P, w) = g_T(T) * g_P(P) * g_W(w)
```

### 4.7 The HUD value

`hud_surface = T_cp`, `hud_core = T_c`, `hud_pressure = P`, and `hud_life = 100 * (g_W(w) - g_W_end) / (1 - g_W_end)` with
`g_W_end` the last knot's factor.

### 4.8 Order of one tick

```
at tick start:   factor = grip_scale(T_eff, P, w)          # given to the tyre force model
tyre hook runs its substeps, collects p_slide, v_slide, Fz
at tick end:     wear (4.3b) using T_cp(before), then temperatures (4.3), then P (4.4)
```

## 5. Constants

Defaults for a **road compound**; each compound class of `parameter-mapping.md` overrides the group marked "compound". All are
design values (chosen so that a car cornering hard warms its tyres to the plateau in about 3 to 6 minutes, reaches a steady state
near 85 C and cools with a time constant of about 20 minutes at rest), except where marked *reference*.

| Symbol | Default | Unit | Group | Reason |
|---|---|---|---|---|
| `C_s` stripe heat capacity | 1800 | J/K | tyre | design: a thin tread layer |
| `C_c` core heat capacity | 10000 | J/K | tyre | design: carcass, gas and rim share |
| `G_ss` stripe to stripe | 20 | W/K | tyre | design |
| `G_sc` stripe to core | 30 | W/K | tyre | design |
| `g_a0`, `g_a1` stripe to air | 3.0, 0.6 | W/K, W s/(K m) | tyre | design: forced convection grows with speed |
| `G_rd0`, `Fz_ref` stripe to road | 5.0, 3500 | W/K, N | tyre | design |
| `g_ca0`, `g_ca1` core to air | 3.0, 0.15 | W/K, W s/(K m) | tyre | design |
| `eta_f` | 0.5 | 1 | tyre | design: the rest of the friction heat goes into the road |
| `c_rr` | 0.012 | 1 | tyre | design: typical road tyre rolling resistance |
| `k_rrp` | 0.5 | 1 | compound | *reference* form `(ideal/pressure - 1)*gain + 1`, gain 0.5 |
| `rho_s` | 0.4 | 1 | tyre | design: 40 % of rolling heat into the tread, 60 % into the carcass |
| `k_cam` | 3.0 | 1/rad | tyre | design: 3 deg of camber moves 16 % of the heat |
| `k_pr` | 1.5 | 1 | tyre | design |
| `w_cp` | 0.25 | 1 | tyre | *reference*: quarter of the way from core to patch |
| `T_opt` | 75 | C | compound | design (the reference default optimum is 80 C) |
| `h` plateau half width | 12 | C | compound | design |
| `s_c`, `s_h` | 45, 45 | C | compound | design: width of the fall below and above the plateau |
| `g_min` | 0.55 | 1 | compound | design: floor of the temperature factor |
| `P_set` | 190 | kPa | compound | design: 27.6 psi cold |
| `P_ideal` | 230 | kPa | compound | design: the hot pressure the tyre likes, 33.4 psi |
| `k_gp` | 0.0012 | 1/kPa | compound | design: 5 psi off ideal costs 4 % |
| `eps_roll` | 0.01 | 1 | compound | design: 1 % of the rolled distance wears like sliding |
| `T_hot`, `k_wt` | 100, 0.02 | C, 1/K | compound | design: 1 K above 100 C adds 2 % wear rate |
| wear knots (km, factor) | (0, 1.00), (0.5, 0.985), (2, 0.95), (4, 0.88), (6, 0.80), (8, 0.75), (10, 0.72) | | compound | design: 10 km of sliding is end of life, 28 % grip lost |
| `T_warm` | 70 | C | tyre | design: warm start |
| `T_amb`, `T_road` defaults | 20, 25 | C | | the values the host reports today (a fallback if it sends none) |
| `P_atm` | 101.325 | kPa | physics | |

## 6. Test vectors

Generated by a script that implements only sections 3 and 4 (float64). Tolerances: **1e-4** for the pure functions, **0.1 K** for
temperatures, **0.2 kPa** for pressures, **0.002** for factors, **0.5 %** for wear after a long run.

**T1. `shares(gamma, P)`:**

| `gamma` | `P` | `(w0, w1, w2)` |
|---|---|---|
| 0 | 230 | 0.33333, 0.33333, 0.33333 |
| -0.035 | 230 | 0.43833, 0.33333, 0.22833 |
| -0.035 | 260 | 0.35257, 0.46377, 0.18366 |
| 0.05 | 230 | 0.18333, 0.33333, 0.48333 |
| -0.20 | 230 | 0.66667, 0.33333, 0.00000 (camber clamp) |
| -0.035 | 150 | 0.56983, 0.13333, 0.29683 (under-inflated: edges take more) |
| 0 | 400 | 0.23333, 0.53333, 0.23333 (`eta` clamp 0.3) |

**T2. `P(T_c)`** with `P_set = 190`, `T_set = 20`: `T_c = 20, 50, 80, -10` give 190.0000, 219.8132, 249.6265, 160.1868.

**T3. `g_T`** (defaults): `T = -10, 20, 40, 63, 75, 87, 100, 130, 200` give 0.67072, 0.83506, 0.94490, 1.0, 1.0, 1.0, 0.98161, 0.83506, 0.56923.
**T4. `g_P`:** `P = 230, 200, 190, 260, 150, 400` give 1.0, 0.96525, 0.95420, 0.96525, 0.91241, 0.83056.
**T5. `g_W`:** `w = 0, 0.25, 1, 2, 3, 6, 9, 10, 15` give 1.0, 0.9925, 0.97333, 0.95, 0.915, 0.8, 0.735, 0.72, 0.72.
**T6. `grip_scale(T, P, w)`:** `(20, 190, 0)` 0.79681; `(75, 230, 0)` 1.0; `(75, 230, 10)` 0.72; `(110, 260, 4)` 0.80262; `(40, 150, 2)` 0.81903.
`hud_life`: `w = 0` gives 100, `w = 6` gives 28.6, `w = 10` and above give 0.

**T7. One tick from cold** (`T_amb = 20`, `P_set = 190`, all temperatures 20, `T_road = 30`, `dt = 1/60`, `Fz = 3500`, `v = 25`, `loaded`):

| Case | `gamma` | `p_slide` | `v_slide` | `T_s` after | `T_c` after | `w` after |
|---|---|---|---|---|---|---|
| cornering (`Fy = 3500`, `vsy = 2.17889`) | -0.035 | 7626.12 | 2.17889 | 20.022352, 20.006777, 20.011865 | 20.001161 | 4.048e-5 |
| traction (`Fx = 3000`, `vsx = 1.25`) | 0 | 3750.0 | 1.25 | 20.009566, 20.003916, 20.009566 | 20.001161 | 2.500e-5 |

**T8. One tick, airborne** (`loaded = false`, `T_s = 60` each, `T_c = 50`, `v = 25`, `T_amb = 20`): `T_s = 59.990556` each, `T_c = 50.001162`, `w` unchanged.
**T9. One tick, hot** (`T_s = 110` each, `T_c = 100`, `Fz = 3500`, `v = 25`, `gamma = 0`, `p_slide = 7000`, `v_slide = 2.0`, `P_set = 190`, `T_set = 20`):
`T_s = 109.98743, 109.996707, 109.98743`, `T_c = 100.001573`, `w = 4.5e-5` (`m_hot = 1.2`).

**T10. Warm-up (sustained circle).** Cold start (`T_amb = 20`, `P_set = 190`, `T_road = 30`), every tick `dt = 1/60`, `gamma = -0.035`, `Fz = 3500`,
`v = 25`, `p_slide = 7626.12`, `v_slide = 2.17889`, loaded:

| `t` (s) | `T_s` (inner, centre, outer) | `T_c` | `P` (kPa) | `T_cp` | `T_eff` | `grip_scale` | `w` (km) |
|---|---|---|---|---|---|---|---|
| 10 | 31.267, 24.139, 26.106 | 20.989 | 190.98 | 28.622 | 22.897 | 0.8138 | 0.02429 |
| 60 | 55.284, 40.548, 41.621 | 30.038 | 199.98 | 48.565 | 34.670 | 0.8833 | 0.14573 |
| 180 | 69.648, 60.240, 56.422 | 51.171 | 220.98 | 63.683 | 54.299 | 0.9682 | 0.43720 |
| 360 | 79.104, 75.244, 67.517 | 69.407 | 239.10 | 75.176 | 70.849 | 0.9657 | 0.87440 |
| 600 | 84.664, 84.055, 74.040 | 80.151 | 249.78 | 82.295 | 80.687 | 0.9404 | 1.45734 |
| 900 | 87.105, 87.916, 76.901 | 84.862 | 254.46 | 85.503 | 85.023 | 0.9166 | 2.18600 |

(The grip rises while the tyre warms, peaks at the plateau about 3 to 6 minutes in, then falls slowly as the pressure climbs above the ideal.)

**T11. Cool-down.** From the state at `t = 900` of T10, the car stands (`v = 0`, `p_slide = 0`, `Fz = 3500`, loaded), `T_amb = 20`, `T_road = 25`:

| `t` (s) | `T_s` | `T_c` | `P` (kPa) | `T_eff` |
|---|---|---|---|---|
| 10 | 84.196, 83.929, 76.803 | 84.496 | 254.09 | 84.036 |
| 60 | 74.784, 74.272, 73.309 | 81.091 | 250.71 | 79.387 |
| 180 | 64.311, 64.296, 64.280 | 71.216 | 240.90 | 69.487 |
| 360 | 53.836 each | 59.029 | 228.79 | 57.731 |
| 600 | 43.892 each | 47.393 | 217.22 | 46.517 |
| 1800 | 25.850 each | 26.281 | 196.24 | 26.173 |

`w` stays 2.186 throughout.

**T12. Wear to end of life** (temperatures held at 60 C so `m_hot = 1`; `v = 25`, `v_slide = 3`, `Fz = 3500`, loaded, `dt = 1/60`): the
increment per tick is `(3 + 0.01*25)/60 * 1e-3 = 5.4167e-5` km; `w` first reaches 10 km after 184 616 ticks (3076.9 s), where
`g_W = 0.72`; it stays 0.72 beyond.

Properties: all temperatures stay between `min(T_amb, T_road)` and the largest heat-balance temperature (no overshoot with
`dt <= 0.5 s`); `T_c` never reacts faster than the stripes; with no input and `T_amb = T_road` every temperature converges to
`T_amb`; `P` is monotonic in `T_c`; `grip_scale` is in `(0, 1]`; `shares` sums to 1.

## 7. Limits

- The temperature of the tread is three stripes without a circumferential position: heat is not stored in the part of the tyre that
  is not on the ground. A lock-up or a long burnout heats the tread continuously instead of locally, so it overstates the effect
  of a short event and ignores flat spots.
- All constants are design values. The warm-up and cool-down times, the plateau and the wear life were not fitted to any measured
  tyre. The compound presets, not this spec, carry the feel.
- Pressure follows the gas law of the core; there is no leak, no pit-stop adjustment during a session and no effect on stiffness
  or contact patch length (only on grip, rolling heat and heat distribution).
- Wear affects grip only. A tyre at end of life keeps 72 % of its grip; it never bursts and never flat-spots.
- The camber term of the heat distribution assumes the camber is that of the suspension stage; on a banked road it is the flat
  road camber.
- `T_road` and `T_amb` are constants of the session unless the host varies them; there is no sun, wind or rain.
- The multiplier enters through the `grip` input, so it scales the peak friction and moves the peak slip by `peak_follow` (a cold
  tyre at 0.8 peaks at 0.85 of its slip, so its cornering stiffness falls by about 6 %); the relaxation length is unchanged, so a
  cold tyre keeps most of its steering response and mainly loses its limit.
- A tyre in the air cools through `g_air` and `g_ca` only (no road contact); a car on a jack cools like a car standing.
