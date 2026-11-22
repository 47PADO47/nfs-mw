# Parameter mapping: from an NFS car to the tyre model
Sources:
- This repository's `docs/modding/car-stats.md`, `libs/blackbox-vehicle/src/stats/build.rs`, `docs/specs/vehicle-calibration.md` and
  `docs/specs/vehicle-suspension-tires.md` (own code and docs): the scale of the game's `STATIC_GRIP` and `DYNAMIC_GRIP` (the
  engine's reference tyre has 2.075 and 1.85), the per-axle and per-wheel conventions, which fields the host's `BbVehicleSpec`
  carries: facts about our own engine.
- Milliken and Milliken, *Race Car Vehicle Dynamics*, SAE 1995, and Gillespie, *Fundamentals of Vehicle Dynamics*, SAE 1992
  (published books): the usual ranges of tyre friction, peak slip angle, load exponent and understeer gradient of road
  and competition tyres: understanding, used only to bound the design values.
- rustyAC, https://github.com/thesupersupersigma/rustyAC, `GPL-3.0-or-later`, `src/data/tyres_ini.rs` (the names and units of
  the reference game's tyre data keys) and `docs/port/tyre_step.md` (their defaults): understanding of which parameters a
  tyre of that family has and what a plausible value is. Facts only (key meaning, a few default numbers marked `[U]`); no
  code, comment, table layout or file format was copied: the TOML schema below is ours.

Written by: spec author session (Claude), 2026-11-20; revised by the spec maintainer session, 2026-11-20 (car id, forced compound, `tuning/`, inertia default)   Implemented by: (pending)

Part of [the AC handling specs](README.md). The parameters are consumed by [tyre-force.md](tyre-force.md) and
[wheel-dynamics.md](wheel-dynamics.md).

## 1. Purpose and scope

NFS data lacks most of what a simulation tyre needs: it has a static and a dynamic friction coefficient and a tyre size, no
slip curve, no load sensitivity, no relaxation length. This file says (a) which of the host's numbers feed which parameter,
(b) which **presets** supply the rest (one per tyre compound class, one modifier set per car class), (c) the **TOML
schema** of `presets/*.toml` and `tuning/<car>.toml` that holds them, and (d) the order in which they combine. The presets and
the per-car overrides are the **main tuning surface** of the mod: most of what the player calls the feel is set here.

Out of scope: runtime settings (grip gain, preset choice: section 7), thermal and suspension parameters (their own specs).

## 2. Inputs and outputs

Input: the read-only `BbVehicleSpec` the host gives when the car (its `revision`) changes (all SI; `f32` unless noted; the field names
are those of [api-handling.md](../../modding/api-handling.md) and the axle pairs are front then rear), and the derived **car id**
of 2.1 (the spec carries no car name). Output: two `TyreParams` sets (front axle, rear axle) with every key of the table in
section 5, built once per car change, then read-only.

**Reference car** `ref_sport` (used by the vectors and by [scenarios.md](scenarios.md)): `mass_kg 1450`, `wheelbase_m 2.45`,
`track_front_m 1.52`, `track_rear_m 1.50`, `cg_height_m 0.50`, `front_weight_fraction 0.52`, `tyre_radius_front_m 0.33`,
`tyre_radius_rear_m 0.34`, `tyre_width_front_m 0.235`, `tyre_width_rear_m 0.265`, `peak_friction_front = peak_friction_rear = 2.075`,
`sliding_friction_front = sliding_friction_rear = 1.85`, `drive_layout 1` (rear), `max_power_w 300000`, `max_power_rpm 7000`,
`redline_rpm 8000`, `final_drive 3.5`, `gear_count 6`, `gear_ratios [3.6, 2.2, 1.6, 1.25, 1.0, 0.82, 0, 0]`,
`brake_torque_front_nm 2000`, `brake_torque_rear_nm 800`, `drag_area_m2 0.65`, `downforce_area_m2 0`.

**What each `BbVehicleSpec` field does** (only these fields are read; everything else about a car is a preset):

| Field | Used for |
|---|---|
| `peak_friction_front`, `peak_friction_rear` | the **grip level** `L = peak_friction / 2.075` of each axle (the game's reference tyre is 1.0; `car-stats` coefficient 1.0 is `L = 0.61`, 1.5 is `L = 0.91`): the front axle's picks the compound class, each axle's sets its `mu_y0` (4.1) |
| `sliding_friction_front`, `sliding_friction_rear` | the sliding over peak ratio `rho = sliding_friction / peak_friction` of each axle: sets its sliding levels `fall_y`, `fall_x` (4.1) |
| `tyre_width_front_m`, `tyre_width_rear_m` | the vertical stiffness `k_vert = k_vert_per_width * width` of each axle (contact patch, aligning torque); the rear width also picks the `super` car class |
| `mass_kg`, `max_power_w` | the car class (4.2) |
| `drive_layout` | the axle slip-angle bias (4.3): a front-driven car is set up to understeer more |
| `front_weight_fraction`, `mass_kg` | the static corner loads, only for the one-line log and the vectors |
| `wheelbase_m`, `track_*`, `cg_height_m` | unused in stage 2 (stage 3 uses them for load transfer references) |
| `tyre_radius_*` | unused: every tyre call carries the wheel radius |
| `max_power_rpm`, `redline_rpm`, `final_drive`, `gear_*` | unused in stage 2 (traction control, stage 4) |
| `brake_torque_front_nm`, `brake_torque_rear_nm` | unused in stage 2 (brake bias and ABS, stage 4) |
| `drag_area_m2`, `downforce_area_m2` | unused (the host applies aerodynamics; its downforce is already inside the vertical load) |

**Assumptions about the spec.** `peak_friction_*` and `sliding_friction_*` are, in NFS:MW, the engine's `STATIC_GRIP` and
`DYNAMIC_GRIP` scale of the car, **not** a friction coefficient near 1: the engine's reference tyre has 2.075 and 1.85, and for a mod
car the value is the stats builder's `coefficient * 1.2576`. (The interface page calls them friction coefficients; the number is the
engine's, which is why `L` divides by 2.075.) The brake torques are **per wheel** at full pedal (`car-stats` says so). The spec has
no car identity, no handbrake torque, **no steering lock** and no compound name, so none of them can be used; where
[steering.md](steering.md) needs a lock it takes a preset value (30 degrees by default, overridable per car).

### 2.1 The car id

`BbVehicleSpec` and the vehicle state carry no car name, so a car is named by its numbers; the id is the file name of its override
(`tuning/<id>.toml`) and the name the log prints:

```
id = "spec-" + M + "-" + W + "-" + P
M = round(mass_kg)    W = round(wheelbase_m * 100)    P = round(max_power_w / 1000)
```

Each field is widened from `f32` to `f64` first, the arithmetic is done in `f64`, and `round` is to the nearest integer with ties
away from zero; each number is printed in decimal digits with no sign, padding or separator. If a field is not finite, or `mass_kg`
or `wheelbase_m` is `<= 0`, or `max_power_w < 0`, the id is `spec-invalid` and no file is looked up. Examples: `ref_sport`
`spec-1450-245-300` (`2.45f32 * 100 = 245.000005`); COMPACT_FWD `spec-1100-245-90`; SUPER `spec-1500-245-480`; HEAVY_AWD
`spec-1900-245-300`; `max_power_w = 0` gives `spec-1450-245-0`. **Collisions:** the id resolves 1 kg, 1 cm and 1 kW, so two cars
(trims) with the same rounded numbers share an id and a file. Only the override is shared: the automatic mapping is cached per
`vehicle_revision` (section 3) and still follows each car's own grip. If a later spec version carries a lower-case car name, the
lookup order becomes `tuning/<name>.toml`, then `tuning/<id>.toml`.

## 3. State

None after construction. The mapping is a pure function of `(BbVehicleSpec, the forced compound if any, files)`; the result is
cached by `vehicle_revision` (not by the id), so a car change always rebuilds it. Changing the grip gain at run time does not
rebuild it; changing the `preset` setting does.

## 4. Equations

### 4.1 Compound class and grip

```
per axle a in {front, rear}:
  L_a    = peak_friction_a / 2.075                         (if not finite or <= 0: L_a = 1.0)
  rho_a  = sliding_friction_a / peak_friction_a            (if either is not finite or <= 0: rho_a = 0.89)
compound = economy if L_front < 0.70; road if < 0.95; sport if < 1.15; semislick if < 1.40; slick otherwise
           (the front axle chooses, unless the compound is forced, below)
per axle a, with the preset C of that compound:
  mu_y0_a  = clamp( preset.mu_y0 + preset.grip_slope * (L_a - preset.level_ref), 0.6, 2.2 )
  fall_y_a = clamp( rho_a - 0.03, 0.6, 0.95 )     fall_x_a = clamp( rho_a - 0.06, 0.6, 0.95 )
```

**Which compound, and what forcing it means.** The first of these that applies chooses it: (1) the player's setting `preset`
when it is not `auto`; (2) the `compound` key of the car's tuning file; (3) the automatic choice above. A **forced** compound
(1 or 2) is a tyre *fitted* to the car, not a label on the car's own grip, so it **re-bases the grip level** on the compound's
reference: `L_front' = level_ref(C)` and `L_rear' = level_ref(C) + (L_rear - L_front)` (the car's front-to-rear difference is
kept, the `rho` values stay the car's own), and these replace `L_a` in the `mu_y0` line. Equivalently, forcing `C` gives the same
parameters as the automatic mapping of the same car whose front level is `level_ref(C)` (the reference levels 0.60, 0.80, 1.00,
1.25, 1.55 each fall inside their own class, so a test can assert it). The front axle therefore gets exactly `preset.mu_y0`, and
at a fixed car the front `mu_y0` rises with the compound (1.00, 1.10, 1.20, 1.35, 1.60): a forced `slick` is never below a
forced `sport`, whatever the car's own grip. (Without re-basing a car of level 1.0 forced to `slick` would get
`1.60 + 1.0 * (1.0 - 1.55) = 1.05`, less than `sport`'s 1.20: the defect this rule removes.) The rear axle keeps the order
as long as `L_rear - L_front > -0.6`. The car class (4.2), the bias and `grip_gain` are not affected by forcing.

All the other keys of the compound come from the preset file (5.2). The reference game's tyres have `rho` 0.85 to 0.9, our
`fall` derivation therefore places the sliding level a few points below the game's own ratio; the longitudinal one lower (a
locked wheel holds less than a drifting one).

### 4.2 Car class

```
super   if tyre_width_rear_m >= 0.285 or max_power_w / mass_kg >= 250
heavy   else if mass_kg >= 1800
compact else if mass_kg < 1250
sport   otherwise                                              (a car file may force it with `class = "..."`)
```

A car class applies `value = value * mul + add` to the keys it names, after the compound (table 5.3). It carries the part of
the feel that depends on the car's size rather than on its tyre: a small light car turns in sooner (longer peak angle, shorter
relaxation), a heavy one lazier, a supercar's wide tyres are stiffer.

### 4.3 Axle bias

The load on both axles is similar for most cars, so an unbiased tyre gives an almost neutral car (an understeer gradient of
0.2 deg/g on the reference car, measured by the scenario model); a road car is set up to understeer slightly for stability.
The bias splits the lateral peak slip angle between the axles:

```
peak_angle_front = peak_angle + axle_angle_bias_deg / 2        peak_angle_rear = peak_angle - axle_angle_bias_deg / 2
axle_angle_bias_deg = 3.0 (front-driven), 2.0 (rear-driven), 2.5 (all-wheel drive)          (overridable)
```

A larger peak angle is a lower cornering stiffness per load; each degree of bias adds about 0.45 deg/g of understeer
gradient on the reference car. Everything else is identical on both axles unless the car file says otherwise.

### 4.4 Combination order

```
1. defaults (the "default" column of 5.1)
2. the compound preset's keys (all keys of its [tyre] table); the compound is chosen as in 4.1 (setting, car file, automatic)
3. per axle: mu_y0 from its level (re-based when the compound is forced, 4.1) and fall_y, fall_x from its rho
4. the car class (4.2): value = value * mul + add
5. exp_y, exp_x clamped to [0.6, 1.0]
6. the car file's [tyre] keys replace the value of BOTH axles (peak_angle_deg here is the axle MEAN, still unsplit)
7. split of the mean peak angle by the bias (4.3): front = mean + bias/2, rear = mean - bias/2, with the bias from the car file's
   top-level `axle_angle_bias_deg` or the layout default
8. the car file's [tyre.front] and [tyre.rear] keys replace the value of one axle (peak_angle_deg here is that axle's own, absolute)
9. clamp every key of both axles to its range of 5.1 (a car class `add`, the bias split or a car-file value can leave it: a mean
   peak angle of 14 with a bias of 8 gives a front 18, clamped to 14); a file whose own keys are out of range is rejected (5.4),
   the clamp only handles combinations
10. mu_x0 = mu_y0 * mu_x_ratio, k_vert = k_vert_per_width * tyre width of the axle
```

At run time `grip_gain` (a setting) multiplies `u_x`, `u_y` of [tyre-force.md](tyre-force.md) through its `grip` input; it is not a
parameter.

## 5. Constants, presets and the TOML schema

### 5.1 Keys, defaults, ranges

All keys are numbers. The default is the value a missing key takes in step 1 of 4.4. A compound preset's `[tyre]` table lists
the keys of table 5.2 (any other key may be listed too); the keys it leaves out keep the default. Ranges are enforced by the
loader.

| Key | Unit | Default | Range | Origin and reason |
|---|---|---|---|---|
| `mu_y0` | 1 | 1.20 | 0.6 to 2.2 | [D] peak lateral friction at 2000 N |
| `mu_x_ratio` | 1 | 1.10 | 0.8 to 1.4 | [D] longitudinal over lateral peak |
| `exp_y`, `exp_x` | 1 | 0.85, 0.88 | 0.6 to 1.0 | [T] load exponents |
| `load_factor_max` | 1 | 1.6 | 1.0 to 3.0 | [D] |
| `peak_angle_deg` | deg | 8.0 | 4 to 14 | [D] mean lateral peak slip angle at 2000 N |
| `axle_angle_bias_deg` | deg | by layout (4.3) | 0 to 8 | [D] front minus rear peak angle |
| `flex` | 1 | 0.20 | 0 to 0.6 | [D] peak slip growth with load |
| `cx_ratio` | 1 | 1.4 | 0.8 to 3 | [D] longitudinal stiffness over lateral |
| `fall_y`, `fall_x` | 1 | 0.85, 0.82 | 0.4 to 1.0 | [N] derived from `rho` (4.1) |
| `q_y`, `q_x` | 1 | 1.2, 1.4 | 0.1 to 6 | [D] fall-off speed |
| `comb_exp` | 1 | 2.0 | 1.2 to 3 | [T] |
| `brake_gain` | 1 | 1.0 | 0.8 to 1.3 | [D] |
| `speed_sens` | s/m | 0.0012 | 0 to 0.02 | [D] |
| `camber_gain`, `camber_d1`, `camber_d2` | | 0.10, 1.15, 11.0 | see tyre-force | [D] unused until the suspension hook |
| `trail_ratio` | 1 | 0.12 | 0.05 to 0.25 | [U] |
| `k_vert_per_width` | N/m per m | 1.1e6 | 0.5e6 to 2e6 | [D] |
| `relax_len` | m | 0.34 | 0.05 to 1.5 | [D] |
| `relax_slide_ratio`, `relax_load_slope`, `relax_tau_max` | | 0.5, 0.3, 0.08 | see wheel-dynamics | [U]/[D] |
| `v_low_x`, `v_low_y` | m/s | 1.0, 0.4 | see wheel-dynamics | [D] |
| `rr_c0`, `rr_c2`, `air_drag_nms` | | 0.012, 1.5e-6, 0.05 | see wheel-dynamics | [D] |
| `stick_cap` | 1 | 0.5 | 0.1 to 1 | [D] |
| `inertia_scale` | 1 | 1.0 | 0.05 to 2 (stable on the real host: 0.5 to 2) | [D] multiplies the host's wheel inertia; 1.0 because the host's drivetrain limit-cycles below 0.5 ([wheel-dynamics.md](wheel-dynamics.md) contract point 3) |

`fz0 = 2000 N` is a constant of the model, not a key.

### 5.2 Compound presets (`presets/compound-<name>.toml`)

Values of the keys that differ by compound (all other keys keep the default). `level_ref` and `grip_slope` are preset-only
keys: `mu_y0` at the level of the car is `mu_y0 + grip_slope * (L - level_ref)`. The classes tile the level axis so the
mapping is continuous across a boundary to within 0.02 of `mu_y0`. Real-world anchors for the design values: a road tyre
reaches 0.9 to 1.1 on dry asphalt at ordinary loads, a sports or summer tyre 1.1 to 1.3, a semi-slick about 1.3 to 1.4,
a slick 1.5 to 1.8 [T]; the numbers are chosen inside these bands and are not measured.

| Key | economy | road | sport | semislick | slick |
|---|---|---|---|---|---|
| `level_ref` | 0.60 | 0.80 | 1.00 | 1.25 | 1.55 |
| `grip_slope` | 0.55 | 0.55 | 0.55 | 0.80 | 1.00 |
| `mu_y0` | 1.00 | 1.10 | 1.20 | 1.35 | 1.60 |
| `mu_x_ratio` | 1.05 | 1.08 | 1.10 | 1.12 | 1.12 |
| `exp_y` | 0.90 | 0.88 | 0.85 | 0.82 | 0.80 |
| `exp_x` | 0.92 | 0.90 | 0.88 | 0.86 | 0.84 |
| `peak_angle_deg` | 9.0 | 8.5 | 8.0 | 7.5 | 7.0 |
| `flex` | 0.25 | 0.22 | 0.20 | 0.18 | 0.15 |
| `cx_ratio` | 1.2 | 1.3 | 1.4 | 1.6 | 1.8 |
| `fall_y` (informational: 4.1 always overrides it) | 0.88 | 0.87 | 0.85 | 0.82 | 0.78 |
| `fall_x` (same) | 0.85 | 0.84 | 0.82 | 0.78 | 0.74 |
| `q_y` | 1.0 | 1.1 | 1.2 | 1.5 | 2.0 |
| `q_x` | 1.2 | 1.3 | 1.4 | 1.8 | 2.4 |
| `speed_sens` | 0.0015 | 0.0015 | 0.0012 | 0.0010 | 0.0008 |
| `relax_len` | 0.40 | 0.36 | 0.34 | 0.30 | 0.26 |

Direction of the design: grippier compounds have a lower peak angle (a sharper tyre), a more pronounced peak and a lower
sliding share (a slick drops off harder, a road tyre forgives), a stronger load sensitivity of the friction coefficient and
a shorter relaxation length.

### 5.3 Car classes (`presets/class-<name>.toml`)

| Class | `mul` | `add` | Meaning |
|---|---|---|---|
| `compact` | `relax_len` 0.90 | `peak_angle_deg` +0.5 | light, small tyres: turns in sooner |
| `sport` | none | none | the default |
| `super` | `relax_len` 1.10, `cx_ratio` 1.05 | `peak_angle_deg` -0.5 | wide, stiff tyres, sharper, longer lag |
| `heavy` | `relax_len` 1.15 | `peak_angle_deg` +0.5, `exp_y` -0.02, `exp_x` -0.02 | more load sensitive, lazier |

### 5.4 TOML schema

All files: UTF-8 TOML, `schema = 1` first, `deny_unknown_fields` at every level, every number finite and inside its range;
a file that fails is rejected whole with a one-line log (file, key, reason) and the layer it replaces falls back to its
default. Order of keys is free.

```toml
# presets/compound-sport.toml
schema = 1
kind = "compound"               # "compound" | "class"
name = "sport"                  # = the file name without "compound-" and ".toml"
level_ref = 1.00
grip_slope = 0.55
[tyre]                          # section 5.1 keys; here the whole list of 5.2
mu_y0 = 1.20
mu_x_ratio = 1.10
# ... exp_y, exp_x, peak_angle_deg, flex, cx_ratio, fall_y, fall_x, q_y, q_x, speed_sens, relax_len

# presets/class-super.toml
schema = 1
kind = "class"
name = "super"
[mul]                           # any numeric 5.1 key; value = value * mul
relax_len = 1.10
cx_ratio = 1.05
[add]                           # value = value + add (after mul)
peak_angle_deg = -0.5

# tuning/spec-1450-245-300.toml  the file name is the car id of 2.1 (or the lower-case car name), no extension
schema = 1
compound = "sport"              # optional: forces the compound instead of the level (4.1 re-bases the grip)
class = "super"                 # optional
axle_angle_bias_deg = 2.5       # optional
[tyre]                          # optional: absolute values for both axles (step 7)
mu_y0 = 1.25
[tyre.front]                    # optional: one axle (step 8)
peak_angle_deg = 9.5
[tyre.rear]
mu_y0 = 1.30
```

`presets/` ships with the mod (5 compounds, 4 classes) and is read from the mod folder; `tuning/` holds the per-car overrides and
may be empty: a car without a file uses the automatic mapping and the mod logs **once** that car's id, the class it chose and `L`
(`ac-handling: car "spec-1450-245-300": L=1.00 -> sport/sport, no tuning/spec-1450-245-300.toml`). The folder is named `tuning/`,
not `cars/`: a mod's `cars/` folder is where the host reads **car manifests** ([modding README](../../modding/README.md), folder
layout), so an override file there would be parsed by the host as a manifest and refused. A missing preset file falls back to the built-in copy of
section 5.2, so the mod works with an empty `presets/` folder. The player's settings never write these files.

## 6. Test vectors

Reproduced by a script that implements only sections 4 and 5 (double precision); `mu_x0 = mu_y0 * mu_x_ratio`. Tolerance:
relative `1e-4`. The four cars differ from `ref_sport` only where listed.

| Car | Changes against `ref_sport` | `L` | compound | class |
|---|---|---|---|---|
| REF | none | 1.0000 | sport | sport |
| COMPACT_FWD | `mass 1100`, `static 1.25`, `dynamic 1.10`, `layout 0`, `max_power 90000`, widths 0.195 / 0.195, `front_weight 0.60` | 0.6024 | economy | compact |
| SUPER | `mass 1500`, `static 2.3`, `dynamic 2.0`, `max_power 480000`, rear width 0.295 | 1.1084 | sport | super |
| HEAVY_AWD | `mass 1900`, `static 1.7`, `dynamic 1.5`, `layout 2` | 0.8193 | road | heavy |

| Car | `mu_y0` | `mu_x0` | `exp_y`, `exp_x` | `fall_y`, `fall_x` | `q_y`, `q_x` | peak angle front, rear | `relax_len` | `cx_ratio` | `flex` | `speed_sens` |
|---|---|---|---|---|---|---|---|---|---|---|
| REF | 1.2000 | 1.3200 | 0.850, 0.880 | 0.8616, 0.8316 | 1.20, 1.40 | 9.000, 7.000 | 0.3400 | 1.400 | 0.200 | 0.0012 |
| COMPACT_FWD | 1.0013 | 1.0514 | 0.900, 0.920 | 0.8500, 0.8200 | 1.00, 1.20 | 11.000, 8.000 | 0.3600 | 1.200 | 0.250 | 0.0015 |
| SUPER | 1.2596 | 1.3856 | 0.850, 0.880 | 0.8396, 0.8096 | 1.20, 1.40 | 8.500, 6.500 | 0.3740 | 1.470 | 0.200 | 0.0012 |
| HEAVY_AWD | 1.1106 | 1.1995 | 0.860, 0.880 | 0.8524, 0.8224 | 1.10, 1.30 | 10.250, 7.750 | 0.4140 | 1.300 | 0.220 | 0.0015 |

Static corner loads (log line only): REF 3698.4 N front, 3413.9 N rear; COMPACT_FWD 3237.3, 2158.2; HEAVY_AWD 4846.1, 4473.4.
Derivation spot checks: REF `fall_y = clamp(1.85/2.075 - 0.03) = 0.8616`; COMPACT_FWD `rho = 1.1/1.25 = 0.88`, so `fall_y` is
0.85 and `fall_x` 0.82; HEAVY_AWD `L = 1.7/2.075 = 0.8193`, `mu_y0 = 1.10 + 0.55 * (0.8193 - 0.80) = 1.1106`, class `heavy`
adds 0.5 to the peak angle (`8.5 + 0.5 = 9.0`, bias 2.5 for all-wheel drive: front `10.25`, rear `7.75`) and subtracts 0.02 from
`exp_y`.

**Per-axle grip**: `ref_sport` with `peak_friction_rear = 2.2`, `sliding_friction_rear = 1.95`: the front is as REF; the rear has
`L = 1.0602`, `mu_y0 = 1.2331`, `mu_x0 = 1.3564`, `fall_y = 0.8564` (`1.95 / 2.2 - 0.03`), `fall_x = 0.8264`; the compound stays
`sport` (the front chooses).

**Override example**: `ref_sport` with a car file `compound = "road"`, `class = "compact"`, `[tyre] mu_y0 = 1.05, q_y = 1.5`
gives `mu_y0 = 1.0500`, `mu_x0 = 1.1340`, `q_y = 1.50`, `fall_y = 0.8616` (from `rho`, untouched), `relax_len = 0.3240`
(`0.36 * 0.90`), peak angle front 10.000, rear 8.000 (`8.5 + 0.5`, bias 2.0 for rear drive).

**Peak angle override** (steps 6 to 8): `ref_sport` with `[tyre] peak_angle_deg = 9.0` gives front 10.000 and rear 8.000 (the value is
the mean, the bias 2.0 of a rear-driven car is split around it); adding `[tyre.rear] peak_angle_deg = 7.5` makes the rear exactly 7.500.

**Forced compound** (4.1, the `preset` setting or a car file's `compound`): `mu_y0` of the front and the rear axle (`ref_sport`
has equal axles; the rear-2.2 car is the per-axle example above with `L_rear - L_front = 0.0602`); in brackets the value the
formula gives without re-basing (the defect):

| Forced | `ref_sport` front = rear | rear-2.2 car: front, rear | COMPACT_FWD (`L = 0.6024`) | HEAVY_AWD (`L = 0.8193`) |
|---|---|---|---|---|
| economy | 1.0000 (1.2200) | 1.0000, 1.0331 | 1.0000 | 1.0000 |
| road | 1.1000 (1.2100) | 1.1000, 1.1331 | 1.1000 | 1.1000 |
| sport | 1.2000 (1.2000) | 1.2000, 1.2331 | 1.2000 (0.9813) | 1.2000 (1.1006) |
| semislick | 1.3500 (1.1500) | 1.3500, 1.3982 | 1.3500 | 1.3500 |
| slick | **1.6000** (1.0500) | 1.6000, 1.6602 | 1.6000 (0.6524) | 1.6000 (0.8693) |

The `fall`, `q` and relaxation values of a forced compound are its preset's keys with the car's own `rho` (so the rows of the
tables above change only in `mu_y0`, `mu_x0` and the keys of the compound). Precedence vector: a `preset` setting `slick` with a car
file `compound = "road"` gives slick; with `preset = auto` the same file gives road; with neither, `ref_sport` gives sport.
Invariant (property test): for each compound `C`, forcing `C` on any car equals the automatic mapping of that car with the front
`peak_friction` set to `level_ref(C) * 2.075`, the rear shifted by the same difference, and both `sliding_friction` values scaled
to keep each axle's `rho`.

**Car id** (2.1): `ref_sport` `spec-1450-245-300`; `mass_kg 1449.5` gives `1450` (ties away from zero) and `1449.49` gives `1449`;
`wheelbase_m 2.455` (`f32` 2.4549999) gives `245`; `max_power_w 299500` gives `300` (and `299499` gives `299`); a NaN mass gives
`spec-invalid`.

Loader tests (no numbers): an unknown key, a number out of range, a missing `schema`, a `name` that differs from the file name,
`kind = "class"` with a `[tyre]` table, each fail the file and are logged once; an empty `tuning/` and an empty `presets/` still
produce the vectors above.

## 7. Limits

- Only grip is per axle. A car that handles badly for any other reason (its rear peak angle, its rear relaxation) needs an
  override in its car file; the model does not guess.
- No identification of the tyre compound beyond the grip level: two cars with the same `peak_friction` get the same tyre.
- The class thresholds (0.70, 0.95, 1.15, 1.40; 1250, 1800 kg; 0.285 m; 250 W/kg) and every `[D]` number are design values;
  the second half of tuning is done in the car files, with the scenarios of [scenarios.md](scenarios.md).
- The car id names a car only by three rounded numbers (2.1); it cannot tell apart trims that share them.
- Stage 2 targets five to eight cars; the others get the automatic mapping, which is plausible but untuned.
- The settings that remain at run time are not part of the schema: `enabled` (master switch, default off), `preset`
  (`auto` or a compound name; forces the compound for every car and re-bases its grip level, 4.1; it outranks a car file's `compound`), `grip_gain` (0.5 to 1.5, default 1.0) and `substeps`
  (0 to 24, default 0); they live in `bb.settings` and `bb.menus`.

## Changes (revision of 2026-11-20, for the implementer's tests)

| Item | Before | Now |
|---|---|---|
| default `inertia_scale` | 0.3 | **1.0** (range 0.05 to 2 unchanged; stable 0.5 to 2 on the host) |
| per-car folder | `cars/<car>.toml` | **`tuning/<id>.toml`** (the host reads `cars/` as car manifests) |
| car name | "as `bb.vehicles` reports it" | derived id `spec-<kg>-<cm>-<kW>` (2.1); the mapping cache is by `vehicle_revision` |
| forced compound | `mu_y0` from the car's own level (slick below sport) | re-based level: front `mu_y0 = preset.mu_y0` exactly (vector table above) |
| compound precedence | not stated | setting, then car file, then automatic |
| step 9 | derived values | clamp every key to its range, then the derived values (step 10); no existing vector changes |
| `mu_y0`, class, per-axle, override, peak-angle vectors | | unchanged |
