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

## The tyre (M5, `magic.rs`, `peaks.rs`, `tyre.rs`, `thermal.rs`)

- **The formula** (`ride6-tyre-mf94.md`): the Magic Formula 94 exactly as the reference's model object evaluates it. Every
  curve is `F = D sin(C atan(E atan(u) + (1 - E) u)) + Sv` with `u = B (x + Sh)`; the builders of `B, C, D, E, Sh, Sv` for
  the longitudinal, lateral and aligning curves are `Mf94::longitudinal_shape` and its siblings (all zero below 1e-8 kN, `E`
  not clamped). The wheel hands the model its `Slip.y` (the sine of the slip angle) as if it were the angle in radians and
  negates the lateral output. The load is not clamped.
- **Peak slips** are the roots `x_s` of `E atan(B x) + (1 - E) B x = tan(pi / (2 C))` (`Shape::peak_offset`, bisection) with the
  shift put back: `sx+- = (+-x_s - Sh) 0.01`, `sy+- = (+-x_s - Sh) 0.0174533`. The game interpolates a table of them and solves
  exactly outside it; the table is a cache of the root, so the root is used everywhere.
- **Slip**: slip speed over the **total** patch speed `V`; below 1 m/s the smoothstep `k = (3 - 2u) u`, `u = 0.99 V + 0.01`,
  multiplies the slip speeds. Effective radius `R - 0.5 (1 - |cos camber|) toroid width - shape_factor squash`.
- **Combination**: normalised slips `n = 2 (s - s-) / (s+ - s-) - 1`, `c = |n| / (|nx| slip_correction + 1)`, effective slips
  along the same direction, `Fx = Fx0 min(1, |nx|/c)`, `Fy = Fy0 min(1, |ny|/c)`; the slip correction blends from its rest
  value to its speed value over the rest speed. The combined-slip solver is an empty stub in the MF94 class and is not used.
- **Efficiency**: `Fx *= 1 + 2 min(b, 0.5) (e - 1)`, `Fy *= 1 + 2 min(1 - b, 0.5) (e - 1)` with `b` 0.5 and `e` from the thermal
  model below (the `min` is against the constant 0.5). The aligning moment is the front wheel's only and is reported, not fed
  back.
- **Rolling resistance**: `Fz (c0 + (c2 v + c1) v + cS |slip_y|^p)` with `v` the speed of the **whole bike**, above 1 m/s, a
  force against the bike's velocity at the contact.
- **Spin**: `(I + I_extra) dw/dt = drive - brake - r_eff Fx(w)` solved for the new spin by bisection (`I_extra` the engine's
  inertia through a locked clutch).
- **Temperature and wear** (`ride6-tyre-thermal.md`): each strip of the tread has a temperature and a thickness and a tent
  weight across the width from the lean. Per step the wear is `h *= 1 + intensity p (rate^(v dt / L) - 1)` with the rate from
  the strip's temperature, and the temperature moves by `(1 - exp(-H k_d dt)) p (T_max - T)` plus `(1 - exp(-k_s dt)) (T_road - T)`
  with `H = Fz (lat_coef ny + long_coef nx) + rolling_coef Rr` in Unreal force units; neighbouring strips relax at a square-root
  rate, and in uniform mode (every bike) the strips take their mean. The efficiency is the weighted mean of
  `clamp(temperature curve x wear curve, limit, 1)`. A tyre without a `thermal` table is always at its best.

## Engine, clutch, gearbox (M6, M7, `powertrain.rs`, `gearbox.rs`)

`T = T_closed + (T_max - T_closed) throttle`, `T_closed = -T_fric(rpm/max)`, `T_max = peak curve(rpm)`; the throttle is held
at `throttle_at_idle` below the idle and cut on and off by the limiter (`limiter_cut_time`) above the rev limit; the traction
control scales it about the zero-torque point `T_fric / (T_fric + T_max)`. The engine has a speed of its own and the inertia
`moi`. The clutch capacity is `engagement x clutch_max_factor x peak torque`; engaged fully it locks (the engine follows the
wheel, its inertia added to the wheel's), otherwise it slips with `sgn(w_e - w_w) capacity` on the engine and `x ratio` on the
wheel, locking when the speeds cross. Engagement is the lever (open for a gear change, back over `CLUTCH_ENGAGE_TIME`), the
launch and the slipper (back torque over `slipper_range`).

**Gearbox** (`ride6-gearbox-auto.md`). The change points are generated, as the handler does, from the torque curve and the
ratios: for gear `n` the engine speed `w` sweeps from the peak of the curve to the rev limit in steps of 1 % of
`max_rpm - idle_rpm`, with `r = G(n+1)/G(n)`, `t1 = T(w)`, `t2 = T(w r) r`; the change-up point is the first `w / max_rpm` where
`t1 up_tolerance < t2`, the change-down point of gear `n+1` the last `w r / max_rpm` where `t2 < t1 down_tolerance`. The decision
compares the **road speed** (the rear wheel's radius, not the measured engine speed) with `speed(a' max_rpm, gear)` for the
change up, `a' = a + (1 - a) hold_up`, and with `speed(f, gear - 1)` for the change down, `f = max_rpm c + (idle_rpm - max_rpm c)
hold_down`; lean holds the up change back and lean and the throttle hold the down change back (`prevent_*`). A change lasts
`up_shift_time` / `down_shift_time` with the clutch open and a throttle blip when changing down. **Launch**: with feathering
the controller of section 3 of the spec raises the clutch input while the engine is under `R = max x max_rpm_factor + peak-rpm x
peak_factor`, asks for the clutch torque that lets the engine gain `(R - rpm) rpm_raise` over its step, and releases as the wheel
catches up; without it the clutch comes in between `launch_rpm` of the rev limit and goes with the road speed. With no throttle
at a start the clutch is pulled. Shifts wait while the controller acts.

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
plus the choice term below. The front tyre's lateral curve is inverted at its load and camber (peaks bracketed, bisection) for the wanted
`Slip.y`; the handlebar angle is the closed form of the reference (`ride6-lean-controller.md`, step 3): the front contact's
velocity without its component along the steering axis is the wheel's direction of travel, the wanted heading is that direction
turned to the left by the slip, and the angle is the heading of that vector projected along the steering axis onto the chassis'
horizontal plane (`lean::handlebar_angle`); blended to `stick x max_angle` between `blend_range`, and filtered (`response`,
`damping_ratio`). **Lean help** adds a side force at the front contact that closes a
share of the gap to the wanted force, within the tyre's lateral peak and the friction ellipse. The roll acceleration is then
**realised** by a torque about the bike's length that makes up what the tyres left, within `ROLL_TORQUE_AUTHORITY` weights
times the height of the centre of gravity; it also holds a bike that stands still.

**Wheelie and stoppie** (all of it in [ride6-tricks.md](ride6-tricks.md); only a bike has any of it). Nothing lifts a wheel but
the rigid body: the engine's thrust or the front brake unloads the other tyre, with the rider's weight where the rider component puts
it. On top, a pure pitch torque (about the bike's right axis, no arm) `help x a_cmd x (chassis mass x pitch scale)` holds the body's
pitch (less the road's slope for a wheelie) at `max_angle x lever x pedal x speed ramp` (the pedal is the throttle for a wheelie and
the front brake for a stoppie); `a_cmd` is the time-optimal bang-bang plan to that angle, limited by `acceleration` and averaged
over `acceleration_time`; `help` falls with the lean and with the speed curve. The rider help cuts the throttle through the engine
as the front lifts (relaxed below a launch speed; the forced range on the pedal when a wheelie is asked) and eases the front brake as
the rear lifts, and its auto-rider moves the rider's weight against the lift. The weight the player asks (`Bike::set_weight`) drives
the help; the weight the rider component sits at (`Bike::rider_weight`) is the auto-rider's.

**Gyro, drag, fall (M10 to M12).** Roll torque `scale cos(lean) (I_f w_f + I_r w_r) yaw_rate` about the length, yaw torque
`-scale (I_f w_f + I_r w_r) roll_rate` about the vertical. Air drag `F = Cd (1 - s) v^2` against the velocity (`ride6-aero-drag.md`;
`s` is the slipstream, `Bike::set_slipstream`, which the pure function `Slipstream::contribution` computes for a host with several
vehicles; the orientation tuning is 1 for every bike read). The rider has fallen when the lean or pitch passes the fall limits,
or, above `min_speed`, when the heading error or the angular speed does; a fallen rider lets go of the throttle and the stick and
the controller's torque stops. **Block fall**: above its speed the target lean is held to `ratio x fall lean x 0.99` for up to
`block_time`, then the assist waits `block_wait`. **Rider on a spring**: the rider is a point mass on a spring and damper
(`rider_k`, `rider_damp`) that lags the bike's acceleration (gravity left out); his deflection moves the combined centre of
gravity. **Traction control**: the driven wheel's slip ratio through `tcs.slip`, faded in with the speed, cuts the throttle about
the zero-torque point.

## `BikeSpec` fields (SI units; angles in radians)

| Group | Fields |
|---|---|
| `chassis` | `mass` kg (wheels included), `rider_mass` kg, `rider_offset`, `rider_forward_shift`, `rider_backward_shift` m (x right, y up, z forward, from the bike's centre of gravity), `rider_weight_rate` 1/s, `rider_k` N/m, `rider_damp` N s/m, `half_dimensions` m, `tensor_scale` m^2 (pitch, yaw, roll), `cg_height` m, `front_weight`, `wheelbase`, `trail` m, `rake` rad |
| `front`, `rear` (wheel) | `radius`, `width` m, `toroid_factor`, `mass` kg, `inertia` kg m^2, `tyre` (`a[15]`, `b[11]`, `c[18]`), `rolling[3]`, `rolling_slip`, `rolling_slip_linearity`, `thermal` (strips with their coefficients and curves, decay and time scales), `lateral_grip_limit` (informational), `tire_stiffness` N/m (0 rigid), `max_tire_deformation` m, `compression_shape_factor`, `slip_correction`, `rest_slip_correction`, `rest_speed_threshold` m/s |
| `front_suspension`, `rear_suspension` | `max_travel` m, `spring` N/m (at the shock), `top_out_stiffness` N/m, `top_out_length` m, `damper` (six numbers), `leverage[(share, ratio)]`, `rest_offset` m, `progressive{start,factor,linearity}`, `hydraulic{stiffness,term}`, `max_speed` m/s, `coupling{chain,traction,brake and their remaps}` |
| `engine` | `peak_torque` N m, `torque_curve[(rpm, share)]`, `friction_torque` N m, `friction_curve[(rpm/max, factor)]`, `idle_rpm`, `max_rpm`, `moi` kg m^2, `throttle_at_idle`, `limiter_cut_time` s |
| `transmission` | `gears`, `final_ratio`, `efficiency`, `up_tolerance`, `down_tolerance` (per gear), `prevent_up_lean`, `prevent_down_lean`, `prevent_down_throttle`, `up_shift_time`, `down_shift_time` s, `blip_throttle`, `clutch_linearity`, `clutch_max_factor` (x peak torque), `slipper_range` N m, `slipper_intensity`, `release_speed` m/s, `feather_factor`, `feather_max_factor`, `feather` (the controller's seven numbers), `launch_rpm` |
| `brakes` | `front_force`, `rear_force` N, `joined_factor`, `joined_front`, `joined_rear`, `abs{front_intensity, rear_intensity, speed_range m/s, front_slip, rear_slip}` |
| `steering` | `max_angle`, `response` 1/s^2, `damping_ratio`, `max_lean`, `lean{k, damping_ratio, damping_multiplier, steer_k, steer_damping_ratio, target_k, target_damping_ratio, time_scale (+ attenuation, sensitivity, speed and input curves), g_tuning, skid_range, skid_linearity, skid_reduction, stopped_limit, stopped_speed, blend_range, rider_mass_scale, help_speed, help_limit}` |
| `fall` | `lean`, `pitch`, `speed_direction` rad, `angular_speed` rad/s, `min_speed` m/s, `block_ratio`, `block_min_speed`, `block_time`, `block_wait` |
| `wheelie` (optional) | `wheelie` and `stoppie` (`deadzone`, `max_angle`, `speed_ramp`, `help_speed`, `lean_help`, `lookahead`, `min_speed`, `acceleration`, `acceleration_time`, `ground_range`, `lean_limit`, `lean_pitch_range`), the throttle and brake ranges of the rider help |
| `tcs` (optional) | `slip` (ratio, start and full), `speed_range` m/s |
| top level | `gyro_roll_scale`, `gyro_yaw_scale`, `drag` |

## What is still a choice (`choices.rs`)

| Choice | Value | Why it is open |
|---|---|---|
| `CLUTCH_OPEN_DURING_SHIFT`, `CLUTCH_ENGAGE_TIME` | true, 0.06 s | the data's clutch and throttle timings of a change (`ClutchTimingOnGearChange`) are ramps of fractions of the change whose exact use is not pinned down |
| `START_SPEED` | 1 m/s | `InStartingSequence` and the neutral logic of the handler were read, not written up |
| `SLIPPER_FALL_RATE`, `ABS_FALL_RATE` | 10, 8 per s | constructor defaults not listed |
| `ROLL_TORQUE_AUTHORITY` | 2 weights x height | the reference closes the prescribed roll with the tyre forces, the lean help and the stability help (off for the stock bike); the pairing of the help reductions with their inputs was not recovered, so a bounded torque stands in |
| `LEAN_FORCE_INCLUDES_TURN_FORCE` | true | the force balance of a held lean is not in the recovered equations; without the term nothing would turn this model's bike |
| `BRAKE_COUPLING_SIGN` | -1 | the sign of `T_applied - T_back` for a braking wheel |
| `BUMP_STOP_FACTOR` | 12 | the end of the travel is only a clamp in the reference (used for a rigid tyre) |
| `AIR_TEMPERATURE`, `ASPHALT_TEMPERATURE` | 20, 30 deg C | the track and weather set them; no default was found |
| `LEAN_AND_PITCH_FALL_AT_ANY_SPEED` | true | the exact fall tests are inferred |
| `RiderHelpSpec::default()` | the constructor defaults of the reference | what a manifest leaves out of `[bike.wheelie.help]` (recovered, [ride6-tricks.md](ride6-tricks.md)) |
| `LATERAL_GRIP_TOLERANCE`, `TYRE_EFFICIENCY_BALANCE`, `SLIP_CORRECTION_MULTIPLIER`, `STANDING_SPEED` | 1, 0.5, 1, 0.5 m/s | recovered defaults (the first two), dry road, numerical guard |

`HANDLEBAR_LEAD` of the first build is gone: with the closed-form handlebar angle the saturated stop (a held turn under full
brakes, and a straight stop from 30 m/s) stays stable without a lead.

## Not modelled

The auto-rider's start control and g-force variant, the g-force interventions of the rider help, the torque ramp of the wheelie help, the engine electronics (ECU) and the shaping of the traction control by lean and
yaw rate, the stability help (off for the stock bike; the pairing of its reductions was not recovered), the brake-reduction
cornering protection of the ABS element (its form is inferred, not read), the rider's tuck-in drag (the rider drag curves were not
read), the terrain terms of the lean limit and the lean limit's brake and throttle deltas (zero by default; the reduction scale and
the sign of the terrain inclination were not recovered), the wet rows of the tyre tables, the swingarm's arc, the forced
downshift under braking (not on the trick path; not looked for).

## Status and verification

Written in two rounds. The first run found the unstable roll acceleration (`filter::step_acceleration`), an arcsine past 1 in the
tyre and four tests that measured the wrong frame; the second round replaced the numerical handlebar solve and the shift choices
by the recovered equations, evaluated the tyre exactly, added the thermal state, the gearbox decision, the launch controller,
slipstream, block fall, the rider's spring and the traction control. Tests: filters, suspension, tyre (formula, peaks against the
spec's test values, smoothstep, combination, rolling resistance, spin solve), thermal (weights, warming, wear), powertrain,
gearbox (generated points and the decision), brakes, lean (limit, plan, inverse, closed-form handlebar, block cap), wheelie
assist, rider, slipstream, and whole-bike scenarios (stands, accelerates without flipping, brakes in a straight line and in a held
turn, countersteers then leans and turns, mirrored turns, wheelie, fall, traction control, rider's spring).
