# Targets: the reference scalars and their bands
Sources:
- No measurement of the reference game was available when this was written (no install on the author's machine): **every
  Assetto Corsa value below is `unmeasured`**. Nothing here was read from the reference game, from a telemetry file, from
  rustyAC's numbers or from a third-party measurement.
- Gillespie, *Fundamentals of Vehicle Dynamics*, SAE 1992, and Milliken and Milliken, *Race Car Vehicle Dynamics*, SAE 1995
  (published books): the general ranges of tyre friction, understeer gradient, yaw overshoot and braking deceleration of road
  and competition cars. Understanding only; no figure is quoted from them: the bands are our own design values and
  their rationale is stated per line.
- The planar column is computed by the model of [tyre-force.md](tyre-force.md) and [wheel-dynamics.md](wheel-dynamics.md) on
  the planar reference car of [scenarios.md](scenarios.md); it is a property of this spec.

Written by: spec author session (Claude), 2026-11-20; revised by the spec maintainer session, 2026-11-20 (planar column and design bands re-derived at `inertia_scale` 1.0)   Implemented by: (pending)

Part of [the AC handling specs](README.md). The inputs behind each scalar are in [scenarios.md](scenarios.md).

## 1. Purpose and scope

The scalars the stage-2 mod must hit, with a tolerance band each, and what each number is worth. It holds scalars only (as the
index says): no recordings, no per-sample data, no game files. It is the one place where "feels like Assetto Corsa" becomes a
number, so it states honestly how far each number can be trusted.

**Status words** (the last column of every row):

| Status | Meaning | The matching test |
|---|---|---|
| `unmeasured` | an AC-derived value nobody has measured yet; a maintainer with their own install fills it with [the refresh](#4-refreshing-a-row) | skipped (never guessed) |
| `design` | a value we **choose**, with the reason given; it stands for "a car with a plausible sport-tyre feel" until an AC number replaces it | enforced: the host run must fall inside the band |
| `planar` | what this spec's own planar reference car gives (regression value, not a target) | enforced to `+/- 3 %` by the pure unit test of the model |
| `measured` | taken on a maintainer's own install of the reference game, with the date and the car | enforced (none exists yet) |

Out of scope: the tuning procedure (the mapping spec), the feel of steering and pedals (the owner's drive at gate A).

## 2. Inputs and outputs

Inputs: the scenarios of [scenarios.md](scenarios.md) on `ref_sport`. Outputs: the pass or fail of each scalar, and the
report line `scalar: value (band, status)`.

**Reference car** `ref_sport`, sport-class tyre (`L = 1.0`): the numbers in this section hold for it and only it. For another
car the harness compares the run against the **class** values of section 3 (same geometry, the compound's grip level) with a
host band around the planar class value: grip scalars (g) times `[0.88, 1.08]`, distances and times times `[0.92, 1.12]` (a host
body loses a little to roll and pitch and gains nothing); the response scalars (understeer, overshoot, settling) are only
checked for the ordering of section 6.

### 2.1 Targets for `ref_sport`

All in the units of the scenario, for the mod at its default `inertia_scale` 1.0 ([wheel-dynamics.md](wheel-dynamics.md) contract
point 3: the host's drivetrain is only stable with the host's own wheel inertia or more). "Band" is the interval the host run must
fall in; "planar" is the exact value of the model, tested to `+/- 3 %` (0.05 s on a time, 0.5 m on a distance, if larger).

| Scenario | Scalar | Unit | AC reference | Design value | Band (host, `design`) | Planar | Rationale |
|---|---|---|---|---|---|---|---|
| S1 | `skidpad_g` | g | `unmeasured` | 1.00 | 0.90 to 1.10 | 0.973 | a sport-compound tyre of peak friction 1.2 at 2000 N falls to about 1.10 at the 3.6 kN corner load (load sensitivity `3.6 / 2` to the `-0.15`); load transfer and the combined slip of the driven rear take another 10 %. A 1 g road-car skidpad figure is the ordinary mark of a good road tyre; the band is 10 % either way. Independent of the wheel inertia (0.967 at 0.3, 0.976 at 2.0) |
| S2 | `brake_m` (normalised to 100 km/h) | m | `unmeasured` | 41.0 | 37.0 to 45.0 | 40.91 | **what it means:** with the default wheel inertia the reference brakes, not the tyres, set the stop. Brake force `2 * 2000 / 0.33 + 2 * 800 / 0.34 = 16.8 kN` acts on the car plus the rotating mass of the four wheels (`2 * 10 / 0.33^2 + 2 * 10 / 0.34^2 = 357 kg`, 25 % of the car): `a = 16.83 / 1.807 = 9.31 m/s^2` (0.95 g) and `v^2 / 2a = 41.4 m`, 41.0 m with the aero drag (`ln(1 + k v^2 / a) / 2k` with `k = 0.5 * 1.2 * 0.65 / 1450`). The planar stop (a steady 0.97 to 0.98 g, nothing locks) agrees within 1 %. The design value is that closed-form estimate and the band is +/- 10 %, so the row checks the model's wheel torque balance, brake clamp and lock-up path with the host's inertia; it does not rate the tyre (the S2t vector does). The earlier 36 m (1.09 g, a tyre-limited stop) needed a wheel of 3 kg m^2, which the host's drivetrain cannot run, so it is withdrawn, not widened |
| S2 | `brake_peak_g` | g | `unmeasured` | 1.00 | 0.90 to 1.10 | 1.036 | the largest `-ax` of the same brake-limited stop: the steady 0.95 to 0.98 g plus a transient of up to 0.1 g in the first 0.2 s while the front tyres' slip builds; the band is 10 % either way. The earlier 1.05 to 1.30 belonged to the tyre-limited stop (1.20 at inertia 0.3) |
| S3 | `understeer_deg_per_g` | deg/g | `unmeasured` | 1.5 | 0.8 to 2.5 | 1.134 | a road car is set up to understeer slightly so it is stable at the limit; zero would be a race car, above 3 a car that ploughs. The tyre sets half of it (axle bias 2 deg); the host's springs and bars set the rest |
| S4 | `yaw_overshoot_pct` | % | `unmeasured` | 10 | 3 to 20 | 11.54 | a stable car overshoots its steady yaw rate a little after a step; 0 is over-damped, above 25 % a nervous car |
| S4 | `yaw_settle_s` | s | `unmeasured` | 0.7 | 0.4 to 1.0 | 0.683 | the yaw response of a 1.4 tonne car at 100 km/h settles in well under a second |
| S5 | `breakaway_rung` | throttle | `unmeasured` | 0.7 | 0.5 to 0.9 | 0.75 | a 300 kW rear-drive car steps out under hard throttle in a corner, but not at half throttle. The heavier wheel moves it from 0.70 (inertia 0.3) to 0.75: more rotating mass to spin up before the rear lets go |
| S5 | `slide_recover_s` | s | `unmeasured` | 0.8 | 0.3 to 1.5 | 0.62 | the recovery of the breakaway rung (a slide of just over 5 degrees): about half a second once the throttle is lifted and the wheel centred; a car that needs more than 1.5 s is lively, one that recovers in under 0.3 s never really slid. The band is unchanged; the definition moved (scenarios.md S5: the old figure sat on the cliff before the spin, and read 1.85 s at inertia 1.0 for a slide that recovers as fast as before) |
| S6 | `launch_d3_m` | m | `unmeasured` | 24 | 21 to 28 | 23.09 | 207 W/kg: about 5 s to 100 km/h when the tyre is not the limit; a good rear tyre carries 0.55 g or more. The default wheel inertia holds the planar car 1.1 m back of the light wheel (25.22 m): the rotating mass is 25 % of the car's. Inside the band, in its lower half |
| S6 | `launch_t100_s` | s | `unmeasured` | 5.0 | 4.4 to 5.8 | 5.517 | same reasoning; the same 25 % shows as +0.5 s (4.98 s at inertia 0.3). The planar value is in the upper part of the band: a host run above 5.8 s is a drivetrain finding, not a tyre one |

Properties between rows the harness also checks: `brake_peak_g` is at least `skidpad_g` (1.036 against 0.973 here; with the tyre
limiting, the S2t vector of scenarios.md gives 1.189: a tyre grips more straight than sideways, `mu_x_ratio > 1`), and
`understeer_deg_per_g` is positive.

## 3. Class values and bands (the other compounds)

The planar car with the geometry and brakes of `ref_sport` and the tyre of each compound class at its reference grip level
(`L = level_ref`, `rho = 0.89`, automatic mapping, `sport` car class, `inertia_scale` 1.0). These are regression values of the
model (`planar`) and, with the host band of section 2, the design targets for a car on that compound. `brake_m` and
`brake_peak_g` are the stock brakes (S2); the S2t columns are the same stop with every brake torque multiplied by 1.3, where the
tyre is the limit.

| Compound (`L`) | `mu_y0` | `skidpad_g` | `brake_m` | `brake_peak_g` | S2t `brake_m` | S2t `brake_peak_g` |
|---|---|---|---|---|---|---|
| economy (0.60) | 1.000 | 0.843 | 51.84 | 0.880 | 49.39 | 0.895 |
| road (0.80) | 1.100 | 0.911 | 42.17 | 1.012 | 42.37 | 1.019 |
| **sport (1.00)** | 1.200 | **0.973** | **40.91** | **1.036** | **37.69** | **1.190** |
| semislick (1.25) | 1.350 | 1.066 | 40.55 | 1.112 | 31.50 | 1.294 |
| slick (1.55) | 1.600 | 1.236 | 40.17 | 1.202 | 31.15 | 1.406 |

| Compound (`L`) | `understeer_deg_per_g` | `yaw_overshoot_pct` | `yaw_settle_s` | `launch_d3_m` |
|---|---|---|---|---|
| economy (0.60) | 1.49 | 16.2 | 0.95 | 18.04 |
| road (0.80) | 1.26 | 13.5 | 0.80 | 20.51 |
| **sport (1.00)** | **1.13** | **11.5** | **0.68** | **23.04** |
| semislick (1.25) | 0.98 | 9.1 | 0.55 | 26.66 |
| slick (1.55) | 0.75 | 6.1 | 0.38 | 32.47 |

Reading it: grip scales the cornering and launch scalars almost with `mu_y0` (the load sensitivity takes about 10 %). The stock
brakes stop improving beyond the road tyre because they, not the tyre, set the stop (the sport, semislick and slick rows differ by
the early transient only); the economy and road rows are limited by the tyre (the economy front wheels reach their peak at 0.88 g and
run past it), and the S2t columns show the tyre ordering the stock brakes hide. A stickier tyre is sharper (lower overshoot,
shorter settling) and slightly less understeery. The economy and road compounds do not reach 100 km/h inside the 6 s window of
S6 (`launch_t100_s` is not defined for them; `T_LEAD(27.78)` is 6.90 s and 6.07 s). The `launch_d3_m` of the sport row (23.04)
differs from section 2 (23.09) only because the class rows use `rho = 0.89` where `ref_sport` has 0.8916.

## 4. Refreshing a row

A maintainer with their own Assetto Corsa install runs the harness in `tools/` (a Python or Rust runner that replays the scripts
of [scenarios.md](scenarios.md) in AC with a car chosen to match the reference car's class, a road-tyre or sport-tyre
compound and a flat surface) and gets a scalar per scenario. What is committed is the **scalar and its bounds**:

```
| S1 | `skidpad_g` | g | measured 2026-MM-DD, <AC car>, <tyre>: <value> +/- <spread> | ... |
```

changing the `AC reference` column from `unmeasured` to `measured` with the date, the AC car and tyre name (a fact, not a
file) and the value with the repeat-to-repeat spread. The `Design value` is then **replaced** by that value and the band set to
the larger of the measured spread times 3 and 5 % of the value. No recording, no per-sample data, no file from the game is
committed; the leak check ([licensing.md](../../licensing.md) rule 1) would reject one. Nobody else needs an AC install, ever.
A row whose measured value falls outside its old design band is a finding: the design value is revised **in the same commit**,
with a line in `docs/provenance/ac-handling.md`.

## 5. Constants

| Name | Value | Use |
|---|---|---|
| planar tolerance | 3 %, or 0.05 s on times and 0.5 m on distances if larger | regression of the model |
| host band for other classes | g scalars x [0.88, 1.08]; distances and times x [0.92, 1.12] | section 2 |
| repeat | the scenarios are deterministic at a fixed tick; one run per scalar | |
| second tick | 1/144 s | S1 and S2 within 2 % of the 60 Hz value (planar: 0.973 and 0.973; 40.91 and 41.11 m) |
| default `inertia_scale` | 1.0 | every planar number here; the sweep is in scenarios.md section 6 |

## 6. Test vectors

The planar column above is the test vector set of this file (reproduced by the pure scenario runner of the appendix of
[scenarios.md](scenarios.md), convention E). Expected outcomes of the checks:

1. `ref_sport`, planar, 60 Hz: every scalar of 2.1 within 3 % of its planar value and inside its design band (all 10 are).
2. The class tables of section 3: each row within 3 % (0.05 s, 0.5 m).
3. `skidpad_g` increases and `understeer_deg_per_g` decreases from `economy` to `slick` (the planar rows are monotone in both), and
   so do the S2t `brake_m` (falling) and `brake_peak_g` (rising).
4. 1/144 s: `skidpad_g` 0.973 (+0.0000 against 60 Hz), `brake_m` 41.11 (+0.2 m).
5. The economy and road compounds do not reach 100 km/h within the 6 s of S6; the other three do (`launch_t100_s` 5.517, 4.95, 4.35).
6. Brake-limited stop: the raw stop of S2, normalised, is within 2 % of the closed form with drag of 2.1 (41.0 m; planar 40.91), so a
   wrong wheel inertia or a wrong brake clamp is seen even when a band still passes.

## 7. Limits

- **No AC number is checked.** The design values express a plausible, stable, sporty feel. They can be wrong about the real
  game by more than their bands: the first measured row may overturn them. Stage 2 is "inside the bands", not "equal to AC".
- The planar car has no suspension, so the host's numbers will differ; the bands were set wide enough (about 10 % for grip
  scalars) to absorb ordinary body effects and no wider.
- **The default wheel inertia costs the S2 and S6 numbers**, because the host's drivetrain is only stable at 0.5 or more
  (wheel-dynamics.md contract point 3): a bare-wheel car would stop in 35 m and launch 0.5 s faster, and the two rows no longer
  measure the tyre (S2t does for braking). If the host later integrates the wheel with its driveline, the key can go back
  toward 0.3 and these two rows take their 0.3 values from the sweep in scenarios.md; the design bands would be re-derived then.
- The rationale column is reasoning, not citation: a reader who disagrees with a design value should say so in the review,
  because every number here is cheap to change and nothing downstream depends on its exact value.
- Only the dry, flat, uniform surface is covered; no wet, no temperature (cold tyres give the same numbers as warm ones until
  the thermal spec exists).
- Scalar 'feel' items (steering weight and noise, tyre sound, progression at the limit) are not in this file.

## Changes (revision of 2026-11-20, for the implementer's tests)

Planar values and design bands of 2.1 (the Rust `REF_SPORT` table), before and after; "-" is unchanged:

| Scalar | Planar before | Planar now | Design / band before | Design / band now |
|---|---|---|---|---|
| `skidpad_g` | 0.967 | 0.973 | 1.00, 0.90 to 1.10 | - |
| `brake_m` | 34.85 | 40.91 | 36.0, 32.0 to 40.0 | **41.0, 37.0 to 45.0** |
| `brake_peak_g` | 1.201 | 1.036 | 1.15, 1.05 to 1.30 | **1.00, 0.90 to 1.10** |
| `understeer_deg_per_g` | 1.119 | 1.134 | 1.5, 0.8 to 2.5 | - |
| `yaw_overshoot_pct` | 12.84 | 11.54 | 10, 3 to 20 | - |
| `yaw_settle_s` | 0.65 | 0.683 | 0.7, 0.4 to 1.0 | - |
| `breakaway_rung` | 0.7 | 0.75 | 0.7, 0.5 to 0.9 | - |
| `slide_recover_s` | 0.83 | 0.62 | 0.8, 0.3 to 1.5 | - (new definition) |
| `launch_d3_m` | 25.22 | 23.09 | 24, 21 to 28 | - |
| `launch_t100_s` | 4.97 | 5.517 | 5.0, 4.4 to 5.8 | - |

Class tables (section 3): all rows recomputed (old sport row 0.967, 34.85, 1.201, 1.12, 12.8, 0.65, 25.14); new columns S2t
`brake_m`, `brake_peak_g`; the economy-only "does not reach 100 km/h" is now economy and road. Section 5: the 1/144 s figures are
0.973 and 40.91 / 41.11 m (were 0.967 and 34.85 / 35.05). Section 6: checks 5 and 6 are new.
