# Driver aids: traction control, ABS, stability control, brake bias
Sources: Bosch, "Automotive Handbook" (Robert Bosch GmbH, 10th ed.), chapters on ABS, traction control and ESP, public literature, understanding (what each aid measures and acts on)
Sources: van Zanten, "Bosch ESP Systems: 5 Years of Experience", SAE 2000-01-1633, public literature, understanding (yaw and side-slip based stability control, brake actuation on one wheel)
Sources: Rajamani, "Vehicle Dynamics and Control" (Springer, 2nd ed. 2012), public literature, understanding (slip ratio thresholds of ABS and traction control)
Sources: rustyAC, https://github.com/thesupersupersigma/rustyAC, `crates/rustyac-physics/src/car/aids.rs`, `brakes.rs` and `docs/port/`, GPL-3.0-or-later (per its LICENSING.md; the repo also carries MIT/Apache files for unrelated parts), understanding of behaviour and facts (periodic check with a 0.05 s default period; traction control cuts the engine while the driven wheel slip ratio exceeds a limit, above 1 m/s; ABS releases a locking wheel's brake for one period when its slip ratio is below minus a limit, only above 20 km/h, with 4, 2 or 1 channels; stability control as a yaw moment proportional to mass, the rear-minus-front slip angle difference, a user gain and a constant 30, only above 5 km/h; brake bias as the front share of the brake torque); no code, comments or structure taken
Sources: this repository, `libs/blackbox-vehicle/src/brakes/mod.rs`, `vehicle/assists.rs`, `vehicle/drive_assists.rs` (what the host does with pedals today), MIT OR Apache-2.0, facts
Written by: spec author session (Claude, stage 0 part 2), 2026-11-20   Implemented by: not yet

Part of [the AC handling specs](README.md), [ADR 0016](../../decisions/0016-handling-hooks.md). Implements the aid stages of the
`controls` hook (`bb.handling` 1.2); steering is in [steering.md](steering.md). Axes: x right, y up, z forward; wheels FL FR RL RR.

## 1. Purpose and scope

Four electronic aids, each with a level `0..N` (0 = off), run once per tick **before** the host turns pedals into torques.

| Aid | Acts on | Measures |
|---|---|---|
| traction control (TC) | throttle | slip ratio of the driven wheels |
| ABS | brake of each wheel | slip ratio of each wheel |
| stability control (ESC) | brake of one front wheel | rear and front slip angles |
| brake bias | brake split front/rear | nothing (a setting) |

Out of scope: electronic differential lock, pit limiter, launch control, hill hold, torque vectoring, an engine map that
shapes the cut (the cut is on the throttle pedal, the host's engine does the rest), any change to the handbrake (it passes
through), and anything on a vehicle the mod does not handle (cops, traffic).

## 2. Inputs and outputs

Called once per tick for the player's vehicle. The tyre state comes from the **previous** tick (the controls stage runs before the
tyre stage); the mod owns the tyre hook, so it has these values itself.

| Input | Unit | Meaning |
|---|---|---|
| `dt` | s | the tick |
| `throttle`, `brake`, `handbrake` | 0..1 | raw pedals |
| `v` | m/s | body forward speed, `+` = forwards; the aids use `abs(v)` |
| `kappa[4]` | 1 | slip ratio per wheel: `(wheel surface speed - ground speed) / ground speed`; `> 0` spinning, `< 0` locking |
| `alpha[4]` | rad | slip angle per wheel: `atan2(lateral patch velocity, abs(longitudinal patch velocity))`, `+` = the patch slides to the right (+x). **Radians**: the mod computes it from the patch velocities of its own tyre calls; the host's `slip_angle` result is in turns, so a value read from there must be multiplied by `2 pi` |
| `driven[4]` | bool | which wheels receive drive torque |

Constants of the vehicle (read once from the `spec`): `mass` (kg), `track_f` front track (m), `w_static[4]` static wheel loads (N),
`beta_ref` = front share of the host's maximum brake torque (`T_f / (T_f + T_r)`).

| Output | Unit | Meaning |
|---|---|---|
| `throttle_out` | 0..1 | replaces the pedal the host feeds to the engine |
| `brake_out[4]` | 0..1.5 | per-wheel brake pedal equivalent: `1` is the maximum brake torque of **that** wheel (what the host calls `brake` on a tyre) |
| `handbrake_out` | 0..1 | unchanged |
| flags `tc_active`, `abs_active[4]`, `esc_active` | bool | for the HUD and sounds |

**Contract notes against ADR 0016.** The ADR returns "filtered pedals, brake bias, steering". ABS and ESC act on single wheels, so
the brake output is **per wheel**, and the bias is folded into it (a per-axle bias alone cannot express either). The host must
then not apply its own brake-balance slider on top (the slider is `0` while the mod is active). The reference stability control
is a yaw torque on the body, which the hook cannot apply; this spec gets the same effect from the brakes (section 4.4) and notes
an optional torque output there.

## 3. State

| State | Initial | Reset when |
|---|---|---|
| `tc_acc` (s), `tc_cut` (bool) | 0, false | level changes, `abs(v) < v_tc`, preset change, teleport |
| `abs_acc` (s), `abs_rel[4]` (bool) | 0, all false | level changes, `abs(v) < v_abs`, preset change |
| `esc_y` (0..1), `esc_wheel` (0 or 1) | 0, 0 | level changes to 0, preset change |
| levels `L_tc`, `L_abs`, `L_esc`, bias `clicks` | from the menu (default 0, 0, 0, 0) | by the player |

## 4. Equations

Order inside one call: traction control, brake bias, ABS, stability control, then the clamp.

### 4.1 Traction control

Level `L_tc` in `0..5`. A higher level intervenes earlier (lower slip limit) and cuts deeper (more authority).

```
if L_tc == 0 or abs(v) < v_tc: tc_cut = false; tc_acc = 0; return throttle
tc_acc += dt
if tc_acc + 1e-6 >= T_chk:
    tc_acc = 0
    s      = max(kappa[i] for driven wheels i)
    tc_cut = s > kappa_tc[L_tc]
throttle_out = throttle * (1 - (tc_cut ? A_tc[L_tc] : 0))
```

The decision is taken every `T_chk` (20 Hz, which is 3 ticks at 60 Hz and 7 at 144 Hz) and held until the next check, so a
cut lasts at least one period. `tc_active = tc_cut`.

### 4.2 Brake bias

The front share of the brake torque is `beta = clamp(beta_base + bias_step * clicks, beta_min, beta_max)`. With `beta == beta_ref`
the output equals the host's own split.

```
front wheels: b = brake * beta / beta_ref
rear wheels:  b = brake * (1 - beta) / (1 - beta_ref)
```

### 4.3 ABS

Level `L_abs` in `0..5`. A wheel is **locking** when `kappa[i] < -kappa_abs[L_abs]`. Authority `A_abs` is the share of that wheel's
brake removed while it is locking. Channels: 4 (each wheel alone) for every level of this spec.

```
if L_abs == 0 or abs(v) < v_abs: abs_acc = 0; abs_rel[all] = false; return b
abs_acc += dt
if abs_acc + 1e-6 >= T_chk:
    abs_acc = 0
    abs_rel[i] = kappa[i] < -kappa_abs[L_abs]          # for each wheel
b[i] = b[i] * (1 - (abs_rel[i] ? A_abs[L_abs] : 0))
```

Same hold rule as TC: a release lasts at least one period, so the brake pulses at about 10 Hz in a sustained lock-up.
`abs_active[i] = abs_rel[i]`.

### 4.4 Stability control (oversteer)

Level `L_esc` in `0..4`, gain `g = gain_esc[L_esc]`. It brakes the **front wheel on the side the tail slides towards**, which
yaws the car back, the way a production ESC does. The size of the demand is a yaw moment `M`:

```
fn esc_target() -> target:                      # 0..1, also sets esc_wheel when it acts
    if L_esc == 0 or abs(v) < v_esc: return 0
    a_front = 0.5 * (alpha[0] + alpha[1])           # mean slip angle of the front axle
    a_rear  = 0.5 * (alpha[2] + alpha[3])
    delta   = abs(a_rear) - abs(a_front)        # rad; > 0 when the rear axle slides more than the front
    if delta <= 0: return 0                     # understeer or neutral: no action
    M          = mass * delta * g * k_esc       # N m, k_esc = 30
    esc_wheel  = (a_rear > 0) ? 1 : 0           # FR when the tail slides right, FL when left
    Fx         = M / (0.5 * track_f)            # N, braking force at the arm track_f/2
    return min(1, Fx / (mu_ref * w_static[esc_wheel]))

esc_y += (esc_target() - esc_y) * (1 - exp(-dt / tau_esc))    # first-order lag, every tick
if esc_y > 0 and L_esc > 0 and not abs_rel[esc_wheel]:
    b[esc_wheel] = max(b[esc_wheel], esc_y)
esc_active = esc_y > 0.02
```

`esc_wheel` is kept between calls (it is state): while the demand fades after the slide has ended, the brake stays on the wheel that
caused it.

The brake demand is independent of the brake pedal (the aid brakes with the pedal released) and is skipped on a wheel that ABS is
releasing. `M` (mass times the axle slip-angle difference times gain times 30) is the size of the yaw moment a production-style stability
control would apply to the body; here it only sizes the brake demand. **Optional output:** if the host later offers a yaw torque input to the hook, send `M` with the sign of
`a_rear` (`+` = yaw right) and set `target = 0`. This spec does not require it.

### 4.5 Clamp

`brake_out[i] = clamp(b[i], 0, 1.5)`, `throttle_out = clamp(throttle_out, 0, 1)`.

## 5. Constants

All level tables are **design values** except where marked *reference*; the ordering (level up, intervene earlier and harder) is
the part that is meant.

| Symbol | Value | Unit | Reason |
|---|---|---|---|
| `T_chk` | 0.05 | s | *reference*: 20 Hz check period |
| `v_tc` | 1.0 | m/s | *reference*: no traction control below 1 m/s (the slip ratio is meaningless there) |
| `v_abs` | 5.556 (20 km/h) | m/s | *reference* |
| `v_esc` | 1.389 (5 km/h) | m/s | *reference* |
| `kappa_tc[1..5]` | 0.20, 0.15, 0.12, 0.09, 0.06 | 1 | design; 0.20 is the *reference* default limit; the tyre peaks at about 0.10 to 0.15 |
| `A_tc[1..5]` | 0.50, 0.65, 0.80, 0.90, 1.00 | share of throttle removed | design; level 5 is the full cut of the reference |
| `kappa_abs[1..5]` | 0.30, 0.25, 0.20, 0.15, 0.10 | 1 | design; 0.20 is the *reference* default limit |
| `A_abs[1..5]` | 0.70, 0.80, 0.90, 1.00, 1.00 | share of brake removed | design; 1.00 is the full release of the reference |
| `gain_esc[0..4]` | 0, 0.25, 0.50, 0.75, 1.00 | 1 | the reference's user gain 0..1 in four steps |
| `k_esc` | 30 | N m per (kg rad) | *reference* |
| `tau_esc` | 0.04 | s | design: the brake cannot build instantly |
| `mu_ref` | 1.0 | 1 | design: the brake force a front wheel can add, as a multiple of its static load |
| `beta_min`, `beta_max` | 0.30, 0.80 | 1 | design range of the bias |
| `bias_step` | 0.005 | per click | design; the menu offers -20..+20 clicks |
| `beta_base` | `beta_ref` | 1 | default: the host's own split |
| brake clamp | 1.5 | pedal | the host's own bias slider already produces up to 1.5 |
| `1e-6` | | s | slack so that three ticks of `1/60` reach `0.05` |

## 6. Test vectors

Generated by a script that implements only section 4 (float64). Tolerance **1e-4 absolute**. `dt = 1/60`, `driven = [F,F,T,T]`,
`mass = 1400`, `track_f = 1.52`, `w_static = [3740, 3740, 3000, 3000]`, `beta_base = beta_ref = 0.58` unless stated. Checks happen on
ticks 3, 6, 9, 12, 15 (the third tick after the previous check).

**TC1.** Level 3 (limit 0.12, authority 0.8), `v = 10`, `throttle = 1`, `brake = 0`, `kappa = [0,0,s,s-0.01]` with `s` per tick:
ticks 1..5 `s = 0.05`; ticks 6..11 `s = 0.20`; ticks 12..15 `s = 0.05`.

| tick | 1-5 | 6-11 | 12-15 |
|---|---|---|---|
| `throttle_out` | 1.0 | 0.2 | 1.0 |
| `tc_cut` | false | true | false |

(The cut starts at the check of tick 6 and ends at the check of tick 12, although the slip returns at tick 12.)
`tc_acc` after ticks 1..4 is 0.0167, 0.0333, 0.0, 0.0167.

**TC2.** Same slip pattern at level 5: `throttle_out` is 0.0 on ticks 6..11, 1.0 elsewhere.
**TC3.** Level 1, `kappa_driven = 0.18`, `throttle = 0.8`, 6 ticks: `throttle_out = 0.8`, no cut.
**TC4.** Level 5, `kappa_driven = 0.5`, `v = 0.5` (below `v_tc`): `throttle_out = 1.0`. **TC5.** Level 0, `kappa = 0.5`: 1.0.

**ABS1.** Level 3 (lock below -0.20, authority 0.9), `v = 20`, `brake = 1`, `throttle = 0`; front-left slip ratio per tick:
ticks 1-2 `-0.05`, ticks 3-6 `-0.30`, ticks 7-12 `-0.05`; the other wheels `-0.05`.

| tick | 1-2 | 3-8 | 9-12 |
|---|---|---|---|
| `brake_out[0]` | 1.0 | 0.1 | 1.0 |
| `abs_rel[0]` | false | true | false |
| `brake_out[1..3]` | 1.0 | 1.0 | 1.0 |

**ABS2.** Level 2 (limit 0.25), all slip ratios `-0.22`, `brake = 1`: `brake_out = [1,1,1,1]` (not locking).
**ABS3.** Level 1 (limit 0.30, authority 0.7), `brake = 0.8`, `kappa = [-0.35,-0.05,-0.35,-0.05]`, 6 ticks:
`brake_out = [0.24, 0.8, 0.24, 0.8]`. **ABS4.** Level 5, `v = 5` (below 20 km/h), `kappa = -0.5` everywhere: `[1,1,1,1]`.

**BIAS.** `brake = 1`, `v = 20`, no aid:

| clicks | `brake_out` (front, front, rear, rear) |
|---|---|
| 0 | 1.0000, 1.0000, 1.0000, 1.0000 |
| +10 (`beta = 0.63`) | 1.0862, 1.0862, 0.8810, 0.8810 |
| -10 (`beta = 0.53`) | 0.9138, 0.9138, 1.1190, 1.1190 |
| +100 (`beta` clamps to 0.80) | 1.3793, 1.3793, 0.4762, 0.4762 |
| -100 (`beta` clamps to 0.30) | 0.5172, 0.5172, 1.5000, 1.5000 (the rear exceeds 1.5 before the clamp: 1.6667) |

`beta_base = 0.65`, `beta_ref = 0.55`, `brake = 0.6`: `[0.7091, 0.7091, 0.4667, 0.4667]`.

**ESC1.** Level 4, `v = 25`, `throttle = 0.5`, `brake = 0`, `alpha = [0.02, 0.02, 0.10, 0.10]` (tail slides right). `delta = 0.08`,
`M = 3360`, `Fx = 4421`, `target = 1` (saturated). `brake_out[1]` per tick 1..6: 0.3408, 0.5654, 0.7135, 0.8111, 0.8755, 0.9179; the other
wheels 0; `esc_active` from tick 1.
**ESC2.** Level 2, `alpha = [-0.01,-0.01,-0.06,-0.06]` (tail slides left), 3 ticks: `brake_out = [0.2636, 0, 0, 0]` (`delta = 0.05`,
`M = 1050`, `target = 0.3694`).
**ESC3.** Level 4, `alpha = [0.10,0.10,0.02,0.02]` (understeer): all brakes 0, `esc_y = 0`. **ESC4.** Level 4, `v = 1.0` m/s (below 5 km/h): all 0.
**ESC5.** After the 6 ticks of ESC1, set all slip angles to 0 (and `throttle = 0.5`): `esc_y` on the next 6 ticks: 0.6051, 0.3989, 0.2630, 0.1734, 0.1143, 0.0754, all on `brake_out[1]` (the wheel that was braked).
**ESC6.** Level 4 with ABS level 3, `v = 25`, `brake = 1`, `kappa = [-0.05,-0.30,-0.05,-0.05]`, `alpha` as ESC1, 6 ticks: front-right is
locking, so ESC does not touch it: `brake_out = [1.0, 0.1, 1.0, 1.0]`, `abs_rel[1] = true`, `esc_y = 0.9179`.

## 7. Limits

- The aids see the **previous** tick's slip, one tick late. At 60 Hz plus a 50 ms check period the ABS and TC reaction is 50 to 67 ms,
  like the reference, but they cannot do better.
- TC only cuts the throttle pedal; it does not retard the ignition or brake a spinning wheel, so a car with a very low gear or a
  locked differential may keep spinning one wheel.
- ABS releases a share of the brake, not a controlled slip target; there is no pressure model, no hydraulic lag and no pump noise.
- Stability control acts only against oversteer and only through one braked front wheel. It does not handle understeer, does not
  cut the throttle and is limited by the tyre force the braked wheel can add (`mu_ref`); the reference's yaw torque has no such limit,
  so at high gain this is weaker than the reference.
- No aid distinguishes a surface (gravel, wet, ice): thresholds are fixed. On a loose surface the slip ratios are measured with the
  same tyre model, which already shifts the peak.
- Brake bias is fixed by the menu; there is no electronic brake-force distribution that follows the load.
- Cars the aids should not touch (cops, traffic, a vehicle without `spec`) are `not handled` and keep the built-in controls.
