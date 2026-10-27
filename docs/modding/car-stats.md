# Stats of a mod car: `[stats]`, upgrades, bars and nitrous

Part of the [modding catalogue](README.md). A car a mod adds ([api-content.md](api-content.md#cars)) is a car of its own: it
inherits nothing from the 2005 game's cars. What it weighs, how its engine pulls, how its gearbox is spaced, what grip its
tyres have and how hard it brakes is in its manifest, in absolute SI units, and the game builds the car's physics from that
table, from the collision bounds in the car's own `BOUNDS.BIN` ([car-collision.md](car-collision.md)) and from the engine's
documented generic values for what the table does not say. The shop and its paints are in
[car-customization.md](car-customization.md), the sound in [car-sound.md](car-sound.md).

## Stats: `[stats]` in the car manifest

```toml
# cars/fastcoupe.toml
[stats]
mass_kg = 1397.0
front_weight_pct = 38.8            # share of the weight on the front axle
cg_height_m = 0.44
wheelbase_m = 2.45
front_track_m = 1.54
rear_track_m = 1.52
front_tire_radius_m = 0.34
rear_tire_radius_m = 0.343
idle_rpm = 1000.0
redline_rpm = 7800.0
torque_curve = [[1000.0, 360.0], [3300.0, 419.0], [5600.0, 440.0], [7800.0, 390.0]]   # [rpm, N m]
gear_ratios = [3.91, 2.29, 1.55, 1.3, 1.08, 0.88, 0.71]
reverse_ratio = 2.68
final_drive = 3.44
drive = "rwd"                      # rwd, fwd or awd
top_speed_kph = 304.0              # the air drag is solved so that the car reaches this
front_tire_grip = 1.5              # peak friction coefficient
rear_tire_grip = 1.4
front_brake_torque_nm = 1300.0
rear_brake_torque_nm = 1350.0
nitrous = true
nitrous_boost = 0.9
```

Every key is optional. A key left out takes the engine's generic car, a 1450 kg rear-drive saloon of about 290 hp
(`Stats::default`, `libs/blackbox-modding/src/content/stats.rs`). A key outside its range, a key that is not in the table, a
torque curve whose rpm does not rise, an idle at or above the redline and an idle at or above the last point of the torque
curve refuse the car; the log names the key.

| Key | Unit, range | Default | Builds |
|---|---|---|---|
| `mass_kg` | kg, 200 to 40000 | 1450 | body mass |
| `front_weight_pct` | percent, 20 to 80 | 52 | where the centre of gravity is along the wheelbase |
| `cg_height_m` | m, 0.1 to 2 | 0.5 | centre-of-gravity height (the chassis' roll centre) |
| `wheelbase_m` | m, 1 to 6 | 2.7 | wheelbase; the wheels sit at plus and minus half of it, so the model is centred on its wheelbase |
| `front_track_m`, `rear_track_m` | m, 0.8 to 3.5 | 1.55 | track between the tyre centres |
| `front_tire_radius_m`, `rear_tire_radius_m` | m, 0.15 to 0.8 | 0.33 | outer tyre radius (the rim takes what a sidewall of 45 percent of the width leaves, then the aspect ratio makes the radius exact); also the height of the hub |
| `front_tire_width_m`, `rear_tire_width_m` | m, 0.1 to 0.5 | 0.235, 0.255 | tyre width |
| `ride_height_m` | m, 0.03 to 0.5 | 0.12 | gap under the body |
| `idle_rpm`, `redline_rpm` | rpm, 300 to 3000, 1500 to 20000 | 900, 7000 | idle and rev limiter |
| `torque_curve` | `[rpm, N m]`, 2 to 16 points, rpm rising, torque 0 to 6000 | 8 points, peak 359 N m | the engine's torque, sampled to 16 evenly spaced values from idle to the last point |
| `engine_braking` | share of the torque, 0 to 1 | 0.15 | closed-throttle drag |
| `gear_ratios` | 1 to 10 ratios, 0.2 to 10, falling | 6 gears, 3.6 to 0.75 | forward gears |
| `reverse_ratio`, `final_drive` | 0.5 to 10 | 3.4, 3.7 | |
| `gear_efficiency` | 0.5 to 1 | 0.9 | power kept by the gearbox |
| `drive`, `front_torque_share` | rwd, fwd, awd; 0 to 1 | rwd, 0.4 | which axle gets the torque; with `awd`, the front's share |
| `front_diff_lock`, `rear_diff_lock` | 0 to 1 | 0, 0.3 | differential lock |
| `top_speed_kph` | km/h, 50 to 700 | none | the air drag is solved so that the car, at full throttle in its top gear, reaches it (see below) |
| `drag` | N per (m/s)², 0.02 to 3 | 0.35 | air drag, used when there is no `top_speed_kph` |
| `downforce` | N per (m/s)², 0 to 0.1 | 0.003 | downforce |
| `front_tire_grip`, `rear_tire_grip` | friction coefficient, 0.3 to 3 | 1.2 | tyre grip, see below |
| `front_brake_torque_nm`, `rear_brake_torque_nm` | N m per wheel, 100 to 20000 | 1400, 950 | brakes |
| `handbrake_torque_nm` | N m, 0 to 20000 | 1500 | handbrake at the rear wheels |
| `steering_scale` | 0.5 to 2 | 1 | steering range |
| `front_ride_frequency_hz`, `rear_ride_frequency_hz` | Hz, 0.5 to 6 | 3.0, 3.1 | spring rate per wheel, `k = m (2 pi f)²` from the mass on that wheel |
| `damping_ratio`, `travel_m` | 0.1 to 1; 0.03 to 0.4 m | 0.7, 0.1 | dampers (`c = 2 zeta sqrt(k m)`, rebound 1.33 times), wheel travel |
| `induction_boost` | share of the torque at the red line, 0 to 1.5 | 0 | a turbo (for a torque curve without one) |
| `nitrous`, `nitrous_boost`, `nitrous_seconds` | bool; 0 to 3; 0.5 to 30 s | false, 0.5, 5 | [Nitrous](#nitrous) |

What the physics reads that the table does not give (flywheel mass, gear-change times, the body's ground and object contact,
yaw assists, the damper and anti-roll shapes, the downforce centre, wall contact) takes the engine's generic value; the list is
at the top of `libs/blackbox-vehicle/src/stats/build.rs`. The collision box, the contact shapes and the damage zones come from
the car's `BOUNDS.BIN`, and the damage numbers are the shared racing table ([car-collision.md](car-collision.md#damage)). The
wheels are drawn at the hubs the stats give (`WheelSetup::from_stats`): plus and minus half the wheelbase along the car, half
the track plus half the tyre width to the outer face, as high as the tyre radius, with no camber.

- **Tyre grip.** `front_tire_grip` is a friction coefficient (1.0 an ordinary road tyre, 1.5 a sports tyre). The engine's
  tyre model is calibrated on a high-performance tyre whose `STATIC_GRIP` is 2.075 and `DYNAMIC_GRIP` 1.85
  (`docs/specs/vehicle-calibration.md`), which the builder calls a coefficient of 1.65: a car's grip and its lateral curve
  scale with `coefficient / 1.65`. That scale is a calibration, not a measurement of every car (**not yet checked by a skidpad
  run**): if the cars handle too stickily or too loosely, change the `GRIP` constants in `build.rs`, not the cars.
- **Drift.** The keys of [the drift keys](#the-drift-keys) tune the drift assist, a layer a mod can turn on; they change
  nothing while it is off.
- **Top speed.** At the top speed the drag force equals the wheel force of the engine in top gear: the engine speed at a road
  speed follows the game's drivetrain, and `drag = wheel force / speed²`. A top speed the car cannot reach before the rev
  limiter gives the speed at the limiter (a little under it). Upgrades keep the drag the stock build got.
- **The AI.** A computer-driven mod car (a cop, say) may use its whole `top_speed_kph`; without one the AI's default limit of
  200 km/h applies (`ai_vehicle_from_stats`).
- A car with no table is the generic car above.

## The drift keys

A mod can turn on the drift assist of the player's car (`bb.vehicles` 1.2 `set_drift_assist`, [api-world.md](api-world.md#vehicles-and-the-player)):
the handbrake and a turned wheel start a slide, the wheel holds its angle, the rear tyres let go part of their grip. These
optional keys say how a car does it. A car without them has the generic numbers (the median of the 2012 cars), and with the
assist off, which is how the game starts, none of them is read.

| Key | Unit, range | Default | Means |
|---|---|---|---|
| `drift_enter_speed_ms` | m/s, 2 to 60 | 10 | the speed along the car from which a slide counts as a drift |
| `drift_enter_angle_deg` | degrees, 2 to 30 | 7.5 | the slip angle from which it does |
| `drift_exit_angle_deg` | degrees, 1 to 20, under the enter angle | 3 | the drift ends once the slip stays under it for 0.25 s |
| `drift_max_angle_deg` | degrees, 10 to 70 | 40 | the largest drift angle; full steering lock holds half of it |
| `drift_yaw_torque_nm` | N m, 0 to 200000 | 0 | the yaw torque that holds a drift. It is divided by the yaw inertia of the body the physics builds and kept between 3 and 10 rad/s^2; 0 is the generic 6 rad/s^2 |

## Upgrades

The shop offers every category three levels (no junkman parts). A level changes the car's own numbers by a share of
what it has, so a strong engine gains more newton metres than a weak one (`libs/blackbox-vehicle/src/stats/upgrades.rs`):

| Category | Per level |
|---|---|
| Tires | grip + 5 percent of the car's own, both axles |
| Brakes | brake torque + 10 percent |
| Suspension | spring frequency + 6 percent, damping ratio + 0.03, ride height - 8 mm, centre of gravity - 15 mm, downforce + 15 percent |
| Transmission | gear efficiency + 0.03 (at most 1) |
| Engine | torque + 6 percent at every engine speed |
| Induction | `induction_boost` + 0.08 |
| Nitrous | level 1 is the car's factory nitrous (a car without one gets a kit of 0.4 boost for 4 s); each further level + 15 percent boost, + 25 percent burn time |

The physics of an upgraded car are `physics_from_stats(stats, package, bounds)`; the garage preview, the world, split screen
and the AI use the same function.

### The shop's bars

The three bars of the shop (top speed, handling, acceleration) are computed from the car's stats. Top speed and acceleration
are the shop's longitudinal estimate (`docs/specs/garage-performance-bars.md`) of the car built from its stats, put on the
scale of the stock cars (the lowest and highest of the stock cars and their maximum builds are 0 and 1, and a faster car is
clamped to 1). Handling is the score of the car's grip, `0.75 x cornering + 0.25 x braking` on a scale from 1.0 g (0) to
1.9 g (1), where cornering is the mean tyre coefficient less up to 25 percent for a high centre of gravity on a narrow track
(`libs/blackbox-vehicle/src/stats/rating.rs`). The bars of the maximum build are the same estimate for the car with every
category at level 3.

## Nitrous

A stock car of the 2005 game has no nitrous until its owner buys the kit. A mod car with `nitrous = true` has it from the
factory: the extra torque while it burns is `nitrous_boost` times the engine's (`0.9` is 90 percent more) and a full tank
lasts `nitrous_seconds`. When the player buys a nitrous level in the shop, level 1 is the factory nitrous, and the levels above
make it stronger and longer ([Upgrades](#upgrades)); a car without nitrous gets a kit of 0.4 boost for 4 seconds. The
tank refills by speed (20 s at 20 mph, 5 s at 100 mph).

The flames come out of the tail pipes, which the car's body solid marks
([car-customization.md](car-customization.md#tail-pipes)); a car without pipe markers boosts but shows no flame.

## Where the wheels are drawn: `[wheels]`

`[stats]` says what the tyres do. Without more, the game draws a mod car's wheels from it: the tyres' centres at plus and minus
half of `wheelbase_m`, as high as their radius, the outer face half a tyre width beyond the track. A model whose hubs are
elsewhere (a hub height that is not the radius, a tyre mesh that is not the size of the physics cylinder, a body not centred
on its wheelbase) gets an optional `[wheels]` table with the tyres exactly as the model has them, one sub-table per axle
(the left tyre; the right is its mirror), in the car's own space (x forward, y left, z up, the origin of the model):

```toml
[wheels.front]
x_m = 1.4025        # centre of the tyre, forward of the origin (negative behind it)
z_m = 0.334         # height of the centre above the ground the model stands on
face_m = 0.9325     # centre line to the tyre's outer face
radius_m = 0.3211
width_m = 0.2649
[wheels.rear]
x_m = -1.4025
# ... the same five keys
```

Every key of both axles is required when the table is there, and the front axle is ahead of the rear. The game draws the
wheel solid (the car's `FRONT_TIRE`) scaled to each axle's radius and width; the wheel's brake, skid marks and damage
effects follow the same hubs. The table changes nothing of the physics, which stay `[stats]`.

## The 2012 cars' stats

The [2012 mod](../../mods/nfsmw-2012-content/README.md) writes a `[stats]` table for each car, read from the car's own handling definition
([chameleon-vehicle-stats.md](../formats/chameleon-vehicle-stats.md)): mass, weight distribution, wheelbase, track, tyre size,
the torque curve, the gearbox, the drive layout, the tyres' peak grip, the brake torque, the design top speed and the nitrous
strength. It leaves out what the 2012 data gives in units that are not decoded (springs, dampers, steering, downforce, the
handbrake, gear efficiency, engine braking), so those take the generic values above. It also writes the [drift keys](#the-drift-keys)
from the car's `DriftBehaviour` (inert unless the "2012 drifting" setting turns the assist on).

**What was not verified.** Several meanings in the 2012 files are marked **[unconfirmed]** in
[chameleon-vehicle-stats.md](../formats/chameleon-vehicle-stats.md): the torque spline's interpolation, the differential
fields that decide the drive layout, the tyre grip curves, the top-speed fields and the nitrous strength. The converted
numbers are the best reading of the data, not confirmed 2012 values, and the grip calibration above has not been checked
by driving. No converted car has been driven by a person: the checks are the conversion of all 93 cars, the parser accepting
every manifest and unit tests of the physics builder.
