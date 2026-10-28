# Bikes: `kind = "bike"` and the `[bike]` table

Part of the [modding catalogue](README.md). A mod car ([api-content.md](api-content.md#cars)) is a four-wheel car by default.
With `kind = "bike"` it is a two-wheeled vehicle: the game drives it with the two-wheel dynamics
(`libs/blackbox-vehicle/src/bike/`, [vehicle-two-wheel.md](../specs/vehicle-two-wheel.md)) instead of the car physics, draws two
wheels instead of four and the rest of the game (garage, camera, sound, effects, collisions) works on it as on any car. How the
two are joined is in [vehicle-two-wheel-integration.md](../specs/vehicle-two-wheel-integration.md).

A bike describes itself in a `[bike]` table in SI units, and nothing else: it has no `[stats]` (the stats the shop and the lists
read are derived from `[bike]`). Every number of the dynamics is a key of the table, and **a key with no meaningful default has
none**: a bike with a number left out is refused with the key named, because a made-up number is a number nobody measured. Three
kinds of key may be left out: those whose zero (or empty list) means the feature is not there (`rider_mass_kg`, `leverage`,
`hydraulic`, `coupling`, `progressive`, `tire_stiffness_n_per_m`, `abs`, the gyroscopic scales, the slipper, the launch slip, the
lean help, the tuning curves); `transmission.down_shift` (the model's choice,
`blackbox_vehicle::bike::choices::DOWN_SHIFT_SHARE`, because the reference game's decision is not recovered); and the
`[bike.wheelie]` table as a whole. The keys map one to one to `blackbox_vehicle::bike::BikeSpec`
(`impl From<&Bike> for BikeSpec`, `libs/blackbox-modding/src/content/bike.rs` and `bike_parts.rs`); the meaning of each is in
[vehicle-two-wheel.md](../specs/vehicle-two-wheel.md) and how a RIDE 6 bike fills them in
[vehicle-two-wheel-parameters.md](../specs/vehicle-two-wheel-parameters.md).

## The manifest

```toml
# cars/mybike.toml
kind = "bike"                  # "car" (default) or "bike"
name = "My Bike"               # the usual keys: name, manufacturer, [customization] (paints), [engine_sound], [car_sound], ...
manufacturer = "Acme"

[wheels.front]                 # optional: where the model has its two hubs (below); face_m is not needed
x_m = 0.7355
z_m = 0.324
radius_m = 0.324
width_m = 0.12
[wheels.rear]
x_m = -0.739
z_m = 0.324
radius_m = 0.324
width_m = 0.15

[bike]
drag = 0.15                    # F = drag v^2 (N, v in m/s)
gyro_roll_scale = 1.0          # optional (0): gyroscopic roll torque of the wheels from the yaw rate
gyro_yaw_scale = 1.0           # optional (0): gyroscopic yaw torque from the roll rate
[bike.chassis]
mass_kg = 113.0                # the bike without the rider, wheels included
rider_mass_kg = 70.0           # optional (0)
rider_offset_m = [0.0, 0.4, -0.05]        # optional: the rider's centre of mass from the bike's, x right, y up, z forward
rider_forward_shift_m = [0.0, 0.0, 0.1]   # optional: where the weight moves at full forward / full back
rider_backward_shift_m = [0.0, 0.0, -0.1]
rider_weight_rate = 6.0        # optional (at once): per second
half_dimensions_m = [0.42, 0.64, 1.06]    # the body box: bounding radius, derived stats
tensor_scale = [0.4, 0.45, 0.2]           # radius of gyration squared (m^2) about x (pitch), y (yaw), z (roll); I = scale x total mass
cg_height_m = 0.62             # the bike's own centre of gravity
front_weight = 0.5             # share of the bike's weight on the front wheel at rest
wheelbase_m = 1.47
rake_rad = 0.4538              # steering-head angle from the vertical
trail_m = 0.105
[bike.front]
radius_m = 0.32
width_m = 0.12
toroid_factor = 0.7            # 0 flat tread, 1 round tyre
mass_kg = 17.0
inertia_kgm2 = 0.65
rolling = [0.02, 0.0004, 0.000001]       # share of the load: constant, per m/s, per (m/s)^2
lateral_grip_limit = 1.25      # informational (derived stats); the tyre curve decides the force
tire_stiffness_n_per_m = 400000.0        # optional (0, rigid): radial stiffness, in series with the suspension
max_tire_deformation_m = 0.03            # optional
compression_shape_factor = 1.0           # optional (0): how much the tyre's squash shortens its rolling radius
slip_correction = 0.05         # optional (0): combined-slip damping at speed ...
rest_slip_correction = 0.25    # ... and at rest, over rest_speed_threshold_mps
rest_speed_threshold_mps = 6.94
[bike.front.tyre]              # Magic Formula 94 coefficients, in the units of the published formula
a = [1.2, -10.0, 1330.0, 1200.0, 6.0, 0.002, 0.0, 0.0, -0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]   # a0..a10, a111, a112, a12, a13
b = [1.5, -10.0, 1250.0, 0.0, 200.0, 0.0, -0.1, 0.0, 0.1, 0.0, 0.0]                           # b0..b10
c = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] # c0..c17
# [bike.rear] and [bike.rear.tyre]: the same keys
[bike.front_suspension]
max_travel_m = 0.28            # at the wheel
spring_n_per_m = 8800.0        # at the shock
top_out_stiffness_n_per_m = 3800.0
top_out_length_m = 0.06        # optional (0): the zone the top-out spring acts over
rest_offset_m = 0.012          # optional: preload, added to the spring's input
max_speed_mps = 5.0            # optional (no limit): compression speed the law sees
# leverage = [[0.0, 2.8], [1.0, 1.97]]  # optional (direct): [share of travel, wheel travel / shock travel]; the rear shock has it
[bike.front_suspension.damper]
slow_bump = 1600.0             # N s/m up to the threshold speed (of the shock)
fast_bump = 1250.0             # N s/m beyond it
bump_threshold = 0.3           # m/s
slow_rebound = 2400.0
fast_rebound = 1650.0
rebound_threshold = 0.3
[bike.front_suspension.hydraulic]        # optional: a progressive second spring, k (x + factor 100^(p-1) x^p) with x in m
stiffness_n_per_m = 400.0
factor = 1.0
linearity = 2.0
[bike.front_suspension.coupling]         # optional: chain pull, traction and brake coupling into the suspension
brake = 35.0
brake_range = [0.0, 1.0]       # the remap of travel share 0 .. 1; chain and traction have chain_range, traction_range
# [bike.front_suspension.progressive]    # optional: start (share of travel), factor (0 off), linearity, for the main spring
# [bike.rear_suspension] and its tables: the same keys
[bike.engine]
peak_torque_nm = 46.0
torque_curve = [[2300.0, 0.7], [5000.0, 0.85], [7500.0, 1.0], [9500.0, 0.9], [11000.0, 0.7]]   # [rpm, share of the peak]
friction_torque_nm = 12.0      # closed-throttle drag at the rev limit, times friction_curve
friction_curve = [[0.0, 0.0], [1.0, 1.0], [1.5, 2.0]]   # optional (linear in rpm): [rpm / max_rpm, factor]
idle_rpm = 2300.0
max_rpm = 11000.0
moi_kgm2 = 0.015               # rotating inertia of the engine
throttle_at_idle = 0.15        # optional (0): least throttle below the idle
limiter_cut_time_s = 0.02      # the limiter cuts and restores the fuel in turn
[bike.transmission]
gears = [2.1, 1.65, 1.35, 1.12, 0.96]    # first gear first, falling
final_ratio = 7.2              # gearbox output to the rear wheel, chain included
efficiency = 1.0
up_shift = 0.9                 # share of the rev limit at which the automatic changes up under throttle
down_shift = 0.45              # optional (the model's choice): ... and below which it changes down
up_shift_time_s = 0.06
down_shift_time_s = 0.15
blip_throttle = 0.3            # optional (0): throttle blip on a change down
clutch_linearity = 1.0
clutch_max_factor = 1.0        # most torque the clutch carries, x peak torque
slipper_range_nm = [25.0, 40.0]          # optional (none): back torque over which the slipper opens
slipper_intensity = 1.0
release_speed_mps = 13.9       # optional (0): road speed above which a start is over
feather_factor = 0.85          # optional: launch engine speed, x the rpm of peak torque, at most feather_max_factor x max_rpm
feather_max_factor = 0.7
[bike.brakes]
front_force_n = 1880.0         # force at the tyre patch at full lever
rear_force_n = 1000.0
joined_factor = 1.0            # optional (0): the rear lever is at least this multiple of the front
[bike.brakes.abs]              # optional (off)
front_intensity = 1.0
rear_intensity = 1.0
speed_range_mps = [2.78, 6.94]
front_slip = [-0.3, -0.5]      # normalised slip ratio (-1 is the force peak) where it starts and where it acts fully
rear_slip = [-0.5, -1.0]
[bike.steering]
max_angle_rad = 0.7
response = 1000.0              # the handlebar filter: stiffness 1/s^2 ...
damping_ratio = 3.0            # ... and damping ratio
max_lean_rad = 1.2217          # upper bound of the lean limit (the grip decides below it)
[bike.steering.lean]           # the rider's controller; see the spec for each key
k = 1000.0
damping_ratio = 3.6
damping_multiplier = 1.0
steer_k = 1000.0
steer_damping_ratio = 2.5
target_k = 1000.0
target_damping_ratio = 2.25
time_scale = 1.0
time_scale_attenuation = 0.0
time_scale_sensitivity = 0.0
time_scale_speed = [[2.78, 1.0], [55.6, 0.65]]       # optional: [m/s, factor]
time_scale_input = [[-1.0, 0.425], [0.0, 1.0], [1.0, 0.7]]   # optional: [stick, factor]
g_tuning = [[0.0, 1.0], [1.1, 0.9]]                  # optional: [lateral g, factor of the lean limit]
skid_range_rad = [0.1745, 0.6981]
skid_linearity = 1.8
skid_reduction = [[0.0, 0.0], [8.33, 0.2]]           # optional: [m/s, cut]
stopped_limit_rad = 0.01745
stopped_speed_mps = 8.33
blend_range_mps = [2.0, 4.0]   # below the first the handlebar follows the stick
rider_mass_scale = 1.0
help_speed = [[0.0, 0.0], [2.78, 0.2]]               # optional (no lean help): [m/s, share of the force gap closed]
help_limit = 1.0
[bike.fall]
lean_rad = 1.2217
pitch_rad = 1.1345
speed_direction_rad = 1.4835
angular_speed = 4.4
min_speed_mps = 2.78
# [bike.wheelie] (optional): wheelie and stoppie assist and the rider help. [bike.wheelie.wheelie] and [bike.wheelie.stoppie]
# have deadzone, max_angle_rad, speed_ramp_mps, help_speed, lean_help, lookahead_s, min_speed_mps, acceleration,
# acceleration_time_s, ground_range_m, lean_limit_rad, lean_pitch_range_rad; [bike.wheelie] itself has
# throttle_pitch_rad, forced_throttle_pitch_rad, throttle_pitch_speed, brake_pitch_rad, brake_pitch_speed (a [0, 0] range is off).
```

The numbers above are the generic example bike of the library (`BikeSpec::example`, a 450 cc supermoto-class machine with its
rider), not any real bike's.

### Ranges

Checked by `libs/blackbox-modding/src/content/bike_check.rs` and `bike_check_parts.rs`; a number outside its range, one that is
not finite, an array of the wrong length or a key the table does not know refuses the car (the log names the key). The ranges
are wide: they catch a wheel of 3 m, not an unusual bike.

| Key | Range |
|---|---|
| `drag`, `gyro_*_scale` | 0 to 3, 0 to 10 |
| `chassis.mass_kg`, `rider_mass_kg` | 20 to 1000 kg, 0 to 300 kg |
| `chassis.rider_offset_m`, `rider_*_shift_m`, `rider_weight_rate` | -2 to 2 m, -1 to 1 m, 0 to 100 |
| `chassis.half_dimensions_m` (each), `tensor_scale` (each) | 0.05 to 5 m, 0.01 to 10 |
| `chassis.cg_height_m`, `front_weight`, `wheelbase_m` | 0.1 to 2 m, 0.2 to 0.8, 0.6 to 4 m |
| `chassis.rake_rad`, `trail_m` | 0 to 0.9 rad, 0 to 0.5 m |
| `front`/`rear.radius_m`, `width_m`, `toroid_factor` | 0.1 to 0.8 m, 0.03 to 0.5 m, 0 to 1 |
| `front`/`rear.mass_kg`, `inertia_kgm2`, `lateral_grip_limit` | 0.5 to 80 kg, 0.01 to 20 kg m^2, 0.3 to 4 |
| `front`/`rear.rolling` | `[0 to 0.2, 0 to 0.1, 0 to 0.01]` |
| `front`/`rear.tire_stiffness_n_per_m`, `max_tire_deformation_m`, `compression_shape_factor` | 0 to 10^7, 0 to 0.2 m, 0 to 1 |
| `front`/`rear.slip_correction`, `rest_slip_correction`, `rest_speed_threshold_mps` | 0 to 10, 0 to 10, 0 to 100 |
| `front`/`rear.tyre.a` (15), `b` (11), `c` (18) | finite, magnitude up to 10^7; `a0` and `b0` in 0.01 to 3 |
| `*_suspension.max_travel_m`, `spring_n_per_m`, `top_out_stiffness_n_per_m` | 0.01 to 0.6 m, 100 to 500000 N/m, 0 to 10^6 N/m |
| `*_suspension.top_out_length_m`, `rest_offset_m`, `max_speed_mps` | 0 to 0.5 m, 0 to 0.3 m, 0 to 50 m/s |
| `*_suspension.hydraulic`, `progressive` | stiffness 0 to 10^6; start 0 to 1, factor 0 to 100, linearity 0 to 10 |
| `*_suspension.coupling` | factors -1000 to 1000, remaps -10 to 10 |
| `*_suspension.damper` slopes, thresholds | 0 to 10^5 N s/m, 0.001 to 5 m/s |
| `*_suspension.leverage` | up to 16 points, share rising within 0 to 1, ratio 0.1 to 10 |
| `engine.peak_torque_nm`, `friction_torque_nm`, `moi_kgm2` | 1 to 1500 N m, 0 to 500 N m, 0.0005 to 5 kg m^2 |
| `engine.idle_rpm`, `max_rpm`, `limiter_cut_time_s` | 300 to 3000, 1500 to 20000, the limit above the idle; 0.001 to 1 s |
| `engine.torque_curve`, `friction_curve` | 2 to 32 points, rpm rising (to above the idle for the torque), share 0 to 2, factor 0 to 10 |
| `transmission.gears` | 1 to 10 ratios, 0.2 to 10, each lower than the one before |
| `transmission.final_ratio`, `efficiency` | 0.5 to 30, 0.5 to 1 |
| `transmission.up_shift`, `down_shift` | 0.3 to 1, 0.1 to 0.9, the down-shift point below the up-shift point |
| `transmission.*_shift_time_s`, `clutch_linearity`, `clutch_max_factor` | 0 to 2 s, 0.1 to 10, 0.1 to 20 |
| `brakes.front_force_n`, `rear_force_n` | 0 to 30000 N, together at least 100 N |
| `steering.max_angle_rad`, `response`, `damping_ratio` | 0.1 to 1.5 rad, 1 to 10^6, 0 to 20 |
| `steering.max_lean_rad` | 0.2 to 1.3 rad |
| `steering.lean.*` | stiffnesses 1 to 10^6, damping ratios 0 to 20, curves up to 32 points with the first number rising |
| `fall.lean_rad`, `pitch_rad`, `speed_direction_rad`, `angular_speed` | 0.2 to 1.5, 0.2 to 1.5, 0.2 to 3.2, 0.5 to 30 |

A bike is also refused when:

- it has a `[bike]` table without `kind = "bike"`, or `kind = "bike"` without the table (the car manifest is
  `libs/blackbox-modding/src/content/car.rs`);
- its role is not `player` or it has a `[traffic]` table: **a bike is driven by a person only** (no police bike, no traffic bike,
  no AI racer);
- its `[customization]` has `rims`: a bike has two different wheels and the shop swaps one rim for all of a car's wheels.

A `[stats]` table next to `[bike]` is accepted and not read.

## The files of a bike

A bike has the files of any mod car ([car-customization.md](car-customization.md#the-files-of-a-car)) with three differences:

- **Two wheel solids per level of detail**: `<ID>_KIT00_FRONT_TIRE_A..D` and `<ID>_KIT00_REAR_TIRE_A..D`. Each is modelled with the
  axle along y and the hub at the origin, centred on the plane y = 0 (a car's wheel solid has the rim face at y = 0 and is turned
  or mirrored for the left side; a bike's is not). Wheels are scaled to the radius and width of the `[wheels]` table (or of the
  `[bike]` wheel, without the table). A level of detail without a rear solid draws the front solid there.
- **The body solid has no wheels** and no brake discs: the game draws no brake parts for a bike.
- **A fork solid per level of detail, optional**: `<ID>_KIT00_FORK_A..D` holds what steers with the handlebar (forks, bars,
  levers, grips, hand guards, headlamp, front fender, gauges, the front caliper). It is modelled in the car's axes (x forward,
  y left, z up) with **the front hub on the origin** (the same point as `<ID>_KIT00_FRONT_TIRE`'s origin), not scaled. The
  game draws it like the front wheel without its spin: turned about the steering axis (rake and trail of `[bike.chassis]`)
  by the handlebar angle, and slid along the axis by the fork's compression. A bike without a fork solid is drawn with its
  front end fixed in the body (only the wheel steers). Its part is `<ID>_FORK` in the front brake slot of the parts database.

`[wheels]` places the two hubs in the model's space (x forward, y left, z up): `x_m` is how far forward of the model's origin the
hub is, `z_m` its height, `radius_m` and `width_m` the tyre. `face_m` is not read (a bike's wheels are on the centre line). Without
the table the hubs are plus and minus half the wheelbase from the origin, as high as the tyre radius. The model's origin is on
the ground, and the bike's body origin (midway between the contact patches) is midway between the two hubs.

`BOUNDS.BIN` is required as for any car ([car-collision.md](car-collision.md)). Its box is what walls, props and other cars hit.

## How it drives

| Input | Bike |
|---|---|
| Throttle | the throttle (cut by the rider help when the front lifts, if `[bike.wheelie]` is there) |
| Brake pedal | the front and the rear brake together (and the linked brakes of `joined_factor`) |
| Handbrake | the rear brake alone (the input layer has one brake axis, so the front brake alone has no key) |
| Steering | the stick: the rider's lean controller prescribes the lean the stick points to and picks the handlebar angle that makes the front tyre give the side force it needs (counter-steering included) |
| Gear shift keys, gear keys, clutch | not read: the gearbox is automatic |
| Nitrous | not read: a bike has none |
| Reset | as for a car |

Behaviour specific to bikes:

- **No reverse.** The brake stops the bike and holds it; it never selects reverse.
- **Falling.** A lean beyond `fall.lean_rad`, a pitch beyond `fall.pitch_rad`, or (above `fall.min_speed_mps`) a heading error or
  angular speed beyond theirs is a fall: the rider lets go (no throttle, no steering), the bike slides to a stop on the
  collision box and, once it has lain still for 2 s, is stood up again where it is, facing the way it did. The fall is not damage.
- **Race grid.** The rider holds the bike with both brakes while the race is staged; there is no launch assist.
- **Gauges and sound.** The tachometer, the gear display and the engine sound follow the bike's rpm and gear; the engine loops are
  the car's own `[engine_sound]` or the built-in engine.
- **Wheels on the ground.** Systems that count wheels read a bike's two wheels as four (each counts twice): both down is "all
  four", one down is "two", so the airborne and fall checks and the audio mixer work unchanged.
- **Rider weight and wheelies.** The library takes the rider's weight (`Bike::set_weight`); the game does not wire a key to it
  yet, so a wheelie or a stoppie only happens by the power or the brakes, held in check by the rider help.

## Not done

Documented limits (each is a decision of the integration, not of the dynamics, and is in
[vehicle-two-wheel-integration.md](../specs/vehicle-two-wheel-integration.md#limits)):

- AI, traffic and police bikes; a rider model and its animation; a key for the rider's weight;
- the shop's upgrade levels change nothing of a bike (they act on a four-wheel car's stats); the shop's bars are estimates from
  the derived stats;
- no tyre damage (spike strips do not puncture a bike), no nitrous, no drift assist, no manual gearbox;
- the damage zones of the car physics hit the collision box; the bike's tyres and frame have no damage state;
- the swingarm and the shock are not drawn as moving parts of the body solid (a body solid is one piece); the front end moves
  as the fork solid, the wheels as theirs.

## Viewing a bike

`nfsmw asset-viewer <ID>` animates the wheels (roll and steering about the steering axis, as a car's wheels do); `nfsmw view-car
<ID>` shows the bike at rest, as it does a car. The garage lists the bike with the name and brand of its manifest.
