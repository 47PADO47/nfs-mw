# Two-wheel vehicles in the game (`CarSim::ride`, the wheel rig, the manifest)

How the two-wheel dynamics ([vehicle-two-wheel.md](vehicle-two-wheel.md), `libs/blackbox-vehicle/src/bike/`) are carried through
the game so that a mod car with `kind = "bike"` ([bikes.md](../modding/bikes.md)) drives, draws and behaves as a bike wherever the
game handles cars. This is the design of the integration: nothing in it comes from decompiled code, and the dynamics themselves
are not described here. The code is in `crates/nfsmw/src/scenes/world/drive/sim/bike.rs` (the simulation),
`crates/nfsmw-data/src/car/` (`mods/bike.rs`, `wheels.rs`, `assemble.rs`, `physics/bike.rs`) and
`libs/blackbox-modding/src/content/bike.rs` (the schema).

## Layers

| Layer | Where | What |
|---|---|---|
| Dynamics | `libs/blackbox-vehicle/src/bike/` | `Bike`, `BikeSpec`: generic, no game names |
| Schema | `libs/blackbox-modding/src/content/{bike,bike_check}.rs` | the `[bike]` table, its ranges, `impl From<&Bike> for BikeSpec` |
| Glue | `crates/nfsmw-data`, `crates/nfsmw` | registry, derived stats, wheel rig, parts, `CarSim` |

The dynamics were extended additively for the integration (`Bike::begin_body`, `Bike::respond`, `Bike::body`, `Bike::body_mut`,
`Bike::rest_pose`, `Bike::shifting`; `Bike::step` is the first two in turn).

## The body double

Walls, props, other cars, the camera, the sounds and the effects all work on a four-wheel `Vehicle`, 170 uses in 25 files. A bike
keeps one as a **body double** rather than a second code path through each: it has the bike's mass and inertia, the collision box
of the car's own `BOUNDS.BIN`, and it never drives (its `respond_step` is not called, so it has no wheel states, no engine, no
forces). The bike's own `RigidBody` is the real body. The two are rigidly attached and are made equal at the points of a step
where they could differ.

Frames. The bike's body origin is on the ground midway between the contact patches at rest; the double's is the centre of the
collision box (so that the game's contact code finds the box where it expects it); the model's origin is where the model
authors it. With `pivot` the box centre relative to the model origin (`CarBounds::pivot`, physics axes: x right, y up, z forward)
and `origin_x` the distance from the model origin forward to the bike's body origin (midway between the hubs of `[wheels]`, 0
without the table):

```
offset  = pivot - (0, 0, origin_x)                 the double's origin from the bike's, in the body frame
double.position = bike.position + R offset         R: the shared orientation
double.cog      = bike.cog - offset                so both have the same world centre of gravity
```

Both bodies then have the same orientation, the same angular velocity and, because the centre of gravity is shared, the same
linear velocity. The double's mass is the bike's and its inertia scale is `inertia_bike / box_inertia(m, half_dimensions)` per
axis, so a hit that turns the double turns the bike by the same amount. `CarSim::model_origin` (`position - R pivot`) is then the
model's origin for the double exactly as for a car.

### One step

| Phase | Hook | Bike | Double |
|---|---|---|---|
| start of the step (`begin_step_with_delta`) | `pull` | takes the double's pose and velocities: whatever the rest of the game did since the last step (a teleport, a reset, a prediction) | |
| `begin_body` | `Bike::begin_body` | integrates the forces of the last step | `push`: takes the bike's pose |
| pairs, props, walls (`finish_body`) | | | the contacts of the game push it |
| end of the contacts (`finish_body`) | `pull` | takes the result | a fallen bike also meets the ground (`Vehicle::finish_body_step`), so it lies on its box |
| `Integration::run` | `Bike::respond` | rider, engine, tyres and suspension make the forces of the next step | `push` |
| `end_step` | `advance` | the wheels roll on | the readouts are published |

Not called for a bike: `Vehicle::begin_body_step`, `Vehicle::finish_body_step` (except when fallen) and `Vehicle::respond_step`,
which would drop the box on the ground, integrate it a second time and run wheels it does not have. `Integration` carries the bike
so that the response job (also used by the parallel AI job) runs it.

Consequences, by design: an explosion's force on a bike's double is lost (the double's force accumulator is not read; the
velocity changes of contacts are kept), and the box of `BOUNDS.BIN` is the bike's whole collision shape.

## Reporting a bike through the four-wheel accessors

The four-wheel readouts (`wheel_sounds`, `wheel_patches`, `tire_contacts`, `tyre_hits`, `wheel_radii`, `audio_telemetry`,
`physics_wheel_states`, `water_contacts`) read library wheel `i` (0 front left, 1 front right, 2 rear left, 3 rear right) through
`CarSim::wheel_state(i)`: a bike answers with its front wheel for 0 and 1 and its rear wheel for 2 and 3 (model order: front,
front, rear, rear), built from the dynamics' wheel state:

| `WheelState` field | From the bike's wheel |
|---|---|
| `position` | the contact point; for a wheel in the air, the hub less one radius |
| `radius`, `steer_angle` | the spec's radius; the handlebar angle for the front wheel, 0 for the rear |
| `angular_velocity`, `simulated_angular_velocity` | the spin |
| `compression`, `on_ground`, `load` | as reported |
| `slip` | `slip_ratio * max(|v|, 1)`: the tyre model defines the ratio against `max(|v|, 1)` |
| `lateral_speed`, `slip_angle` | `tan(slip_angle) * max(|v|, 1)`; the angle as reported |
| `slide_speed`, `skid`, `smoke` | the library's ramps (`SKID_RANGE`, `SMOKE_RANGE`) of `hypot(slip, lateral_speed)` |
| `traction` | `1 - skid` |
| `locked` | on the ground, faster than 2 m/s and `|spin| radius` under 0.1 m/s |
| `lateral_force`, `longitudinal_force` | the ground's force along the body's right and forward axes |

Wheels 1 and 3 are repeats, so the readouts that list contacts (`tyre_hits`, `tire_contacts`) report a bike's wheels once (0 and
2); the ones that fill a four-wheel array fill it front, front, rear, rear.

Counts of wheels on the ground (`Telemetry::wheels_on_ground`, `CarSim::wheels_down`) read **two per bike wheel**: both down is 4,
one down is 2, none is 0. So the checks written for a car keep their meaning (`>= 4` all down, `> 2` the body is sitting on the
road, `> 0` somewhere on the ground, `== 0` airborne, `>= 3` the road check, the mixer's `/ 4.0`).

Other readouts: `rpm` and `gear` are the bike's (telemetry gear 1 for first, as for a car); `red_line` and `max_rpm` are the rev
limit; nitrous, induction and shift lights are off; the HUD's throttle is the lever; `controls()` (brake lamp, steering and gas
readouts) is what the rider asked in the last step. The camera follows the bike's centre of gravity through the double.

## Input

`DriveInput` (and `InputState`) become the rider's levers:

| Input | `BikeInput` |
|---|---|
| `throttle` | `throttle` |
| `brake` | `front_brake` and `rear_brake` |
| `handbrake` | `rear_brake = max(brake, handbrake)`; the front is the brake pedal alone |
| `steer` | `steer` |
| `shift_up`, `shift_down`, `gear_select`, `clutch`, `nos` | not read |

Every lever is held to its range and a value that is not a number is 0. The input layer has one brake axis, so the front brake alone
and the rear brake alone cannot both be asked for separately by a key; a pad or a script that wants it can use `BikeInput` through
the library. When the race grid holds the car (`Vehicle::staging`) the rider pulls both brakes and ignores the stick. A bike never
selects reverse: the brake holds it.

## The wheels

A bike draws two wheels from two solids (`<ID>_KIT00_FRONT_TIRE_A..D`, `<ID>_KIT00_REAR_TIRE_A..D`), each modelled with its axle
along y and its hub at the origin, centred on y = 0. `WheelSetup::bike` (`BikeMount { rake, trail }`) marks a bike's setup and
`wheels::place_bike` makes the four `Corner`s: the front wheel in corner 0, the **handlebar group** in corner 1 (the front
mount, which turns with the fork and ignores the travel), the rear wheel in 2 and 3 (so every index holds one); **wheel
placements exist only for corners 0 and 2**, so corners 1 and 3 never draw a wheel (corner 1 places the bars solid, below). `CarPose::wheels` keeps its four entries
and a bike fills 0 and 2.

Car space is x forward, y left, z up. With rake `r`, trail `t`, hub `H` of the front wheel and radius `R`:

```
A = (-sin r, 0, cos r)           the steering axis, up and back
P = H + (t, 0, -R)               where the axis meets the ground at rest (the geometry of bike/geometry.rs, in car space)
front wheel = translate(A travel) * rotate_about_axis(P, A, steer) * translate(H) * rotate_y(spin) * scale(rs, ws, rs)
rear wheel  = translate(Z travel) *                          translate(H) * rotate_y(spin) * scale(rs, ws, rs)
```

`travel` is the fork's (or swingarm's) compression, `steer` the handlebar angle (positive left in the model; the dynamics steer
positive to the right and the physics space is left-handed, so the sign flips as for a car's wheels), `spin` the roll, increasing
as the bike goes forward. `rs` and `ws` scale the solid to the radius and width of the setup. `P` does not move with the
steering, so the hub swings round the axis as the real wheel does. The travel is limited to 0.25 m as a car's is.

**The fork and the bars.** A bike may have two more solids per level, modelled in the car's axes with the front hub on their
origin: `<ID>_KIT00_FORK_A..D`, what steers and slides with the suspension (fork legs, fender, caliper), and
`<ID>_KIT00_BARS_A..D`, what only steers (handlebar, levers, grips, headlamp, gauges), as the real bike's skeleton has it
(`BONE_HANDLE` steers, its child `BONE_F_SUSP` also slides). Their parts `<ID>_FORK` and `<ID>_BARS` are in the front and
the rear brake slots (`mods/parts.rs`), the slots a bike's wheels do not use; `CarSim::wheel_poses` fills corner 1 with the
front pose so the bars steer. The bars use corner 1, whose rig has `travels: false` (`wheels.rs`). `assemble.rs` places it as a placement of slot
`FRONT_BRAKE` with corner 0, and every consumer that poses a corner's brake (`CarRig::instances_detail`,
`CarScene::pose_wheels`) poses it with `Corner::posed(pose).1`: `lift * steer * translate(H)`, which is the front wheel's
matrix without spin and without the scale, so the fork turns about the same axis and moves along it by the same travel
as the wheel (tested in `wheels.rs`); the bars solid is placed with corner 1's brake matrix, which turns the same way and
never slides. A bike without these solids has no such placements.

The parts database holds the two solids as two parts of the car's own type, `<ID>_FRONT_WHEEL` and `<ID>_REAR_WHEEL`. The game's
front and rear wheel slots take the same part id, so the rear part is found by its name (`stock.rs`); the front wheel part is the
first match of the slot, as for a car.

The hubs come from the `[wheels]` table (`wheel_layout`, `face_m` not needed) and otherwise from the spec: plus and minus half the
wheelbase, as high as the radius. `nfsmw asset-viewer` animates the pair (`CarScene::pose_wheels`: front steered, both rolling).
Rims are not swapped on a bike (`[customization] rims` refuses it): the stock assembly would give both wheels the same solid.

## Derived stats

The shop's lists, the garage's validity check and the body double read a `CarStats`. A bike has none of its own, so
`car::mods::bike::derived_stats` derives it:

| `CarStats` | From |
|---|---|
| `mass_kg` | bike plus rider plus wheels |
| `front_weight_pct`, `cg_height_m`, `wheelbase_m` | the chassis |
| `track_m` | the width of the inertia box (a bike has no track) |
| `tire_radius_m`, `tire_width_m` | the wheels |
| `idle_rpm`, `redline_rpm`, `torque_curve` | the engine; the curve is the peak times each share |
| `engine_braking` | `friction_torque / peak_torque` |
| `gear_ratios`, `final_drive`, `gear_efficiency` | the transmission |
| `tire_grip` | the lateral grip limits |
| `brake_torque_nm` | the front brake force and the rear brake force times the wheel radius |
| `travel_m` | the fork's travel |
| everything else | the generic car's value |

The motion of the bike does not use these. They also mean that the shop's performance levels act on stats the bike does not read:
buying a part changes the bars, not the bike.

## The fall

The dynamics end a rider's control at the fall limits of the spec (`BikeSpec::fall`: lean, pitch, heading error and angular speed; `Bike::fallen`)
and **the rider leaves the bike** ([ride6-rider-fall.md](ride6-rider-fall.md)): `Bike` drops his mass, his spring and his drag, lets
the levers and the bars go, and hands him over once as a `RiderRelease` (kind of fall, place, velocity). The integration then
(a) refits the double to the bike without him (`BikeRide::sync_double`: its mass, its inertia, its centre of gravity), (b) lets the
double meet the ground, so the bike lies and slides on its collision box with nothing but the ground's friction to slow it, and
(c) after 2 s down, once slower than 1.5 m/s, stands it up with `Bike::place_on_ground` where it lies, facing its heading, the
rider back on it (the double is refitted again). The release goes to the presentation (`CarSim::take_rider_release`,
`CarSim::rider_off`): the rider's ragdoll lives in `drive/rider/fall.rs`, never in the simulation, so a car and the network
prediction see nothing of it. The simulation never stops and never leaves the world.

## Limits

- Free roam and the player only: a bike is refused as a cop, traffic or `both`; `place_traffic_moving` and `place_kinematic`
  refuse a bike if something asks.
- Split-screen and co-op seats step the same `CarSim`, so they should drive a bike (not checked); the network prediction saves
  and restores it whole.
- No tyre damage, nitrous, induction, drift assist, manual gearbox, launch assist, force from explosions. The rider's weight (`Bike::set_weight`, which moves the centre of gravity of the bike and the rider and drives the wheelie and stoppie assist) has no key yet; a host that keeps a second body in step with the bike should take `body().cog()` again after a step, because it moves with the weight.
- The collision shape is the box and the contact set of `BOUNDS.BIN` only; the damage zones act on the double.
- Moving parts are the two wheels, the fork and the bars; the swingarm, the shock and the chain are part of the body solid.

## Choices

Numbers of the integration that are neither data nor a published formula; they are named constants and can change without
touching the dynamics.

| Choice | Value | Where | Why |
|---|---|---|---|
| Time a fallen rider lies before the bike is stood up | 2 s | `GET_UP_AFTER` | the dynamics end at the fall; some time on the ground is needed to read it |
| Speed under which it is stood up | 1.5 m/s | `GET_UP_SPEED` | do not stand a bike up while it still slides |
| Velocity lost per step while fallen | none (it was 8 percent) | | the double's ground friction stops the bike; the old damping stopped it in a second |
| Brake at the grid | both levers 1 | `Integration` | holds a bike, which cannot hold itself, until GO |
| Reported `traction`, `locked` | see the table | `BikeRide::wheel_state` | the library's fields have no counterpart in the tyre model |
| Wheels counted twice | 2 per wheel | `wheels_down` | keeps the four-wheel checks meaningful |

## Verified by tests

`libs/blackbox-vehicle`: the two halves of a step equal a step, the rest pose. `libs/blackbox-modding`: schema parse, every range,
unknown and missing keys, arrays of the wrong length, the conversion to `BikeSpec`, the kind/role/rims rules, `[wheels]` without a
face. `crates/nfsmw-data`: the registry, the derived stats, the wheel setup with and without a layout, the rig (rest pose, roll,
steering about the axis, travel along it, the rear wheel). `crates/nfsmw` (`sim/bike_ride_tests.rs`, `sim/bike_tests.rs`): the
double's mass, inertia and shared centre of gravity, the contact round trip, the input mapping, the stand-up, and a `CarSim` bike
standing, accelerating, turning, braking, falling and being saved and restored on a flat road; `sim/bike_fall_tests.rs`: the rider handed
over once at the fall, the double lighter without him, the bike sliding on its side and stood up with him again. Not tested: a drive in the real
game with a real bike, collisions with walls and traffic, the garage.
