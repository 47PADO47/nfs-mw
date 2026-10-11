# Traffic: the cars and how they drive

How the ordinary road cars of the original behave once they exist: which models they are, the single driving
action they run, how they follow a lane, set their speed, avoid and react to other cars, and what happens after a
crash. Where cars come from and go to is in [ai-traffic-spawning.md](ai-traffic-spawning.md); traffic lights, horns
and the scripted drag-race traffic are in [ai-traffic-world.md](ai-traffic-world.md). The road graph and the
navigator are in [ai-road-network.md](ai-road-network.md), the look-ahead trail ("cookies") in
[ai-road-nav-trail.md](ai-road-nav-trail.md); steering, throttle and the shared AI vehicle code are in
[ai-driver-control.md](ai-driver-control.md). Provenance: [provenance/ai-traffic.md](../provenance/ai-traffic.md).
For the tag meanings, see [evidence tags](../README.md#evidence-tags).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), under `src/Speed/Indep/Src`: `AI/Actions/AIActionTraffic.cpp`, `AI/Common/AIVehicleTraffic.cpp`,
  `AI/Common/AIVehicle.cpp`, `AI/Common/AIGoal.cpp`, `AI/AIVehicle.h`, `AI/Activities/AvoidableManager.cpp`,
  `AI/Actions/AIActionRace.cpp` (curvature speed function), `World/Common/WRoadNetwork.cpp` (`GetNextTraffic`,
  traffic lane helpers, `PullOver`, `HolePunchAvoidables`, `UpdateOccludedPosition`), `World/WRoadElem.h`,
  `Physics/Behaviors/{EngineTraffic,SuspensionTraffic,RBVehicle,RBTractor,DamageVehicle}.cpp`,
  `Generated/AttribSys/Classes/{pvehicle,aivehicle}.h`, `Misc/MWAttribUserTypes.h`. Read for understanding; no code
  copied.
- **Data inputs:** AttribSys classes `pvehicle` (traffic cars), `aivehicle` (`street`, `van`, `truck`, `traffic`,
  `default`), `collisionreactions` (`traffic`, `street`, `van`, `truck`, `semi`), `damagespecs/traffic`,
  `rigidbodyspecs/traffic`, `transmission/traffic`, `chassis/trafstreet`; the road network profile lane types.
  Values marked **[verified]** were read from the install.

Everything is **[decomp]** unless marked. Nothing was measured in the running game.

## 1. Overview and update rates

- A traffic car is a physics vehicle with driver class **traffic** and the behaviours `RBVehicle` (or `RBTractor`
  for semi tractors), `EngineTraffic`, `SuspensionTraffic`, `DrawTraffic`, `SoundTraffic`, `DamageVehicle`, `PInput`
  and the AI behaviour `AIVehicleTraffic` **[verified]**.
- The AI "think" runs at **10 Hz** (`0.1 s` period). Each new traffic car gets a start offset that grows by 0.1 s and
  wraps at 1.0 s, so the cars' thinks are spread over ten phases. A think: update the AI base (smoothed yaw and yaw
  rate), age the spawn timer, then run the goal.
- The goal of a spawned traffic car is **"traffic"**, which has exactly one action, **`AIActionTraffic`** (score 0.1,
  never finished by itself). No stuck recovery, no reverse, no airborne or damage actions: a blocked traffic car
  waits until it is removed ([ai-traffic-spawning.md §8](ai-traffic-spawning.md#8-removing-cars)). The same action
  is also part of the cop goals "patrol" and "pursuit" (for cops that cannot see their target), see section 6.
- The think only decides a **drive target** (a world point), a **drive speed** and the flag word `3`
  (steer + gas/brake). The controls are then computed on every physics step from those three values by the shared AI
  vehicle code ([ai-driver-control.md](ai-driver-control.md)); for traffic the only special case there is the
  shock rule of [section 5.4](#54-shock).
- The goal's action is chosen when the goal is created, at spawn. `can_be_attempted` is true if the car has a rigid
  body, an input and a vehicle, and a traffic navigator started at the car's position and heading with force-centre-lane
  true and direction weight 1 is valid (a road lies within the 32 m search of [ai-road-network.md §3.1](ai-road-network.md#31-closest-segment-decomp)).
  If the car has a pursuit attached, the condition is instead "the target is not in sight".

## 2. The traffic cars

Traffic uses ordinary `pvehicle` collections (the same class as the player's cars) whose engine, suspension and
audio behaviours are the `Traffic` variants. All numbers below are **[verified]** from `attributes.bin`.

| Group (abstract base) | Mass | `aivehicle` | Cars |
|---|---|---|---|
| `street` | 1680 kg | `street` | `trafha`, `trafficcoup`, `traf4dseda`, `traf4dsedb`, `traf4dsedc`, `traftaxi`, `trafstwag`, `trafcourt`, `trafpizza` |
| `van` | 3000 kg | `van` | `trafsuva`, `trafvanb`, `trafcamper`, `trafpickupa`, `trafnews`, `trafminivan` |
| `truck` | 5000 kg | `truck` | `trafcemtr`, `trafdmptr`, `trafgarb`, `traffire`, `trafamb` |
| `tractors` (`RBTractor`) | 10 000 kg | `default` | `semi` (no trailer), `semia`, `semib`, `semicmt`, `semicon`, `semicrate`, `semilog` |
| `trailers` (`RBTrailer`) | 10 000 kg (the base 1000) | `default` | `trailera`, `trailerb`, `trailercmt`, `trailercon`, `trailercrate`, `trailerlog` |

- Pattern lists use 20 of these ([ai-traffic-spawning.md §4](ai-traffic-spawning.md#4-the-trafficpattern-data-verified));
  `trafsuva`, `traf4dsedb`, `trafcamper`, `trafamb`, `semi`, `semia` and `semicmt` are defined but in no pattern
  (the name `trafamb` suggests an ambulance).
- Every traffic car shares: engine / tires / brakes `911turbo` (the same entries as the player's Porsche), the
  gearbox `transmission/traffic`, `rigidbodyspecs/traffic`, `damagespecs/traffic` and the `collisionreactions` of its
  group. Only mass, `chassis` (`trafstreet`, `trafvan`, `traftruck`, `semi`), model and the sound ids differ.
  The cars are therefore light, fast-accelerating cars; the speed they actually drive is set by the posted speed
  (section 3), not by the engine.
- The semi **tractor** creates its own **trailer** (the `Trailer` reference, a `pvehicle` with driver class "none")
  when it is built, joins the two at a "5th wheel" joint and spawns, poses, kills and unspawns the trailer with
  itself. The trailer receives the tractor's brake and handbrake inputs. The joint is released when the tractor or
  trailer tips over (up-vector y below 0.75), when their up vectors diverge (dot below 0.8) or when either is on fewer
  than two wheels for more than 2 s. A hitched trailer is not an obstacle for its own tractor and the tractor does
  not collide with it.
- Sound / render ids carried by the data: `HornType` 5, 6 (a few cars) or 11 (trucks, semis), `TrafficEngType` 0
  (cars), 2 (vans, pickups, wagons, trucks `trafcemtr` `trafdmptr`), 8 (`trafgarb`, `traffire`), `WooshType` 9 (cars,
  vans) or 6 (trucks, semis, ambulance) **[verified]**. See [ai-traffic-world.md](ai-traffic-world.md#3-horns-engine-and-drive-by-sounds).

## 3. The driving action

### 3.1 Per-think update

`dT` is the think period (about 0.1 s). With `speed` the rigid body speed and `radius` its bounding radius:

```
look = lerp(10, 30, ramp(speed, 0, 25)) + speed * dT + radius          # metres; 30 + ... when blocked by a car
if the navigator is occluded by another car:  look = 30 + speed*dT + radius
update the navigator cursor so that it is at least `look` metres ahead of the car      (3.2)
set the avoidable radius of this car to `look`                                          (section 5.1)
desired = compute_speed(speed, dT)                                                      (3.3)
if the navigator is valid:  drive target = occluded position;  drive speed = desired
else:                       drive speed = 0
accident logic (section 5.3)
unless pulling over / in an accident:  request driving flags 3 (steer + gas/brake)
```

The drive target is the point the car steers at (the navigator's "occluded position", [ai-road-nav-trail.md §4](ai-road-nav-trail.md#4-the-occlusion-update-steering-target-decomp));
the steering and gas/brake from target and speed are in [ai-driver-control.md](ai-driver-control.md).

### 3.2 The cursor ahead of the car

The action keeps a **traffic-type navigator** (nav type traffic, lane type traffic, path type none, with a cookie trail
and the decision filter off) per car. When the action starts (`begin_action`, at spawn) the cursor is re-initialised
at the car's position and heading with "select a valid lane" (initialise at the closest point, then snap to the nearest
traffic lane), the cursor is advanced until it is 30 m ahead of the car, the accident state is cleared and, for a cop that was just respawned and
has no sight of its target, the speeds below are replaced by the cop patrol speeds.

Every think:

```
d = dot(nav_forward_unit, nav_position - car_position)     # signed distance of the cursor ahead of the car
if not dead_end and d < look:
    target = unit vector to the AI target if one is valid, else (0,0,0)
    advance the cursor by (look - d), toward `target`, max look-ahead = look
then update the occluded position (with other cars punched out of the trail)
```

- A traffic car has **no AI target**, so the "toward" vector is zero. At every junction the cursor therefore takes the
  random branch of [ai-road-network.md §4.3](ai-road-network.md#43-choosing-the-next-segment-traffic-navs-decomp):
  a uniformly random legal exit, preferring to keep the car in its lane counted from the centre and, on the rightmost
  entrance, from the curb. Cops that do have a target steer the same code towards it.
- The cursor stays in the car's lane for the whole drive: **traffic never changes lanes** by choice. The only lateral
  motion is inside the narrow corridor of the trail (section 5.1). The lane keeps its number across segments
  (`nth` traffic lane from the centre) and takes the lane's own offset on each segment.
- Driving direction follows the lane: lanes right of the profile's middle zone run along the stored segment direction
  (drive on the right). One-way segments only carry traffic in the stored direction. A node with fewer than two
  segments is a **dead end**: the cursor stops there, `compute_speed` returns 0 and the car stops at the end
  (traffic never turns around, never reverses).
- The cursor never leaves the traffic lanes: lane type "traffic" is selectable and drivable only on zone type 1
  (masks `2`) **[decomp]**.

### 3.3 Speed

`compute_speed(current_speed, dT)`:

```
if the cursor is at a dead end:  return 0
posted = SpeedHighway if the segment has 4 or more traffic-lane zones (both directions together) else SpeedStreet
desired = posted
if not fixed_speed:
    k = trail curvature (car position, car velocity)                       # ai-road-nav-trail.md §6
    a = (is_cop ? 1.6 : 0.6) * 9.8                                          # lateral acceleration allowed
    limit = min(posted, sqrt(a / max(a / posted^2, |k|)))                   # = sqrt(a / |k|), capped at posted
    desired = min(posted, limit)
    if occluded by another car and not occluded from behind:
        m = rigid body mass (doubled for a tractor)
        length = 2 * bounding radius
        dist = max(distance(apex position, car position) - length, 0)
        stop = 3 + 47 * clamp(current_speed * max(m * 0.0005, 1) / 80, 0, 1)   # metres
        if dist < stop:
            desired = clamp(occluding_trail_speed * dist / stop, 0, desired)
            desired = min(desired, current_speed)
if not is_cop:  desired = min(desired, current_speed + 2 * dT)
return desired
```

- `SpeedStreet` / `SpeedHighway` start as 35 and 55 mph (15.65 and 24.6 m/s) and are replaced by the spawn message
  with the active pattern's values ([ai-traffic-spawning.md §7](ai-traffic-spawning.md#7-creating-reusing-and-activating-a-car)):
  35 / 55 in every shipped pattern **[verified]**. The same message can carry a `fixed` flag that switches off the
  curvature and the car-ahead rules (used by scripted traffic, see world doc); nothing in the traffic manager sets it.
- "Highway" is decided by the lane count of the whole profile, not by a road type: a segment with two traffic lanes
  in each direction already counts. The cars slow and speed up as they cross such a boundary.
- **The road-curvature term never limits traffic.** Each cookie's curvature is clamped to 0.01 1/m before averaging
  ([ai-road-nav-trail.md §6](ai-road-nav-trail.md#6-curvature-for-speed-choice-decomp-unsolved)), so `limit >= sqrt(5.88/0.01) = 24.2 m/s`, which is more than
  the street speed and about the highway speed. Only the **apex term** (an occluded view of the corridor, e.g. a tight
  junction turn) or the car-ahead rule can pull the speed below the posted one. (Derived from the constants,
  **[decomp, derived]**.)
- The `+ 2 dT` clamp only limits how far the *target* may exceed the current speed (about 0.2 m/s per think); the throttle
  is still full whenever the target is above the current speed, so it is not an acceleration limit.
- The gas/brake rule (shared code): gas full while `desired > speed`; when `desired < speed` brake full only if the
  difference exceeds 2.5 m/s or `desired < 5 m/s`, otherwise coast; `desired < 0.5` brakes fully.

## 4. What traffic needs from the road network

Everything below is implemented by the road-network module; this is the traffic-side contract (details in
[ai-road-network.md](ai-road-network.md)):

| Call | Use |
|---|---|
| `init_at_point(position, heading, force_centre_lane, dir_weight)` with nav type traffic | place a cursor in the closest traffic lane |
| `can_traffic_spawn()` | spawn test and random lane choice ([spawning §6](ai-traffic-spawning.md#6-where-to-spawn)) |
| `advance(distance, toward, max_look)` | move the cursor; picks the next segment and lane with the traffic rules, flags a dead end |
| `update_occluded_position(true)` | funnel the trail past other cars; gives the steering point, apex, "occluded by car", "from behind" and the occluding car's trail speed |
| `trail_curvature(position, velocity)` | speed limit input |
| `num_traffic_lanes(segment)` | posted speed class (4 or more lanes) |
| `pull_over()` | move the cursor to the curb side (not reachable in this build, section 6) |
| `is_on_legal_road()` | a segment where traffic is allowed (used by the pursuit code) |

Segment data used: the **NoTraffic** flag (bit 1, 1356 of 6538 segments), the one-way flag (bit 6), the decision and
intersection flags (bits 0 and 3) and zone type 1 (traffic) in the node profiles ([ai-road-network.md](ai-road-network.md)).
The NoTraffic flag also feeds the cops' road filter (`NoTraffic xor CopsXorTraffic`).

## 5. Other cars, the player, crashes

### 5.1 Avoiding cars ahead

Every AI car (traffic, racers, cops, the player's car) is an **avoidable**: a region of radius
`max(avoid_radius, 2 * speed)` around the car, refreshed at **4 Hz** by the avoidable manager (a sweep-and-prune
over all avoidables; each car gets the list of overlapping neighbours). A traffic car sets its `avoid_radius` to its
look-ahead each think, so its neighbour list is the cars within roughly 10 to 30 m (more when fast).
During the occlusion update the neighbours that are ahead are punched out of the car's trail (algorithm in
[ai-road-nav-trail.md §5](ai-road-nav-trail.md#5-hole-punching-around-other-cars-decomp)); the traffic-specific
settings of that algorithm are:

- the trail corridor of a traffic cursor is only `half_width` wide ([nav-trail §2](ai-road-nav-trail.md#2-corridor-bounds-decomp));
  the minimum width kept after a cut is 0.1 m (1 m for others);
- other cars count as "in front" when `dist_ahead + his_extent < my_extent` at all speeds (others: only below
  20 m/s); a car that is not closing is ignored only beyond `my_extent + his_extent + 0.5*speed + 2*my_extent`, and a
  car that is not closing and lies clearly beside the lane (more than 1 m beyond the side extents) is ignored;
- "blocked traffic" cars are considered even when the time to closest approach is 3 s or more.

Net effect: traffic does not overtake or swerve around anything. It stays in its lane, slows to the speed of the
car ahead (the occluding trail speed scaled by the distance to the occluder, section 3.3), and stops behind a
stopped car. Cars cross junctions without any priority rule: crossing and merging cars are handled only by this
avoidance.

### 5.2 The player

Nothing in the traffic code reacts to the player specifically: no braking for the player's approach, no swerving, no
fleeing, no honking from the AI. The player's car is one more avoidable, and honks are a sound-side effect
([ai-traffic-world.md §3](ai-traffic-world.md#3-horns-engine-and-drive-by-sounds)). Hits on traffic are reported to
the pursuit and race code when the player causes them ([ai-pursuit.md](ai-pursuit.md),
[ai-pursuit-heat.md](ai-pursuit-heat.md)).

### 5.3 Crashes with the player, racers and cops (the "accident" state)

The action listens to the collisions of its own car. For a collision between objects (not with the world; cops
ignore this rule), for each of the two parties other than itself:

```
skip if the party is the car itself or the collision point is more than 80 m from the camera
other = the party as a vehicle
if other exists and (other.absolute_speed >= 5 mph  or  accident != OVER)
   and other.driver_class in {human, cop, racer}:
        if this car is a tractor:   accident = OVER;       timer = 0
        else:                       accident = IN_PROGRESS; timer = 3
```

So only a collision with a player, racer or cop car matters; traffic against traffic, props and the world do nothing.
During the accident states (every think):

- `IN_PROGRESS`: `timer -= 1` **per think** (not per second), so with the 10 Hz think the phase lasts **3 thinks =
  0.3 s** (the code treats it as a counter, probably meant as seconds; keep the behaviour). Controls: gas 0, brake 0,
  handbrake 0 and, except for tractors, steering at full lock (value 1). No lane following. When the timer reaches 0 the
  state becomes `OVER`.
- `OVER`: gas 0, brake 1 and (not tractors) steering stays at 1. The car brakes to a halt, wheels turned, and stays
  there. The state is only cleared by `begin_action`, i.e. when the car is respawned. A traffic car you hit therefore
  becomes a stationary obstacle until it is removed by the off-screen rule.

### 5.4 Shock

A hard enough object collision puts a car "in shock" (`DamageVehicle`): for a collision with closing speed of at
least 1 m/s, `scale = (force / mass) / SHOCK_FORCE`; if `scale > 0.2` (needs `force/mass > 2` with the traffic
`SHOCK_FORCE = 10`) the shock level becomes `min(max(level, scale), 1)`. The level falls by `dT / SHOCK_TIME` per
physics step (`SHOCK_TIME = 3 s` for traffic), so a shock lasts up to 3 s (at least 0.6 s). Shock is **[verified]**
tuned by `damagespecs/traffic` (`SHOCK_TIME 3`, `SHOCK_FORCE 10`).

For traffic cars the gas/brake step shifts the gearbox to **neutral** while in shock and back to first (or reverse
if it was reversing) afterwards, and returns without touching the controls while in shock: the car coasts with no
braking and no throttle. This applies to every collision with an object, including traffic against traffic.

### 5.5 How hard traffic is to push around

When a traffic car (or any non-player AI car) is hit by a **racer or human** car, its rigid-body response is
modified from the `collisionreactions` record of its `aivehicle` (`PlayerCollisionsDefault`; a list
`PlayerCollisions` of per-goal overrides exists but traffic uses the default). Four records, chosen by where the
contact point is relative to the car centre (front or rear half; "side" when the contact normal is within 45 degrees
of the lateral axis, else "end"): `FRONT`, `FRONTSIDE`, `REAR`, `REARSIDE`. A record has six floats
(`Elasticity`, `RollHeight`, `WeightBias`, `MassScale`, `StunSpeed`, `StunTime`); the effect is:

```
elasticity += record.Elasticity  (then clamped to 0..1)
if record.MassScale > 0:  inertia *= MassScale;  mass *= MassScale
centre of gravity: y += RollHeight;  z += WeightBias
```

`StunSpeed` / `StunTime` are not read anywhere in the sources (all zero in the data). **[verified]** values (elasticity /
roll height / weight bias / mass scale):

| Class | FRONT | FRONTSIDE | REAR | REARSIDE |
|---|---|---|---|---|
| `traffic` (base) | 0 / 0.1 / -0.9 / 0.95 | 0 / 0 / 0 / 0 | 0.1 / 0.1 / 0.9 / 0.85 | 0 / 0 / 0 / 0 |
| `street` | 0 / -0.15 / -0.9 / 0.8 | 0 / -0.3 / 0 / 0.85 | 0.17 / -0.15 / 0.9 / 0.8 | 0 / -0.3 / 0.8 / 0 |
| `van` | 0 / -0.15 / -0.9 / 0.6 | 0 / -0.3 / 0 / 0.85 | 0.25 / -0.15 / 0.9 / 0.65 | 0 / -0.3 / 0.8 / 0 |
| `truck` | 0 / -0.15 / -0.9 / 0.8 | 0 / -0.3 / 0 / 0.85 | 0.15 / -0.15 / 0.9 / 0.8 | 0 / -0.3 / 0 / 0 |
| `semi` | 0.4 / 0 / 1 / 7 | 0 / 0 / 1 / 2 | 0 / 0 / 1 / 2 | 0 / 0 / 1 / 2 |

(Cars are made lighter and their centre of gravity lowered and shifted in the hit direction so they spin and flip
readably; the semi is made much heavier on the nose.)

The generic collision rules (mass ratio, friction, ground collisions skipped for settled AI cars) are in
[vehicle-rigid-body.md](vehicle-rigid-body.md). Of those, one is traffic-specific: a traffic car that has not been
involved in a collision tests the world only every 4th physics step below 2 m/s and every 2nd step below 15 m/s.

## 6. Pulling over, cops and the dormant parts

- **Pull over.** The action has a three-state machine (none, pulling over, pulled over) and the cursor has a
  `pull_over()` ([ai-road-network.md §6](ai-road-network.md#6-queries)), but the function that decides whether to
  pull over is a stub that always answers no. So in this build traffic never pulls over, the cursor is never moved
  to the curb and the pull-over goal (`AIGoalPullOver`, a stripped constructor whose factory builds a ram goal)
  is unused. If implemented: in "pulling over" the trail is cleared, the cursor moves sideways to the outer edge of
  its contiguous traffic lanes; the state turns "pulled over" when the car has reached the cursor
  (`dot(unit(car - cursor), cursor_forward) > -0.5 * speed * dT`); while "pulled over" the controls are gas 0, brake 1,
  steering 0 and no driving; when the condition ends it re-initialises the cursor on the current lane.
- **Stop sign / intersection flags.** The action has `stop_sign` and `clear_intersection` booleans, initialised to
  false and never used. There is no stopping at junctions ([ai-traffic-world.md §1](ai-traffic-world.md#1-traffic-lights-and-stop-signs)).
- **Cops.** The same action drives cops that patrol or search without sight of the target. Differences: no
  acceleration limit, lateral acceleration 1.6 g instead of 0.6 g for the curvature limit, and, for a cop respawned
  during a pursuit without sight of the target, the cruising speeds are the default `pursuitlevels` collection's
  `SearchModeCityMPH` / `SearchModeHwyMPH` (50 / 71 mph, see [ai-pursuit-heat.md](ai-pursuit-heat.md)). A cop's own
  collisions are not treated as accidents. The action ends as soon as the cop sees the perpetrator.
- **Traffic as pursuit input.** The pursuit code counts hits on traffic cars and reports "hit and run";
  see [ai-pursuit.md](ai-pursuit.md).

## 7. Traffic car physics (differences only)

The generic vehicle model is in the `vehicle-*` specs. The traffic variants:

- **`EngineTraffic`**: the torque loop of the racer engine without nitrous or forced induction (those queries return
  constants), automatic gearbox only (shifts from precomputed shift points), a torque converter factor from `TORQUE_CONVERTER` (1.5 in `transmission/traffic`), and a
  "neutral" mode used whenever the car is in neutral, on its side (up vector y below 0.3) or the total ratio is zero.
  Engine speed targets `red_line * throttle`. `GEAR_RATIO` is `[3.2 (reverse), 0 (neutral), 3.82, 2.05, 1.41, 1.12,
  0.92, 0.75]`, `FINAL_GEAR 3.44`, `SHIFT_SPEED 0.25`, torque split 1 **[verified]**. Reverse is never used because the
  traffic action never asks for it.
- **`SuspensionTraffic`**: four wheels with the usual spring, damper, sway-bar and travel from the `chassis`
  entry, but a much simpler tyre: lateral force `-2 * lateral_speed * load * GRIP_SCALE` (divided by the slip speed
  when that exceeds 1 m/s; scaled down by `min(|lateral speed|, 1)` below 1 m/s), longitudinal force from the applied
  torque, a handbrake that locks the rear wheels and drag `speed * DRAG_COEFFICIENT` opposing the velocity. Steering is
  the input times a fixed **45 degrees**. Drive torque goes to the rear axle by `TORQUE_SPLIT` (all of it for 1),
  half per wheel. The roll lever arm of the wheel forces is halved (`TrafficRollAdjust = 0.5`). The traffic
  `chassis/trafstreet`: wheelbase 2.35 m, front axle 1.13 m, drag coefficient 0.35 **[verified]**.
- **Rigid body.** `rigidbodyspecs/traffic`: ground friction 0.55 / 0.45, wall friction 0.2 / 0.1, wall elasticity
  0.15 (z), object elasticity 0.1 (z), sleep velocity 1 **[verified]**.
- The vehicle may sleep only when it has no wheel on the ground (see [vehicle-rigid-body.md](vehicle-rigid-body.md)).

## 8. Constants

| Value | Meaning | Tag |
|---|---|---|
| 0.1 s (+ stagger 0.1 s steps) | traffic AI think period | **[decomp]** |
| 10 to 30 m (+ speed*dT + radius) | cursor look-ahead, blended over 0 to 25 m/s | **[decomp]** |
| 35 / 55 mph | default and pattern street / highway speed | **[decomp]** / **[verified]** |
| 4 lanes | traffic-lane zone count that selects the highway speed | **[decomp]** |
| 0.6 g (cops 1.6 g) | lateral acceleration used for the curvature speed limit | **[decomp]** |
| 3 m to 50 m over 0 to 80 | stopping distance table, argument `speed * max(mass/2000, 1)` | **[decomp]** |
| 2 dT | max target speed above current speed | **[decomp]** |
| 80 m | distance to the camera inside which a crash starts an accident | **[decomp]** |
| 5 mph | other car's speed to restart an accident after it ended | **[decomp]** |
| 3 thinks (0.3 s) | accident phase | **[decomp]** |
| 2, 3 s / 0.2 / 10 | shock threshold, `SHOCK_TIME`, `SHOCK_FORCE` | **[decomp]** / **[verified]** |
| 0.25 s | avoidable refresh period | **[decomp]** |

## 9. How to check it

In the original (PC build), free roam:

1. Follow a taxi for a minute: constant speed near 35 mph in town and 55 mph on wide roads; it stays in its lane at
   junctions and turns at random.
2. Stop in front of an oncoming car's lane: the car should stop behind you rather than swerve; stop a car with the
   brake and note how it re-accelerates (full throttle to the posted speed).
3. Ram a traffic car with the player's car: it should lose control for about 0.3 s (no throttle, no brake, wheels hard
   over) then stand still with brakes on until it drops out of view. Ram it with another traffic car (a rare event):
   no stopping.
4. Look for any traffic light or stop sign that changes behaviour: none expected.
5. A semi: tractor and trailer move as a unit; hit the trailer hard and it may detach.
6. Compare the dead ends: traffic should stop at the end of a dead-end road, not reverse.

## 10. Open questions

- The exact controls that `AIVehicle::OnSteering` / `OnGasBrake` produce are in [ai-driver-control.md](ai-driver-control.md);
  only the traffic-specific pieces are described here.
- Hole punching and the trail curvature are marked unfinished in the decomp; the shipped behaviour of traffic around
  parked or stopped cars needs a measurement.
- Whether the PC build still has the 0.3 s accident phase (the unit looks like a bug but may be shipped).
- The sign convention of the steering input (the accident steers to full positive lock).

## 11. Rust implementation notes

- Keep three layers: a **spawner** (2 Hz, [ai-traffic-spawning.md](ai-traffic-spawning.md)), a **traffic driver**
  (10 Hz, produces target point + target speed + accident state) and the **shared vehicle controller**
  (every physics step). Traffic never needs more than a handful of cars (cap 10 total), so plain vectors are enough.
- Read `trafficpattern`, `pvehicle`, `aivehicle` and `collisionreactions` straight from the attribute database with
  the class and field names above; do not hard-code the car list. Only three constants live in code: 0.6 g, the
  stopping-distance pair `3 / 50` over `0 / 80`, the accident timing, and the look-ahead range `10 / 30 / 25`.
- The road module must offer the contract of section 4; implement the "traffic" next-segment rules and the lane
  numbering exactly (nth lane from centre, curb-relative on the rightmost entrance) so cars keep their lanes.
- Reproduce the accident quirk (decrement per think) behind a named constant so it can be changed, and make the
  state reset on respawn only.
- The avoidable list can be a brute-force O(n^2) overlap test at 4 Hz for at most ~20 cars.
- Do not implement lights, stops, lane changes or pull-over; they are not in the original data/code path.
- Keep the shock-to-neutral rule in the controller, not in the driver, because it is applied after the driver's
  controls.
- Determinism: use one seeded random stream for spawn offsets, lane choices and junction exits so replays match.
