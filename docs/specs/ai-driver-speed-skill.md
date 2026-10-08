# AI driver: the speed target, performance matching, skill and nitrous

Where an AI car's "drive speed" comes from when it follows a route (the race action, which racers use
directly and cops use in pursuit mode): the curvature speed limit, the speed governor that mimics what the
car can really do, the matching of AI cars to the weakest player car, the skill number and the catch-up
("rubber band") multipliers that the driver layer applies, and the nitrous decision. Third part of
[ai-driver-control.md](ai-driver-control.md); the controllers that follow the speed are in
[ai-driver-control-pid.md](ai-driver-control-pid.md). Racer goals, the catch-up skill computation and the race
logic belong to the racer specs; this file documents only what the driver layer does with those numbers. For
the tag meanings, see [evidence tags](../README.md#evidence-tags).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled), under
  `src/Speed/Indep/Src`: `AI/Actions/AIActionRace.cpp`, `AI/Common/AIAction.cpp` (`performance_limiter`),
  `AI/Common/AIVehicle.cpp` (skill, glue, cheat, `compute acceleration` call), `AI/Actions/AIActionRam.cpp` and
  `AI/Actions/AIActionPursuitOffRoad.cpp` (only the use of the `aivehicle` multipliers),
  `Physics/PhysicsInfo.{hpp,cpp}`, `Physics/Behaviors/{EngineRacer,SuspensionSimple}.cpp` (consumers of the
  catch-up cheat), `Generated/AttribSys/Classes/{aivehicle,tires,chassis,rigidbodyspecs,nos}.h`. Read for
  understanding; no code copied.
- **Data inputs:** `aivehicle` (`MAXIMUM_AI_SPEED`, `AccelerationMultiplier`, `TopSpeedMultiplier`), `tires`
  (`STATIC_GRIP` front/rear), `chassis` (`AERO_COEFFICIENT`), `pvehicle` (`MASS`) and its `rigidbodyspecs`
  (`GRAVITY`, -9.8128 for cars [verified]), `nos`, the player cars' performance, the race settings (difficulty,
  catch-up flag), the character's skill level in career. Conventions as in the main file.

## 1. The per-think update of the race action

Each think (30 Hz for racers) the race action does, in this order [decomp]:

```
1  navigation / route (see ai-road-nav-trail.md, ai-pathfinder.md): choose the aim point
2  look_ahead = Table over speed_limit 0..100 m/s of [30, 100]  (drag race: also [30, 100])
   move the nav cursor so its planar distance from the car equals look_ahead; refresh the occluded position
   set the avoidable radius to look_ahead
3  curvature = nav.trail_curvature(position, velocity)
4  skill = ai.skill()
5  potential_speed = GetPotentialSpeed(curvature, skill)                 (section 3)
6  potential_accel = GetPotentialAcceleration(max(speed, speed_limit), skill, nos_on)   (section 4)
7  governor update: speed_limit                                          (section 5)
8  drive_target = occluded position; drive_speed = speed_limit (0 if wrecked or the route is invalid)
9  nitrous                                                               (section 7)
10 do_driving(7)
```

Staging (the grid before a race or drag start): `speed_limit = lerp(2.5, 3.0, skill)` mph and the drive target
is the plain nav position, no avoidance. `unstage timer` is set to 2.5 s while staging and counts down once the
car passes 24 m/s (the timer is not otherwise used). Also a separate task of the action (schedule `Physics`,
rate 0.25 with start offset 1.0, so every fourth tick) checks whether a non-pursuit car is off its route: if the nav's out-of-bounds value is above 2 m and a
fresh nav at the car's position lies on a segment that is not in the cookie trail or the path and is closer to
the road, it resets the drive-to nav to a valid lane, advances it by the look-ahead (table above, at the current
speed) and refreshes the occluded point. [decomp]

## 2. Performance matching ("AI never out-drives the weakest player car")

When the race action starts (`begin`) and when the car is prepared for a race, `compute potentials` builds the
car's effective limits. Let `players` be all player-controlled cars (local and remote).

```
bias.{top_speed, handling, acceleration}:
  career race with at least one player car:  bias = ramp(min_perf, P, 1)  = clamp((min_perf - P) / (1 - P), 0, 1)
        with P = the best (maximum) rating over all players for that stat, min_perf = the racer character's
        "minimum AI performance" (0 for a car without a character, which gives bias 0)
  anything else (quick race, roaming, cops, no race):  bias = 0
start_grip = lerp(min over {self, players} of S, S_self, bias.handling)      S = min(STATIC_GRIP.front, .rear)
end_grip   = lerp(min over {self, players} of E, E_self, bias.handling)
        E = S * (9.8128 + AERO_DOWNFORCE(top_speed) / mass) / 9.8128       (grip at top speed, section 3.1)
nos_boost  = lerp(min over {self, players} of boost, boost_self, bias.acceleration)       boost = NosBoost - 1 (>= 0)
usable_nos = (min capacity / own capacity) * (nos_boost / own boost)      0 if no usable nitrous
             (pursuit or flee mode: usable_nos = 1)
bottle_time = own NOS capacity (seconds of a full tank)                  (0 if none)
top_speed  = lerp(min over {self, players} of top_speed (> 0 only), top_speed_self, bias.top_speed)
```

So with `bias = 0` (the default outside a career race) every AI car is **limited to the weakest of itself and
the human cars** for grip, nitrous strength and capacity, and top speed. A stronger AI car is therefore tuned
down to the player's; a weaker AI car keeps its own. The ratings are `Performance { TopSpeed, Handling,
Acceleration }` from the estimator `EstimatePerformance`, whose body is not in the sources (open question 2).
`AccelerationMultiplier` and `TopSpeedMultiplier` (section 3, 4) are applied after this matching, so a cop with
`TopSpeedMultiplier` above 1 can exceed it. [decomp]

The car's own top speed and acceleration come from `GetTopSpeed()` and `GetAcceleration(speed)` of the AI
behaviour: a 10-sample table of full-throttle acceleration over speed `0..top_speed`, built once when the AI is
created from the car's `pvehicle` by `compute acceleration table`. That function is not in the sources
(open question 1). `GetAcceleration(v)` is the linear interpolation of the table at `|v|`; it is 0 when the
top speed is 0.

## 3. Potential speed

### 3.1 Cornering speed from curvature

Lateral grip is modelled as linear in speed: `mu(v) = f0 + f1 * v`. Solving `kappa * v^2 = g * mu(v)` for the
speed, capped at the top speed [decomp]:

```
g = 9.8
speed_limit_for(kappa, f0, f1, top) = n / max(n / max(0.1, top), 2 * |kappa|)       # = min(top, n / (2 |kappa|))
        n = g*f1 + sqrt((g*f1)^2 + 4 * |kappa| * g * f0)
```

For a racer (not pursuit): `f0 = start_grip`; `scale = lerp(0.36, 0.90, skill)` (the table `[0.36, 0.9]` over
skill 0..1); `f1 = (lerp(start_grip, end_grip, scale) - start_grip) / top_speed`. So low-skill drivers use
only 36 percent of the extra grip that downforce would give at top speed, skilled drivers 90 percent.
`end_grip` uses the downforce of section 2 of [vehicle-steering-assists-aero.md](vehicle-steering-assists-aero.md)
(`downforce(v) = v * 2 * AERO_COEFFICIENT * 1000` N, linear in speed) at `top_speed`: `end_grip = start_grip *
(9.8128 + downforce(top)/mass) / 9.8128`. A dead end ahead (nav says so) gives 0. In a **drag race** the speed
limit is simply the top speed.

### 3.2 Pursuit mode (the same action driven by a cop, or fleeing)

If the race action was started inside a pursuit (and not in flee mode) it replaces the skill-scaled cornering limit
by `f0 = start_grip`, `f1 = (end_grip - start_grip) / top_speed` (no skill) and also limits the speed by the
distance to the target [decomp]:

```
distant = MAXIMUM_AI_SPEED km/h (x1.1 if the pursuit flags the target "is a jerk")
seek_dir = unit(last_find_position - target.position)           # where the path search was aimed, seen from the target
off = position - target.position;  s = dot(off, seek_dir)        # > 0: the cop is ahead of the target
fwd_near = clamp(target.speed - (s > 0 ? 100 : 200) km/h * 0.01 * s, 10 km/h, distant)
rev_near = s > 0 ? clamp(-target.speed + s * 50 km/h * 0.01, 40 km/h, distant) : distant
near = lerp(rev_near, fwd_near, clamp(dot(forward, seek_dir) + 0.5, 0, 1))
side = 2.5 * |off - seek_dir * s|;   if s > 0: s *= 0.5;   apparent = hypot(s, side)
near_scale = clamp(1 - (apparent - 150) / 150, 0, 1) * clamp(|dot(steer_dir, target_steer_dir)| + 0.2, 0, 1)
max_cop_speed = clamp(near_scale * near + (1 - near_scale) * distant, 0, distant)
potential = min(max_cop_speed, cornering limit)
```

In flee mode the normal (skill) cornering limit is used. `MAXIMUM_AI_SPEED` is therefore a ceiling that only
pursuit cars feel, and it is also the starting `max_speed` of the ram and off-road pursuit actions (section 6).

### 3.3 Final scale

```
attrib = TopSpeedMultiplier
if pursuit mode and a race is actively running:   attrib = attrib * 2
elif the pursuit flags the target "is a jerk":    attrib = attrib * 1.2
skill_scale = lerp(0.85, 1.0, skill)                  (drag: the same table [0.85, 1.0])
potential_speed = potential * attrib * skill_scale
if pursuit mode: potential_speed = min(max_cop_speed, potential_speed)
```

(The pursuit-mode line `attrib += attrib` is a doubling.) [decomp]

## 4. Potential acceleration

```
a0 = lerp(min over {self, players} of acceleration_table(speed), own acceleration_table(speed), bias.acceleration)
mult = AccelerationMultiplier
if pursuit mode and a race is actively running: mult *= 2      elif "jerk": mult *= 1.5
accel_scale = lerp(0.65, 1.0, skill)        (drag: lerp(0.82, 1.0, skill))
nos_scale = using_nos ? (nos_boost + 1) : 1
catchup = AiCatchupAcceleration(cheat)       Table over cheat 0..1 of [1, 1.33, 1.66, 2.0]
grade = (mode == simulated) ? -9.81 * forward.y : 0         # uphill takes acceleration away, downhill adds
potential_accel = max(a0 * mult * nos_scale * accel_scale * catchup + grade, 0)
```

`speed` for the table is `max(actual speed, speed_limit)`. [decomp]

## 5. The speed governor

`speed_limit` is the speed the driver *asks for*. It is not the potential speed: it follows it at the
acceleration the car is expected to have, so the pedal does not demand an impossible speed and the car does not
leap ahead of its own limits. Per think with `dT` the think interval and `a_act = (speed - last_speed) / dT`:

```
if a_act < 0 and speed_limit < potential_speed and speed_limit > speed:
        speed_limit += min(potential_accel + a_act, 0) * dT       # pull the request down toward reality when slowing
if speed_limit < potential_speed:
        t = ramp(speed_limit, 0, potential_speed);  e = lerp(1.5, 2.0, skill)
        d = clamp(potential_accel - a_act * t^e, 0, potential_accel)
        speed_limit += d * dT
speed_limit = clamp(speed_limit, 0, potential_speed)
```

So the request rises by the expected acceleration minus a fraction (`t^e`) of what the car is really achieving
(which, when the car is already speeding up as fast as expected, nearly stops the request from running away),
and snaps down immediately to a lowered `potential_speed` at corners. `begin` sets `speed_limit = max(speed, 0)`.
[decomp]

## 6. The other multiplier users, and `performance_limiter`

The ram and off-road pursuit actions use a shared speed limiter plus the same multipliers [decomp]:

```
max_speed = MAXIMUM_AI_SPEED km/h; speedmult = TopSpeedMultiplier; accelmult = AccelerationMultiplier
if the pursuit says "jerk": speedmult *= 1.2; accelmult *= 1.5; max_speed *= 1.1
max_speed = min(max_speed, target.top_speed * speedmult)
max_accel = target.acceleration_at(speed) * accelmult                (off-road: + slope term, x0.2 while the target burns nitrous)
limiter.update(speed, max_speed, max_accel, dT)       # performance_limiter
desired_speed = min(requested speed, limiter.speed_limit)
```

`performance_limiter.update(speed, max_speed, max_accel, dt)` for forward speed:

```
if speed < limit:  t = (limit - speed) / 5 km/h;  limit -= min(1, dt * t^2) * (limit - speed)      # fall toward real speed
limit = max(speed, limit) + max_accel * dt
limit = min(max_speed, limit)
```

(a mirrored form is used for negative speeds). The effect is a request that stays within `max_accel` of the
current speed. The pursuit specs describe the actions around it.

## 7. Nitrous decision

### 7.1 Race action

Only for AI cars (never the autopilot player). `was_nos = controls.nos and tank > 0`.

```
potential_nos(speed):  # seconds of nitrous the AI is willing to use now
    0 if speed < 10 m/s or speed >= speed_limit, or no nitrous, or usable_nos <= 0
    useable = usable_nos * lerp(0.33, 1.0, skill);  off_limit = 1 - useable
    needed  = was_nos ? off_limit : useable * lerp(0.5, 0.3, skill)
    if tank <= needed: 0   else (tank - needed) * bottle_time
can = not turning-around, not wrecked, not staging, potential_nos > 0
if can:
    sc = lerp(0.25, 1.0, skill)                     (+1.0 if a speed trap is close, below)
    speed_gap = sc * (potential_speed - speed);   accel_gap = sc * (potential_accel - a_act)
    if not was_nos:
        off_time = lerp(20, 4, skill)
        want = nos_timer < -off_time and not occluded and speed_gap > 15 m/s and accel_gap > 0.5 * potential_accel
    else:
        want = true; on_time = lerp(2, 4, skill)
        if nos_timer > on_time and (speed_gap < 5 or accel_gap < 0.15 * potential_accel): want = false
nos_timer = want ? max(nos_timer + dT, 0) : min(nos_timer - dT, 0)
controls.nos = want
```

The timer counts seconds *on* (positive) or *off* (negative), so a car must have been off for `off_time`
seconds (20 s at skill 0, 4 s at skill 1) before starting, and starts the race with the timer at 0, i.e. it
waits `off_time` first. Speed-trap races: if a speed-trap trigger is enabled within `speed * lerp(1, 3, skill)`
metres ahead, in front (dot > 0.5), and reachable within the nitrous time on offer (or nitrous is already on),
`sc` gets `+1`. The check only runs in a speed-trap race while racing, above 10 m/s, and when the car is already
burning or within 5 m/s of its `speed_limit`. [decomp]

### 7.2 Pursuit (off-road action) uses `DoNOS`

Hysteresis on the desired and actual speeds and alignment [decomp]:

```
on_state = controls.nos
want = desired > (on ? 90 : 120) km/h and speed > (on ? 50 : 80) km/h and (desired - speed) > (on ? 10 : 60) km/h
       and dot(target - position, velocity) > (on ? 0.8 : 0.95) * |target - position| * speed
```

## 8. Skill and catch-up

### 8.1 Skill

`skill() = clamp(base_skill + glue_skill, 0, 1)` for racers and cops (the human autopilot reports 1.0;
traffic and non-perp AI 0) [decomp].

`base_skill`, computed when a racer joins a race (and again when prepared for it) [decomp]:

- Quick race: `[0.15, 0.40, 1.00]` indexed by difficulty (easy, medium, hard) when the catch-up setting is on;
  `[0.15, 0.40, 0.80]` when catch-up is off.
- Career: `c = character.SkillLevel * 0.01` (0..1); `adaptive` = the race status' adaptive difficulty;
  if `adaptive > 0` multiply by `Table[0.5, 0.75, 1.0](c)`, if `<= 0` by `Table[0.5, 0.375, 0.25](c)`;
  `base = clamp(c + adaptive, 0, 1)`.
- Not racing, or the player's own car: 0. A perp object that was never given race info keeps its constructor
  value **0.5** (cops).

### 8.2 Glue (rubber band) in the driver layer

For racers that are racing and not staging, once per second the driver records
`glue_error = (race_length / 100) * (average player percent complete - own percent complete) * performance_ratio`
into a 10-sample error history, with `performance_ratio = 1 + (1 - player.TopSpeed_rating) * 0.5`. The race status
turns this history into `glue_output` and `glue_skill` (a signed number added to the skill) in
`ComputeCatchUpSkill`, which is not in the sources (open question 3). The driver then scales a positive glue
skill by `Table[0.33, 0.66, 1.0](base_skill)` (by 0.5 in speed-trap races) and a negative one by
`Table[1.0, 1.0, 0.66](base_skill)` (by 0.5 in speed-trap races); a car in emulated off-world mode keeps the
raw value. [decomp]

### 8.3 The catch-up cheat and its consumers

`cheat = clamp(base_skill + glue_skill - 1, 0, 1) * 0.5` (constant table `[0.5, 0.5, 0.5]`), 0 for humans.
Maximum 0.5. Used by [decomp]:

| Consumer | Effect with cheat `c` |
|---|---|
| race action acceleration | `x Table[1, 1.33, 1.66, 2](c)`: 1.0 to 1.5 |
| engine drive torque (positive only) | `x (1 + 0.5 c)` |
| nitrous drain / recharge | drain `x lerp(1, 0.5, c)`, recharge `x lerp(1, 2, c)` |
| simple chassis aerodynamics | drag `x (1 - c)`, downforce factor `lerp(1, 1.5, c)` |
| simple chassis (racer, racing style) | steering drag reduction `lerp(0.23, 0.5, c)` |

The last two rows live in the simple (cop) chassis. In the shipped data racers use the racer chassis, which
does not read the cheat [verified: `pvehicle` table in [ai-simulation-lod.md](ai-simulation-lod.md)], and cops
never have a cheat, so those two rows are inactive; do not port them unless a car with a simple chassis
and a racer driver exists.

Cops: skill reads 0.5 (constructor value) + glue 0, cheat 0. Skill-driven scales for a base skill of 0.5:
cornering scale 0.63, speed scale 0.925, acceleration scale 0.825. [decomp]

## 9. Constants

All [decomp] unless marked. NOS: min speed 10 m/s, speed gap on 15 / off 5 m/s, accel gap on 0.5 / off 0.15,
off time 20..4 s, on time 2..4 s, availability 0.33..1, needed capacity 0.5..0.3, trap time 1..3 s. Scales over
skill: speed `[0.85, 1.0]`, drag speed `[0.85, 1.0]`, acceleration `[0.65, 1.0]`, drag acceleration `[0.82, 1.0]`,
cornering `[0.36, 0.9]`, acceleration exponent `1.5..2.0`, nitrous availability scale `[0.25, 1.0]`. Nav
look-ahead `[30, 100]` over 0..100 m/s (human autopilot: `[50, 60]`; human drag `[8, 40]`). Quick-race skills
as above. `kTurnAroundSpeed` 40 km/h (pursuit turn-around: if the car moves faster than that and its velocity points
more than ~107 degrees from the drive target, it steers at the future road point with speed 0).
`AiSeparation` tables and `fDragDifficulty = 0.5` are unused by the sources read.

## 10. How to check it

- Park an AI car at a known speed on a long curve of known radius and read the requested speed: it should equal
  `speed_limit_for(1/R, f0, f1, top)` with the quoted constants for the car's tire and chassis data.
- Launch a racer from rest on a straight: `speed_limit` should rise at roughly the car's real acceleration (not
  jump to the top speed) and the car should track it within a few m/s.
- Put a stronger AI car beside a weaker player car in a quick race: the AI's top speed and grip match the
  player's; in career with a high `MinimumAIPerformance` they separate.
- Count nitrous uses of a skill-1 racer on an open road: the first burn can start after 4 s, the next only after
  the previous ended more than 4 s ago.

## 11. Open questions

1. `compute acceleration table` and `EstimatePerformance` are not in the sources. A faithful rebuild: the
   10 samples are the full-throttle acceleration of the car at 10 evenly spaced speeds between 0 and the
   top speed (gearbox at the best gear, no tire slip), the top speed being where the drivetrain and drag
   balance (or the speed limiter). Calibrate against a recorded AI launch **[unconfirmed]**.
2. The three ratings' definition and the 7 `PerformanceWeights` entries.
3. `ComputeCatchUpSkill` (inputs: the glue history, the race status, whether the car is off world).
4. Whether the unstage timer influences anything (only set and decremented in the sources).

## 12. Rust implementation notes

- A `SpeedGovernor { speed_limit, last_speed, last_accel }` with `update(...)` as section 5 and the pure
  functions `speed_limit_for_curvature`, `potential_speed`, `potential_accel`.
- The vehicle crate should expose a one-off `performance_table(&VehicleSpec) -> ([f32; 10], top_speed)`
  computed by a headless full-throttle run on flat ground (cache it per car), `start_grip()`
  (`min(static_grip front, rear)`), `aero_downforce(speed)` (exists in the aero module), `mass`, and the nitrous
  boost and capacity (exist in `NosSpec`).
- Skill, glue and cheat belong in the game crate (they read race state); the cheat multipliers need hooks in
  `blackbox-vehicle`: a `catchup: f32` field on `Vehicle` applying drive torque, nitrous rates, and drag/downforce
  factors as in section 8.3.
- Performance matching needs the list of player cars' `(grip, nitrous, top speed, acceleration table)`; build it
  once per race start, not per tick.
