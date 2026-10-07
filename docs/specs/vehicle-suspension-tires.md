# Vehicle suspension, tires, steering and aero

How NFS: Most Wanted turns a driver's pedals and wheel into forces on a car body: four spring/damper
corners, a tire model, steering geometry, drag and downforce, and the helper "assists" layered on top.
This is the per-step behaviour of the player/racer chassis. Engine, transmission torque, the rigid-body
integrator, collision response and damage are separate specs; here they are only inputs.

- **Sources read** (all read for understanding, nothing copied; formulas and tables below are written
  in our own words):
  - [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled, mostly the GameCube
    build): `src/Speed/Indep/Src/Physics/Behaviors/SuspensionRacer.cpp` (tire, steering, drift, drive
    split, per-wheel loop), `Behaviors/Chassis.{h,cpp}` (state snapshot, centre of gravity, Ackermann,
    aero, grip/traction scales, sleep, jump stabiliser), `Physics/Wheel.h` and `Physics/Common/Wheel.cpp`
    (wheel position and velocity), `Physics/PhysicsInfo.cpp` (`WheelDiameter`, `AerodynamicDownforce`),
    `Physics/PhysicsTunings.h`, `Misc/Table.cpp` (table lookup), `Misc/MWAttribUserTypes.h` (`AxlePair`),
    `Tools/Inc/ConversionUtil.hpp` (unit factors), `Interfaces/Simables/ISuspension.h`,
    `Generated/AttribSys/Classes/{chassis,tires,brakes,simsurface,transmission}.h`.
    `Behaviors/SuspensionSimple.cpp` was skimmed: it is the same corner model without the assists
    (used for AI/traffic-like cars) and is not specified separately.
- **Data inputs:** AttribSys classes `chassis`, `tires`, `brakes` (per car, reached through the car's
  `pvehicle` record), `transmission` (`DIFFERENTIAL`, `TORQUE_SPLIT`), `simsurface` (per road surface),
  and the player's tuning sliders. Field lists with units are in [Fields read](vehicle-steering-assists-aero.md#fields-read).
  Rigid-body inputs (mass, inertia, dimensions, velocity) come from the body spec.
- **Split:** sections 4 to 8 (assists, drive split, aero, steering, stabilisers, tunings) and the field
  table are in the companion [vehicle-steering-assists-aero.md](vehicle-steering-assists-aero.md);
  section numbers are shared between the two files and open questions are numbered here.

Evidence tags as in the [docs README](../README.md#evidence-tags). **Nothing here is [verified]**: no
measurement against the running game has been made yet. [inferred] is used where a meaning follows from
how a value is used but no source states it (stronger than [unconfirmed]).

## Conventions

- **Physics space** is **x right, y up, z forward**, right-handed, metres [decomp: the body matrix rows
  are right, up, forward; `local_vel.z` is forward speed]. This is *not* the car-solid space of
  [car-assembly.md](car-assembly.md) (+x forward, +y left, +z up). Mapping: `phys = (-asm.y, asm.z, asm.x)`
  [inferred].
- **Wheel order in physics:** 0 = front left, 1 = front right, 2 = rear **left**, 3 = rear **right**
  (arms at x = -, +, -, +) [decomp for the arm signs; naming inferred from `x = right`]. Car assembly lists
  the rear as 2 = rear right, 3 = rear left. Whatever the render side does, physics index 2 is the wheel
  at -x. See open question 1.
- `axle = wheel_index / 2` (0 front, 1 rear). `AxlePair` fields are `(Front, Rear)`.
- **Angles inside the tire/steering code are "turns"** (1.0 = 360 degrees): `slip_angle`, `steer_input`
  after conversion, `state.slipangle`. Conversions: `turns = deg / 360`, `rad = turns * 2 pi`.
- Unit factors [decomp]: 1 inch = 0.0254 m; 1 lb/in = 175.1268 N/m (spring rate) and likewise
  lb*s/in = 175.1268 N*s/m (damper rate); 1 ft*lb = 1.3558 N*m; 1 mph = 0.44703 m/s.
- `Ramp(a, lo, hi) = clamp((a - lo) / (hi - lo), 0, 1)` (0 if `hi - lo` is not positive).
  `Table(values[n], lo, hi)` = piecewise-linear lookup at evenly spaced points from `lo` to `hi`,
  clamped to the first/last value outside the range.
- Time step: `dT` is the physics sub-step passed to the behaviour (`state.time`). All rates below are
  per second. The step length is a Sim setting outside this spec (see open question 9).

## Per-step order

One call per car per physics step [decomp, `OnTaskSimulate`]:

1. Skip entirely if there is no rigid body.
2. Set the **centre of gravity** (§ 1.1) using the ride-height tuning.
3. Build the **state snapshot** (§ 1.2) from the body and the input controls.
4. Decay the post-collision steering timer by `dT`. Compute the "speed-break" (`gameBreaker`) level
   (a player boost; 1 while active, ramps out at 1/3 per second afterwards; when > 0 an extra
   `-2 g * level * mass` force acts along the body's up axis). It is a gameplay feature, optional for a
   first port; hooks are marked "(speed-break)".
5. Compute the three per-step scales: `max_slip` (§ 3.8), lateral `grip_scale` and `traction_scale`
   (§ 3.7). Add `0.25 * perfect_launch` to `traction_scale` unless staging.
6. `BeginFrame` on each tire: store the scales, zero torques, forces and slip-state that must be rebuilt
   (§ 3.1).
7. **Aerodynamics** (§ 5): drag and downforce forces on the body.
8. **Drive forces** (§ 4.2): split the transmission torque to the tires. Uses burnout state from the
   *previous* step.
9. **Wheel forces** (§ 2, the main loop), in this order inside it:
   1. steering and Ackermann (§ 6);
   2. `TuneWheelParams` (§ 4.1): brake bias, handbrake, yaw-control boost, burnout state update, staging,
      drift state;
   3. per-wheel: ground probe, compression, spring/damper force, tire forces (§ 3);
   4. sum forces and torques about the centre of gravity and apply to the body once; yaw-torque rescale;
      lift-out of the ground; landing damping (§ 2.6).
10. Tire housekeeping: tires advance their own timers; tire-heat bookkeeping (cosmetic, skid effects).
11. **Airborne stabilisers** (§ 7.3), then **sleep** damping (§ 7.4).

Order consequences worth keeping [decomp]: drive torque is added to the tires *before* the per-wheel
loop reads it; `TuneWheelParams` scales traction boost *after* the drive step has already scaled it, so
the boosts multiply; the drift friction set in step 9.2 is used by the tire force in step 9.3 of the same
step; burnout state set in 9.2 only affects the drive split of the *next* step.

## 1. Setup shared by every step

### 1.1 Centre of gravity [decomp]

```
front_z = FRONT_AXLE                       rear_z = FRONT_AXLE - WHEEL_BASE        (m, body frame)
bias    = (FRONT_WEIGHT_BIAS + extra_bias) * 0.01      (extra_bias = 0 here)
bias    = 0.5 if no wheel is on the ground            (so a flying car is balanced)
cg.z    = (front_z - rear_z) * bias + rear_z
cg.y    = ROLL_CENTER[m] - (dim.y + max(RIDE_HEIGHT.front + ride_extra, RIDE_HEIGHT.rear + ride_extra)[m])
cg.x    = 0
```

`ride_extra` is the ride-height tuning value in inches (§ 8). `dim.y` is the body's half-height
(`dimension.y`), see open question 4. The rigid body is told this position as its centre of mass every
step, so its torque and rotation pivot follow it.

### 1.2 State snapshot [decomp]

Read once per step, then used by all sub-systems:

| Field | Meaning |
|---|---|
| `matrix` | body orientation + position (rows right, up, forward, position) |
| `linear_vel`, `local_vel` | world and body-frame velocity (m/s); `local_vel.z` is forward speed |
| `angular_vel`, `local_angular_vel` | rad/s |
| `speed` | `|linear_vel|` (m/s) |
| `slipangle` | body slip angle in turns: 0 if `local_vel.z < 1`, else `atan2(local_vel.x, local_vel.z)` |
| `gas`, `brake` | pedals clamped to [0, 1] |
| `ebrake` | handbrake as given (not clamped) |
| `steer_input` | clamp to [-1, 1]; +1 is right |
| `world_cog` | `cog` rotated into world orientation |
| `ground_effect` | `0.25 * wheels_on_ground` (count from the previous step) |
| `mass`, `inertia`, `dimension` | from the rigid body |
| `gear`, `nos_boost`, `shift_boost` | from transmission/engine (1 when absent), `driver_style`, `blown_tires` mask, staging and destroyed flags |

If the engine is blown or the car is destroyed: `brake = 1`, `gas = 0`, `ebrake = 1`. A tire reported
as blown by the spike-strip system sets bit `i` of `blown_tires`.

### 1.3 Per-car geometry (built once) [decomp]

```
radius_f = wheel_diameter(front) / 2           radius_r = wheel_diameter(rear) / 2
wheel_diameter(axle) = RIM_SIZE[axle] in metres + 2 * SECTION_WIDTH[axle] * 0.001 * ASPECT_RATIO[axle] * 0.01
radius = max(radius, 0.1)                       (per tire)
axle_w[axle] = TRACK_WIDTH[axle] - SECTION_WIDTH[axle] * 0.001
arm[0] = (-axle_w[0]/2, -dim.y, FRONT_AXLE)               arm[1] = (+axle_w[0]/2, -dim.y, FRONT_AXLE)
arm[2] = (-axle_w[1]/2, -dim.y, FRONT_AXLE - WHEEL_BASE)  arm[3] = (+axle_w[1]/2, -dim.y, FRONT_AXLE - WHEEL_BASE)
```

`arm[i]` ("local arm") is the wheel's contact point in the body frame, at the *bottom* of the body box
(y = -half-height). `RIM_SIZE` is in inches, `SECTION_WIDTH` in millimetres, `ASPECT_RATIO` in percent.
`TRACK_WIDTH`, `WHEEL_BASE`, `FRONT_AXLE` are metres [inferred from how they are added to metres].
`SECTION_WIDTH * 0.001` is subtracted, so `TRACK_WIDTH` is outer-edge to outer-edge of the tires, not
centre to centre [inferred].

## 2. Spring, damper and anti-roll bars (per wheel)

Each corner is a one-dimensional spring along the body's *up* axis, fed by a ground probe. State kept
per wheel: `compression` (m, >= 0; 0 means not touching), `air_time`.

### 2.1 Axle constants (converted once per step) [decomp]

```
spring   = SPRING_STIFFNESS[axle]  lb/in -> N/m
shock    = SHOCK_STIFFNESS[axle]   -> N*s/m   (damping while compressing)
shock_ex = SHOCK_EXT_STIFFNESS[axle] -> N*s/m (damping while extending)
sway     = SWAYBAR_STIFFNESS[axle] -> N/m
travel   = TRAVEL[axle] in -> m
ride     = (RIDE_HEIGHT[axle] + ride_extra) in -> m
prog     = SPRING_PROGRESSION[axle]             (1/m)
valving  = SHOCK_VALVING[axle] in -> m/s?       digress = 1 - SHOCK_DIGRESSION[axle]
```

`SHOCK_VALVING` is converted with the inch factor, so it is a velocity threshold in m/s numerically
(the rate unit is not stated anywhere) [inferred].

### 2.2 Ground probe [decomp + unconfirmed]

For each wheel, in order 0..3, using the *current* body state:

```
world_arm = rotate(arm[i], body orientation)
position  = body_pos + world_arm
point_vel = linear_vel + angular_vel x (world_arm - world_cog)
probe tolerance = min( max(-point_vel.y*dT, 0) + radius + |point_vel.xz|*dT , body_height/2 )
```

The probe asks the collision/world query for the surface below `position`: a unit normal `n` and a signed
penetration `n.w` (metres the point is below the surface, positive = under the surface) plus the road
surface record (§ 3.9). The probe caches last step's polygon when the wheel was on the ground
(`usecache`). The world-query internals (`WWorldPos`) are not specified here; see open question 3.

### 2.3 Compression [decomp]

```
upness = clamp(dot(n, body_up), 0, 1)                    how flat the wheel is to the ground
c_new  = ride * upness + n.w                             (n.w = penetration)
if wheel.compression == 0:  lift_need = max(lift_need, c_new - travel)   (touching down hard)
c_new  = max(c_new, 0)
if c_new > travel:                                       bottomed out
    lift_need = max(lift_need, c_new - travel); c_new = travel; wheel.bottom_out_time = now
```

So a wheel carries compression equal to the ride height plus any penetration of the contact point below
the ground, clamped to `[0, travel]`. The rigid body is later lifted by `lift_need` (§ 2.6) so a car
never stays more than `travel` deep in the ground.

A wheel is **loaded** if `c_new > 0` and `upness > 0.2` (roll-stop threshold). Otherwise it is **free**
(§ 3.6). `air_time += dT` while `c_new == 0`, else reset to 0. `wheels_on_ground` counts loaded wheels.

### 2.4 Vertical force of a loaded wheel [decomp]

```
rise   = (c_new - c_old) / dT                         m/s, + when compressing
if valving > 1e-6 and digress < 1 and |rise| > valving:
        |rise| := valving * (|rise| / valving) ^ digress          (damper digression, keeps sign)
spring_f = c_new * spring * (1 + c_new * prog)        progressive spring
damp_f   = rise * (rise > 0 ? shock : shock_ex)
if damp_f > SHOCK_BLOWOUT * 9.81 * mass:  damp_f = 0  (blow-off valve opens)
sway_f   = (c_left - c_right) * sway       for the left wheel, negated for the right wheel
           (uses the compressions stored at the start of this step for both wheels of the axle)
F_spring = max(damp_f + spring_f + sway_f, 0)         pushes the body along its up axis
```

Compression speeds use the *stored* old compression, so the damper sees last step's value. The anti-roll
bar is a spring on the left-right compression difference, per axle, equal and opposite on the two wheels;
there is no separate roll model. `SHOCK_BLOWOUT` is a multiplier of body weight [inferred from the
expression `BLOWOUT * g * mass`].

### 2.5 Tire load and contact forces [decomp]

```
f = steered wheel forward direction (§ 6; the body forward vector for rear wheels)
c = f x n ; c = c x f                                  (n projected perpendicular to f)
load_factor = max(4 * dot(c, n) - 3, 0.3)              dot(c, n) = 1 - dot(f, n)^2 for unit vectors
load        = load_factor * F_spring                   N; includes dampers and sway
lat_dir     = unit(n x f)                              wheel sideways direction, in the ground plane
fwd_speed   = dot(point_vel, f)    lat_speed = dot(point_vel, lat_dir)
Fy = tire.update_loaded(lat_speed, fwd_speed, body_speed, load, dT)    (§ 3)
Fy_limit = |lat_speed / dT| * 0.25 * mass                      one wheel can cancel a quarter of body motion
lateral_force = lat_dir * clamp(Fy, -Fy_limit, +Fy_limit)
drive_force   = unit(lat_dir x n) * tire.longitudinal_force
force on body at wheel = lateral_force + drive_force + up * F_spring
```

`load_factor` drops from 1 on flat ground toward 0.3 as the wheel's ground plane tilts along the wheel's
heading (a slope of about 17 degrees pitch already gives 0.3); the spring force is applied to the body in
full, only the tire's grip load is reduced.

### 2.6 Applying the forces [decomp]

After the loop, if any wheel was loaded:

```
for each wheel i:   p = arm[i]; p.y += compression - ride[axle]      (arm lowered/raised by the spring)
                    p_world = body.transform(p); r = p_world - world_cog_point
                    torque_i = r x force_i
total_force  = sum(force_i)            total_torque = sum(torque_i)
yaw          = dot(body_up, total_torque)
total_torque += body_up * (yaw * YAW_SPEED - yaw)     i.e. the yaw component is scaled by YAW_SPEED
if drag-race driver: yaw component additionally x 1.6
apply (total_force, total_torque) to the rigid body once
```

`YAW_SPEED` (tires class) therefore scales how fast the car can be yawed by tire forces (1 = unmodified).
After that:

- if `lift_need > 0`, translate the body up by `lift_need` (and every wheel position by the same);
- if wheels were all off the ground last step and some are loaded now (landing), multiply the body's local
  angular velocity about **y (up)** and **z (forward)** by 0.5.

The stored wheel render position is `p_world`; `compression` becomes `c_new`. Render systems read it back
as "suspension travel"; `RENDER_MOTION` (chassis class) is an exposed multiplier for visual body motion
only [decomp: read by `GetRenderMotion`, not used by physics].

## 3. Tire model

A tire has state: `AV` (rotation speed, rad/s), `load`, `lateral_force`, `longitudinal_force`,
`drive_torque`, `brake_torque`, `traction` (1 = gripping, below 1 = sliding), `slip` (m/s), `slip_angle`,
`road_speed`, `lateral_speed`, `brake_locked`, `last_sign`, `last_torque`, `angular_acc` and per-step
modifiers. It never carries mass; the effective rotational inertia is a constant.

### 3.1 Per-step modifiers (`BeginFrame`) [decomp]

Reset every step: drive and brake torque = 0, forces = 0, `traction_circle = (1, 1)`,
`drift_friction = 1`; set `max_slip`, `grip_boost` (the lateral `grip_scale`), `traction_boost`
(the `traction_scale`) and `drag_reduction` (steer-drag reduction, § 3.5). `lateral_boost` persists
until `TuneWheelParams` sets it again (1 except in speed-break).

### 3.2 Constants [decomp, hard-coded]

| Name | Value | Use |
|---|---|---|
| wheel moment of inertia | 10 kg m^2 | all tires |
| rolling friction | 2.0 (x `simsurface.ROLLING_RESISTANCE`) | N m per rad/s |
| brake torque scale | 4.0 | multiplies `BRAKES` |
| handbrake torque scale | 10.0 | multiplies `EBRAKE` |
| brake lock static/dynamic ratio | 1.2 | brake lock test |
| brake lock AV factor | 100 | brake lock test (N m per rad/s) |
| force-ellipse ratio | 1.5 | § 3.4 |
| cornering scale | 1000 | lateral force |
| grip factor | 2.5 | lateral force; the source marks it "doesn't exist" (see open question 2) |
| load factor | 0.8 | lateral force load units |
| max slip angle table | 12 deg | see below |
| "pilot factor" base | 0.85 | § 3.3 |
| slip eps | 1e-6 | |
| ~1 mph | 0.447 m/s | low-speed cases |

### 3.3 Lateral force curve (`ComputeLateralForce`) [decomp; PC differs, see open question 2]

Input: wheel `load` (N) and absolute slip angle. Output: lateral force magnitude (N).

```
angle_deg = |slip_angle| * 360 ;  seg = angle_deg / 2 ;  k = floor(seg) ;  frac = seg - k
x = load * 0.001 * 0.8                      (kN scaled by 0.8)
families F0..F6 = rows of the table below, each indexed by x with Table(lo=0, hi=10, 6 entries)
if k >= 6:            value = F6(x)
else:                 value = F_k(x) + frac * (F_{k+1}(x) - F_k(x))
force = GRIP_SCALE[axle] * 1000 * grip_boost * 2.5 * value
```

Curves (6 values at x = 0, 2, 4, 6, 8, 10; slip angle in degrees on the left):

| slip angle | x=0 | 2 | 4 | 6 | 8 | 10 |
|---|---|---|---|---|---|---|
| 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| 2 | 0 | 1.2 | 2.3 | 3.0 | 3.0 | 2.8 |
| 4 | 0 | 1.7 | 3.2 | 4.3 | 5.1 | 5.2 |
| 6 | 0 | 1.8 | 3.5 | 4.9 | 5.8 | 6.1 |
| 8 | 0 | 1.83 | 3.6 | 5.0 | 5.96 | 6.4 |
| 10 | 0 | 1.86 | 3.7 | 5.1 | 6.13 | 6.7 |
| 12 | 0 | 1.9 | 3.8 | 5.2 | 6.3 | 7.1 |

Shape: roughly linear in load at small x, saturating by 3 to 6 kN of corrected load (a "load
sensitivity" in which doubling the load gives less than double the force), rising with slip angle toward
a plateau near 12 degrees and beyond. The whole expression is multiplied by `lateral_boost` afterwards
(§ 3.4). In the PC build the final multiplication uses an extra, copy-protected multiplier whose value is
not in the source (open question 2); the numbers above are the GameCube build.

### 3.4 Loaded tire update (`UpdateLoaded`) [decomp]

Inputs: `lat_vel`, `fwd_vel` (contact patch velocity in the wheel's frame, m/s), `body_speed` (m/s),
`load` (N), `dT`. Returns the lateral force (the longitudinal one is read afterwards).

1. **Free rolling:** if the previous `load <= 0` and not brake-locked, `AV = fwd_vel / radius`.
2. `fwd_acc = (fwd_vel - road_speed_prev) / dT`; then `road_speed = fwd_vel`, `load = max(load, 0)`,
   `lateral_speed = lat_vel`.
3. **Brake torques.** `brake_spec = BRAKES[axle] * 1.3558 * 4`, `ebrake_spec = EBRAKE * 1.3558 * 10`,
   `bt = brake * brake_spec`, `ebt = ebrake * ebrake_spec`.
   - if `|fwd_vel| < 1`: brake torque is `-brake * load * fwd_vel / radius` (and the same for the
     handbrake), so a nearly stopped car is held by a force proportional to load and speed rather than
     a fixed torque; drive torque is also cancelled by `-drive_torque * ebrake`;
   - otherwise brake torque opposes rotation: `-bt` and `-ebt` if `AV > 0`, else `+bt`, `+ebt`.
     Brake torque is ignored (not added) while `brake_locked`.
4. `slip_angle = atan2(lat_vel, |fwd_vel|)` (turns). `slip = AV*radius - fwd_vel` (m/s, + when the wheel
   spins faster than the ground).
5. `skid = sqrt(slip^2 + lat_vel^2)`. If `skid > eps` and the patch is moving:
   - `dyn = DYNAMIC_GRIP[axle] * traction_boost * pilot_factor` (§ below);
   - `ground_friction = load * dyn / skid` (a viscous-style friction that gives full `load*dyn` at any
     slip direction);
   - brake-lock test (§ 3.4a) using `|fwd_vel| * load*dyn / hypot(fwd_vel, lat_vel)`.
   Otherwise `ground_friction = 0`, `dyn = 1`.
6. **Branch by last step's `traction`:**
   - sliding (`traction < 1` or locked): `Fx = ground_friction * slip`, `Fy = -ground_friction * lat_vel`;
     if `body_speed < 1 mph` and `dyn > 0.1`, both are divided by `dyn`; `Fx` is then limited in magnitude
     to `|total_torque| / radius`.
   - gripping: `brake_locked = false`; `Fx = total_torque / radius` (drive plus brake, all of it);
     `Fy = lateral_curve(load, |slip_angle|)` (§ 3.3) with sign opposing `lat_vel` (negative when
     `lat_vel > 0`).
7. `Fy *= lateral_boost`. If `traction >= 1` and not locked: add the wheel-inertia term
   `(angular_acc*radius - fwd_acc) * I / radius` to `Fx`.
8. **Force ellipse:** if `total_torque * fwd_vel > 0` and not locked (power being applied), temporarily
   `Fx *= 1.5` so the longitudinal axis of the friction ellipse is 1.5x the lateral one.
9. **Traction circle:** `Fy *= traction_circle.x`, `Fx *= traction_circle.y` (set from the handling tuning,
   § 8).
10. **Friction limit:** `len = hypot(Fx, Fy)`, `max_f = load * STATIC_GRIP[axle] * traction_boost *
    drift_friction * pilot_factor`. If `len > max_f` and `len > 0.001`: `ratio = max_f/len`;
    `traction = ratio`; scale `Fx, Fy` by `ratio`; and `max_slip := ratio^2 * max_slip`. Otherwise
    `traction = 1`, and if the ellipse stretch was applied, undo it (`Fx /= 1.5`).
    Then, if `|slip| > max_slip` (both measured in m/s): `traction *= max_slip / |slip|`.
11. **Surface:** `Fy *= surface.LATERAL_GRIP`, `Fx *= surface.DRIVE_GRIP`.
12. **Steering drag:** if `fwd_vel > 1`: `Fx -= sin(slip_angle) * Fy * drag_reduction / GRIP_SCALE[axle]`;
    else `Fy *= min(|lat_vel|, 1)` (fades lateral force out at standstill).
13. **Wheel spin integration:** if brake-locked, `angular_acc = 0` (AV stays 0). Else:
    - if `traction < 1` (spinning/sliding): `torque = (total_torque - Fx*radius + last_torque) / 2`,
      `last_torque = torque`, `eff = torque - AV * 2.0 * surface.ROLLING_RESISTANCE`,
      `angular_acc = eff / 10 - traction*slip / (radius*dT)`;
    - then in all cases `angular_acc = lerp(angular_acc, fwd_acc/radius, traction)` so a gripping wheel
      follows the ground and a slipping one follows the torque.
    - `AV += angular_acc * dT`, then `check_sign`.
14. `check_sign` stops AV from changing sign within a step: if the previous non-zero sign was + and AV is
    now negative (or vice versa) AV is set to 0; the sign memory is then updated with a deadband of 1e-6.

**Pilot factor** reduces grip of *rear (non-steering) wheels* at low speed: 1 for front wheels, when
`brake_locked` or when `AV < 0`; otherwise `0.85 + 0.15 * clamp((body_speed - 30 mph)/(20 mph), 0, 1)`
(0.85 below 30 mph, 1 at 50 mph). It scales both dynamic and static friction.

### 3.4a Brake lock test [decomp]

```
lock_spec  = BRAKE_LOCK[axle] * BRAKES[axle]*1.3558 * 4                 (axle's lock threshold)
available  = (brake * lock_spec + ebrake * EBRAKE*1.3558*10) * 1.2
if available > ground_force * radius + |AV| * 100:   locked = (available > 1);  AV = 0
else locked = false
```

`ground_force` is the grip limit estimate from step 5 above; in the unloaded case it is 0. `BRAKE_LOCK`
therefore is a per-axle multiplier on how readily the brakes lock the wheel. A locked wheel still slides
(`traction` branch "sliding").

### 3.5 Steer drag [decomp]

`drag_reduction` is `0.15` normally; in speed-break it blends linearly toward `1.0`. It controls how much
of the lateral force's component along the wheel's travel is subtracted from `Fx` (step 12), i.e. the
"cornering drag".

### 3.6 Unloaded tire (`UpdateFree`) [decomp]

For a free wheel: `load = 0`, `slip = 0`, `traction = 0`, `slip_angle = 0`, forces 0. Brake-lock test with
ground force 0; if locked, `AV = 0`. Otherwise brake and handbrake torque oppose rotation as above and
`angular_acc = total_torque / 10`; `AV += angular_acc*dT`; `check_sign`. A drive torque is not applied
to a free wheel in the drive split (§ 4.2), so wheels do not spin up in the air from engine torque
except by the stored `AV` coasting [decomp: drive split only adds torque to grounded wheels].

### 3.7 Speed-dependent grip and traction scales [decomp]

Both use `r = Ramp(speed, 0, 85 mph)` and a 10-entry table over `r in [0, 1]`:

| r step | 0 | 1/9 | 2/9 | 3/9 | 4/9 | 5/9 | 6/9 | 7/9 | 8/9 | 1 |
|---|---|---|---|---|---|---|---|---|---|---|
| traction table | 0.909 | 1.045 | 1.09 | 1.09 | 1.09 | 1.09 | 1.09 | 1.045 | 1.0 | 1.0 |
| lateral grip table | 0.833 | 0.958 | 1.008 | 1.0167 | 1.033 | 1.033 | 1.033 | 1.0167 | 1.0 | 1.0 |

```
grip_scale     = GripTable(r) * 1.2         ; drag-race driver: 3.0
traction_scale = TractionTable(r) * 1.1     ; drag-race driver: 1.1 ; reverse gear: 2.0
```

`grip_scale` becomes each tire's `grip_boost` (scales the lateral curve). `traction_scale` becomes
`traction_boost` (scales both friction limits). Both are then multiplied by the assists in § 4.

### 3.8 Allowed slip speed [decomp]

```
max_slip = 0.5 + Ramp(speed, 10 m/s, 71 m/s)            m/s; 71 m/s in reverse gear
```

The slip speed beyond which a driven wheel counts as "lost" (traction multiplied down, § 3.4 step 10).

### 3.9 Surface friction [decomp]

The probe returns a `simsurface` collection. Fields read by the chassis: `LATERAL_GRIP` (multiplies the
lateral force), `DRIVE_GRIP` (multiplies the longitudinal force), `ROLLING_RESISTANCE` (rolling friction
of spinning wheels). Other fields exist (`WORLD_FRICTION`, `STICK`, noise and effects) but are not read by
the tire code; `STICK` is kept in the wheel record but unused here (open question 5).

## Formula chain, one wheel

`penetration -> compression -> spring/damper/sway force F -> load = factor*F -> tire forces
(Fy from slip angle & load, Fx from torque or friction) -> friction ellipse limit load*STATIC_GRIP
-> surface multipliers -> force on body and torque about cog`, plus the wheel's own AV integration.

## Open questions

1. **Wheel side/order.** Physics arms put index 2 at -x and 3 at +x with x = right (so 2 = rear left),
   while [car-assembly.md](car-assembly.md) lists 2 = rear right. One of the two naming/sign readings is
   wrong, or `TireOffsets` is mapped to physics indices with a swap. Needs a runtime check.
2. **PC lateral force.** The PC build multiplies the lateral curve by a copy-protected variable; the
   GameCube numbers (grip factor 2.5, cornering scale 1000) are what § 3.3 gives. PC tuning may differ;
   calibrate against a skidpad measurement.
3. **Ground probe.** `WWorldPos` (surface query, cached polygon, smoothing flag, the meaning of
   `normal.w` and how tolerance is used) was not read.
4. **`dimension`.** Whether it is half-extent (assumed) or full size; `arm.y = -dim.y` and `dim.y*2` as
   vehicle height suggest half-extent.
5. **`STICK` / `WORLD_FRICTION`** of `simsurface`: not read by the tire code; used elsewhere (body
   collisions?) or unused.
6. **`SHOCK_VALVING`** unit (converted with the inch factor, used as a velocity).
7. **Speed tables** are indexed by `local_vel.z` in m/s with a 160 upper bound (named MAX_SPEED); the
   unit might have been intended as mph. Verify by steering lock vs speed.
8. **Input remap** is computed but only feeds the averages; check in game whether stick curves feel
   remapped (possible unintended dead code in the decompilation).
9. **Step length** of the physics tick, and substepping: not fixed here.
10. **Speed-break, nitrous, perfect launch** values come from other systems; stubs are safe for a first
    version.

## How to check it

1. **Static ride:** drop a stock car on flat ground in free-run: each wheel's final compression `c` should
   satisfy `4 * spring * c * (1 + c*prog)` ~ weight on that axle (use `FRONT_WEIGHT_BIAS`) with the damper
   zero; verify against the in-game ride height for 3 cars.
2. **Lateral curve:** skidpad at fixed steering; read slip angle (tire 0..3) and lateral acceleration, check
   the plateau and the dependence on load against the table in § 3.3 (the PC multiplier question).
3. **Steering lock:** read the front wheel angle vs speed at full gamepad input and compare to the `range`
   table; check Ackermann by measuring outer/inner angle.
4. **Drag/downforce:** coast down from top speed with a trace; check linear-in-speed downforce and
   quadratic drag, and the 2x off-throttle drag.
5. **Assists:** handbrake turn at 40 mph (yaw control reduction), drift entry timing (12 degrees, 30 mph),
   burnout from rest (fishtail count).
