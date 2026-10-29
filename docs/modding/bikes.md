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
lean help, the tuning curves, the rider's spring, the block-fall assist, `rolling_slip`); the tables `[bike.wheelie]`,
`[bike.tcs]` and `[bike.front.thermal]`/`[bike.rear.thermal]` as a whole; and the constructor defaults of the feathering
controller (`[bike.transmission.feather]`). The keys map one to one to `blackbox_vehicle::bike::BikeSpec`
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
rider_k_n_per_m = 20000.0      # optional (rigid): the spring and the damper that hold the rider to the bike
rider_damp = 5000.0
half_dimensions_m = [0.42, 0.64, 1.06]    # the body box: bounding radius, derived stats
tensor_scale = [0.4, 0.45, 0.2]           # radius of gyration squared (m^2) about x (pitch), y (yaw), z (roll); I = scale x total mass
cg_height_m = 0.62             # the bike's own centre of gravity
front_weight = 0.5             # share of the bike's weight on the front wheel at rest
wheelbase_m = 1.47
rake_rad = 0.4538              # steering-head angle from the vertical
trail_m = 0.105

Where the rider sits and how his body is held (optional; every key has the default of the reference game's constructor, a rider
who only shifts along the bike with the weight is the empty table; `docs/specs/ride6-rider-body-aero.md`). Offsets in m, x right,
y up, z forward; speeds in m/s:

```toml
[bike.rider]
base_offset_m = [0.0, 0.0, 0.0]            # added to the rest place always
lateral_offset_m = [-0.12, 0.0, 0.0]       # at full lean or lateral weight: x with the side of the lean, y and z with its size
stopped_forward_offset_m = [0.0, 0.0, 0.0] # these three replace the moving ones below stopped_speed_range_mps
stopped_backward_offset_m = [0.0, 0.0, 0.0]
stopped_lateral_offset_m = [-0.25, 0.0, 0.0]
stopped_speed_range_mps = [0.0, 0.0]       # all stopped below the first, all moving above the second; (0, 0): never stopped
weight_influence = 0.0                     # how much the lateral weight input asks for the lateral shift (default 1)
lean_influence = 1.0                       # how much the planned lean over the lean limit does (default 0)
damp_speed_range_mps = [11.111112, 16.666668]  # the damper goes from damp_multiplier to 1 over this range
damp_multiplier = 1.0
correct_gravity = true                     # raise the spring's rest place so the rider does not sag
inertia_scale = [0.28, 0.19, 0.23]         # m^2 x mass, pitch, yaw, roll
angular_k_n_m_per_rad = 2500.0             # 0 (default): the rider's body is rigid on the frame
angular_damp = 250.0
angular_damp_chassis_factor = 0.0          # 0: the damper works against the world, 1: against the bike
torque_scale = [1.0, 1.0, 1.0]             # per axis, the torque of his body on the bike (fades in over 0..5 km/h)
drag_scale = 1.25
drag_curve = [[0.0, 1.2], [1.0, 0.65]]     # [tuck 0..1, factor]
drag_weight_curve = [[0.0, 0.1]]           # [weight -1..1, coefficient]; none: the rider adds no drag of his own
```

The rider's own air drag is `drag_weight_curve(weight) * drag_curve(tuck) * drag_scale * (1 - slipstream) * |v| v`, on top of the
bike's `drag` (`docs/specs/ride6-rider-round6.md`, section 1). The library takes the lateral weight (`Bike::set_weight_lateral`)
and the tuck input (`Bike::set_tuck`); the game wires no key for the first. With `[bike.wheelie.help.auto_tuck]` the bike tucks
by itself (below). The lean already moves a rider whose `lean_influence` is above 0.
[bike.front]
radius_m = 0.32
width_m = 0.12
toroid_factor = 0.7            # 0 flat tread, 1 round tyre
mass_kg = 17.0
inertia_kgm2 = 0.65
rolling = [0.02, 0.0004, 0.000001]       # share of the load: constant, per m/s, per (m/s)^2 (of the speed of the whole bike)
rolling_slip = 0.2             # optional (0): ... plus this times the lateral slip to the power ...
rolling_slip_linearity = 1.0   # optional (1)
lateral_grip_limit = 1.25      # informational (derived stats); the tyre curve decides the force
tire_stiffness_n_per_m = 400000.0        # optional (0, rigid): radial stiffness, in series with the suspension
max_tire_deformation_m = 0.03            # optional
compression_shape_factor = 1.0           # optional (0): how much the tyre's squash shortens its rolling radius
slip_correction = 0.05         # optional (0): combined-slip damping at speed ...
rest_slip_correction = 0.25    # ... and at rest, over rest_speed_threshold_mps
rest_speed_threshold_mps = 6.94
# [bike.front.thermal]         # optional: the temperature and wear of the tread (ride6-tyre-thermal.md)
# sections_dependency, sections_angle_rad, heat_transfer (0), initial_thickness, min_temperature_c, max_temperature_c,
# asphalt_importance, wear_length_m, uniform, static_time_scale, dynamic_time_scale, decay_distance_factor, wear_intensity,
# then one [[bike.front.thermal.sections]] per strip: efficiency_limit, static_coef, dynamic_coef, lat_slip_coef,
# long_slip_coef, rolling_coef and the curves temperature_to_wear, wear_to_efficiency, temperature_to_efficiency
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
brake_reduction_factor = 1.0   # optional (0): the engine drops its drag as the rear slides; with _load = [1.0, 0.0, 1.0, 0.9]
brake_reduction_slip = [0.0, -0.33, 0.0, 1.0]   # and _slip, each [in low, in high, out low, out high] (left out: 1 for every input)
[bike.transmission]
gears = [2.1, 1.65, 1.35, 1.12, 0.96]    # first gear first, falling
final_ratio = 7.2              # gearbox output to the rear wheel, chain included
efficiency = 1.0
up_tolerance = [1.0, 1.0, 1.0, 1.0, 1.0]      # per gear: the automatic gearbox generates its change points from the torque curve and the
down_tolerance = [0.4, 0.9, 1.0, 1.0, 1.0]     # ratios; these scale the torques it compares (ride6-gearbox-auto.md)
prevent_up_lean = [0.5, 30.0]  # optional: [factor, linearity] hold-offs of the changes (lean, throttle)
prevent_down_lean = [0.5, 50.0]
prevent_down_throttle = [1.0, 0.5]
up_shift_time_s = 0.06
down_shift_time_s = 0.15
blip_throttle = 0.3            # optional (0): throttle blip on a change down
clutch_linearity = 1.0
clutch_max_factor = 1.0        # most torque the clutch carries, x peak torque
clutch_timing = [0.125, 0.2]   # optional ([0, 0]): fractions of a gear change: clutch out by the first, back from the second
throttle_timing = [0.05, 0.9]  # optional ([0, 0]): the throttle is shut between the two fractions
release_throttle_on_shift = false      # optional (false): no throttle timing, throttle shut for the change
modulate_throttle_on_upshift = false   # optional (true): no throttle timing, throttle follows the clutch on a change up
press_clutch_at_min_rpm = true         # optional (true): the clutch is pulled as a coasting bike slows to the idle road speed
min_rpm_speed_scale_range = [1.05, 1.1]  # optional: multiples of that speed, fully pulled to fully let go
slipper_filter_speed = 0.0     # optional (0): filter of the slipper clutch, 0 for none
over_limit_rpm_share = 0.94    # optional (the reference's constructor defaults): over_limit_enabled, _min_speed_mps, _change_time_s, _threshold_s, _rpm_share
force_downshift_factor = 1.0    # optional (0 = off): hard braking at speed forces a change down; also _brake_range = [0.5, 1.0],
force_downshift_duration_s = 0.3   # _duration_s, _linearity = 3.0 and _speed_range_mps = [41.7, 41.7]
slipper_range_nm = [25.0, 40.0]          # optional (none): back torque over which the slipper opens
slipper_intensity = 1.0
release_speed_mps = 13.9       # optional (0): road speed above which a start is over
feather_factor = 0.85          # optional: launch engine speed, x the rpm of peak torque, at most feather_max_factor x max_rpm
feather_max_factor = 0.7
launch_rpm = [0.4, 0.98]       # optional: the launch clutch without feathering, shares of the rev limit
[bike.transmission.feather]    # optional (the constructor defaults): the feathering controller of a start
delta_time_s = 0.05            # also target_tolerance, exit_tolerance, throttle_cap, min_slip, rpm_raise, release_factor
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
[bike.brakes.abs.reduction]    # optional (off): the brake reduction; speed_kmh = [10.0, 25.0, 0.0, 1.0] is [in low, in high, out low, out high]
speed_kmh = [10.0, 25.0, 0.0, 1.0]
[bike.brakes.abs.reduction.rear]   # also .front; a term left out is 0: intensity, lean (share of the limit), drift (deg), load (over the load
intensity = 1.0                    # at rest) and throttle, each [in low, in high, out low, out high]; the sum of lean, drift and load times
lean = [0.0, 0.9, 0.0, 0.5]        # the throttle term is what the wheel loses
drift = [0.0, 35.0, 0.0, 0.2]
load = [1.0, 0.0, 0.0, 0.8]
throttle = [0.0, 0.05, 1.0, 0.0]
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
help_stopped_range_mps = [0.0, 2.78]    # optional: between these speeds the help factor goes from 1 (standing) to help_speed's
help_balance_speed = [[2.78, 0.0], [8.33, 1.0]]   # optional: [m/s, share of the help that is a force at the front contact; the rest
                                        # is a roll torque]; empty: all force
help_skid_range_rad = [0.1745, 0.7854]  # optional: the skid cut of the help factor (heading error where it starts and is full) ...
help_skid_linearity = 2.0
help_skid_reduction = [[0.0, 0.0], [8.33, 1.0]]   # ... cut against the speed (no curve, no cut) ...
help_skid_input = [[-0.1, 0.0], [0.0, 1.0]]       # ... and its factor against throttle less brake (empty: 1)
stability_speed = [[0.0, 0.05], [22.2, 0.05], [33.3, 0.0]]   # optional (no stability help): [m/s, factor]; a yaw torque towards g tan(lean) / v
stability_lean_influence = -1.0       # the factor times max(0, |lean share|^stability_lean_sensitivity x this + 1); sensitivity default 1
stability_throttle_reduction = [[0.0, 0.0], [0.014, 1.0]]   # [m/s, cut]: times the throttle; stability_brake_reduction likewise
stability_speed_range_mps = [2.78, 4.17]   # default; below the first there is none
stability_max_lean_rad = 1.309        # default; the lean the desired yaw rate is held to
stability_terrain_normal = true       # the yaw axis is the road's up
terrain_speed = [[2.78, 0.0], [13.9, 1.0]]   # optional: [m/s, factor] on the road's bank in the lean limit; terrain_max_angle_rad (1.571),
terrain_increment = 0.667             # ... terrain_increment and terrain_reduction (default 1) are the shares of the bank added to the limit
terrain_max_angle_rad = 0.2618
[bike.abs_level]                # optional: how the rider's ABS level (key J) reaches the brake; curves are [level, factor]
default = 1.0                   # the level a bike starts with
front = [[0.0, 0.1], [0.25, 0.2], [0.5, 0.45], [0.75, 1.0], [1.0, 1.0]]      # factor on the front ABS intensity (empty: the level)
# rear = [...]                  # factor on the rear ABS intensity (empty: the level)
rear_enabled = [[0.0, 0.0], [0.25, 0.0], [0.5, 1.0], [1.0, 1.0]]             # the rear works where this is 0.5 or more (empty: always)
brake_relief = [[0.0, 1.0], [0.25, 0.75], [0.5, 0.5], [0.75, 0.0], [1.0, 0.0]]   # factor on the start of the stoppie help's brake relief range
[bike.fall]
lean_rad = 1.2217
pitch_rad = 1.1345
speed_direction_rad = 1.4835
angular_speed = 4.4
min_speed_mps = 2.78
block_ratio = 0.975            # optional (off): the block-fall assist holds the target lean to this share of lean_rad ...
block_min_speed_mps = 11.1     # ... above this speed, for block_time_s, then waits block_wait_s
block_time_s = 1.0
block_wait_s = 5.0
terrain_influence = 1.0        # optional: the road's bank (rad, positive when lower on the right) shifts the lean the tests see by this share
terrain_reduction = 1.0        # ... or this share when the bank and the lean lean opposite ways
terrain_max_delta_rad = 0.87   # ... held to this shift (0: no hold)
lean_by_grip = true            # optional: the ground fall lean limit follows atan(grip) (lean_rad is its start and the airborne limit) ...
lean_rise = 0.025              # ... by this share of the gap per step when it is above ...
lean_fall = 0.01               # ... and this when below
lean_duration_s = 0.01         # the lean must stay beyond the limit this long on the ground
airborne_lean_rad = 1.2217     # the limits with no wheel on the road (0: the ground limits)
airborne_pitch_rad = 1.1345
airborne_angular_speed = 7.0
highside_lean_rad = 1.2217     # optional (0: never): a velocity against the heading with the lean under this is a high side
[bike.tcs]                     # optional: the traction control
slip = [0.09, 0.2]             # slip ratios where it starts to cut the throttle and where it cuts it fully
speed_range_mps = [0.0, 1.39]  # the speeds over which it comes in
[bike.input]                   # optional (left out: the axes follow at once): rates per second of the rider's axes
throttle_rise = 3.0            # a key takes 1/rise s to open the throttle and 1/fall s to close it; the same for the brake levers
throttle_fall = 5.0
brake_rise = 3.0
brake_fall = 5.0
steer_rise = 25.0              # the stick away from the middle, in either direction, and back to it
steer_fall = 6.0
# [bike.wheelie] (optional): wheelie and stoppie assist and the rider help. [bike.wheelie.wheelie] and [bike.wheelie.stoppie]
# have deadzone, max_angle_rad, speed_ramp_mps, help_speed, lean_help, lookahead_s, min_speed_mps, acceleration,
# acceleration_time_s, ground_range_m, lean_limit_rad, lean_pitch_range_rad; [bike.wheelie] itself has
# throttle_pitch_rad, forced_throttle_pitch_rad, throttle_pitch_speed, brake_pitch_rad, brake_pitch_speed (a [0, 0] range is off).
# The two parameter sets also take terrain_influence (the share of the road's slope taken off the pitch) and
# target_filter_timescale (1/s; 0: the target angle is not filtered).
# [bike.wheelie.help] (optional, every key optional): launch_speed_mps, launch_throttle_pitch_rad, launch_throttle_multiplier,
# throttle_multiplier, forced_brake_pitch_rad, brake_multiplier, throttle_pitch_electronic, throttle_speed_electronic,
# brake_pitch_electronic, brake_speed_electronic, intensity, weight_deadzone, lookahead_s; and [bike.wheelie.help.auto_rider]:
# wheelie_range_rad, stoppie_range_rad, multiplier, pitch_speed_range, throttle_curve, throttle_gain, brake_curve, brake_gain,
# engage_speed, manual_engage_speed. Without the auto_rider table the weight is the player's. Meaning: docs/specs/ride6-tricks.md.
# [bike.wheelie.help.auto_tuck] (optional; every key needed): min_speed_mps, max_normalised_lean (negative: not tested),
# min_throttle, max_brake: the rider tucks in above that speed, under that lean, over that throttle, under that front brake.
# [bike.wheelie.help] also takes tuck_raise_rate and tuck_fall_rate (1/s, default 2): the tuck level the animation reads.
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
| `input.*` (the six rates) | 0 to 100000 |
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
| `transmission.gears` | 1 to 10 ratios, 0.2 to 20, each lower than the one before |
| `transmission.final_ratio`, `efficiency` | 0.5 to 30, 0.5 to 1 |
| `transmission.up_tolerance`, `down_tolerance`, `prevent_*`, `launch_rpm` | up to one per gear, 0.1 to 10; 0 to 100; 0 to 1.5 |
| `transmission.*_shift_time_s`, `clutch_linearity`, `clutch_max_factor` | 0 to 2 s, 0.1 to 10, 0.1 to 20 |
| `brakes.front_force_n`, `rear_force_n` | 0 to 30000 N, together at least 100 N |
| `steering.max_angle_rad`, `response`, `damping_ratio` | 0.1 to 1.5 rad, 1 to 10^6, 0 to 20 |
| `steering.max_lean_rad` | 0.2 to 1.3 rad |
| `steering.lean.*` | stiffnesses 1 to 10^6, damping ratios 0 to 20, curves up to 32 points with the first number rising |
| `fall.lean_rad`, `pitch_rad`, `speed_direction_rad`, `angular_speed` | 0.2 to 1.5, 0.2 to 1.5, 0.2 to 3.2, 0.5 to 30 |
| `fall.block_*` | ratio 0 to 1, speed 0 to 100 m/s, times 0 to 60 s and 0 to 600 s |
| `fall.airborne_*_rad`, `fall.highside_lean_rad` | 0 to 3.2 |
| `*.thermal`, `*.rolling_slip`, `chassis.rider_k_n_per_m`, `rider_damp`, `tcs` | 1 to 9 strips with curves of up to 32 points, temperatures -100 to 1000 deg C; 0 to 10; 0 to 10^7 N/m, 0 to 10^6 N s/m; slips 0 to 5 |

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

**Exhaust flames.** A bike pops and flicks flames like a mod car ([exhaust-flames.md](../exhaust-flames.md#bikes-the-ride-6-mod)) at
the markers `EXHAUST`, `LEFT_EXHAUST` and `RIGHT_EXHAUST` of its body solid ([the tail pipes](car-customization.md#tail-pipes);
the marker's z axis is the way the flame shoots), one for each silencer, in every level of detail. The flames are drawn at
0.4 of a car's size and a down-shift with the pedal up flicks them. A bike without a marker shows no flame. The RIDE 6 importer
finds the outlets in the silencer mesh ([ride6-bikes.md](ride6-bikes.md#the-exhaust-flames)).

## The rider

A bike gets a rider when the mod has rider files ([formats/ride6-rider.md](../formats/ride6-rider.md)): one **rig** shared by
every bike (`files/RIDER/`: skeleton, animations, meshes, pictures, `attach.json`) and, per bike, **contact points**
(`files/CARS/<ID>/RIDER/contacts.json`): the seat the rider's root sits on, the two hubs, and where his hands and feet go (the
left and right grip, the left and right peg), in the bike's own frame. The game lays the bike's contact points over the rig's
`attach.json`, so a sport bike's clip-ons, a cruiser's high bars and a scooter's forward footboards each get hands and feet on
their own controls; the animation itself (the pose, the lean, the tuck) is the rig's and plays the same on every bike. A bike
without `contacts.json` reaches the rig's own points; a rider needs only `attach.json` to be drawn. If the arms cannot reach a
bike's grips the body leans forward until they can (at most about 29 degrees), as the original's full-body solve moves the spine.
A `contacts.json` the game cannot use (another version, a point more than 1.5 m from the seat) is logged and ignored.

`ride6-bike-import` writes `contacts.json` for every bike it imports, from the bike's skeleton in `parts.json`
([ride6-bikes.md](ride6-bikes.md#partsjson-schema-1): `bones{}` and `sockets{}`); `ride6-bike-import contacts <exporter output>
<mod folder> --physics <dump>` writes only those files for bikes already imported ([ride6-bikes-batch.md](
ride6-bikes-batch.md#the-riders-contact-points-contacts)). Another game's converter writes the same file from its own skeleton.

## How it drives

| Input | Bike |
|---|---|
| Throttle | the throttle (cut by the rider help when the front lifts, if `[bike.wheelie]` is there) |
| Brake pedal | the front and the rear brake together (and the linked brakes of `joined_factor`) |
| Handbrake | the rear brake alone (the input layer has one brake axis, so the front brake alone has no key) |
| Rider weight (`rider_weight` axis) | forward / back: a stoppie under the front brake, a wheelie under the throttle (below). **Keys: G forward, V back (Numpad 8 / 2). Pad: the left stick, Y: up forward, down back, together with X steering.** |
| ABS level (`abs_level` button) | **Key J, pad left-stick click:** each press goes to the next lower level, 1, 0.75, 0.5, 0.25, 0, then 1 again (the new level is written to the log, `ABS level 0.5`) |
| Steering | the stick: the rider's lean controller prescribes the lean the stick points to and picks the handlebar angle that makes the front tyre give the side force it needs (counter-steering included) |
| Gear shift keys, gear keys, clutch | not read: the gearbox is automatic |
| Nitrous | not read: a bike has none |
| Reset | as for a car |

Behaviour specific to bikes:

- **No reverse.** The brake stops the bike and holds it; it never selects reverse.
- **Falling.** A lean beyond the ground limit (`fall.lean_rad`, or the grip-following one), a pitch beyond `fall.pitch_rad`, an angular
  speed beyond `fall.angular_speed` (all at any speed), or (above `fall.min_speed_mps`) a heading error beyond its limit is a fall. The rider **leaves the bike**: his mass and his spring stop acting on it (the bike goes on under its own mass), the
  throttle, the brakes and the bars are let go, and he is thrown as a ragdoll at the velocity the bike had two steps before (a high
  side also throws him 1.7 m/s up; a rider without `RIDER/ragdoll.json` stays on the bike). The bike slides on its side on its
  collision box, slowed only by the ground's friction, and once it has lain still for 2 s is stood up again where it is, facing the
  way it did, with the rider back on it. A velocity against the heading with the lean under `fall.highside_lean_rad` is a high
  side, otherwise a low side. The fall is not damage ([ride6-rider-fall.md](../specs/ride6-rider-fall.md)).
- **Race grid.** The rider holds the bike with both brakes while the race is staged; there is no launch assist.
- **Gauges and sound.** The tachometer, the gear display and the engine sound follow the bike's rpm and gear; the engine loops are
  the car's own `[engine_sound]` or the built-in engine.
- **Wheels on the ground.** Systems that count wheels read a bike's two wheels as four (each counts twice): both down is "all
  four", one down is "two", so the airborne and fall checks and the audio mixer work unchanged.
- **Rider weight, wheelies and stoppies.** Only a bike does them ([ride6-tricks.md](../specs/ride6-tricks.md)); a car has no such
  input. The `rider_weight` action is an axis, -1 (back) to 1 (forward): **G / Numpad 8** forward, **V / Numpad 2** back, or on a
  pad the **left stick** up / down: pull it down for the weight back (wheelie), push it up for the weight forward (stoppie), and
  it still steers with the stick's X, so a bike can lean through a corner on one wheel (the stick's circular gate is opened to a
  square for this axis, so a stick at the rim on the diagonal gives the full lever; the right stick is left to a car's gear
  shifts). For a **wheelie**: be above
  15 km/h (the data's `MinSpeed`), hold the weight back beyond the auto-rider's dead zone (the dead zone of the help is 0.3, the
  auto-rider lets the player have the weight beyond 0.75, so push the lever firmly) and open the throttle in a low gear: the
  front lifts, the help holds the angle the lever and the throttle ask for (up to 35 degrees at full lever and throttle, and the
  rider help cuts the power through the engine beyond 33 to 45 degrees). For a **stoppie**: weight fully forward beyond 0.95, the
  throttle shut and the brake pedal down: the rear lifts and the help holds up to 45 degrees; the rider help eases the front brake
  if it goes further. **Set the ABS to 0 first** (press **J** or the left-stick click until the log says `ABS level 0`): the ABS
  of the reference stays on through a stoppie and, at level 1, holds the front brake below the force that lifts the rear. At
  level 0 the front ABS is down to a tenth of its intensity and the rear's is off. With the weight left alone the auto-rider keeps the bike down: forward under the throttle, back under the
  brake, and against any lift beyond about 4 degrees. The weight does not travel on the network (a network seat sends none).
  Rebind the axis with the `[bindings]` table (`rider_weight`).

## Not done

Documented limits (each is a decision of the integration, not of the dynamics, and is in
[vehicle-two-wheel-integration.md](../specs/vehicle-two-wheel-integration.md#limits)):

- AI, traffic and police bikes; a rider model and its animation;
- the shop's upgrade levels change nothing of a bike (they act on a four-wheel car's stats); the shop's bars are estimates from
  the derived stats;
- no tyre damage (spike strips do not puncture a bike), no nitrous, no drift assist, no manual gearbox;
- the damage zones of the car physics hit the collision box; the bike's tyres and frame have no damage state;
- the swingarm and the shock are not drawn as moving parts of the body solid (a body solid is one piece); the front end moves
  as the fork solid, the wheels as theirs.

## Viewing a bike

`nfsmw asset-viewer <ID>` animates the wheels (roll and steering about the steering axis, as a car's wheels do); `nfsmw view-car
<ID>` shows the bike at rest, as it does a car. The garage lists the bike with the name and brand of its manifest.
