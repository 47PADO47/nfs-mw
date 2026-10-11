# Cop tactics: goals, chase driving, ramming, retirement

How a police car decides what to do and how it drives while it is chasing: the goal and action tables, the
close-chase steering, the path-following chase, ramming, the stop and retire actions, how cops see the target and how
a cop's performance is tied to the target's car. Formations (box-in, pit, herd, ...) and the heavy/leader support
vehicles are in [ai-pursuit-formations.md](ai-pursuit-formations.md); roadblocks, spike strips and pursuit breakers in
[ai-pursuit-roadblocks.md](ai-pursuit-roadblocks.md); sight, the jerk rule, damage and per-car tuning in
[ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md); the helicopter in [ai-helicopter.md](ai-helicopter.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube build), under
  `src/Speed/Indep/Src`: `AI/AIAction.h`, `AI/Common/AIAction.cpp`, `AIGoal.cpp`, `AI/Actions/AIAction{PursuitOffRoad,
  Ram,HeadOnRam,JackKnife,StopShort,StaticRoadBlock,TooDamaged,Strafe,GetUnstuck,Airborne,Race,Traffic}.cpp`,
  `AI/AIVehicleCopCar.h`, `AI/Common/AIVehicleCopCar.cpp`, `AIVehiclePursuit.{h,cpp}`, `AIVehicle.cpp` (goal switching,
  reverse override, targeting, seek-ahead, perpetrator part), `AIVehicleRacecar.cpp` (the PID driver), `AISteer.cpp`,
  `AIPursuit.cpp` (call sites), `Physics/Behaviors/{DamageCopCar,DamageVehicle,RBCop,RBVehicle}.cpp`. Read for
  understanding; no code copied.
- **Data inputs:** AttribSys `pursuitlevels` (`SpeedReactionTime`, `CollapseAggression`), `aivehicle` (`MAXIMUM_AI_SPEED`, `TETHER_WEIGHT`,
  `TopSpeedMultiplier`, `AccelerationMultiplier`, `PlayerCollisions`), `pvehicle` (cop cars), `damagespecs`,
  `collisionreactions`. Tables are in [../formats/pursuit-data.md](../formats/pursuit-data.md).

Evidence tags as in the [docs README](../README.md#evidence-tags). **Everything is [decomp]** unless marked
**[verified]** (read from the install's AttribSys vaults with a throwaway reader). Nothing was measured in the running
game. Units are metres and seconds unless a value is written in km/h or mph. The game's unit helpers are
`KPH2MPS(x) = x / 3.6` and `MPH2MPS(x) = x * 0.44703`. The steering, throttle and brake controller that turns a
"drive target point and drive speed" into pedals is the shared driver described in the driver-control spec
(`ai-driver-control.md`); this spec only says what the cop feeds it.

## 1. Where a cop sits in the AI

A cop car is a `Behavior` (class `AIVehicleCopCar`, derived from `AIVehiclePursuit`, derived from the shared PID
driver). Its AI think task runs at a fixed 0.125 s period; each instance gets a different phase offset (the offset grows
by 0.125 s per constructed cop and wraps at 1.0 s) so the cops do not all think in the same frame. [decomp]

Per think (`AIVehicleCopCar::Update`), in order:

1. **Physics mode.** An off-world cop (no road under it, e.g. spawned over unstreamed ground) gets "simple physics"
   ([ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md) section 5); when it is back on the world it is placed 1.5 m above its position along its heading and the real
   physics resumes.
2. `AIVehiclePursuit::Update`: shared driver update (damped yaw/angular-velocity filters, simple-physics step),
   siren state machine (audio only), then the **sight test** ([ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md) section 2).
3. spawn timer (`mLastSpawnTime += dT`), reverse-override timer, targeting (`mDrivableToTargetPos` = a ray from the cop
   to the target hits no world geometry), road nav info (`mDrivableToNav` likewise for the drive-to nav point).
4. **The current goal's update** (choose an action, run it).
5. `WatchForPerps` (a patrol cop looking for someone to chase; cop-cars spec section 3).

A cop never switches goal by itself during a chase; goals are set from outside (`StartPursuit`, `StartPatrol`,
`StartRoadBlock`, `StartFlee`, the formation code's "in position" goal, the support-vehicle goal). [decomp]

## 2. Goals and actions

A goal is a list of actions in a fixed order. Every think the goal runs `ChooseAction`:

```
current_done  = current action exists and current.IsFinished()
current_score = current.score          (0 if none)
chosen = none
for action in goal.actions (in list order):
    if action != current and action.CanBeAttempted(dT):
        if current_done or action.score >= current_score:
            chosen = action; current_done = false; current_score = action.score
if chosen: current.FinishAction(); current = chosen; chosen.BeginAction()
current.Update(dT)
```

So the highest score wins, ties go to the action later in the list, a running action is only displaced by an
action of equal or higher score, and a finished action is displaced by any attemptable one. An action is never
re-selected while it is current, so an action that "finishes" while nothing else is attemptable simply keeps running.
`CanBeAttempted` may have side effects (the stuck and airborne detectors live there). [decomp]

Scores: Race 0.0, StopShort 0.0, Airborne 0.0, HeliPursuit 0.0, HeliExit 0.0, Strafe 0.0 (never attemptable),
PursuitOffRoad 0.01, Traffic 0.1, Ram 0.1, HeadOnRam 0.1, StaticRoadBlock 1.0, TooDamaged 1.0, GetUnstuck 1.0,
JackKnife 1.0. [decomp]

| Goal (name hash input) | Actions in list order | Used for |
|---|---|---|
| `AIGoalPatrol` | Traffic, TooDamaged | cruising cop looking for trouble |
| `AIGoalPursuit` | PursuitOffRoad, Race, Traffic (not if the owner is a player), TooDamaged, GetUnstuck, Airborne | normal chase; the default after `StartPursuit` |
| `AIGoalRam` | Ram, PursuitOffRoad, Race, TooDamaged, GetUnstuck, Airborne | formation "in position" goal for box-in and rolling block; also the body of the pull-over goal |
| `AIGoalPit` | same list as `AIGoalRam` | pit-manoeuvre formation goal |
| `AIGoalPullOver` | built from the `AIGoalRam` class (same list); `AIActionRam` switches on the goal *name* | the collapse (cop circle around a slow target) |
| `AIGoalHeadOnRam` | HeadOnRam, Race, TooDamaged, GetUnstuck, Airborne | heavy "ram" support SUVs; also sets the in-position offset to (0, 0, -3) |
| `AIGoalStaticRoadBlock` | StaticRoadBlock | cars of a roadblock |
| `AIGoalFleePursuit` | Race, TooDamaged, GetUnstuck, Airborne | cops released from the pursuit |
| `AIGoalStopShort` | StopShort, TooDamaged, GetUnstuck, Airborne | all cops once the target is busted |
| `AIGoalHeliPursuit`, `AIGoalHeliExit` | see [ai-helicopter.md](ai-helicopter.md) | helicopter |
| `AIGoalRacer`, `AIGoalTraffic`, `AIGoalNone` | not cop goals | racers, traffic |

`AIGoalHeliRoadBlock` is requested by `StartRoadBlock` for a helicopter but has no definition in the sources read (the
factory would return nothing). It is unreachable in practice: `roadblockhelichance` is 0 in all 21 pursuit levels
**[verified]**. [decomp]

Effective priority in a chase (`AIGoalPursuit`): stuck or destroyed (1.0) over "target not in sight, drive like traffic"
(0.1) over close chase (0.01) over path-following chase (0.0). Traffic is attemptable only while the pursuit says the
target is out of sight, and is finished as soon as the target is in sight; it drives the cop along the normal traffic
rules at `SearchModeCityMPH` (50 to 55 mph) or `SearchModeHwyMPH` (71 to 100 mph) on roads with four or more traffic
lanes while searching. Those two search speeds are read once at action start, only when the cop has been alive for a
moment and its pursuit says the target is out of sight, from the **`default`** `pursuitlevels` collection (50 mph and
71 mph), not from the heat level; otherwise traffic mode uses 35 mph (55 mph on four or more lanes). A cop is allowed
a higher cornering grip in this mode (friction 1.6 instead of 0.6 for the curvature speed limit). [decomp][verified]

## 3. Close chase: `AIActionPursuitOffRoad` (score 0.01)

The name is historical; this is the free-form chase that follows the target closely and may leave the road.

### 3.1 When

```
ShouldDoIt():
    limit = 60 m
    if target has a vehicle AI:  limit += distance(target position, target's current road point)
    dist(me, target) <= limit
    and |v_target - v_me| <= 140 km/h
    and mDrivableToTargetPos and mDrivableToNav
```

`CanBeAttempted` also requires the cop's "chicken" flag to be clear (never set anywhere in the sources read, so always
clear), and non-null interfaces. `IsFinished` is "target invalid or not ShouldDoIt". [decomp]

### 3.2 Start

If the cop has just spawned (spawn timer at 0): set its speed to 60 mph along its heading (the cop is teleported
rolling). Configure the drive-to road nav: type *direction*, lane type *cop*, cookie trail on and reset, then
re-initialise it at the cop's current lane. Reset the speed delay filter to the target's speed and initialise the
performance limiter to the cop's own speed. [decomp]

### 3.3 Update (every think)

Four steering vectors are summed in the horizontal plane (all in m/s of "desired velocity"), then limited:

```
delayed_speed = speed_history.sample(SpeedReactionTime)      # target speed SpeedReactionTime seconds ago
seek    = UpdateSeek()
avoid   = UpdateAvoidWalls()
sep     = vehicle separation (static 2.7, dynamic 5.3)       # section 3.5
affin   = UpdateRoadAffinity()                               # keep off the road edges, section 3.6
```

`speed_history` is a 16-sample ring of the target's speed over a 2.0 s window (0.125 s per slot, linear interpolation,
the sample is added every think); `SpeedReactionTime` comes from `pursuitlevels`: 0 on the default and race levels,
1.75 s (heat 1), 1.5 (heat 2), 0.75 (heat 3), 0.5 (heat 4), 0.1 (heats 5 and 6), 0.5 (heat 7). A cop at low heat
therefore chases a target that was driving a moment ago. [decomp][verified]

**Seek point** (where the cop wants to be), in words:

1. Take the target's road: `current` and `future` road points of the target's AI (`UpdateRoads`: the road point under
   the target and a point `speed * max(1, 2/speed)` metres ahead; see the road-network spec). If both are valid the
   "centre" is the current road point and the travel direction is `future - current`; otherwise the centre is the
   target's position and the direction its velocity (or body heading when slower than 1 m/s).
2. Place the seek point at `centre + dir * offset.z + side * offset.x` with `side = (dir.z, 0, -dir.x)` and `offset`
   the cop's **pursuit offset** (its formation slot, [ai-pursuit-formations.md](ai-pursuit-formations.md) section 3; zero when not in a formation).
   Do the same for the future centre to get a second point.
3. `roadoff` = (future seek point - seek point) rescaled to length `delayed_speed`; `simpleoff` = target velocity
   rescaled to `delayed_speed`. These are the "where the slot will be in one second" predictions.
4. `seekoff = seek point - cop position`. If it is not degenerate, let `ahead = dot(seekoff, dir) / |seekoff|`
   and `slide = clamp(ahead * 10, -1, 1) * 0.5 + 0.5`; `rscale = min(1, max(20 km/h, 0.2 * delayed_speed) / |seekoff|)`;
   `scale = slide * 1.8 + (1 - slide) * rscale`. The component of `seekoff` along `roadoff` is multiplied by `scale`
   (the perpendicular component is kept). A cop behind its slot (`ahead` > 0) therefore asks for 1.8 times the
   longitudinal gap (so it closes in fast), a cop ahead of its slot asks for a small fraction of the gap (so it eases
   off and lets the target come to it).
5. `tether = TETHER_WEIGHT * 0.01` (`aivehicle`, 0 for every cop collection except `copcompact`, 100) and
   `blended = lerp(roadoff, simpleoff, tether) + seekoff + position`.
6. **Stay on the road if the slot is near it.** If the drive-to nav is valid, set its lane type to *cop reckless* when the
   target's heat is at least 3, else *cop*. Advance the nav along the road in 5 m steps while the slot is more than
   5 m ahead of the nav point along the road direction, then by the remaining distance (at least 0.1 m). The lateral
   difference plus the nav's lane offset is snapped to the nearest selectable lane. If that snap moved it by less than
   4 m, accept: change lane, update the nav's *occluded* position (the position that avoids avoidables and respects the
   cookie trail) and replace `blended` by `position + (occluded - position) * |blended - position| / |occluded - position|`
   (head for the occluded road point, but keep the original distance). Otherwise (slot is more than 4 m from any
   lane: the target is off-road) leave `blended` as it is and re-initialise the nav at the cop's own position next
   think. This is the whole "off-road pursuit": when the target leaves the road the cop beelines at the slot.
7. `seek = blended - position`.

**Speed ceiling.** If the target is a vehicle with AI:

```
speedmult = TopSpeedMultiplier, accelmult = AccelerationMultiplier          # aivehicle of this cop
max_speed = MAXIMUM_AI_SPEED (km/h -> m/s)
if pursuit.IsAJerk: speedmult *= 1.2; accelmult *= 1.5; max_speed *= 1.1     # cop-cars spec section 1
max_speed = min(max_speed, target.top_speed * speedmult)
max_accel = target.accel(table at cop speed) * accelmult + forward.y * (-9.81)     # grade term
if target's nitrous is engaged: max_accel *= 0.2
performance_limiter.update(speed, max_speed, max_accel, dT)  ->  max_speed = limiter.speed_limit
seek = seek clamped to length <= max_speed
```

`target.top_speed` and `target.accel(speed)` are the target car's own computed top speed and acceleration table (built
from its `pvehicle` data when its AI was constructed; see the driver spec). The cop's ceiling is therefore relative
to the car it chases, up to its own `MAXIMUM_AI_SPEED`. The *performance limiter* is a smoothed speed limit:

```
if speed > 0:
    if speed < limit:                                   # the cop is slower than its limit: pull the limit down
        t = (limit - speed) / 5 km/h;  limit -= min(1, dT * t * t) * (limit - speed)
    limit = max(speed, limit) + max_accel * dT          # then let it grow at the allowed acceleration
    limit = min(max_speed, limit)
else: mirror image for reverse
```

**Combining.** `steer = avoid + sep + affin`. If `steer` points against `seek`
(`counter = -dot(steer, seek) / |seek| > 0.0001`): split `seek` into the part along `steer` and the rest, multiply the
along-`steer` part by `clamp((35 km/h - counter) / 25 km/h, 0, 1)` (counter in m/s, constants converted), keep the rest.
Then `steer += seek`.

**Curvature limit.** If speed and `|steer|` are above 0.1: `curv = (1 - cos(angle between velocity and steer)) * speed / |steer|`;
`limit = max(40 km/h, 450 km/h - curv * 950 km/h)` (in m/s); if `|steer| > limit` scale `steer` to `limit`.

**Output.** `SetDriveSpeed(|steer|)`, `SetDriveTarget(position + steer)`, `DoDriving(7)` (steer, gas/brake and the
reverse logic all enabled). [decomp]

**Nitrous.** After the driving call: if the cop's countdown is positive, count it down and force nitrous off. Otherwise,
if the target's nitrous input just rose (target pressed it this think and not the previous one) and the cop's own is
off, start a 1.5 s countdown (the cop will not fire its nitrous for 1.5 s after the player does). Otherwise run the
shared nitrous heuristic: want nitrous when `drive speed > 120 km/h` (90 if already on), own speed `> 80 km/h` (50) and
the speed deficit `> 60 km/h` (10) and the velocity points at the drive target
(`dot(to_target, velocity) > 0.95 * |to_target| * speed`; 0.8 if already on). Whether a cop car has nitrous at all is
`nos` data: the `copsport` family (`copsport`, `copsportghost`, `copsporthench`, `copcross`) has the `cops` tank;
`copmidsize`, `copghost`, `copsuv*` and `copgto*` use `copnonos`. [decomp][verified]

### 3.4 Wall avoidance

If speed is at least 2 m/s cast a ray from the cop along its velocity with length `min(80, speed + 10)`. On a hit with
normal `n`, `c = dot(velocity, n) / speed` (negative when heading into the wall) and `t = hit_distance / speed`; if
`c < 0`, `avoid = n * (c^2 * 10 km/h) / t^2`. Stronger the closer in time and the more head-on. [decomp]

### 3.5 Vehicle separation (cop parameters)

The shared separation steering (`VehicleSeperation`) is evaluated against every other vehicle on the cop's avoidable
list, with `absolute = 2.7` and `relative = 5.3` for cops. Each vehicle is a capsule (axis along the longer horizontal
dimension, half-length `b - a^2/b`, radius `sqrt(c^2 + a^2)` with `a = min, b = max` of width and length half
extents). The push is `s * r^2 / max(d^3, 0.1)` with `s` the closest-point vector between the two capsule axes and
`r` the radius sum; it is then scaled by `absolute + relative / (0.5 * closest_distance^2 + closest_time)^2`, where the
closest approach is computed for the current relative velocity and only counts for `closest_time > 0.05` s; the length
is clamped to `160 km/h`. The generic function belongs to the driver-control spec; the two scale numbers are the cop
tuning. [decomp]

### 3.6 Road affinity

Using the left and right edge points of the road at the cop's nav cookie, with `side` the road's right-hand
perpendicular: `llen`, `rlen` are the cop's distances from the left and right edges along `side`, plus 1 m, clamped to
at least 0.1; `sidev` is the sideways speed. `lscale = 0.5 / llen^2 + 2.0 * (max(0, -sidev) / llen)^2`, same for the
right with `max(0, sidev)`, each clamped to 100 km/h. `affinity = side * lscale - side * rscale` (pushes the cop away
from whichever edge it is closing on). [decomp]

## 4. Path-following chase: `AIActionRace` in pursuit mode

When the close-chase conditions fail (target far away, out of line of sight, too fast relative to the cop), the cop
runs `AIActionRace` in *pursuit mode* (it is also what the flee goal runs). The racing behaviour for racers is in the
racers spec; the pursuit-specific parts are:

- **Mode.** `pursuit mode` = the cop's target is the pursuit's target (or flee mode); `bDontSeekAhead` is set when the
  cop's support goal is `AIGoalHeadOnRam`. Lane type *racing*, drive-to nav in path mode with `FindPath`. [decomp]
- **Goal point.** Pursuit mode: `FindPath` to `target.GetSeekAheadPosition()`, or to the target's own position when
  `bDontSeekAhead`. The seek-ahead point is a point on the cop road network
  (path type cop, lane type cop, cop filter if the target's road segment has the "cops consider" flag) at
  `min(500, speed * 7.8 + 8)` metres along the target's road from its current road point, refreshed every 0.33 s
  (`7.8 * 0.4 = 3.12 s` instead of 7.8 s if the pursuit is flagged as a jerk). Flee mode: see 4.3. [decomp]
- **Turn around.** If the cop is faster than 40 km/h and the drive target lies behind it
  (`dot(velocity, to_target) < -0.3 * speed * |to_target|`), it aims at its own future road point with drive speed 0,
  which makes the shared driver brake and reverse-steer (the "steering behind" branch). [decomp]

### 4.1 Potential speed (pursuit branch)

All values m/s; `distant = MAXIMUM_AI_SPEED` (km/h, times 1.1 for a jerk pursuit).

```
seek_dir  = unit(last_goal_point - target.position)
off       = cop.position - target.position
along     = dot(off, seek_dir)                 # > 0: the cop is ahead of the target
fwd_near  = target.speed - (along > 0 ? 100 : 200 km/h) * 0.01 * along ; clamp to [10 km/h, distant]
rev_near  = along > 0 ? -target.speed + along * 50 km/h * 0.01 : distant ; clamp to [40 km/h, distant]
dirscale  = clamp(dot(cop.forward, seek_dir) + 0.5, 0, 1)
near      = dirscale * (fwd_near - rev_near) + rev_near
side      = |off - seek_dir * along| * 2.5 ; if along > 0: along *= 0.5
apparent  = sqrt(along^2 + side^2)
nearscale = clamp(1 - (apparent - 150) / 150, 0, 1) * clamp(|dot(unit(nav - cop), unit(target_nav - target))| + 0.2, 0, 1)
maxcop    = clamp(nearscale * near + (1 - nearscale) * distant, 0, distant)
curve     = dead_end ? 0 : SpeedLimit(curvature, f0 = start_grip, f1 = (end_grip - start_grip) / top_speed, top_speed)
result    = min(maxcop, curve) * attrib_scale * skill_scale ; then result = min(result, maxcop)
```

`attrib_scale = TopSpeedMultiplier`, doubled if the cop is in pursuit mode while a race is actively running
(`GetActivelyRacing`), else times 1.2 for a jerk pursuit. `skill_scale` interpolates 0.85 to 1.0 with skill; cops have
skill 1.0 (`GetSkill` returns 1 for cop cars). `SpeedLimit` is the shared grip-limited cornering speed
(`g1 = f1 * 9.8; v = (g1 + sqrt(g1^2 + 4 * 9.8 * f0 * |curv|)) / max(that / top_speed, 2 * |curv|)`); `start_grip`
is the lower of the car's front/rear static grip, `end_grip` that value scaled by the downforce at top speed.
`top_speed` is the cop's own computed top speed. [decomp]

Read: a cop right behind a slow target slows to roughly the target's speed (plus 20 km/h per 10 m it has to close, at most
the cop's `distant` speed), a cop far away runs at its full `MAXIMUM_AI_SPEED` limited by corners and by grip.

### 4.2 Speed limit evolution and acceleration

```
potential_accel = own_accel_table(max(speed, limit)) * AccelerationMultiplier
                  * (pursuit and racing ? 2.0 : jerk ? 1.5 : 1.0) * skill_accel(=1) * nos_scale * catchup(=1)
                  + (-9.81 * forward.y) ; floor 0
if measured accel < 0 and limit < potential_speed and limit > speed:  limit += min(potential_accel + accel, 0) * dT
if limit < potential_speed:
    t = limit / potential_speed;  e = lerp(1.5, 2.0, skill)
    limit += clamp(potential_accel - accel * t^e, 0, potential_accel) * dT
limit = clamp(limit, 0, potential_speed);  drive_speed = destroyed ? 0 : limit
```

`nos_scale` is `nos boost + 1` only while the nitrous input is on (the cop's race action also has the generic nitrous
heuristic of the racers spec, gated by skill; cops use their pursuit nitrous rules of 3.3 only in the close chase).
The check every 0.25 s `CheckOffPath` only applies when not in pursuit mode. [decomp]

### 4.3 Flee

`AIGoalFleePursuit` runs the same action with `bIsFleeMode`: the nav's race filter is cleared and the goal point is
recomputed whenever the nav is not already finding a path. Let `P` be the local player's position and `u` its velocity
direction (body forward if almost stopped). If the cop is more than 50 m from `P`, `u` is replaced by the unit vector
from the cop to `P`. The goal point is `P - 500 u`; if it is within 60 m of the previous goal point and `P` is more than
450 m from that old point, the old point is kept (stability). So a released cop drives to a spot 500 m from the player
on the far side of itself, or 500 m behind the player if it is already close. The cop manager removes cops that are out
of view; flee only gets them away. [decomp]

## 5. Ramming and the other action sets

### 5.1 `AIActionRam` (score 0.1)

Used by the box-in, rolling-block, pit and pull-over goals. It chases a slot relative to the target defined by the cop's
**in-position offset** (a different vector from the pursuit offset of the close chase). Conditions: target
drivable-to (ray clear) and the drive nav drivable-to; no chicken flag. Start: 60 mph teleport if just spawned, limiter
initialised.

```
fwd  = target.velocity if target.speed >= 5 km/h else target.body_forward ; normalised, side = (fwd.z, 0, -fwd.x)
slot = target.position + fwd * offset.z + side * offset.x
if pull-over goal and the straight line to the slot passes within (min(|slot - target|, |me - target|) - 0.2) of the target:
    go round the circle: pick the tangent point (tangent length 2.1 * |slot - target|) on the side closer to the slot
newseek = slot, then lead the target:  newseek = position + seekoff + target_vel * (0.7 * dot(seekoff, target_vel) / |target_vel|^2)
steer   = (newseek + target_vel * 1 s) - position
```

Speed: `desired = min(|steer|, performance-limited ceiling)`, the same ceiling as 3.3 (the target's top speed times
`TopSpeedMultiplier`, 1.2/1.5/1.1 jerk multipliers). `SetDriveSpeed(desired)`, `SetDriveTarget(position + steer)`,
`DoDriving(7)`, nitrous forced off.

**Pull-over (the collapse).** If the goal *name* is `AIGoalPullOver`, `aggression = CollapseAggression` of the target's
pursuit level (1.0 if the pursuit is a jerk); `staticavoid = (1 - aggression) * 4`, `dynamicavoid = staticavoid + 1.5`;
the separation steering with those two numbers is added as in 3.3 (the opposing-component weight is
`clamp((55 km/h - counter) / 55 km/h, 0, 1)` here instead of `(35 - counter) / 25`). When `|seek| < 0.5 km/h + (1 - aggression) * 0.2 km/h` (in m/s) the cop is "arrived": gas 0, brake 1, handbrake 1,
steering 0, nitrous off, and it stays that way. The ring of cops around a stopped target is the result of the formation
code ([ai-pursuit-formations.md](ai-pursuit-formations.md) section 5). [decomp]

### 5.2 `AIActionHeadOnRam` (score 0.1)

Used by the heavy-support SUVs of the "ram" strategy. Attemptable when the target is drivable-to and the drive nav is
drivable-to (no chicken). It never finishes while the target exists. At start (just spawned): speed set to
`min(60 mph, GetDesiredSpeedToTarget(distance, target_speed))`. Every think it aims at an **intercept point**:

```
Ram(my_pos, my_speed, target_pos, target_vel):         # horizontal plane
    para = unit(target_vel), perp = (para.z, 0, -para.x) ; d = target_pos - my_pos
    dpara = dot(para, d), dperp = dot(perp, d), vt = |target_vel|
    a = vt^2 - my_speed^2, b = dpara * vt, c = dpara^2 + dperp^2
    if |a| > 1e-5:  roots t = (-b +- sqrt(b^2 - a c)) / a ; take the smaller non-negative root (else the other, else 0)
    elif |b| > 1e-5: t = -c / (2b) if positive else 0
    intercept = target_pos + target_vel * t
    result = unit(intercept - my_pos) * my_speed
```

`DriveTarget = position + result`, `DriveSpeed = 100 km/h` fixed, `DoDriving(7)`. No road following, no avoidance. The
helper `GetDesiredSpeedToTarget(dist, target_speed)` (used only for the start speed) is: `dist < 0`: `max(10 mph, 0.5 dist +
target_speed)`; else `min(max(25 mph, 1.2 target_speed), 0.5 dist + target_speed)`. [decomp]

### 5.3 Stopping, stuck and airborne actions

| Action | Controls | Finishes |
|---|---|---|
| `AIActionStopShort` (0.0, always attemptable) | gas 0, brake 1, handbrake 1, steering 0, vertical steering 0, nitrous off | when the pursuit-AI "breaker" flag is clear; that flag is never set in the sources read, so it is always finished and is only displaced by an attemptable action (TooDamaged, GetUnstuck, Airborne) |
| `AIActionStaticRoadBlock` (1.0) | gas 0, brake 1, handbrake **0**, steering 0, nitrous off | never |
| `AIActionTooDamaged` (1.0) | all pedals 0; brake 0.25 if the car is not in shock and its horizontal speed is at least 2.5 m/s | never |

`TooDamaged` is attemptable when the vehicle `IsDestroyed()`; its `BeginAction` calls `EndPursuit` on the cop (lights off,
in-pursuit flag cleared). A destroyed cop coasts to a stop; the manager removes it once it can respawn and has been
off-screen long enough ([ai-pursuit-cops.md](ai-pursuit-cops.md) section 5). Roadblock cars have no TooDamaged; a destroyed
roadblock car is removed from its roadblock by the manager. `AIActionGetUnstuck` (3.0 s window, 3 m, reverse for 2.0 s at
15 m/s) and `AIActionAirborne` (controls released while no wheel touches the ground after 1.5 s of accumulated air time) are
the shared actions described in [ai-driver-control.md](ai-driver-control.md) section 7; cops use them unchanged. [decomp]

### 5.6 Not used by cops

`AIActionStrafe` is an empty shell (never attemptable). `AIActionJackKnife` (1.0) is for a tractor-trailer: it is only
in no goal list read; it triggers when speed is at least 50 mph and the time to impact with the local player (circle
approximation of the two bodies) is within (0, 4.25] s at an angle under 25 degrees to the heading (or by a message to
that vehicle), then drives straight for 0.5 s at full gas and afterwards handbrakes with full left steering, playing a
"jackknife" stream sound, and un-hitches the trailer below 10 mph. [decomp]

## 6. Sight, the jerk rule, damage and per-car tuning

How a cop sees the target, the "jerk" pursuit boost, spotting by patrol cops, simple physics, damage and the collision
weight sets are in [ai-pursuit-cop-cars.md](ai-pursuit-cop-cars.md). The numbers this spec uses from it: the jerk
flag (speed 1.2, acceleration 1.5, `MAXIMUM_AI_SPEED` 1.1, seek-ahead 3.12 s instead of 7.8 s, collapse speed 125 km/h,
pull-over aggression 1), `time_since_target_seen` (negative while the target is visible), and the per-car
`MAXIMUM_AI_SPEED`, `TopSpeedMultiplier`, `AccelerationMultiplier`, `TETHER_WEIGHT`.


## 7. Constants not in AttribSys (observed in the code)

| Name | Value |
|---|---|
| Think period (cop) | 0.125 s, 0.125 s stagger step |
| Sight test period | 0.25 s |
| Seen/not-seen counters | `-0.25` on sight, initial 99 s |
| Close-chase range | 60 m (+ target's distance to its road), 140 km/h relative speed |
| Wall ray | `min(80, speed + 10)`, strength `10 km/h * cos^2 / t^2`, min speed 2 m/s |
| Affinity | static 0.5, dynamic 2.0, cap 100 km/h |
| Separation (cop) | absolute 2.7, relative 5.3, cap 160 km/h |
| Steering curvature cap | `max(40, 450 - 950 * curv)` km/h |
| NOS lockout after the player's nitrous | 1.5 s |
| Stuck detector | 3.0 s window, 3 m, reverse 2.0 s, drive speed 15 m/s |
| Airborne trigger | 1.5 s |
| Flip death | 6 s, up.y < 0.5, ground dot < 0.5 |
| Jerk | on 3.0, off 1.75, lag factor 0.1 per second |
| Seek-ahead | `min(500, 7.8 s * speed + 8)`, refresh 0.33 s |
| Spawn teleport speed | 60 mph |

## 8. How to check it

1. **Priority.** Chase a player at 150 m with a clear road: the cop uses Race (path follow). Close to under 60 m with a
   clear line: close chase. Hide behind a wall: back to Race; break line of sight for 7 s (outside this spec): traffic
   behaviour.
2. **Speed ceiling.** Drive a slow car (top speed under 150 km/h) away from `copsuv` (multiplier 0.5, cap 50 km/h):
   it cannot catch up. Drive a fast car away from a `copsportghost`: it keeps within the car's top speed times 1.5 up
   to 400 km/h. Fire nitrous: the cop's allowed acceleration drops to 20 percent for the duration.
3. **Reaction time.** At heat 1, brake hard: cops start to ease off about 1.75 s later. At heat 5 they react in 0.1 s.
4. **Stuck.** Put a cop nose-first against a wall: after about 3 s it reverses for 2 s.
5. **Flip.** Roll a cop onto its roof: gone after 6 s; if the player flipped it, immediately.
6. **Head-on ram.** Start a "RAM" support SUV pair 340 m ahead: they should arrive on a straight line to where the
   player will be (intercept), not follow the road curve.

## Open questions

- **Q1** The direction conventions of `UMath::Lerp(a, b, t)`/`RotateInXZ` are inferred (lerp = `a + (b - a) t`; angles in
  turns). The seek blend `lerp(roadoff, simpleoff, tether)` is zero-weighted for almost every cop, so it hardly matters.
- **Q2** The shared PID driver's gains (steering and throttle) and the reverse/steering-behind logic are in the driver
  spec; verify there how drive speed above the cop's top speed is clipped.
- **Q3** `chicken` and `breaker` pursuit-AI flags are never set in the sources read; check on the PC build whether
  other code (stripped) sets them.
- **Q4** The PC build differs from the GameCube build read here in unknown ways (the original header of the PC
  `AIPursuit.cpp` is 14 percent matched). Re-measure the speed ceilings in the running game.
- **Q5** `AIActionAirborne`'s accumulator never resets; whether the PC build has the same quirk is unknown.

## Rust implementation notes

- Model goals as small enums with a fixed `[ActionKind; N]` list and implement the selection loop exactly (score, list
  order, `>=`); keep `can_be_attempted(&mut self)` mutable because the stuck and airborne detectors live there.
- Keep one `ChaseState` per cop with: speed-history ring (16 floats, 0.125 s), performance limiter (one float),
  nitrous countdown, stuck anchor/timer, flipped timer.
- The seek, avoid, separation and affinity vectors are pure functions of positions and velocities; unit-test them with
  hand-made scenes (target straight ahead, slot offset, wall ahead).
- Treat the target's top speed and acceleration table as inputs from the target's `AiVehicleModel` (computed once from
  its `pvehicle` data); do not read the player's tuning at runtime beyond that.
- The think rate (0.125 s) is part of the behaviour (reaction delays, ring buffer resolution); do not run cops at the
  frame rate.
