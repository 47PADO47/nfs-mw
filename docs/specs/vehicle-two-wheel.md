# Two-wheel vehicle dynamics (`blackbox-vehicle::bike`)

The model of a motorcycle: a rigid body with a rider's mass on a fork and a swingarm, two round tyres, an engine with a clutch
and an automatic gearbox, and a rider who prescribes the lean and steers the front tyre to the force that lean needs. The
code is `libs/blackbox-vehicle/src/bike/`; it is generic (no game names). The numbers of a real bike reach it through
`BikeSpec`, filled by a mod importer ([bikes.md](../modding/bikes.md)). The behaviour follows the clean-room specs written from
the RIDE 6 reference ([ride6-lean-controller.md](ride6-lean-controller.md), [ride6-tyre.md](ride6-tyre.md),
[ride6-suspension.md](ride6-suspension.md), [ride6-engine-drivetrain.md](ride6-engine-drivetrain.md),
[ride6-rider-help.md](ride6-rider-help.md), [ride6-rider-body-aero.md](ride6-rider-body-aero.md)); the mapping of its
fields to `BikeSpec` is in [vehicle-two-wheel-parameters.md](vehicle-two-wheel-parameters.md). Nothing is decompiled code
([licensing.md](../licensing.md)). Provenance: [ride6-physics.md](../provenance/ride6-physics.md).

## Sources

| What | Source |
|---|---|
| Tyre forces | The Magic Formula 94 of Pacejka, *Tyre and Vehicle Dynamics*, appendix 3 (lateral `a0..a13`, longitudinal `b0..b10`, aligning `c0..c17`; loads in kN, angles in degrees, slip in percent) |
| Equations of the controllers, suspension, clutch, brakes | The clean-room specs above, written in our own words from the reference game's data and behaviour |
| Anything else | A named choice in `choices.rs` (table at the end) |

## The body (M1, M2)

One `RigidBody` carries bike, wheels and rider. Physics space is x right, y up, z forward; the origin is on the ground midway
between the contact patches at rest.

- **Mass**: `total = chassis.mass + chassis.rider_mass`. The bike's mass already includes the wheels (the sprung mass is the
  total less both wheel masses).
- **Inertia**: `I_axis = tensor_scale_axis * total` (kg m^2), `tensor_scale` being the radius of gyration squared about x
  (pitch), y (yaw) and z (roll). It is handed to the body as a scale of its box inertia.
- **Centre of gravity**: `cg = (M cg_bike + m_r p_rider) / (M + m_r)` with `cg_bike = (0, cg_height, wheelbase (front_weight
  - 0.5))` and `p_rider = cg_bike + rider_offset + shift(weight)`. The weight input (`Bike::set_weight`, -1 back to 1 forward,
  rate-limited by `rider_weight_rate`, the filters of the data pass their input) shifts the rider by `rider_forward_shift` or
  `rider_backward_shift`. The world position of the centre of gravity stays put when the rider moves (the frame moves the other
  way). A host that keeps a second body in step with the bike should take `body().cog()` again.
- The static front share of the weight is `BikeSpec::front_share()`.

## The front end and the rear end

Geometry as before (`geometry.rs`): the steering axis tilts back by the rake and meets the ground one `trail` ahead of the
patch; the fork compresses along the axis, the swingarm along the body's y axis; the tyre is a torus
(`R - r` ring, tube `r = toroid_factor width / 2`), so a leaned bike stands on the shoulder of the tread.

### Suspension (M4, `suspension.rs`, `wheel_step.rs`)

`x` is the wheel travel from full extension. With the leverage ratio `LR(x / travel) >= 1` (wheel travel per shock travel)
and the shock compression `x_s = x / LR`:

```
g(x; k, r, f, p) = k (x + f sgn(x) 100^(p-1) max(|x| - r travel, 0)^p)      # the law of the cm units, in SI
F_spring  = g(x_s + rest_offset; spring, progressive)
F_hyd     = g(x_s; hydraulic.stiffness, hydraulic.term)
F_topout  = top_out_stiffness max(top_out_length - x, 0)
F_wheel   = (F_spring + F_hyd) / LR - F_topout + F_damper + F_coupling
F_damper  = c(v_s) / LR,  v_s = v / LR,  c: slow slope up to the knee speed, fast slope beyond (bump and rebound apart)
```

The wheel has a mass and is integrated (trapezoid, implicit damper) while free; the bump stop and the top-out end of the
travel clamp it. On the ground the penetration is shared between the suspension and the tyre in series: with `reach` the
compression that would put a rigid tyre on the ground, `share = min(1, (spring / LR) / tire_stiffness)`,
`tyre = min(share reach, max_tire_deformation)` and `x = max(reach - tyre, x_free)`. The load on the tyre is
`(F_wheel + m_w g_axis + m_w x'')/cos`, plus `tire_stiffness x overload` when the travel ran out. The rate at which the
frame approaches the ground gives the compression speed. **Coupling forces** (all resist compression when positive, scaled
by a remap of `x / travel`): chain pull `-100 f_chain T_wheel`, traction `-f_traction Fx` and brake `sign f_brake 0.01
T_brake / R`.

## The tyre (M5, `tyre.rs`, `peaks.rs`)

- Effective radius `R - 0.5 (1 - |cos camber|) toroid width - shape_factor squash`.
- Slip speed over the **total** patch speed `V`; below 1 m/s the smoothstep `k = (3 - 2u) u`, `u = 0.99 V + 0.01`, multiplies
  the slip speeds. `Slip.y` is the sine of the slip angle.
- Peak slips of the pure forces (negative and positive side) come from the Magic Formula at the present load and camber
  (scan and refinement, not a table). They normalise the slips: `n = 2 (s - s-) / (s+ - s-) - 1`.
- Combination: `c = |n| / (|nx| slip_correction + 1)`, effective slips along the same direction, `Fx = Fx0 min(1, |nx|/c)`,
  `Fy = Fy0 min(1, |ny|/c)`; the slip correction blends from its rest value to its speed value over the rest speed. Efficiency
  factors and the aligning moment (front only, reported only) as in the tyre spec.
- Spin: `(I + I_extra) dw/dt = drive - brake - r_eff Fx(w)` solved for the new spin by bisection (`I_extra` the engine's
  inertia through a locked clutch). Rolling resistance `Fz (c0 + c1 v + c2 v^2)`.

## Engine, clutch, gearbox (M6, M7, `powertrain.rs`)

`T = T_closed + (T_max - T_closed) throttle`, `T_closed = -T_fric(rpm/max)`, `T_max = peak curve(rpm)`; the throttle is held
at `throttle_at_idle` below the idle and cut on and off by the limiter (`limiter_cut_time`) above the rev limit. The engine has a
speed of its own and the inertia `moi`. The clutch capacity is `engagement x clutch_max_factor x peak torque`; engaged
fully it locks (the engine follows the wheel, its inertia added to the wheel's), otherwise it slips with
`sgn(w_e - w_w) capacity` on the engine and `x ratio` on the wheel, locking when the speeds cross. Engagement is the lever
(open for a gear change, back over `CLUTCH_ENGAGE_TIME`), the launch (engine speed towards `min(feather x peak-torque rpm,
feather_max x max)`, or road speed over `release_speed`) and the slipper (back torque over `slipper_range`). The gearbox
changes up under throttle above `up_shift x max_rpm` of coupled engine speed and down below `down_shift x max_rpm`; a
change lasts `up_shift_time` / `down_shift_time` with the clutch open and a throttle blip when changing down.

## Brakes (M8, `brakes.rs`)

Joined brakes: `rear = max(rear, front joined_factor)` while moving (or the shares of the larger lever). Torque at the wheel
`lever x ABS multiplier x force x radius` with `front_force` for the front brake and `rear_force` for the rear (the reference
game's brake balance puts the whole brake force on the front wheel and the rear is the hand-brake channel). The ABS releases
each wheel's brake as its normalised longitudinal slip nears the force peak, fading in with speed; it rises at once and falls
at `ABS_FALL_RATE`.

## The rider (M3, M9, M10, `lean.rs`, `handling.rs`, `pitch_assist.rs`)

**Lean controller.** The stick passes the steer filter; the lean limit is `atan(front lateral grip)` (the lateral peak over the
load), blended to the stopped limit below the stopped speed, times the lateral-g tuning, times the skid cut, pulled to the
wheelie or stoppie lean limit with the lift angle, capped by `max_lean`. The target lean is the limit times the filtered stick
through the second filter (step scaled by the time scale of speed and stick). A spring-damper gives the roll acceleration
`a = -K (lean - target) - 2 z sqrt(K) lean'` (`z` = damping ratio times its multiplier; held over a 60 Hz step the damping
term alone is unstable, `2 z sqrt(K) dt` being 3.8, so the model applies the acceleration that carries the spring-damper to its
exact state after the step, `filter::step_acceleration`), the roll torque `tau = I_roll a`
(`I_roll` = body roll inertia + `rider_mass_scale m_r z_r^2`) and the wanted front side force
`F = -tau / max(h_cg, 0.05)` (positive to the right: leaning right first asks a push to the left, which is the counter-steer)
plus the choice term below. The front tyre's lateral curve is inverted at its load and camber (peaks bracketed, bisection)
for the wanted slip angle; the wanted wheel heading is the angle of the front contact velocity less that slip, the handlebar
angle is found from it numerically on the real geometry (rake, trail, lean), blended to `stick x max_angle` between
`blend_range`, and filtered (`response`, `damping_ratio`). **Lean help** adds a side force at the front contact that closes a
share of the gap to the wanted force, within the tyre's lateral peak and the friction ellipse. The roll acceleration is then
**realised** by a torque about the bike's length that makes up what the tyres left, within `ROLL_TORQUE_AUTHORITY` weights
times the height of the centre of gravity; it also holds a bike that stands still.

**Wheelie and stoppie.** The lever is the rider's weight; the dead zones give the position `-1 .. 1`. A pure pitch torque
(about the bike's right axis, no arm) `help x a_cmd x (chassis mass x pitch scale)` holds the lift angle at `max_angle x position x
speed ramp`, `a_cmd` a bang-bang with a square-root approach limited by `acceleration` and smoothed by `acceleration_time`;
`help` falls with the lean, the height of the lifted wheel and (stoppie) the speed curve. The rider help cuts the throttle
as the front lifts (the forced range when a wheelie is asked) and eases the front brake as the rear lifts. The lift angle is
the angle of the line through the hubs from the ground, less its rest value.

**Gyro, drag, fall (M10 to M12).** Roll torque `scale cos(lean) (I_f w_f + I_r w_r) yaw_rate` about the length, yaw torque
`-scale (I_f w_f + I_r w_r) roll_rate` about the vertical. Air drag `F = drag v^2` opposing the velocity. The rider has
fallen when the lean or pitch passes the fall limits, or, above `min_speed`, when the heading error or the angular speed
does; a fallen rider lets go of the throttle and the stick and the controller's torque stops.

## `BikeSpec` fields (SI units; angles in radians)

| Group | Fields |
|---|---|
| `chassis` | `mass` kg (wheels included), `rider_mass` kg, `rider_offset`, `rider_forward_shift`, `rider_backward_shift` m (x right, y up, z forward, from the bike's centre of gravity), `rider_weight_rate` 1/s, `half_dimensions` m, `tensor_scale` m^2 (pitch, yaw, roll), `cg_height` m, `front_weight`, `wheelbase`, `trail` m, `rake` rad |
| `front`, `rear` (wheel) | `radius`, `width` m, `toroid_factor`, `mass` kg, `inertia` kg m^2, `tyre` (`a[15]`, `b[11]`, `c[18]`), `rolling[3]`, `lateral_grip_limit` (informational), `tire_stiffness` N/m (0 rigid), `max_tire_deformation` m, `compression_shape_factor`, `slip_correction`, `rest_slip_correction`, `rest_speed_threshold` m/s |
| `front_suspension`, `rear_suspension` | `max_travel` m, `spring` N/m (at the shock), `top_out_stiffness` N/m, `top_out_length` m, `damper` (six numbers), `leverage[(share, ratio)]`, `rest_offset` m, `progressive{start,factor,linearity}`, `hydraulic{stiffness,term}`, `max_speed` m/s, `coupling{chain,traction,brake and their remaps}` |
| `engine` | `peak_torque` N m, `torque_curve[(rpm, share)]`, `friction_torque` N m, `friction_curve[(rpm/max, factor)]`, `idle_rpm`, `max_rpm`, `moi` kg m^2, `throttle_at_idle`, `limiter_cut_time` s |
| `transmission` | `gears`, `final_ratio`, `efficiency`, `up_shift`, `down_shift` (shares of `max_rpm`), `up_shift_time`, `down_shift_time` s, `blip_throttle`, `clutch_linearity`, `clutch_max_factor` (x peak torque), `slipper_range` N m, `slipper_intensity`, `release_speed` m/s, `feather_factor`, `feather_max_factor` |
| `brakes` | `front_force`, `rear_force` N, `joined_factor`, `joined_front`, `joined_rear`, `abs{front_intensity, rear_intensity, speed_range m/s, front_slip, rear_slip}` |
| `steering` | `max_angle`, `response` 1/s^2, `damping_ratio`, `max_lean`, `lean{k, damping_ratio, damping_multiplier, steer_k, steer_damping_ratio, target_k, target_damping_ratio, time_scale (+ attenuation, sensitivity, speed and input curves), g_tuning, skid_range, skid_linearity, skid_reduction, stopped_limit, stopped_speed, blend_range, rider_mass_scale, help_speed, help_limit}` |
| `fall` | `lean`, `pitch`, `speed_direction` rad, `angular_speed` rad/s, `min_speed` m/s |
| `wheelie` (optional) | `wheelie` and `stoppie` (`deadzone`, `max_angle`, `speed_ramp`, `help_speed`, `lean_help`, `lookahead`, `min_speed`, `acceleration`, `acceleration_time`, `ground_range`, `lean_limit`, `lean_pitch_range`), the throttle and brake ranges of the rider help |
| top level | `gyro_roll_scale`, `gyro_yaw_scale`, `drag` |

## What is still a choice (`choices.rs`)

| Choice | Value | Why it is open |
|---|---|---|
| `DOWN_SHIFT_SHARE`, `UP_SHIFT_MIN_THROTTLE` | 0.45, 0.3 | the automatic shift comparison and the tolerances were not located |
| `CLUTCH_OPEN_DURING_SHIFT`, `CLUTCH_ENGAGE_TIME` | true, 0.06 s | the clutch ramps of a change are not pinned down |
| `LAUNCH_ENGAGEMENT_FROM_ENGINE_SPEED`, `LAUNCH_ENGAGE_START_FACTOR` | true, 1.1 | the feathering controller was not read (its target speed is recovered) |
| `SLIPPER_FALL_RATE`, `ABS_FALL_RATE` | 10, 8 per s | constructor defaults not listed |
| `ROLL_TORQUE_AUTHORITY` | 2 weights x height | the rest of the closure of the prescribed roll (stability help, pairing of the help reductions) |
| `LEAN_FORCE_INCLUDES_TURN_FORCE` | true | the force balance for a held lean was not recovered |
| `BRAKE_COUPLING_SIGN` | -1 | the sign of `T_applied - T_back` for a braking wheel |
| `BUMP_STOP_FACTOR` | 12 | the end of the travel is only a clamp in the reference |
| `TYRE_EFFICIENCY`, `TYRE_EFFICIENCY_BALANCE` | 1, 0.5 | temperature and wear are not integrated |
| `LEAN_AND_PITCH_FALL_AT_ANY_SPEED` | true | the exact fall tests are inferred |
| `RIDER_HELP_RAMPS_ARE_LINEAR` | true | the throttle and brake formulas of the rider help were not read |
| `ROLLING_HAS_SLIP_TERM` | false | the rolling resistance formula of the tyre model was not read |
| `LATERAL_GRIP_TOLERANCE`, `SLIP_CORRECTION_MULTIPLIER`, `STANDING_SPEED` | 1, 1, 0.5 m/s | recovered default, dry road, numerical guard |
| `HANDLEBAR_LEAD` | true (the lead is the filter's slow time constant, 0.18 s for K 1000, ratio 3) | the front wheel follows its own contact velocity through the lagging handlebar filter; with the weight forward (braking) the front tyre is stiffer than the rear one and the lag makes it push the nose round, so a stop spins the bike out. What stabilises the reference was not recovered |

## Not modelled

The auto-rider that moves the weight by itself, the traction control and engine electronics, tyre temperature and wear, the
combined-slip solver (the stock bikes do not use it), the stability help (off for the stock bike), the brake-reduction
cornering protection of the ABS element, the rider's spring-damper (the rider is rigid on the bike), the swingarm's arc, the
block-fall assist, the tuck-in drag reduction, the terrain terms of the lean limit.

## Status and verification

The model and its tests were written without a build; the first run found the unstable roll acceleration above, an arcsine of a combined slip past 1 in the tyre (NaN), and the yaw divergence of a braking bike (`HANDLEBAR_LEAD`), and corrected four tests that measured the wrong frame or the wrong tyre state. Tests: filters,
suspension law (cm law, leverage, top-out, damper knee, coupling, free wheel), tyre (smoothstep, combination, peaks,
spin solve), powertrain (launch slip, lock, shifts, limiter, ramp), brakes (joined, ABS), lean (limit, plan, inverse,
steer solve), wheelie assist, rider mass, and whole-bike scenarios (stands, accelerates without flipping, brakes,
countersteers then leans and turns, mirrored turns, slow bike held up, wheelie, fall).
