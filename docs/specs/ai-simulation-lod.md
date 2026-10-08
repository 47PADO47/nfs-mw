# AI simulation level of detail: what runs for far, off-screen and inactive cars

How the original keeps many AI cars affordable: which cars use a cheaper physics model by type, how a racer
or cop that has left the loaded world is moved by a kinematic stand-in ("simple physics"), which per-tick work
is skipped for AI cars, how cars sleep or freeze, and the limits, radii and timers that decide when cars are
active, inactive or removed. The finding to keep in mind: **there is no distance-based re-rating of AI think
or physics** in the original. The only distance-like switch is "is the car off the loaded world", plus the
traffic and cop managers removing cars. Companion of [ai-driver-control.md](ai-driver-control.md) (the
driver) and [vehicle-rigid-body.md](vehicle-rigid-body.md) (the body). For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src`: `Sim/Common/Simulation.cpp`, `Sim/SimTypes.h`, `Sim/Simulation.h`,
  `Sim/SimObject.h`, `AI/Common/AIVehicle.cpp` (simple physics, switching, respawn timers),
  `AI/Common/AIVehicleRacecar.cpp`, `AI/Common/AIVehicleCopCar.cpp`, `AI/Common/AIVehicleTraffic.cpp`,
  `AI/Activities/{AITrafficManager,AICopManager,AvoidableManager}.cpp`, `AI/AISpawnManager.h`,
  `Physics/Behaviors/{RigidBody,RBVehicle,RBCop,Chassis,ResetCar}.cpp`, `Physics/Behaviors/Suspension{Racer,
  Simple,Traffic}.cpp`, `Physics/Behaviors/Engine{Racer,Traffic}.cpp`, `Physics/Behaviors/BehaviorSpecs.cpp`,
  `Physics/{Behavior,PhysicsObject,VehicleBehaviors,PVehicle}.h`, `Interfaces/Simables/{IVehicle,
  ICollisionBody}.h`. Read for understanding; no code copied. Many of the files that would decide the rest
  (`PVehicle.cpp`, `PhysicsObject.cpp`, `Behavior.cpp`) are empty in the decompilation.
- **Data inputs:** AttribSys `pvehicle` (`BEHAVIOR_MECHANIC_*`, `aivehicle`, `rigidbodyspecs`), `system`
  (`SimTasks`), `rigidbodyspecs` (`SLEEP_VELOCITY`, `GRAVITY`), `trafficpattern`. Layouts:
  [attributes.md](../formats/attributes.md).

## 1. Summary

| Mechanism | What it does | Section |
|---|---|---|
| Behaviour set by car type | traffic and trucks use a cheaper engine and chassis; cops a cheaper chassis | 2 |
| Physics mode `EMULATED` | racer or cop that is off the world is moved kinematically at think rate | 3 |
| `IsOffWorld` | the trigger for the above; also removes traffic and cops | 3.4 |
| Collision throttles | traffic world tests every 2nd or 4th tick when slow; AI skips ground tests when all wheels are down | 5 |
| Sleep and freeze | rigid-body sleep, chassis sleep, "not modelled" state | 6 |
| Active, inactive, pooled | only active cars think and drive; traffic is pooled and re-used | 7 |
| Budgets | 64 rigid bodies (52 for low priority spawns), 96 simple bodies, 10 active cars for traffic | 7 |
| Rates | racer think 30 Hz, cop 7.5 Hz, traffic about 6 Hz; unchanged by distance | 4 |

## 2. Physics tiers by car type

The car's `pvehicle` record names the behaviour for each mechanic [verified on the install]. Counting the
collections that share each set:

| Rigid body | Engine | Suspension | AI data | Cars (collections) |
|---|---|---|---|---|
| `RBVehicle` | `EngineRacer` | `SuspensionRacer` | `racers` | 56 collections: all drivable cars, racers, and the challenge-series racer-like cars |
| `RBVehicle` | `EngineRacer` | `SuspensionRacer` | `cssuv`, `semi`, `cscopmidsize` | challenge-series "cops" and trucks |
| `RBCop` | `EngineRacer` | `SuspensionSimple` | `cops`, `copsuv`, `copsport`, ... | all 14 cop collections (the base `cops`, `copmidsize` and its two cutscene variants, and one per other cop car) |
| `RBVehicle` | `EngineTraffic` | `SuspensionTraffic` | `traffic`, `street`, `van`, `truck` | traffic cars (24) |
| `RBTractor` | `EngineTraffic` | `SuspensionTraffic` | `default` | semi-truck tractors (8) |
| `RBTractor` | `EngineRacer` | `SuspensionRacer` | `cssuv` | `cs_semi` |
| `RBTrailer` | none | `SuspensionTrailer` | `default` | trailers (7) |
| `RBVehicle` | `SimpleChopper` | none | `default` | helicopters |

The input behaviour is `PInput` for every collection [verified]; a player's car gets `InputPlayer` at spawn.
This is a **permanent** model choice, not a distance one. The cheaper models [decomp]:

- **Traffic engine:** no clutch slip model, no limiter logic, no nitrous, no induction. Engine speed chases
  `redline * throttle`; torque comes from the torque curve (with an engine-braking curve when off the
  throttle); an automatic box picks the gear; the drive is disengaged (no drive torque, neutral-style
  free-revving) when the gearbox is in neutral, the gear ratio is zero or the car is flipped (up axis y below
  0.3). A `match speed` call picks the gear that fits a given road speed (used when a car is placed moving).
- **Traffic suspension:** four tires with a linear lateral force `-2 * lateral_speed * load * GRIP_SCALE`
  and a longitudinal force from the applied torque, no friction circle, no assists, no tire heat, a speed
  proportional drag `DRAG_COEFFICIENT * speed`, steering `45 degrees * input`; it still has springs and
  dampers per corner so that cars sit and pitch on the road. When the chassis is asleep (below) it stops the
  tires.
- **Simple (cop) suspension:** a spring, damper and anti-roll corner model with the car's own `suspension`
  data, a simplified tire (its own friction formula; skimmed, not specified, see
  [vehicle-suspension-tires.md](vehicle-suspension-tires.md)), no player assists, steering `45..60 degrees * STEERING * input`, optional grip help against oversteer (up to
  `+15 percent` rear traction), and it skips its whole step once the chassis has been fully asleep for 3 s.
  Parts that use the catch-up cheat are inactive for cops (cheat 0).

The racer chassis and engine are the player's models from [vehicle-suspension-tires.md](vehicle-suspension-tires.md),
[vehicle-engine-drivetrain.md](vehicle-engine-drivetrain.md) and [vehicle-steering-assists-aero.md](vehicle-steering-assists-aero.md);
the AI differs only by using the AI steering path ([ai-driver-control.md](ai-driver-control.md) section 4.1) and
not the human input shaping.

## 3. Physics modes and the simple (emulated) physics

`PhysicsMode`: `INACTIVE` 0, `SIMULATED` 1, `EMULATED` 2 [decomp]. A car is `SIMULATED` normally (full model).
`EMULATED` is selected only by the AI behaviour and means "a kinematic stand-in moves the car".

### 3.1 When it is switched on

Every think [decomp]:

- Racer / human class: `want = not player and not animating (cutscene) and not staging and car is off world`.
  Switch on when wanted and off, switch off when not wanted and on.
- Cop class: on when off world, off when back on world (before switching off it lifts the car 1.5 m and
  calls place-on-ground on the current position and heading, twice if the first fails).
- Traffic and others never use it (traffic off the world is despawned instead).

Constants `bRacerSimplePhysics = false` and `bCopSimplePhysics = false` in the sources are unused flags.

### 3.2 What the stand-in does each think

Runs inside the AI update (so at the **think rate**: racers 30 Hz), using the think `dT` [decomp]:

```
dest = drive_target + (0, 1, 0);  dir = unit(dest - position)                 # 3-D, includes the climb
cur  = body speed;  v = cur
if cur > drive_speed:  v = max(cur - 30 * dT, drive_speed)                    # 30 m/s^2 braking
else:                  v = min(cur + accel_table(cur) * dT * lerp(0.5, 1.0, skill), drive_speed)
top  = top_speed * lerp(0.75, 1.0, skill);  v = clamp(v, 0, top)
if gearbox in reverse: v = v * -0.5
position += dir * v * dT
height:  on world: ground height at the point;  off world: (nav cookie centre height + 1) if the nav has a trail else dest.y
         height += max ride height of the four wheels + body half height;   position.y = height
orientation: forward = dir, right = unit(cross(up_axis, forward)), up from the ground normal when the ground
         normal has y >= 0.707 (and the car is on world), else (0,1,0) re-orthogonalised
linear velocity = dir * v;  angular velocity = 0
```

The drive target and drive speed are the ones the race action computed for a normal car (the road navigator
keeps working because the road graph is always loaded). The speed governor and glue logic run as usual; in
the race action the grade term is dropped (it is added only in `SIMULATED` mode) and the catch-up glue skill is
not scaled by `base_skill` while emulated ([ai-driver-speed-skill.md](ai-driver-speed-skill.md) section 8.2). The
skill for the stand-in is the car's skill (cheats do not apply). [decomp]

The rigid-body step for the car is presumably suppressed while emulated (the body is put in the "not
modelled" state, section 6) **[unconfirmed]**: the setter of the mode is in the empty `PVehicle.cpp`. The
evidence is that the chassis and the airborne action both return early when the body is not modelled, and that
the stand-in sets position, orientation and both velocities itself every think.

### 3.3 Switching back

When the car returns to the world (or is disabled while the mode is on), [decomp]:

1. mode back to `SIMULATED` (only if it was `EMULATED`);
2. place the car on the ground at its position and heading (the vehicle reset helper snaps the wheels to the
   road);
3. restore the angular velocity, set the forward speed to the old speed;
4. give the car **1.0 s** of invulnerability ("from physics switch") so the first contacts do not damage it.

### 3.4 `IsOffWorld`

Its body is not in the sources. Every use is consistent with "the car is outside the part of the world that is
loaded (no collision data under it)": the stand-in reads the ground height only when the car is *not* off world;
traffic that is off world is despawned; cops that are off world are removed; and off world is what flips racers
into the stand-in **[unconfirmed]**. It is not a camera-distance test. How far from the player the world is loaded
is a streaming matter ([visible-sections.md](visible-sections.md)). A rewrite should define it as "no collision
section loaded at the car's position".

## 4. Update rates

Constant, set at spawn, never changed for vehicles (no call to re-rate a vehicle's tasks exists) [decomp]. A
task with rate `r` runs every `1/r` ticks; "variable" tasks count render frames
([ai-driver-control.md](ai-driver-control.md) section 2).

| Task | Rate | Mode | Period at 60 Hz |
|---|---|---|---|
| racer / player-car AI think | 0.5 | fixed | 2 ticks (30 Hz) |
| cop AI think | 0.125 | fixed | 8 ticks (7.5 Hz) |
| traffic AI think | 0.1 | variable | about 10 render frames |
| AI drive step (controllers) | 1 | physics | every tick |
| vehicle engine, suspension, rigid body | 1 | physics | every tick |
| avoidable sweep-and-prune | 0.25 | fixed | 4 ticks |
| race action off-path check | 0.25 | fixed (start offset 1.0) | 4 ticks |
| nav road update (current and future road) | cached 0.33 s | | an update per 0.33 s; between, cursor nudges at most each 0.02 s |
| seek-ahead position | cached 0.33 s | | |
| traffic manager | 0.5 | variable | about 2 render frames |
| pursuit manager | 0.25 and 1.0 | variable | |

Hence a far-away racer costs as much think and drive work as one next to the player. The per-car saving is in
the stand-in and in the cheaper traffic models.

## 5. Per-tick work skipped for AI cars

From `RBVehicle` and `RigidBody` (the rigid-body spec has the surrounding code) [decomp]:

- **World collision of traffic.** A traffic car that has not collided yet tests the world only every 4th tick
  when `v^2 < 4` (under 2 m/s), and every 2nd tick when `v^2 < 225` (under 15 m/s); at 15 m/s and above every
  tick. Any car that has hit something ("has had collision") uses the generic rule. A player's car always tests.
- **Ground collision.** For a non-player vehicle with more than two wheels where *every* wheel is on the
  ground and no car-to-car hit has happened, the body-versus-ground test is skipped; the suspension holds the
  car. A player's car, or one with a wheel in the air, tests normally.
- **Collision listeners** (`Sim::Collision`) are registered per participant; the dispatcher reserves for 160
  participants and 256 listener entries.
- **Trigger flags**: AI vehicles raise world triggers with the "other" flag (8), the player with the "player"
  flag (4), cops with an extra bit (`0x20000`), all with `0x20`; world triggers use them to filter who fires.
- **Avoidance** is global once per 4 ticks, not per car per tick.
- Car-versus-car broad phase and resolution are in [vehicle-rigid-body.md](vehicle-rigid-body.md) section 6.

## 6. Sleeping and freezing

Three levels, all in the existing specs [decomp]:

1. **Rigid body states** ([vehicle-rigid-body.md](vehicle-rigid-body.md) section 3): awake (0), asleep (1),
   not modelled (2). Awake becomes asleep when `|v| + |w| * radius < SLEEP_VELOCITY` (1 for cars; 2 for
   default and debris) and more than two contact points. A *vehicle* additionally refuses to sleep while it is
   animating, had an object hit this tick, or has wheels on the ground (unless it is a destroyed non-player
   car resting on at least half its wheels for 2 s). So a healthy parked AI car is never rigid-body asleep:
   its suspension keeps it awake. A sleeping body is not integrated; any force or a hit by an awake body wakes it.
   State 2 is "frozen": no integration or collision; the AI and chassis check it.
2. **Chassis sleep** (`Chassis::DoSleep`, called each tick by the racer, simple and traffic suspensions):
   - *all asleep* when speed `< 0.5 m/s`, every wheel on the ground, `brake + handbrake > 0`, `gas == 0`, yaw
     rate `< 0.25 rad/s` and no collision: the body velocity is damped to zero (`damp(1 - speed)`) and wheel
     spin is set to 0;
   - *lateral* when speed `< 1`, yaw rate `< 0.25`, `gas <= 0`: the lateral velocity and yaw rate are scaled by
     the speed (so the car stops creeping sideways) and force and torque are partly cancelled;
   - the simple chassis counts the seconds it is fully asleep and, past 3 s, skips its tire and spring step.
3. **Inactive vehicles** are not simulated at all (section 7).

The staging state pins a car to the grid: while staging only the vertical velocity survives and the car does
not move sideways or forward (`RBVehicle` begin-frame rule), and the chassis never sleeps.

## 7. Active, inactive, pooled and budgets

- **Active flag.** `IsActive` gates both AI tasks and the physics steps of AI cars: an inactive vehicle runs no
  think and no drive. `Activate` and `Deactivate` toggle it; `UnSpawn` switches the stand-in off, clears the
  goal and deactivates [decomp]. Lists: `VEHICLE_ALL, PLAYERS, AI, AIRACERS, AICOPS, AITRAFFIC, RACERS, REMOTE,
  INACTIVE, TRAILERS`.
- **Pooling.** Traffic cars are created on demand, deactivated when they leave, and re-activated for the next
  spawn of the same car type (`GetAvailableTrafficVehicle`). A new instance is only created when
  `mNewInstanceTimer > SpawnTime` (4 s in the shipped patterns); otherwise an inactive one is reused or the
  spawn waits. A pooled vehicle keeps its body and tires.
- **Budgets.** `Sim::MaxRigidBodies` 64, `MaxSimpleBodies` 96, `MaxVehicles` 30, `MaxCollisionListeners` 160,
  `MaxModels` 2366 (the header notes these were not rechecked for this game) [decomp]. `can spawn rigid body`: if
  the count is above 64 (high priority) or 52 (low priority) it kills the *non-required sleeping* body that is
  **farthest** from the spawn position, then reports whether a slot is free (count below the limit). `can spawn
  simple body`: limits 96 / 72; above 72 it kills required simple bodies until under.
- **Traffic budget.** The manager spawns only while fewer than **10** vehicles are active in total (all
  classes counted; inactive traffic does not count). Per pattern record there are caps (`MaxInstances`,
  percent of the traffic budget).
- **Cop budget** is in the pursuit specs.

### 7.1 Radii and timers (traffic, cops, racers)

| Rule | Value | Who |
|---|---|---|
| Traffic spawn point | ahead of a traffic centre (normally the camera or player) along its heading rotated by up to +-0.125 rad, at `200 + random(-50, 50) + max(speed along heading, 0)` m; oncoming chance `lerp(1.0, 0.5, ramp(speed, 0, 50 m/s))` | traffic manager |
| Spawn blocked | within 150 m (planar) of any traffic centre, within 20 m (planar) of any active car, or no world height under the point | traffic manager |
| `mTrafficMinSpawnDist`, `MaxSpawnDist` | 225 m, 300 m (declared; the spawn point code above is the one used for new cars) | traffic manager |
| Traffic despawn | off the world, **or** off-screen time above `T(d)` **and** farther than `D(d)` from the camera, with `d` the traffic density 0..1 | traffic manager |
| `D(d)` m over `d` = 0 to 1 in 10 steps | 130, 120, 110, 100, 90, 80, 70, 60, 55, 45, 40 | |
| `T(d)` s | 12, 10, 9, 8, 7, 6.5, 6, 5.5, 5, 4.5, 4 | |
| Visible counts as "on screen" | any child model of the car in view | |
| Camera distance | the nearer of the two views' camera distances (second view's weight 1, first scaled by its flag) | `DistanceToCamera` |
| Density | race traffic density setting or 1.0 when roaming, 0 in cutscenes or while a cop request is pending, `x0.75` during a pursuit; spawn rate multiplier `TrafficDensitySpawnRates = [0, 0.05, 0.1, 0.125, 0.2, 0.4, 0.6, 1.0, 3.0, 5.0, 8.0]` | traffic manager |
| Cop spawn distances | `mCopMinSpawnDist` 150 m, `mCopMaxSpawnDist` 400 m (spawn spots ahead of the target use `min + 80`) | cop manager |
| Cop removal | each manager pass, an active cop is removed when spawning is disabled, when it is off world, or when a respawn is allowed (`can respawn` and a spot is available; a wrecked cop off-screen for over 3 s also qualifies) *and* the cop is farther than 375 m (helicopter 600 m) from the point 75 m ahead of its pursuit target; to free a slot the cop that has been off-screen over 5 s and is farthest from the view (over 100 m) is chosen | cop manager |
| Respawn permission | `can respawn`: true when the spawn timer exceeds 8 s and a spawn spot is not currently available, or the timer exceeds 10 s ("never visible") when one is | AI behaviour |

Traffic patterns set `SpeedStreet 35 mph`, `SpeedHighway 55 mph`, spawn start speed `0.75 * min(street, highway)`.
The full traffic rules are in [ai-traffic.md](ai-traffic.md). [decomp, tables read from the constants]

## 8. What a far car receives

A racer that is beyond the loaded world gets exactly the same inputs as a near one: the drive target and drive
speed from the race action, driven by the road graph, skill and catch-up (the glue term is the racer layer's
rubber band, which exists precisely to correct these cars). Nothing about the target changes with distance. The
differences are: the output goes to the stand-in rather than to the controls (controls are still written by
the drive step but nothing reads them while the body is not modelled) **[unconfirmed]**; collision with the
world and other cars is off; nitrous still runs as a decision but has no effect. On re-entry the car is placed
on the road with the speed it had. Cops and traffic are dropped instead of being emulated.

## 9. How to check it

- Log the physics mode of a racer while it leaves the loaded area (ride away from a racer in a race, or
  start it far away): it should flip to emulated, the engine and tires should stop receiving forces, and its
  speed should still obey the governor and `top_speed * lerp(0.75, 1, skill)`.
- Measure a racer's speed through the switch: speed continuity and the 1.0 s invulnerability.
- Count traffic world-test rate: set a hidden counter in the world query; a traffic car at 1 m/s should test
  every 4th tick, at 10 m/s every 2nd.
- Park a racer with the handbrake: confirm the chassis damps to rest at the thresholds of section 6.
- Spawn tests: fill the 10-car budget and verify no new traffic until a car drops out.

## 10. Open questions

1. `IsOffWorld` and `SetPhysicsMode` bodies (empty `PVehicle.cpp`); whether emulated mode freezes the rigid body.
2. Which behaviour class and which task order the spawn code assigns (`PhysicsObject.cpp`, `Behavior.cpp` empty).
3. The speed of the camera distance factor `mCameras[0].w` (set 1 when a view exists).
4. Whether any shipped content ever spawns racers or traffic far enough to use the stand-in for long; the data
   suggests the stand-in matters mostly in race events whose route leaves the streamed area.
5. The relation between the 10-car traffic cap and `Sim::MaxVehicles` 30 (header comment doubts the value).

## 11. Rust implementation notes

- **Vehicle crate:** add `Vehicle::set_modeled(bool)` that sets `BodyState::Frozen` (already exists) and a
  `Vehicle::place_moving(pos, orient, speed)` (exists) for re-entry; add `Vehicle::chassis_asleep()` and
  apply the chassis-sleep rule of section 6.2 in the existing step (the rigid body currently sleeps only by
  the body rule). Expose `max_steering_rad()`, `top_speed()` and `acceleration_at(speed)` (see
  [ai-driver-speed-skill.md](ai-driver-speed-skill.md) section 12) for the stand-in and the governor.
- **Cheaper models** (`TrafficVehicle`: engine and suspension variants) are not required for correctness
  at first; a traffic car can run the full model with the AI controller of ai-driver-control until the
  budget needs them. Keep the `VehicleKind { Racer, Cop, Traffic }` choice on the spec so they can be added.
- **Game crate:** a `SimLod` module with the stand-in step (section 3.2) as a pure function
  `emulate(&mut Pose, drive_target, drive_speed, skill, accel_table, top_speed, dt, ground_height)`;
  budgets as constants; traffic radii/timers as tables in the traffic module.
- **Scheduler:** fixed think periods per class (2, 8, 10 ticks) with the stagger offsets; the avoidable sweep
  every 4 ticks; do not re-rate by distance.
- **Determinism:** the stand-in uses only think-rate integration and table lookups; keep `f32` and fixed order.
