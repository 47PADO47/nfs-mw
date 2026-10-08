# AI driver control: how a computer-driven car is composed and how it drives

How a non-player car (racer, cop, traffic, and the player's own car when it is on autopilot) turns "go to that
point at that speed" into the same gas, brake, handbrake, steering and nitrous inputs a human produces. This
file covers the composition of an AI driver (behaviour, goal, action), the update schedule, the simple
controller used by traffic, reversing, stuck recovery, airborne and wrecked cars, the output interface and
the `aivehicle` tuning data. The PID controllers that racers and cops use are in
[ai-driver-control-pid.md](ai-driver-control-pid.md); the speed target, performance matching, skill and nitrous
choice are in [ai-driver-speed-skill.md](ai-driver-speed-skill.md); distance simulation and far cars are in
[ai-simulation-lod.md](ai-simulation-lod.md). For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; mostly the
  GameCube build), under `src/Speed/Indep/Src`: `AI/AIVehicle.h`, `AI/Common/AIVehicle.cpp`,
  `AI/Common/AIVehicleRacecar.cpp`, `AI/Common/AIVehicleTraffic.cpp`, `AI/Common/AIVehicleCopCar.cpp`,
  `AI/Common/AIVehiclePursuit.cpp` (update rate only), `AI/AIGoal.h`, `AI/Common/AIGoal.cpp`, `AI/AIAction.h`,
  `AI/Common/AIAction.cpp`, `AI/Actions/{AIActionGetUnstuck,AIActionAirborne,AIActionNone,AIActionSpline,
  AIActionStopShort,AIActionTooDamaged,AIActionTraffic,AIActionRace}.cpp`, `AI/AITarget.h`, `AI/Common/AITarget.cpp`,
  `AI/AIAvoidable.h`, `AI/Activities/AvoidableManager.cpp`, `AI/AISteer.h`, `AI/Common/AISteer.cpp`,
  `AI/AIMath.h`, `AI/Common/AIMath.cpp`, `AI/aireflectedtypes.h`, `Sim/Common/Simulation.cpp`,
  `Sim/SimTypes.h`, `Interfaces/Simables/{IINput,IVehicle,ICheater}.h`, `Physics/Behaviors/PInput.{h,cpp}`,
  `Physics/Behaviors/{SuspensionRacer,SuspensionSimple,SuspensionTraffic,EngineRacer}.cpp` (only where they read
  the AI's output). Read for understanding; no code copied.
- **Data inputs:** AttribSys classes `aivehicle` (per car, reached through `pvehicle.aivehicle`),
  `collisionreactions`, `pvehicle` (`BEHAVIOR_MECHANIC_*`, `BEHAVIOR_ORDER`), `system` (`SimTasks`), `tires`
  (`STEERING`). Field lists: [attributes.md](../formats/attributes.md).
- **Neighbours:** the road graph, lanes, cursor and path finding are in [ai-road-network.md](ai-road-network.md)
  and [ai-pathfinder.md](ai-pathfinder.md); the look-ahead trail, the occluded steering point and the curvature
  number are in [ai-road-nav-trail.md](ai-road-nav-trail.md). This file only uses their outputs: a point to
  steer at, a forward vector and a curvature. Racer goals and rubber banding live in the racer specs, pursuit
  goals in [ai-pursuit.md](ai-pursuit.md) and [ai-pursuit-heat.md](ai-pursuit-heat.md), traffic in
  [ai-traffic.md](ai-traffic.md).

Conventions as in [vehicle-rigid-body.md](vehicle-rigid-body.md) section 0: +y up, +z forward, +x right, metres,
seconds, radians; speeds in m/s. Steering is signed +right, -left.

## 1. Composition of an AI driver

A car has one **AI behaviour** among its physics behaviours (the others are rigid body, input, engine,
suspension, damage, draw, sound; the order is the car's `BEHAVIOR_ORDER`). The AI behaviour does not move the
car. It writes the car's input controls (`gas`, `brake`, `handbrake`, `steering`, `nos`) through the same input
interface the human's input behaviour uses (section 6). Everything downstream (clutch, gearbox, tires, steering
geometry) is the normal vehicle model.

```
AI behaviour (one per car)            -- drives: turns "target point + speed" into controls
  owns a goal (by name)               -- a set of candidate actions
    goal picks one action             -- by score, among those that "can be attempted"
      action decides target point, target speed, nitrous, and which controllers run (flags)
  owns a road navigator (drive-to nav), a target (AITarget), an avoidable record, tuning (aivehicle)
```

Behaviour classes found [decomp]; which one a car gets is decided when the car is spawned, in code that is
empty in the decompilation, so the mapping below is the evident one **[unconfirmed]**:

| Class | Used for | Controller | Think rate (section 2) |
|---|---|---|---|
| empty (`AIVehicle` with no goal) | parked or inert cars | none | 1.0, variable |
| traffic | `DRIVER_TRAFFIC` | simple bang-bang (section 4) | 0.1, variable |
| racecar | `DRIVER_RACER`, also the base of the others | PID | 0.5, fixed |
| human | `DRIVER_HUMAN`, `DRIVER_REMOTE` (the player's car) | PID, only when autopilot or drag steering | 0.5, fixed |
| cop car | `DRIVER_COP` | PID (via the pursuit class) | 0.125, fixed |
| helicopter | `CHOPPER` class | own | see the pursuit specs |

Driver classes (`DriverClass`): human 0, traffic 1, cop 2, racer 3, none 4, nis 5, remote 6. Driver style:
racing 0, drag 1. [decomp]

### 1.1 Goals

A goal is a named bag of actions. The behaviour holds the current goal name (a hash of the goal name,
Jenkins lookup2, case-sensitive, [verified] against `aivehicle.PlayerCollisions`) and instantiates it. Changing
the goal replaces the object; setting the same name does nothing. [decomp]

| Goal | Actions, in insertion order |
|---|---|
| `AIGoalNone` | none-action (never attempted, always finished) |
| `AIGoalTraffic` | traffic |
| `AIGoalPatrol` | traffic, too-damaged |
| `AIGoalRacer` | race, get-unstuck |
| `AIGoalPursuit` | pursuit-off-road, race, traffic (not for the player), too-damaged, get-unstuck, airborne |
| `AIGoalFleePursuit` | race, too-damaged, get-unstuck, airborne |
| `AIGoalRam`, `AIGoalPit` | ram, pursuit-off-road, race, too-damaged, get-unstuck, airborne |
| `AIGoalHeadOnRam` | head-on-ram, race, too-damaged, get-unstuck, airborne |
| `AIGoalStopShort` | stop-short, too-damaged, get-unstuck, airborne |
| `AIGoalStaticRoadBlock` | static-road-block |
| `AIGoalHeliPursuit`, `AIGoalHeliExit` | helicopter actions, too-damaged |

Constructing a goal calls `choose action` once with `dT = 0`. The goal names are used by the pursuit specs.
The racer goal has no airborne or damage action; traffic has no stuck recovery. (There is also a spline
action that is never attempted and does nothing, and a none-action; both are stubs.) [decomp]

### 1.2 Choosing an action

Each action has a fixed **score** and answers `can be attempted(dT)`, `is finished`, `begin`, `finish`,
`update(dT)`. Scores [decomp]: get-unstuck 1.0, too-damaged 1.0, race 0.0, airborne 0.0, stop-short 0.0,
none 0.0, traffic 0.1.

```
choose_action(dT):                              # runs at the start of every goal update
  done  = current.is_finished()   if current else false
  score = current.score           if current else 0
  new = none
  for a in actions (insertion order):
      if a != current and a.can_be_attempted(dT):
          if done or a.score >= score:          # ties go to the later action
              new = a; done = false; score = a.score
  if new: current.finish(dT); current = new; new.begin(dT)
goal.update(dT): choose_action(dT); current.update(dT)
```

So a higher-scored action pre-empts the current one at once; an equal score also pre-empts (so airborne can
take over from race); a lower-scored candidate only takes over after the current action reports finished.
`can be attempted` is only evaluated for actions that are not current, and some have side effects (the stuck
timer, section 7). Race and traffic never finish, so they are left only by a higher score. [decomp]

Which actions are racer-specific (race: route following, nitrous) is in [ai-driver-speed-skill.md](ai-driver-speed-skill.md);
traffic's is in section 4 and [ai-traffic.md](ai-traffic.md); the pursuit actions are in the pursuit specs.

## 2. Update schedule

Fixed simulation tick 1/60 s multiplied by the time scale ([vehicle-rigid-body.md](vehicle-rigid-body.md) section 1).
Within one tick the tasks run in the order of `system/default.SimTasks` [verified]:

`SimStart, Comment, AIVehicle, Physics, AvoidableManager, AITrafficManager, AICopManager, AIPursuit, FX,
WorldUpdate, Speech, AIParkedCarSpawner, SimEnd, GameplayActivity`

`SimStart` integrates the rigid bodies with last tick's forces; `WorldUpdate` fetches the human's input.
There are two AI tasks per car:

1. **Think** (schedule `AIVehicle`): updates the goal, which picks the action, which sets the **drive
   target**, **drive speed** and **drive flags**; also the housekeeping of section 8. Skipped while the car is
   paused or the online race has ended; the drive flags are cleared first and the update only runs for an
   *active* vehicle. Rates in the table above. [decomp]
2. **Drive** (the behaviour's per-tick physics step, schedule `Physics`, rate 1): if the car is active and the
   drive flags are not zero, run the reverse, steering and gas/brake controllers once with the tick's `dT` and
   write the controls. [decomp] The flags keep their value between thinks, so with a 30 Hz think the target
   point and speed change every second tick, but the controllers (PID memory, steering) step every tick.
   Whether the AI's drive step runs before the car's engine and suspension in the same tick follows the
   behaviour order (`RIGIDBODY, AI, INPUT, ENGINE, SUSPENSION, ...`) **[unconfirmed]**; the rewrite should run
   it before them so the controls are never a tick old.

A task with `rate r` accumulates `r` every tick and runs when the sum reaches 1, so `r = 0.5` runs every second
tick, `0.125` every eighth, `0.1` every tenth. A fixed task passes `dT_sim / r` (the true interval, 1/30 s,
1/7.5 s). A *variable* task counts only on ticks where a new render frame has begun and is meant to receive
the render-frame time; the decomp passes its schedule accumulator instead (flagged unsolved) **[unconfirmed]**,
so traffic's think interval is about ten render frames, 1/6 s at 60 fps. The start offset (stagger) delays the
first run: racers use 0, 0.5, 1.0 and repeat; cops use 0, 0.125, ..., 0.875; traffic 0, 0.1, ..., 0.9. [decomp]

The think rate does not depend on the distance to the player or the camera (no task is re-rated for vehicles)
[decomp]; see [ai-simulation-lod.md](ai-simulation-lod.md).

## 3. The drive state and the flags

The behaviour keeps (all set by the current action each think):

| Field | Meaning |
|---|---|
| `drive speed` (m/s) | speed the car should have (a signed target is never used: 0 means stop) |
| `drive target` (position) | the point to steer at |
| `drive flags` | bit 1: run steering; bit 2: run gas/brake; bit 4: run the reverse logic |
| `reverse override` timer | seconds left of a forced reversing manoeuvre (section 7) |
| `reversing speed`, `steering behind` | flags set by the reverse and steering controllers, read by the gas controller |
| `avoidable radius` (m) | look-ahead distance, default 20, set by the action |
| the drive-to navigator, a collision navigator, a current-road and a future-road navigator | see [ai-road-nav-trail.md](ai-road-nav-trail.md) |

Actions request a mode with `do driving(flags)`: racer, ram, pursuit and helicopter actions use **7**,
traffic and stuck recovery **3** (no automatic reverse). [decomp]

The per-tick order of the controllers is **reverse, steering, gas/brake** because steering sets the
`steering behind` flag the gas controller reads.

## 4. The simple controller (traffic, and the base of the PID one)

### 4.1 Steering

Run when flag 1 is set. Controls are zeroed first (steering and the vertical steering).

```
if drive_speed == 0 and speed_xz < 1: steer = 0; return
if driver is traffic: drive_target = nav.occluded_position (refreshed every tick)
d = unit(planar(drive_target - position))            # planar = y dropped
f = unit(planar(forward))
angle = asin(clamp(cross(f, d).y, -1, 1))             # signed, + = target on the right, range +-90 degrees
steer = angle / max_steering_angle                    # max_steering from the suspension, in radians
steering_behind = false
if in reverse:           steer = -steer
elif dot(d, f) < -0.2:   steer = sign(steer) * 1 (+1 if steer >= 0, else -1); steering_behind = true
(the "oversteer correction" term added otherwise is a stub that returns 0)
steer = clamp(steer, -1, 1)
```

`max steering angle` is what the suspension reports: for the racer chassis `STEERING * 45 degrees` for racing
(45 for drag); for the simple chassis (cops) 45 degrees, rising linearly to 60 as the larger of brake and
handbrake, or lifting off the gas, goes from 0 to 1; for the traffic chassis a fixed 45. [decomp]
The chassis then turns the normalised steering back into wheel angles with *no* shaping for AI: target angle
= `45 degrees * tires.STEERING * steer` (see [vehicle-steering-assists-aero.md](vehicle-steering-assists-aero.md)
section 6.1, "AI cars"; the simple chassis uses `45..60` degrees times `STEERING` but reports only the `45..60` as its
maximum, so a cop over-steers by the `STEERING` factor, 1.0 to 1.1). So an AI that asks for
`angle / (45 * STEERING)` gets exactly that angle, with no speed-sensitive range and no rate limit. [decomp]

### 4.2 Gas and brake

Run when flag 2 is set. Gas, brake, handbrake and vertical steering are cleared first.

```
if traffic and has transmission:
    if car is in shock (hit hard) XOR gearbox is in neutral:  shift to neutral if shocked else to the drive gear
    if in shock: return                                       # coast in neutral, no pedals
if not reversing_speed and steering_behind: gas = 1; handbrake = 1; return    # handbrake turn
cur = forward speed (signed);  want = drive_speed
if want < 0.5:                      brake = 1; return
if reversing gear:                  (cur > 1 ? brake = 1 : gas = 1); return
if cur < -1:                        brake = 1; return         # rolling backwards in a forward gear
if want < cur:                      if |want - cur| > 2.5 or want < 5: brake = 1;  return   # else coast
gas = 1
```

So traffic is bang-bang: full gas when below the wanted speed, full brake when more than 2.5 m/s above it (or
when the wanted speed is under 5 m/s), coasting inside the 2.5 m/s band. The traffic action keeps the wanted
speed no more than `2 * dT` above the current speed (section 5) so that this acts as a gentle acceleration
limit. [decomp]

### 4.3 Reverse logic (flag 4)

```
if flag 4 not set or reverse override active or no transmission: return
if gearbox not in reverse and speed >= 15: reversing_speed = false; return
reversing_speed = true
d = unit(drive_target - position)   (3-D);   facing = dot(forward, d)
if in reverse and facing > 0:           shift to first
elif not in reverse and facing < -0.707: shift to reverse     # target is more than 135 degrees behind
```

Shifting is the plain gear change of the drivetrain (a short shift delay, clutch open) from
[vehicle-engine-drivetrain.md](vehicle-engine-drivetrain.md) and [vehicle-manual-shifting.md](vehicle-manual-shifting.md).
The AI never presses shift buttons: forward shifting is the engine's own automatic box (the AI's
`is automatic shift` is true; the "perfect shift" bonus is off for AI). [decomp]

Note the arcsine in 4.1: a target exactly behind gives an angle near zero, so the reverse rule (and, for the
PID controller, full lock while backing up) is what turns a car round. See the PID file for the reversing
steering.

## 5. The traffic action: target and speed

Traffic uses the simple controller. Per think the traffic action [decomp]:

- Look-ahead `L = lerp(10, 30, ramp(speed, 0, 25))` m (30 when the nav says the lane is occluded by another car)
  `+ speed * dT + body radius`. The nav cursor is advanced to `L` ahead, the occluded position refreshed, and the
  avoidable radius set to `L`.
- Wanted speed `v`: posted speed `55 mph` on segments with four or more traffic lanes, else `35 mph`
  (`MSetTrafficSpeed` can replace both, and fix the speed); `v = min(posted, curve_limit)`, with
  `curve_limit = sqrt(g * mu / max(k_min, |curvature|))`, `g = 9.8`, `mu = 0.6` (1.6 for cops),
  `k_min = g * mu / posted^2` (so the limit never exceeds the posted speed) and `curvature` the trail curvature
  ([ai-road-nav-trail.md](ai-road-nav-trail.md) section 6).
- If the nav is occluded by an avoidable that is not behind: `mass = body mass` (doubled for a tractor),
  `length = 2 * body radius`, `dist = max(|apex - position| - length, 0)`,
  `stop_dist = Table(speed * max(mass * 0.0005, 1))` over x in 0..80 of `[3, 50]` (linear, clamped);
  if `dist < stop_dist`: `v = clamp(occluding_trail_speed * dist / stop_dist, 0, v)` then `v = min(v, speed)`.
- A dead end gives 0. Non-cops: `v = min(v, speed + 2 * dT)`.
- Accident reaction and pull-over: when a racer, cop or human car is involved in a collision within 80 m of the
  camera, a non-tractor traffic car drops gas and brake and holds full right steering for three thinks
  (`timer 3`, decremented by 1 per think), then brakes with full right steering for good; a tractor brakes. The
  pull-over state machine is present but its trigger is a stub (always false). Details in
  [ai-traffic.md](ai-traffic.md).

Drive target = the occluded nav position; drive speed = `v`; flags 3.

## 6. Output interface: what the AI writes

The AI writes the car's `InputControls` [decomp]:

| Field | Range | Set by the AI |
|---|---|---|
| `gas` | 0..1 | yes (PID output clamped; or 0/1) |
| `brake` | 0..1 | yes |
| `handbrake` | 0..1 | 1 or 0 (turns, stop, busted) |
| `steering` | -1..1 | yes, normalised by the max steering angle |
| `steering vertical` | | always 0 |
| `nos` | bool | yes |
| `banking`, `strafe`, `action button` | | not touched |

Differences from the human path (`InputPlayer` in `PInput.cpp`, spec [vehicle-input-induction-brakes.md](vehicle-input-induction-brakes.md) section 8):
no dead zone snapping, **no automatic reverse through the brake pedal** (the AI shifts to reverse itself), **no
idle auto-brake**, no handbrake-cancels-brake rule, no shift requests, no steering remap or shaping. The
human's `InputPlayer` is replaced by the plain `PInput` for every non-player car, whose per-tick step does
nothing; the AI writes straight into its control block. [decomp] The consumers then behave as for a human:
engine reads `gas` and `nos`, the chassis reads `steering`, `brake`, `handbrake`.

The AI also reads: signed forward speed, planar speed, position, linear velocity, orientation, body half
dimensions and radius, wheels on the ground, `max steering`, `is reversing`, `NOS capacity`, `is staging`,
`is destroyed`, `in shock`, the heading used by the nav, and the player cars' tuning-derived performance
([ai-driver-speed-skill.md](ai-driver-speed-skill.md)).

**Human car on autopilot.** The player's car has the AI behaviour of the human class. With autopilot off it only
maintains its nav (for the HUD wrong-way flag, pursuit path finding and drag lanes). `set ai control(true)`
sets the goal `AIGoalRacer` and the car then drives like a racer with automatic shifting; `is player steering`
is false. In a **drag race** the player's steering is taken by the AI controller (flag 1 each think, target =
nav position along the drag lane) unless the speed is under 1 m/s, the game-breaker is active, or the car is
facing the wrong way; left/right inputs only change lane. [decomp]

## 7. Stuck recovery, airborne and wrecks

### 7.1 Get-unstuck (score 1.0; racer, pursuit, ram, stop-short goals)

`can be attempted` runs every think while the action is not current and keeps a timer [decomp]:

```
if no reverse override and (gas >= 0.5 or vertical_steering >= 0.5):
    if staging or timer <= 0:  anchor = position; timer = dT
    else:  timer += dT
           if timer >= 3.0 s:   timer = 0
                                if distance(anchor, position) < 3 m:  stuck
else: timer = 0
on stuck: reverse_override(2.0 s); reset the drive-to nav to the centre lane; attempt succeeds
```

`reverse override(t)` shifts the gearbox into reverse (or into first if it was already in reverse) and sets the
timer; each think `timer -= dT`, and at zero the box returns to first. While the override is on the PID
controller falls back to the simple steering of 4.1 (inverted in reverse), the reverse logic is off, and the
action drives with flags 3 at a wanted speed of **15 m/s**, so the gas controller backs up (gas when speed
>= -1, brake if rolling forward). The action is finished when the override ends; finishing re-snaps the nav to
a valid lane. The anchor is not moved when a window ends without being stuck, so slowly creeping cars are
re-anchored only at the next window. [decomp]

### 7.2 Airborne (score 0.0)

Active only while the rigid body is modelled. Each think: if flagged airborne, add `dT` to a timer that is
never reset (once it has exceeded 1.5 s in total, the action is attempted whenever the car is airborne at a
check); then airborne = no wheel on the ground. While current, it zeros gas, brake, steering, vertical
steering and handbrake and refreshes the flag; it is finished when a wheel touches down. [decomp]

### 7.3 Wrecked (too-damaged, score 1.0) and stop

- *Too damaged*: attempted when the vehicle is destroyed. On begin it ends the car's pursuit. Each update it
  zeros all controls and, unless the car is in shock and while it moves faster than 2.5 m/s planar, applies
  `brake = 0.25`. Never finishes. The racer/pursuit action also stops (drive speed 0) when destroyed.
- *Stop short* (pursuit "breaker"): gas 0, brake 1, handbrake 1, steering 0, nitrous off; finished when the
  pursuit's breaker flag clears.
- *Busted*: the perp's update (every think) forces gas 0, brake 1, steering 0, handbrake 1, nitrous off while the
  pursuit marks it busted. [decomp]

## 8. Housekeeping in the think

Every think the AI behaviour [decomp]:

- Integrates two damped springs (angle and angular velocity, spring 5.6, damper 3.0). They feed only the stubbed
  oversteer correction, so they have no effect on the output.
- Runs simple physics (section 3 of [ai-simulation-lod.md](ai-simulation-lod.md)) if the car is in emulated mode.
- Racers: spawn timer `+= dT` (used by respawn tests: a car may respawn when the timer is above 8 s and it has
  been flagged "can respawn", or above 10 s when a respawn spot is available); reverse override countdown;
  targeting: "drivable to target" = no world hit on a ray from the car to the target (a ray longer than 200 m
  counts as blocked), raised 0.5 m at both ends, ignoring a hit within 0.5 m of the target.
- Clears the world-causality tag after 2 s for non-perp cars.

## 9. Avoidables (collision avoidance, driver side)

Each AI car is also an *avoidable*: a record other cars' navigators query. It publishes its position and a sweep
radius `max(avoidable radius, 2 * speed)` while active and its radius is above 0 (the default radius is 20 m;
actions set it to their look-ahead). Every 4 ticks (the `AvoidableManager` task has rate 0.25) all avoidables are
put into a sweep-and-prune grid and each gets a list of the others whose sweep volumes overlap. The navigator
reads that list (excluding a hitched trailer, and for a pursuit formation the target and fellow formation cops)
and cuts holes in its trail around those cars ([ai-road-nav-trail.md](ai-road-nav-trail.md) section 5), which
changes the occluded steering point and the curvature. The driver itself has no steering-away force for cars:
avoidance is entirely "steer at the occluded point" plus the traffic braking rule of section 5. The older
steering helpers (seek, ram intercept, capsule separation with a 160 km/h cap) are used only by pursuit actions
([ai-pursuit.md](ai-pursuit.md)); the flee/avoid helpers are stripped from the decomp. A world-wall probe
(`world avoidance info`: two rays ahead-left and ahead-right) exists but nothing in the sources read calls it.
[decomp]

## 10. The `aivehicle` data

One `aivehicle` collection per car, through `pvehicle.aivehicle` [verified] (racers `aivehicle/racers`, cops one
per cop car, traffic `traffic`/`street`/`van`/`truck`, challenge series `challengeseries`/`cssuv`/...).

| Field | Unit | Used by |
|---|---|---|
| `MAXIMUM_AI_SPEED` | km/h | ceiling for pursuit-mode speed (race action) and for ram/off-road actions; see the speed file |
| `AccelerationMultiplier` | x | scales the car's acceleration table in the race and pursuit actions |
| `TopSpeedMultiplier` | x | scales the speed limit |
| `TETHER_WEIGHT` | percent | pursuit off-road action (0 everywhere in the data) |
| `RepPointsForDestroying[10]`, `CostToStateForDestroying`, `DetachmentID` | | pursuit rewards, not driver control |
| `PlayerCollisions[10]`, `PlayerCollisionsDefault` | | see below |

Values read from the install [verified]: default 200 km/h, 1.0, 1.0; `racers` and `challengeseries` family 400
km/h, 1, 1; `traffic` family 200, 1, 1. Cops (`MAXIMUM_AI_SPEED` km/h, acceleration x, top speed x):
`copsuvpatrol` 250, 1.05, 0.9; `copsuv` and `copsuvl` 50, 1.0, 0.5; `copcompact` 200, 1.0, 1.0;
`copmidsize` 280, 1.05, 1.6; `copghost` 310, 1.09, 1.15; `copgto` 325, 1.1, 1.2; `copgtoghost` 350, 1.15, 1.25;
`copsporthench` 400, 1.2, 1.3; `copsportghost` 400, 1.25, 1.5; `copsport` and `copcross` 400, 1.5, 2.0.
Other fields are unused by the driver.

**Collision reactions.** Whenever the goal changes on a traffic, cop, racer or human car with a vehicle body,
the AI looks in `PlayerCollisions` for the record whose `Goal` equals the new goal's name hash and loads the
`collisionreactions` collection it names; if none matches it uses `PlayerCollisionsDefault`. The rigid body uses
it when this car hits a racer or human driver ([vehicle-rigid-body.md](vehicle-rigid-body.md) section 6, "Resolution"). The record is 16
bytes: goal hash, class key of `collisionreactions` (0xB32682F1), collection key, zero. [verified] examples:
racers: `AIGoalRacer` -> `collisionreactions/racing`; most cops: `AIGoalStaticRoadBlock` -> `roadblock` (or a
car-specific `roadblock*`), and `copsuvpatrol`, `copsport`, `copsportghost`, `copsporthench`, `copmidsize`,
`copghost` add `AIGoalPit` -> `coppit`. `copsuvl` has one record naming a different collection.

## 11. How to check it

- Record a racer for 60 s on a straight road: `gas`/`brake` should only be 0 or 1 at the extremes and otherwise
  continuous (PID); traffic should show only 0, 1 and coasting.
- Hold a traffic car with a wall 3 m ahead: after 3 s of gas with under 3 m of travel nothing should happen
  (traffic has no get-unstuck), while a racer should reverse for 2 s, then reset its nav to the centre lane.
- Park a racer facing away from its target at under 15 m/s: it should shift to reverse, back up with opposed
  full lock, and shift to first when the target is ahead.
- Measure think intervals: racers 2 ticks, cops 8 ticks, traffic about 10 render frames.

## 12. Open questions

1. Which behaviour class each driver class receives, and the order of the AI drive step relative to the engine
   and suspension within the Physics schedule (spawn code and `PhysicsObject` are empty in the sources).
2. The `dT` handed to variable-rate tasks (traffic think); the decomp passes the accumulator.
3. `world avoidance info` has no caller in the sources; the 1.0.x executable may use it.
4. The oversteer correction is a stub (returns 0); the debug symbols suggest a longer function.
5. Whether traffic cars could ever use the PID controller (a build-time switch for a PID traffic class exists
   in names only).

## 13. Rust implementation notes

- Put the driver in a new crate or module beside `libs/blackbox-vehicle`, with the pure part (controllers) taking
  plain numbers and returning a control struct; keep the nav, goal and action selection in the game crate
  (`crates/`), since goals are game-specific.
- `AiControls { gas: f32, brake: f32, handbrake: f32, steer: f32 /* -1..1 */, nos: bool, gear: Option<GearRequest> }`
  where `GearRequest` is `Reverse` or `First`. Convert to `InputState` with `throttle = gas`, `brake`,
  `handbrake`, `steer`, `nos`; no `shift_up/down`.
- The existing `input::shape` applies a dead zone, auto-reverse and idle auto-brake that AI must skip: for AI
  cars set `ControlConfig { dead_zone: 0.0, auto_reverse: false, auto_brake: false, automatic: true,
  steering_device: SteeringDevice::Ai }` (the AI steering device already exists: full angle times input), and
  call `Vehicle::shift_to(GEAR_REVERSE | GEAR_FIRST)` for gear requests.
- The vehicle must expose: `max_steering_deg()` = `45 * tires.steering` (racing), signed forward speed
  (exists: `forward_speed`), planar speed, wheels on ground (exists), `gear`, nitrous level (exists
  `nos_level`), `is_reversing`, an "in shock" flag (hit harder than a threshold; time-limited) for traffic, and
  `destroyed` (exists as `disabled`).
- A fixed 60 Hz sim with think counters: racers every 2 ticks, cops every 8, traffic every 10 (use ticks, not
  render frames, for determinism). Run the drive step every tick with the latest drive target and speed.
- Controller state (PID memories, stuck timer, airborne timer, reverse timer, `reversing_speed`,
  `steering_behind`) is per car; reset it when the goal changes or the car respawns.
