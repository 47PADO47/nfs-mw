# Scenarios: the scripted tests behind the targets
Sources:
- This repository's `docs/modding/drive-script.md` (own docs): the `--drive-script` syntax and its keys.
- SAE J266 (*Steady-State Directional Control Test Procedures for Passenger Cars and Light Trucks*), ISO 4138 (steady-state
  circular driving behaviour), ISO 7401 (lateral transient response, step input), ISO 3888 and ISO 21994 (stopping distance): the
  names and general shape of the standard manoeuvres (published standards, general understanding only; none of their text or
  tables is reproduced and the procedures below are adapted to an open-loop script).
- Gillespie, *Fundamentals of Vehicle Dynamics*, SAE 1992, and Milliken and Milliken, *Race Car Vehicle Dynamics*, SAE 1995
  (published books): the definition of the understeer gradient, yaw-rate overshoot and settling time: understanding.
- The planar reference car of the appendix is a simplified model of ours, written for this spec from the equations of
  [wheel-dynamics.md](wheel-dynamics.md) and [tyre-force.md](tyre-force.md); no restricted source contributed to it.

Written by: spec author session (Claude), 2026-11-20   Implemented by: (pending)

Part of [the AC handling specs](README.md). The expected numbers of each scenario are in [targets.md](targets.md).

## 1. Purpose and scope

Six manoeuvres, each a fixed open-loop input in `--drive-script` syntax and one scalar that the harness reads from a run
and that `targets.md` bounds. They check the **model against its targets** (feel in numbers), not the host: a scenario that
fails says the tuning of the tyre, the presets or the mapping is off, not that something is broken.

Out of scope: closed-loop driving (there is no driver model: the inputs are fixed and a car that leaves the manoeuvre
simply fails the metric), track driving, wet or loose surfaces, damage.

## 2. Inputs and outputs

**Where they run.** (a) In the host: `nfsmw view-world --drive <CAR> --at <flat spot> --muted --drive-script "<script>"` on a
flat, dry, uniform surface (`SurfaceGrip` 1, 1, 1; at least 1000 m long and 300 m wide), at the fixed 60 Hz tick (and again at
1/144 s: the scalars must agree within the tolerance of `targets.md`). (b) Without the game: the **planar reference car** of
the appendix, an idealised vehicle that uses only the tyre and wheel specs, so the model can be checked by a pure unit test
with no install. The two have different bodies (the host has springs, roll and pitch), so `targets.md` keeps a separate
column for the planar numbers and bands for the host.

**The reference car** `ref_sport` is the `BbVehicleSpec` of [parameter-mapping.md](parameter-mapping.md) section 2. In the host
it is a test car with these `[stats]` (the `car-stats` keys): `mass_kg 1450`, `front_weight_pct 52`, `cg_height_m 0.50`,
`wheelbase_m 2.45`, `front_track_m 1.52`, `rear_track_m 1.50`, `front_tire_radius_m 0.33`, `rear_tire_radius_m 0.34`,
`front_tire_grip = rear_tire_grip = 1.65` (the engine's reference tyre, `STATIC_GRIP` 2.075 = 1.65 times 1.2576), `drive = "rwd"`,
`torque_curve [[1000, 250], [3000, 410], [5000, 430], [7000, 409], [8000, 358]]` (300 kW at 7000 rpm), `redline_rpm 8000`,
`gear_ratios [3.6, 2.2, 1.6, 1.25, 1.0, 0.82]`, `final_drive 3.5`, `drag 0.398` (N per (m/s)^2, so `drag_area_m2` = 0.65), `downforce 0`,
`front_brake_torque_nm 2000`, `rear_brake_torque_nm 800`.

**Signals the harness records every tick** (from the host's per-vehicle state, or the planar model's state): time `t`, body-frame
forward and lateral velocity `vf`, `vr` (m/s, `vr` positive to the right), yaw rate `r` (rad/s, positive turning right), the
body's specific lateral and longitudinal force per unit mass `ay`, `ax` (m/s^2, what a g meter reads; positive to the right
and forward), the front wheel angle `delta` (rad, positive to the right), the four wheel spins and loads, the distance travelled
along the body axis. Derived: `V = hypot(vf, vr)`, body slip `beta = atan2(vr, max(|vf|, 0.5))`.

**Steering normalisation.** The scripts below are written for a front wheel angle of **0.5 rad at `steer = 1`**, at every speed
(the planar car). A host car has its own lock and speed-sensitive range. Before a lateral scenario the harness runs a
calibration segment (`3:throttle=0.5;1:steer=0.2`, reads `delta` at its end) and rescales every `steer` value of the
scenario by `0.1 / delta_measured` (the planar car gives `delta = 0.1` for `steer = 0.2`); the factor is part of the report.
`throttle` values are nominal too: the harness may rescale the hold-speed throttle so the speed stays within the stated
band, and says so in the report.

**Lead-in speed.** `T_LEAD(v)` below is the time a full-throttle run from rest takes to reach `v`: the harness runs
`T:throttle=1` once to find it (rounded to 0.01 s), then regenerates the script with that number. For the planar
`ref_sport`: `T_LEAD(27.78 m/s) = 4.98 s`, `T_LEAD(20 m/s) = 3.57 s`.

**Time stamps.** The time of a tick is the time at its start (the first tick has `t = 0`); the recorded values are the state after the
tick. So `launch_d3_m` is the distance at the end of the 180th tick, and `launch_t100_s` the start time of the first tick whose `V` is
`>= 27.78 m/s`. An unspecified key of a script segment is 0 (`docs/modding/drive-script.md`).

**Scripts** are given one segment per line for reading; the actual string joins the lines with nothing between the `;`.

## 3. State

None: every scenario starts the car at rest (or at the lead-in speed) on the flat surface with fresh tyre state, the mod
switched on, grip gain 1, settled for 1 s with all inputs at 0 before the script starts.

## 4. Equations

### S1 Skidpad: peak sustained lateral acceleration

A circle at a fixed steering input with a slowly increasing throttle: the speed rises until the car runs out of grip.

```
3:throttle=0.6
1:throttle=0.12,steer=0.12      then, one segment per second, the throttle rising by 0.02 each time:
1:throttle=0.14,steer=0.12
...
1:throttle=0.70,steer=0.12      (30 segments: throttle = 0.12 + 0.02 k, k = 0 .. 29, all with steer 0.12)
```

**Scalar `skidpad_g`**: the largest mean of `|ay|` / 9.81 over any 0.5 s window of the run in which `|beta| < 25 deg` for the whole
window (so a spin does not count), in g. The steer value (0.12, a front angle of 0.06 rad: a 41 m radius at low speed) is large
enough that the car reaches its limit before the throttle ends and small enough that it does not plough at once.

### S2 Braking distance from 100 km/h

```
T_LEAD(27.78):throttle=1
6:brake=1
```

**Scalar `brake_m`**: the distance along the body axis from the first tick with `brake > 0` until `V < 0.2 m/s`, normalised
to exactly 100 km/h: `brake_m = d * (27.78 / V0)^2` with `V0` the speed at the first tick of braking. The same run gives the
peak deceleration `brake_peak_g` = the largest `-ax` / 9.81. No ABS: the harness switches the player's assists off; the
reference car's brake bias (0.71 front, 2000 / 800 N m per wheel) makes the front lock first.

### S3 Understeer gradient

A staircase of steering at roughly constant speed.

```
4:throttle=0.6
2.5:throttle=0.09,steer=0.01     steer = 0.01 k for k = 1 .. 15, throttle 0.09 held, 2.5 s each
2.5:throttle=0.09,steer=0.02
...
2.5:throttle=0.09,steer=0.15     (15 segments)
```

For each segment, average the last 0.5 s of `V`, `ay` and `delta`, `r` over the window and form
`x = mean(ay) / 9.81` (g) and `y = (mean(delta) - L * mean(r) / V) * 180 / pi` (degrees), with `L = 2.45 m` the wheelbase.

**Scalar `understeer_deg_per_g`**: the slope of the least-squares line of `y` against `x` over the segments with
`0.1 <= x <= 0.5`. A positive value is understeer. The speed must stay within 22 +/- 4 m/s over those segments (the harness
may adjust the 0.09 throttle).

### S4 Step steer at 100 km/h

```
T_LEAD(27.78):throttle=1
1:throttle=0.12
4:throttle=0.12,steer=0.06
```

At the start of the last segment the steering input steps; `delta` follows through the host's own rate limit (the planar
car limits it to 4 rad/s). With `r_ss` the mean of `r` over the last 1 s and `r_max` its maximum after the step:

**Scalars**: `yaw_overshoot_pct = (r_max - r_ss) / r_ss * 100`, and `yaw_settle_s` = the time from the step until `r` stays inside
`r_ss +/- 5 %` for the rest of the run. The report also gives the steady lateral acceleration (`mean(ay)` over the last second)
and requires it to be between 0.45 and 0.75 g (outside, the harness rescales `steer` and reruns), because overshoot depends on
the amplitude.

### S5 Slide and recover

A power-on slide followed by letting go, six times with increasing throttle (the rungs).

```
T_LEAD(20.0):throttle=1        (rung r: lead-in to 20 m/s, not part of the metric)
1:throttle=0.15,steer=0.10     a steady corner
1:throttle=R,steer=0.10        the rung: R = 0.5, 0.6, 0.7, 0.8, 0.9, 1.0 (one run each)
6:throttle=0.15                steering centred, throttle lifted
```

For each rung: `beta_peak` = the largest `|beta|` of the run; **recovered** at the earliest time `t_rec` after the end of the
rung segment from which `|r| < 0.05 rad/s` and `|beta| < 2 deg` hold for the rest of the run (`recovery = t_rec - t_rung_end`); a
rung is **spun** when `beta_peak >= 45 deg`.

**Scalars**: `breakaway_rung` = the lowest throttle `R` whose `beta_peak >= 5 deg`; `slide_recover_s` = `recovery` of the **highest
rung that did not spin** (it may be `breakaway_rung` or higher). If every rung spins the scalar is `spin` and fails; if none
reaches 5 degrees `breakaway_rung` is `none` and the rung count is reported.

### S6 Launch traction

```
6:throttle=1
```

**Scalars**: `launch_d3_m` = the distance along the body axis after 3.0 s; `launch_t100_s` = the first time `V >= 27.78 m/s`.
Secondary (not bounded, reported): the peak slip ratio `(R w - V) / V` of the driven wheels averaged over the first 2 s after
`V > 3 m/s`, to see wheelspin. The run measures the model's launch grip; the gear changes, the clutch and the engine
are the host's.

## 5. Constants

| Name | Value | Use |
|---|---|---|
| tick | 1/60 s (1/144 s as a second check) | the scenario step |
| `T_LEAD(27.78)`, `T_LEAD(20.0)` | 4.98 s, 3.57 s | the lead-in times of the planar `ref_sport` |
| steering | 0.5 rad at `steer = 1` | the planar car; host cars rescaled (section 2) |
| rate limit of `delta` | 4 rad/s | the planar car |
| windows | 0.5 s (S1), last 0.5 s of 2.5 s (S3), last 1 s (S4) | averaging |
| thresholds | `|beta| < 25 deg` (S1), `5 deg`, `45 deg`, `2 deg`, `0.05 rad/s` (S5) | metric definitions |

## 6. Test vectors

The planar model of the appendix, `ref_sport`, automatic mapping (`sport` compound, `sport` class), the tyre of the mapping
vectors, host inertia 10 with `inertia_scale 0.3`, 60 Hz, 10 substeps per wheel per tick. These are the **planar** numbers of
`targets.md` (the model of this spec, not the reference game; S4 and S5 start the planar car directly at the lead-in speed with
rolling wheels instead of running the lead-in, S2 runs it); tolerance for a reimplementation: the bands of `targets.md`
section 2, and a looser `3 %` or 0.05 s relative to these values when the same planar model is reimplemented exactly.

| Scenario | Scalar | Planar value |
|---|---|---|
| S1 | `skidpad_g` | 0.967 |
| S2 | `brake_m`, `brake_peak_g` | 34.85 m (raw 34.95 m from 100.15 km/h, normalised to exactly 100), 1.201 g |
| S3 | `understeer_deg_per_g` | 1.119 |
| S4 | `yaw_overshoot_pct`, `yaw_settle_s`, steady `r_ss`, steady `ay` | 12.84, 0.65, 0.2055 rad/s, 0.600 g |
| S5 | rung `beta_peak` and recovery: 0.5: 1.8 deg, 0.18 s; 0.6: 2.2 deg, 0.20 s; 0.7: 7.3 deg, 0.83 s; 0.8: 56.2 deg (spun); 0.9: 86.9 deg (spun); 1.0: 88.0 deg (spun) | `breakaway_rung` 0.7, `slide_recover_s` 0.83 |
| S6 | `launch_d3_m`, `launch_t100_s` | 25.22 m, 4.97 s |

Sanity properties of the scenario code itself (tests): a car that cannot move (zero throttle) gives `skidpad_g = 0` and no braking
distance; an S3 run on a tyre with identical axles (`axle_angle_bias_deg = 0`) gives a gradient below 0.5 deg/g on `ref_sport`
(the planar value is 0.23); doubling `mu_y0` raises `skidpad_g` and `brake_peak_g` by less than a factor of 2 (load
sensitivity); S1 and S2 at 1/144 s agree with the 60 Hz values within 2 %.

## 7. Limits

- Open loop: nothing keeps the car on a circle or a line. A scenario that depends on a thin margin (S5 near the breakaway)
  is sensitive to the car; the ladder of rungs exists to make the metric robust to that, not exact.
- The scripts are written for one car; other cars are checked in classes (`targets.md`), not by the numbers of `ref_sport`.
- The host's body (roll, pitch, hub motion) changes the numbers: the planar column is a model check, not a prediction of
  the host.
- The metrics do not measure feel (steering weight, progressiveness, sound): the owner's drive at gate A does.
- S6 mixes the model's grip with the host's drivetrain; a slow launch can be a clutch problem. It is bounded loosely.

## Appendix: the planar reference car

For the pure-test harness (no game). State: `V_f`, `V_r` (body-frame forward and rightward velocity), yaw rate `r` (positive right),
heading, front wheel angle `delta`, the four wheels' state of [wheel-dynamics.md](wheel-dynamics.md). Per 1/60 s tick:

```
a = (1 - f) L,  b = f L  (f = front_weight_fraction, L = wheelbase);  Iz = 1.05 m a b;  rho = 1.2
delta moves towards 0.5 * steer by at most 4 rad/s * dt
loads:  Fz_FL,FR = f m g/2 - m ax h/(2L) +- 0.5 m ay h / track_f      (FL gets +, FR -)
        Fz_RL,RR = (1-f) m g/2 + m ax h/(2L) +- 0.5 m ay h / track_r   (ax, ay = last tick's specific forces; clamp >= 0)
wheel positions (x right, z forward): FL (-tf/2, a) FR (tf/2, a) RL (-tr/2, -b) RR (tr/2, -b); the front wheels are turned by delta
patch velocity of wheel i: v_f = V_f - r x_i,  v_r = V_r + r z_i;  in the wheel frame
     Vx = v_f cos(d) + v_r sin(d),   Vy = -v_f sin(d) + v_r cos(d)       (d = delta for the front, 0 for the rear)
call wheel_tick (the call of wheel-dynamics.md section 4.8) with  T_drive, T_brake = pedal * brake_torque (per wheel), host inertia 10,
     surfaces 1, grip 1, corner_mass_kg = f m / 2 (front wheels) or (1 - f) m / 2 (rear wheels)
force back into the body frame:  F_f = fx cos(d) - fy sin(d),  F_r = fx sin(d) + fy cos(d)
sum_f, sum_r = sum of F_f, F_r  minus  0.5 rho CdA |V| (V_f, V_r)
moment about the vertical axis (clockwise positive) = sum over wheels of (z F_r - x F_f);  aligning torques are ignored
V_f += dt (sum_f / m + r V_r);  V_r += dt (sum_r / m - r V_f);  r += dt moment / Iz      (explicit Euler: all three use the
     start-of-tick V_f, V_r, so V_r does not see the new V_f)
ax = sum_f / m,  ay = sum_r / m,  the heading advances by r dt
drive (RWD, equal torque on RL and RR): the gear with the largest wheel torque whose engine speed is below 0.97 redline;
     engine speed rpm = mean rear wheel spin * ratio * final_drive * 60 / 2 pi, torque evaluated at max(rpm, 3500):
     T = min(1.05 P / w_pp, P / w) with w the engine speed (rad/s), w_pp the power-peak speed;
     zero above the redline; axle torque = throttle * T * ratio * final_drive * 0.9, split equally: RL and RR each get half of it
```

`CdA = 0.65 m^2`, `P = 300 kW` at 7000 rpm for `ref_sport`; downforce zero. All forces are the **mean** forces returned by the
tyre calls; the wheels' states persist from tick to tick.
