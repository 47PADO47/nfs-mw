# Steering: ratio, lock, speed-sensitive lock, rate limit, force-feedback signal
Sources: Gillespie, "Fundamentals of Vehicle Dynamics" (SAE, 1992), steering system and aligning torque chapters, public literature, understanding (steering ratio, column torque from tyre aligning torque)
Sources: Pacejka, "Tyre and Vehicle Dynamics" (Butterworth-Heinemann, 3rd ed. 2012), public literature, understanding (self-aligning torque, pneumatic trail)
Sources: Milliken and Milliken, "Race Car Vehicle Dynamics" (SAE, 1995), public literature, understanding (steering ratio and lock, caster and steering feel)
Sources: rustyAC, https://github.com/thesupersupersigma/rustyAC, `crates/rustyac-physics/src/car/chassis.rs` (steering and force-feedback parts), GPL-3.0-or-later (per its LICENSING.md; the repo also carries MIT/Apache files for unrelated parts), understanding of behaviour and facts (road wheel angle is steering lock times input divided by ratio; force feedback is the sum of the front steering torques, low-pass filtered, plus a term from steering rate times front wheel speed times wheel inertia, shaped by a power-law curve and reduced at low speed); no code, comments or structure taken
Sources: this repository, `libs/blackbox-vehicle/src/steering/` (the host's steering shaping today), MIT OR Apache-2.0, facts
Written by: spec author session (Claude, stage 0 part 2), 2026-11-20   Implemented by: not yet

Part of [the AC handling specs](README.md), [ADR 0016](../../decisions/0016-handling-hooks.md). Implements the steering stage of the
`controls` hook (`bb.handling` 1.2); the aids are in [aids.md](aids.md). Axes: x right, y up, z forward; `+` steering is to the
right; yaw `+` is to the right (rotation about +y).

## 1. Purpose and scope

Turn the player's steering input into the target angle of the front wheels, with a lock and ratio that describe the car, a
speed-sensitive lock and a rate limit for pad and keyboard, and (optionally) turn the tyres' self-aligning torque into a
normalised force-feedback value.

**Whether the game has a force-feedback output is not known** (plan, open items; checked in stage 5). The force-feedback signal
below is specified completely so it can be built and tested, but it is an **optional output**: the mod computes it, the host may
ignore it, and nothing else depends on it.

| In scope | Out of scope |
|---|---|
| steering wheel lock and ratio, giving the road-wheel angle | the Ackermann split into left and right angles (the host keeps it) |
| speed-sensitive lock, input shaping, rate limit and return for pad and keyboard | collision-limited steering after a hit (the host keeps it), AI steering |
| counter-steer allowance when the rear slides | rear-wheel steering, steer-by-wire, power-steering assist maps |
| force-feedback signal from aligning torque, gyro term, filter, curve, low-speed reduction | flat-spot vibration, kerb rumble, a rumble motor, wheel-device friction and damping settings |

## 2. Inputs and outputs

One call per tick, for the player's vehicle. `device` is chosen in the menu: `wheel` (a steering wheel with a one-to-one
mapping, the device's own filtering applies) or `pad` (gamepad stick or keyboard).

| Input | Unit | Meaning |
|---|---|---|
| `dt` | s | the tick |
| `steer_in` | -1..1 | raw steering input, `+` = right |
| `v` | m/s | body forward speed (`abs(v)` is used) |
| `alpha_rear` | rad | mean slip angle of the two rear tyres, `+` = the patches slide to the right (from the mod's tyre state, in **radians**: the host's `slip_angle` is in turns, multiply by `2 pi`) |
| `Mz[2]` | N m | self-aligning torque of FL and FR (`aligning_torque_nm` of the tyre result), last tick; **sign: `+` acts on the wheel to turn it to the right (to increase the steering angle)**. A tyre sliding to the right in a left turn produces `+` |
| `omega[2]` | rad/s | spin of FL and FR (for the gyro term) |

| Output | Unit | Meaning |
|---|---|---|
| `steer_target` | rad | target angle of the inside front wheel, `+` = right; the same quantity as the host's steering update returns, which the host splits with its Ackermann function |
| `ffb` | -1..1 | optional force-feedback value; `+` = the wheel is pulled to the right |
| `col_torque` | N m | optional, the column torque behind `ffb` (logging, a HUD gauge) |

**No steering lock in the host's data.** `BbVehicleSpec` has no steering lock, ratio or angle, so `theta_max` and `delta_max` are
**preset values** (section 5 defaults, overridable per car in the car file), not read from the car; the target angle the host
actually reaches is limited by its own lock.

**Contract notes against ADR 0016.** The ADR lists "steering" as an output without a unit. This spec fixes it as `steer_target`
in radians and requires the host to **bypass its own steering shaping** (the speed-dependent range, rate and counter-steer window
of `steering/human.rs`) for a vehicle whose `controls` hook answered, otherwise the lock is shaped twice. The host keeps Ackermann and
the post-collision limit. Force feedback needs two things the ADR does not list: `Mz` with the sign above from the tyre hook and
a place to send `ffb`, if the game has one.

## 3. State

| State | Initial | Reset when |
|---|---|---|
| `delta` current target angle (rad) | 0 | preset change, teleport, device change |
| `delta_prev` previous tick's `delta` (rad) | 0 | same |
| `y_mz` filtered column torque (N m) | 0 | same |

## 4. Equations

### 4.1 Lock and ratio

`theta_max` is the steering wheel rotation from centre to one side (rad); `delta_max` is the road-wheel angle at that lock (rad).
The steering ratio is `R = theta_max / delta_max` (steering wheel angle per road-wheel angle). A full input gives `delta_max`:
`delta = delta_max * steer_in` before shaping. A wheel device reports the real wheel angle, so for `wheel` the lock and ratio
are the only mapping.

### 4.2 One tick of the steering stage

```
u0 = clamp(steer_in, -1, 1)
gamma_in = (device == pad) ? gamma : 1
u  = sign(u0) * |u0|^gamma_in                          # input shaping (soft around the centre for analog sticks)

f     = (device == pad) ? lock_factor(|v|) : 1
lim   = f * delta_max
if sign(u) != 0 and sign(u) == sign(alpha_rear):       # steering into a slide
    lim = max(lim, min(delta_max, c_cs * |alpha_rear|))
target = u * lim

delta_prev = delta
if device == wheel:
    if tau_f > 0: delta += (target - delta) * (1 - exp(-dt / tau_f))
    else:         delta  = target
    return clamp(delta, -delta_max, delta_max)
r_out = rate_out(|v|)
r     = ((target - delta) * delta < 0) ? rho_ret * r_out : r_out     # toward the centre is faster
delta += clamp(target - delta, -r * dt, +r * dt)
return clamp(delta, -delta_max, delta_max)
```

with

```
lock_factor(v) = 1 - s_sens * (1 - f_min) * v^2 / (v^2 + v0^2)     # 1 at standstill, tends to 1 - s_sens*(1 - f_min) at high speed; s_sens in 0..1
rate_out(v)    = r_lo + (r_hi - r_lo) / (1 + (v / v_r)^2)          # rad/s, away from the centre
```

`s_sens` is the "speed sensitivity" slider (0 = full lock at any speed, 1 = the table above). For `wheel` devices the lock factor and
the rate limit are off, the human's hands do both. The return rate also applies when the lock shrinks with speed (the target is
closer to the centre than the current angle), so the angle is pulled in at `rho_ret * r_out`.

### 4.3 Force-feedback signal (optional output)

Called after 4.2 in the same tick, with `delta_dot = (delta - delta_prev) / dt`:

```
col      = (Mz[0] + Mz[1]) / R                                  # N m at the steering wheel
y_mz    += (col - y_mz) * (1 - exp(-dt / tau_ff))               # low-pass on the tyre torque
gyro     = -g_gyro * J_w * (|omega[0]| + |omega[1]|) * delta_dot / R    # resists steering motion
tau      = y_mz + gyro
k_ls     = m_ls + (1 - m_ls) * clamp(|v| / v_ls, 0, 1)          # lighter at parking speed
x        = tau * k_ls * g_ffb / tau_ref
ffb      = sign(x) * min(1, |x|)^p_curve
col_torque = tau
```

`ffb > 0` pulls the wheel to the right. In a steady left turn the tyres' `Mz` is `+` (restoring), so `ffb` is `+`: the wheel
tries to centre itself. As the front tyres slide past their peak `Mz` falls and the wheel goes light, which is the understeer
cue; no separate "fake understeer" term is added. `p_curve < 1` raises small forces.

## 5. Constants

| Symbol | Default | Unit | Range | Reason |
|---|---|---|---|---|
| `theta_max` | 450 deg (7.854 rad) | rad | 180..720 deg | design: 900 degrees lock to lock; per-car in the preset |
| `delta_max` | 30 deg (0.5236 rad) | rad | 20..45 deg | design: typical road car; per-car (the host's absolute limit is 45 deg) |
| `R` | 15 (derived) | 1 | 10..25 | from the two above |
| `s_sens` (pad) | 1.0 | 1 | 0..1 | menu slider; wheel devices ignore it |
| `f_min` | 0.20 | 1 | 0.1..0.5 | design: pad lock at very high speed is a fifth of full |
| `v0` | 22 | m/s | 10..40 | design: the lock is 60 % of full at 79 km/h with `s_sens = 1` |
| `gamma` (pad) | 1.3 | 1 | 1..2 | design: only matters for analog sticks |
| `r_hi`, `r_lo` | 3.0, 0.9 | rad/s | 0.5..6 | design: the stick turns the wheels at 170 deg/s standing and 52 deg/s fast |
| `v_r` | 15 | m/s | 5..40 | design |
| `rho_ret` | 1.6 | 1 | 1..3 | design: returning to centre is faster than steering away |
| `c_cs` | 1.0 | 1 | 0..2 | design: steering into a slide may reach the rear slip angle (the same idea as the host's window) |
| `tau_f` (wheel) | 0 | s | 0..0.1 | design: off; a wheel's own filter is enough |
| `tau_ref` | 12 | N m | 3..40 | design: column torque of a full-scale force |
| `tau_ff` | 0.02 | s | 0.005..0.1 | design |
| `g_gyro` | 0.2 | 1 | 0..1 | design: the gyro term is a hint, not the main force |
| `J_w` | 1.6 | kg m^2 | 0.5..4 | design: typical road wheel; use the tyre hook's inertia when present |
| `v_ls` | 2.8 (10 km/h) | m/s | 0..8 | design |
| `m_ls` | 0.2 | 1 | 0..1 | design |
| `g_ffb` | 1.0 | 1 | 0..3 | menu: force-feedback gain |
| `p_curve` | 1.0 | 1 | 0.4..1.5 | menu: force-feedback curve |

## 6. Test vectors

Generated by a script implementing only section 4 (float64). Tolerance **1e-4 absolute** (angles in rad, `ffb` unitless, torques in
N m). Defaults of section 5, `dt = 1/60`, `device = pad` unless stated; `alpha_rear = 0` unless stated.

**S1. Lock factor and rate** (pad):

| `v` (m/s) | `lock_factor` | limit (deg) | `rate_out` (rad/s) |
|---|---|---|---|
| 0 | 1.00000 | 30.000 | 3.0000 |
| 10 | 0.86301 | 25.890 | 2.3538 |
| 22 | 0.60000 | 18.000 | 1.5664 |
| 40 | 0.38580 | 11.574 | 1.1589 |
| 80 | 0.25625 | 7.687 | 0.9713 |

**S2.** `v = 0`, `steer_in = 1`, from rest: `delta` after ticks 1, 2, 5, 10, 11, 12 = 0.05, 0.10, 0.25, 0.50, 0.5236, 0.5236 (the rate limit,
then `delta_max`).
**S3.** `v = 30`, `steer_in = 1`: ticks 1, 2, 10 = 0.0220, 0.0440, 0.2200 (the limit at 30 m/s is 0.2512).
**S4 (return).** `delta = 0.4`, `steer_in = 0`, `v = 10`, 5 ticks: 0.08615 (return rate 3.7662 rad/s).
**S5 (cross).** `delta = +0.2`, `steer_in = -1`, `v = 10`, one tick: 0.13723 (it first returns toward the centre at the fast rate).
**S6 (lock shrinks).** `delta = 0.45`, `steer_in = 1`, `v = 40`, one tick: 0.4191 (target 0.2020, pulled in by 0.0309 per tick).
**S7 (shaping).** `steer_in = 0.5`, `v = 0`, 60 ticks: 0.21265 (`0.5^1.3 * delta_max`).
**S8 (counter-steer).** `v = 40`, `steer_in = 1`, 60 ticks: `alpha_rear = +0.30`: `delta = 0.3000` (the limit becomes 0.30, above the 0.2020 of
the speed lock); `alpha_rear = -0.30` (wrong side): `delta = 0.2020`.
**S9 (wheel).** `device = wheel`, `v = 40`, `steer_in = 0.5`: 0.2618 (no speed lock); `steer_in = 1.7`: 0.5236 (input clamp).
**S10 (wheel filter).** `device = wheel`, `tau_f = 0.05`, `steer_in = 1` from rest, ticks 1, 2, 3: 0.14842, 0.25477, 0.33098.

**F1. Force feedback ramp.** `device = wheel`, state zero, `v = 20`, `Mz = [40, 40]`, `omega = [60, 60]`, `delta` constant:
`ffb` per tick 1..5 = 0.25129, 0.36050, 0.40796, 0.42859, 0.43755 (`y_mz` = 3.0155, 4.3260, 4.8955, 5.1431, 5.2506).
**F2. Steady state.** `y_mz = 80/15`, same inputs: `ffb = 0.44444`, `col_torque = 5.33333`. With `Mz = [-40,-40]` and `y_mz = -80/15`: `-0.44444`.
**F3. Clip.** `y_mz = 100`, `Mz = [300, 300]`: `ffb = 1.0`. **F4. Low speed.** steady as F2 with `v = 1.4`: `ffb = 0.26667` (`k_ls = 0.6`).
**F5. Gyro.** `Mz = 0`, `y_mz = 0`, `delta - delta_prev = 0.5 * dt` (road wheels turning right at 0.5 rad/s), `omega = [60,60]`, `v = 20`:
`col_torque = -1.28`, `ffb = -0.10667`. **F6. Curve.** F2 with `p_curve = 0.5`: `ffb = 0.66667`.

Properties: `ffb` is odd in `(Mz, delta_dot)`; `|ffb| <= 1`; with `Mz = 0` and `delta_dot = 0` the output decays to 0; with `s_sens = 0`
the lock is `delta_max` at every speed; the pad angle never changes by more than `rho_ret * r_hi * dt` per tick.

## 7. Limits

- Pad and keyboard only get the speed-sensitive lock and rate; a wheel device is trusted. A keyboard (digital) input reaches full
  lock at the rate limit, which at low speed is fast (0.17 s from centre to lock at standstill).
- `steer_target` is one angle for the inside wheel; Ackermann, toe and the host's steering-geometry effects are the host's.
- Force feedback assumes the tyre hook reports `Mz` about the steering axis at the wheel. Caster and kingpin trail, power-steering
  assist and column friction are not modelled, so the feel depends on the tyre's pneumatic trail alone.
- Nothing here is specific to a force-feedback standard; scaling to a device's torque range is the host's job. If the game has no
  force-feedback output the signal is computed and dropped.
- Speed sensitivity uses speed only; it does not look at steering rate, yaw rate or understeer.
- Counter-steer help only widens the limit; it never steers for the player.
- The constants are design values; none has been measured against a reference game. They are tuned in stage 4 on the owner's feel.
