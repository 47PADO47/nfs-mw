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

Written by: spec author session (Claude), 2026-11-20   Implemented by: (pending)

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

All in the units of the scenario. "Band" is the interval the host run must fall in; "planar" is the exact value of the
model, tested to `+/- 3 %`.

| Scenario | Scalar | Unit | AC reference | Design value | Band (host, `design`) | Planar | Rationale |
|---|---|---|---|---|---|---|---|
| S1 | `skidpad_g` | g | `unmeasured` | 1.00 | 0.90 to 1.10 | 0.967 | a sport-compound tyre of peak friction 1.2 at 2000 N falls to about 1.10 at the 3.6 kN corner load (load sensitivity `3.6 / 2` to the `-0.15`); load transfer and the combined slip of the driven rear take another 10 %. A 1 g road-car skidpad figure is the ordinary mark of a good road tyre; the band is 10 % either way |
| S2 | `brake_m` (normalised to 100 km/h) | m | `unmeasured` | 36.0 | 32.0 to 40.0 | 34.85 | mean deceleration `v^2 / 2d`: 36 m is 1.09 g, a sport-tyre road car on dry asphalt without ABS; the reference brakes (2000 / 800 N m per wheel, 71 % front) cap the figure near 34 m even with a stickier tyre |
| S2 | `brake_peak_g` | g | `unmeasured` | 1.15 | 1.05 to 1.30 | 1.201 | the peak of the longitudinal curve (1.32 at 2000 N times the load factor `0.93` at 3.6 kN) before the front tyres lock |
| S3 | `understeer_deg_per_g` | deg/g | `unmeasured` | 1.5 | 0.8 to 2.5 | 1.119 | a road car is set up to understeer slightly so it is stable at the limit; zero would be a race car, above 3 a car that ploughs. The tyre sets half of it (axle bias 2 deg); the host's springs and bars set the rest |
| S4 | `yaw_overshoot_pct` | % | `unmeasured` | 10 | 3 to 20 | 12.84 | a stable car overshoots its steady yaw rate a little after a step; 0 is over-damped, above 25 % a nervous car |
| S4 | `yaw_settle_s` | s | `unmeasured` | 0.7 | 0.4 to 1.0 | 0.65 | the yaw response of a 1.4 tonne car at 100 km/h settles in well under a second |
| S5 | `breakaway_rung` | throttle | `unmeasured` | 0.7 | 0.5 to 0.9 | 0.7 | a 300 kW rear-drive car steps out under hard throttle in a corner, but not at half throttle |
| S5 | `slide_recover_s` | s | `unmeasured` | 0.8 | 0.3 to 1.5 | 0.83 | a modest slide dies in about a second once the throttle is lifted and the wheel centred; a car that needs more than 1.5 s is lively, one that recovers in under 0.3 s never really slid |
| S6 | `launch_d3_m` | m | `unmeasured` | 24 | 21 to 28 | 25.22 | 207 W/kg: about 5 s to 100 km/h when the tyre is not the limit; a good rear tyre carries 0.55 g or more |
| S6 | `launch_t100_s` | s | `unmeasured` | 5.0 | 4.4 to 5.8 | 4.97 | same reasoning; mostly the engine, a check that the tyre does not hold the car back |

Properties between rows the harness also checks: `brake_peak_g` is at least `skidpad_g` (a tyre grips more straight than
sideways: `mu_x_ratio > 1`), and `understeer_deg_per_g` is positive.

## 3. Class values and bands (the other compounds)

The planar car with the geometry and brakes of `ref_sport` and the tyre of each compound class at its reference grip level
(`L = level_ref`, `rho = 0.89`, automatic mapping, `sport` car class). These are regression values of the model (`planar`)
and, with the host band of section 2, the design targets for a car on that compound.

| Compound (`L`) | `mu_y0` | `skidpad_g` | `brake_m` | `brake_peak_g` | `understeer_deg_per_g` | `yaw_overshoot_pct` | `yaw_settle_s` | `launch_d3_m` |
|---|---|---|---|---|---|---|---|---|
| economy (0.60) | 1.000 | 0.832 | 46.27 | 0.938 | 1.45 | 17.8 | 0.90 | 19.57 |
| road (0.80) | 1.100 | 0.903 | 41.60 | 1.087 | 1.30 | 14.9 | 0.75 | 22.23 |
| **sport (1.00)** | 1.200 | **0.967** | **34.85** | **1.201** | **1.12** | **12.8** | **0.65** | **25.14** |
| semislick (1.25) | 1.350 | 1.061 | 34.44 | 1.305 | 0.97 | 10.2 | 0.52 | 29.34 |
| slick (1.55) | 1.600 | 1.238 | 33.81 | 1.403 | 0.78 | 7.0 | 0.37 | 35.58 |

Reading it: grip scales the cornering and launch scalars almost with `mu_y0` (the load sensitivity takes about 10 %); the braking
distance stops improving beyond the sport tyre because the reference brakes, not the tyre, set it (the economy figure is long
because the front wheels lock without ABS: `brake_peak_g` 0.94 is the instant before the lock); a stickier tyre is sharper
(lower overshoot, shorter settling) and slightly less understeery. The economy compound does not reach 100 km/h inside the
6 s window of S6 (`launch_t100_s` is not defined for it). The `launch_d3_m` of the sport row (25.14) differs from section 2
(25.22) only because the class rows use `rho = 0.89` where `ref_sport` has 0.8916.

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
| second tick | 1/144 s | S1 and S2 within 2 % of the 60 Hz value (planar: 0.967 and 0.967; 34.85 and 35.05 m) |

## 6. Test vectors

The planar column above is the test vector set of this file (reproduced by the pure scenario runner of the appendix of
[scenarios.md](scenarios.md)). Expected outcomes of the checks:

1. `ref_sport`, planar, 60 Hz: every scalar of 2.1 within 3 % of its planar value and inside its design band (all 10 are).
2. The class table of section 3: each row within 3 %.
3. `skidpad_g` increases and `understeer_deg_per_g` decreases from `economy` to `slick` (the planar rows are monotone in both).
4. 1/144 s: `skidpad_g` 0.967 (+0.0001 against 60 Hz), `brake_m` 35.05 (+0.2 m).

## 7. Limits

- **No AC number is checked.** The design values express a plausible, stable, sporty feel. They can be wrong about the real
  game by more than their bands: the first measured row may overturn them. Stage 2 is "inside the bands", not "equal to AC".
- The planar car has no suspension, so the host's numbers will differ; the bands were set wide enough (about 10 % for grip
  scalars) to absorb ordinary body effects and no wider.
- The rationale column is reasoning, not citation: a reader who disagrees with a design value should say so in the review,
  because every number here is cheap to change and nothing downstream depends on its exact value.
- Only the dry, flat, uniform surface is covered; no wet, no temperature (cold tyres give the same numbers as warm ones until
  the thermal spec exists).
- Scalar 'feel' items (steering weight and noise, tyre sound, progression at the limit) are not in this file.
