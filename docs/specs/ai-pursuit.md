# AI pursuit: the pursuit object, target tracking, busted and evade

How the original runs a police pursuit: the object that owns the chase, when it is created, what its status
means, how the cops decide they can see the player, how the busted meter fills and the evade (cool-down) meter
runs, and how a pursuit ends. Companion files: [ai-pursuit-heat.md](ai-pursuit-heat.md) (heat, the
`pursuitlevels` table, bounty, infractions, the HUD values), [ai-pursuit-cops.md](ai-pursuit-cops.md) (the cop
manager, spawning, counts, support rows; formations and the finisher are in [ai-pursuit-formations.md](ai-pursuit-formations.md)) and [ai-pursuit-speech.md](ai-pursuit-speech.md) (radio speech).
The road network the cops use (cop lanes, `WRoadNav`) is described in the parent road-network spec
(`ai-road-network.md`); only the calls consumed here are named. Cop steering, ramming, roadblock layouts, spike
strips and the helicopter are the sibling specs [ai-pursuit-tactics.md](ai-pursuit-tactics.md) and [ai-pursuit-formations.md](ai-pursuit-formations.md).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube build),
  under `src/Speed/Indep/Src`: `AI/AIPursuit.h`, `AI/Common/AIPursuit.cpp`, `AI/Activities/AICopManager.cpp`,
  `AI/AIVehiclePursuit.h`, `AI/Common/AIVehiclePursuit.cpp`, `AI/Common/AIVehicleCopCar.cpp`, `AI/Common/AIVehicle.cpp`
  (the perpetrator behaviour `AIPerpVehicle`), `AI/Common/AIGoal.cpp`, `AI/Actions/AIActionTooDamaged.cpp`,
  `AI/Actions/AIActionStopShort.cpp`, `Interfaces/Simables/IAI.h`, `Interfaces/SimActivities/ICopMgr.h`,
  `Sim/Common/Simulation.cpp` (task scheduling), `Frontend/HUD/FePursuitBoard.cpp`, `FeHeatMeter.cpp`,
  `FeRadarDetector.cpp`, `Animation/AnimChooseArrest.cpp`. Read for understanding; no code copied.
- **Data inputs:** AttribSys `pursuitlevels`, `pursuitescalation`, `pursuitsupport` ([formats/attributes.md](../formats/attributes.md)),
  `gameplay` race-bin fields. Values quoted as **[verified]** were read from the install's `attributes.bin` /
  `gameplay.bin` with a throwaway reader.

Evidence tags as in the [docs README](../README.md#evidence-tags). Everything not tagged is **[decomp]**. Times are
seconds, distances metres, speeds m/s unless a `mph` / `kph` is written (the data stores `BustSpeed`,
`CollapseSpeed` in km/h and the 65 mph test in mph).

## 1. Objects and who owns what

- **Perpetrator** (`IPerpetrator`): the behaviour on every car that cops can chase (the player, AI racers). It owns
  the **heat** (float), the **cost to state** (int, a running property-damage value), the **pending rep points**
  (two counters, section 5 of the heat spec), the "hidden from cars / helicopters" flags, the 911-call timer and the
  time of the last traffic hit. Its heat selects the `pursuitlevels` and `pursuitsupport` collections (section 2).
- **Pursuit** (`AIPursuit`, an `IPursuit`): a Sim activity, one per chased perpetrator, created by the cop manager.
  It holds the target, the list of cops in the chase, the contingent counts, the timers and the status of
  section 3. Everything below that is "the pursuit" means this object.
- **Cop manager** (`AICopManager`, `ICopMgr`): the singleton activity that creates pursuits, spawns and removes cop
  cars, roadblocks and support, and keeps the global limits ([ai-pursuit-cops.md](ai-pursuit-cops.md)).
- **Cop vehicle** (`AIVehicleCopCar` over `AIVehiclePursuit`): a car with the cop driver class. Its goal is one of
  `AIGoalPatrol`, `AIGoalPursuit`, `AIGoalFleePursuit`, `AIGoalStopShort`, the formation goals, the roadblock goal
  (heli: `AIGoalHeliPursuit`, `AIGoalHeliExit`).

### 1.1 Update rates (important for porting)

- The pursuit's main task runs at rate 0.25 of the sim tick, as a "variable frame" task, and a second task of the same
  object (rate 1.0) integrates the busted timer. The cop manager runs at rate 0.5 of the sim tick, "fixed frame"; the
  cop vehicles' AI at 0.125 with a staggered start; the perpetrator at 0.5. **[decomp]**
- In the decompiled scheduler a *fixed* task receives the elapsed sim time, but a *variable* task appears to receive its
  accumulated tick counter instead of seconds. That cannot be the intent (the pursuit adds this number to
  `TotalPursuitTime`, divides it into heat, and posts the whole seconds to the HUD), so this spec treats every
  `dT` of the pursuit as **real elapsed seconds since its last run**. The busted timer then fills at 1.0 per second
  (section 6). **[unconfirmed]**: the scheduler function is marked unsolved in the decomp.
- A rewrite should run the pursuit logic at 4 Hz (0.25 s step) and the busted integration every tick, in seconds.

### 1.2 Pursuit creation

A pursuit comes into being in one of three ways:

1. **A patrolling cop notices a perpetrator** (section 4.2): the cop's target is set; the cop manager sees the cop
   "asking for a pursuit" (`PursuitRequest` returns the target) and creates the pursuit for that target.
2. **Forced start** (`PursueAtHeatLevel`, message `MForcePursuitStart`): used by scripted events and by the
   *pursuit start* path zones (a perpetrator with no pursuit that drives into a zone of type `PURSUIT_START` clears the
   cop lockout and posts the message with its current heat). It raises the heat to the requested minimum, creates
   the pursuit, and spawns a first cop.
3. **Pursuit race** events and the final pursuit are scripted through the same call.

When the player's pursuit is created (target attached) the game clears the previous pursuit's post-race data, calls the
infraction manager's `PursuitStarted`, tells the gameplay manager, and raises the heat to the pursuit's **base heat** if
it is lower (section 3 of the heat spec). The perpetrator's pending rep points are cleared.

## 2. Which collection a pursuit reads

The perpetrator keeps two attribute instances and re-creates them whenever `int(heat)` changes, or when the player
goes in or out of a race event: **[decomp]**

```
idx   = int(heat) - 1                       # heat 1.0..1.99 -> table slot 0
if racing (race play mode and the race is not a pursuit race):
      pursuitlevels = pursuitescalation.racetable[idx]      pursuitsupport = pursuitescalation.supportracetable[idx]
else: pursuitlevels = pursuitescalation.heattable[idx]      pursuitsupport = pursuitescalation.supporttable[idx]
```

`pursuitescalation` has one collection (`default`) with four 10-entry reference arrays (`racetable`, `heattable`,
`supportracetable`, `supporttable`). **[verified]** Heat 0 would index -1; the heat is always clamped to at least the
pursuit's base heat, and a perpetrator starts at 1.0, so the index is valid in practice. The 57 fields of
`pursuitlevels` are in [ai-pursuit-heat.md](ai-pursuit-heat.md) section 2.

The pursuit reads its attribute through the target's `GetPursuitLevelAttrib()` every update, so a heat change takes
effect on the next run. `LockInPursuitAttribs` copies three fields at a point in time and keeps them: `FullEngagementCopCount`
into `NumCopsRequiredToEvade`, `NumCopsToTriggerBackup`, and `evadetimeout` into the required cool-down time; it also
resets the evaded-cop counter. It runs the first time the pursuit updates (when `NumCopsRequiredToEvade` is 0), when a
forced start raises the heat, and when a backup countdown ends.

## 3. The pursuit status

`ePursuitStatus` (read by the HUD, the music and the speech): **[decomp]**

| Value | Name | Meaning |
|---|---|---|
| 0 | `PS_INITIAL_CHASE` | normal chase; the status at creation |
| 1 | `PS_BACKUP_REQUESTED` | enough of the first wave is gone and a backup countdown is running |
| 2 | `PS_COOL_DOWN` | no cop has seen the perpetrator for more than 7 s; the evade meter runs |
| 3 | `PS_BUSTED` | the busted timer passed 5 s |
| 4 | `PS_EVADED` | the evade meter filled, or the player entered a safehouse during cool-down |

```
INITIAL_CHASE --(remaining wave cops <= NumCopsToTriggerBackup)--> BACKUP_REQUESTED
BACKUP_REQUESTED --(BackupCallTimer elapsed, while the perp is in sight and not busted)--> INITIAL_CHASE (re-locks the wave)
INITIAL_CHASE|BACKUP_REQUESTED --(no cop saw the perp for >7 s, and the meter mode timer > 2.5 s)--> COOL_DOWN
COOL_DOWN --(a cop sees the perp again, mode timer > 2.5 s)--> INITIAL_CHASE
COOL_DOWN --(evade level >= 1, or safehouse)--> EVADED
any --(busted timer > 5, not an online race, not bailed)--> BUSTED
```

Note that `PS_BACKUP_REQUESTED` is set only from `PS_INITIAL_CHASE` and the status is overwritten by the cool-down
transition, so backup and cool-down never overlap. `BailPursuit` (used for AI racers, section 7) does not change
the status; it sets a separate "bailed" flag.

## 4. Target tracking

### 4.1 Per-cop visibility test

Every cop car runs, each AI update while in a pursuit with a valid target, a visibility test at most every 0.25 s
(timer `VisibiltyTestTimer`). If the test passes, `TimeSinceTargetSeen` is set to -0.25, otherwise it grows by the update
step; "can see you" everywhere else means `TimeSinceTargetSeen <= 0`. A cop with no pursuit or no valid target holds the
value at 0.25. **[decomp]** The test (`CanSeeTarget` of the cop car):

```
if target is hidden from cars (a HIDDEN path zone) and this cop already failed because of it: fail without testing
front = pursuitlevels.frontLOSdistance; rear = pursuitlevels.rearLOSdistance     # of the target's current heat; fallback 150 / 50
d = distance(cop, target); dir = unit(target - cop)
out_of_sight = d >= front  or  (d >= rear and dot(cop_forward, dir) <= -0.3)
out_of_sight |= not pursuit.PursuitMeterCanShowBusted()      # true while the cool-down meter is shown for < 2.5 s
if not out_of_sight: cast a ray from cop+0.5 m up to target+0.5 m up against the world; a hit means out of sight
if out_of_sight and the target is hidden: remember "hidden from me" (latched until the hidden flag clears)
```

Heat 1 to 7 tables use front = rear = 151 (an all-round 151 m sight circle, the cone only matters if rear < front);
heat 8 to 10 use front 201, rear 101; race tables use rear 51 or 101. **[verified]**

### 4.2 Starting a pursuit from patrol (`WatchForPerps`)

A cop that is not in a pursuit checks each player-controlled car (and, when the manager allows pursuing racers, each AI
racer) every update:

1. the cop can see the car (4.1);
2. then the car is **ignored** (no pursuit) when *all* of: it is not already pursued, it has no active 911 call
   (`911 timer <= 0`), it did not hit traffic in the last 1 s, its heat is at most 3 (whole part) and its speed is
   below 65 mph (29.06 m/s). Otherwise the cop takes the car as its target (and the manager then creates the pursuit).

So at heat 1 to 3 a cop on patrol only reacts to **speeding over 65 mph, a recent traffic hit, or a 911 call**; at
heat 4 and above being seen is enough. A cop that is hit by a car on purpose (a closing speed above 4 m/s, a head-on
or deliberate hit by a player or, if allowed, a racer) also acquires that car as its target (`OnCausedCollision`).
Which infractions the original counts as speeding or reckless driving is *not* in the decompiled sources (section
4 of the heat spec).

### 4.3 The pursuit's own view (`mIsPerpInSight`, last known position)

Each pursuit update, over all active, not destroyed cops: **[decomp]**

```
time_since_any_saw = min(time_since_any_saw + dt, min over cops of cop.TimeSinceTargetSeen)    # starts at -5
in_sight           = time_since_any_saw < 7.0                       # a 7 s grace window, not "currently visible"
if a roadblock exists and the target is not hidden from cars:
      d_rb = nearest roadblock cop's distance
      if d_rb < min(distances): min = d_rb;  if d_rb < 60: in_sight = true; time_since_any_saw = 0
if in_sight: hidden_zone_time = 0; last_known_position = target position
```

The final pursuit ("epic" race) forces `time_since_any_saw = 0` every update, so it can never be evaded. **[decomp]**

**Hiding spots.** `TRACK_PATH_ZONE_HIDDEN` zones in the track path data (type "car" = hidden from cars, "heli" =
hidden from helicopters, anything else = both) set the perpetrator's hidden flags after the perpetrator has been inside
one for `latch` seconds (0.05 s when no cop currently sees it, otherwise the latch is effectively infinite until it
leaves, so a visible player does not become hidden). A zone with an elevation only counts when the car's ground height
is within 1.25 m of it. While hidden **and** not in sight, `hidden_zone_time += dt * HiddenZoneTimeMultiplier`
(4 to 8 at heat 1 to 7; 2 or 4 at heat 8 to 10). **[decomp + verified]** The field `TimeToHideInZone` (1 or 3 s) is
in the table but is not read by any code in the sources; the latch constants above are hard-coded. **[unconfirmed]**
The HUD "hiding" backing is shown when the HUD's `TimeUntilHidden` is positive; the feeder is not in the sources.

### 4.4 The pursuit meter value (HUD "busted bar")

A single float `PursuitMeter` in [-1, 1] summarises the situation; `TimeUntilBusted()` is what the HUD shows. **[decomp]**

```
if time_since_any_saw > 7:  meter = -1                                     # cool-down / lost
else if time_since_any_saw > 0.29: meter = clamp(-0.5 - time_since_any_saw / 14, -1, -0.5)     # fading
else:  # a cop sees the perp now
    meter = 0
    if D = min_distance_to_target > MeterDeadZoneEvadeDist (50):
         meter = clamp(-0.1 - 0.4 * (D - 50) / (frontLOSdistance - 50), -0.5, -0.1)       # far: negative
    else if D < MeterDeadZoneBustedDistance (35):
         if target speed > 70 mph: meter = 0
         else: Dn = clamp((35 - D) / (35 - 15), 0, 1)
               Sn = clamp((100 kph - speed) / (100 kph - BustSpeed), 0, 1)
               meter = (0.3 * Dn + 0.7 * Sn) * 0.4 + 0.1                                      # 0.1 .. 0.5
TimeUntilBusted():
    if busted_timer > 0.03:  b = min(1, busted_timer * 0.2);  return (1 - meter) * b + meter
    if evade_level >= 0.05:  return -1                                                         # cool-down display
    return meter
```

`min_distance_to_target` is the smallest 3D distance from an active, living cop (or a roadblock cop) to the target. The
HUD clamps the value to 0.99 until the pursuit is busted. **[decomp]** The HUD bar uses five segments over the range -1
to +1 (section 8).

## 5. Busted

### 5.1 The busted timer

Every pursuit update decides the timer's *rate*; the timer task adds it every tick. **[decomp]** (units: section 1.1)

```
if already busted:                         rate = +0.25 * dt   (counts on, drives the HUD time)
else if in_sight and not bailed:
      flashing = target is in the invulnerable-from-manual-reset state (the post-reset blink)
      reach    = 15 m, or 90 m when flashing
      slow     = flashing  or  target speed < BustSpeed (km/h of the heat's table: 9, 12, 19, 20, 25, 25, 19, 5, 5, 5)
      rate = +0.25 * dt  (x4 if flashing)   if slow and MinDistance_xz < reach
             -0.5  * dt                    otherwise
      rate = 0 during a world-moment cutscene
else: rate = -0.5 * dt
timer = max(0, timer + rate)       # applied per tick; net effect +1.0 / second filling, -2.0 / second draining
```

`MinDistance_xz` is the horizontal distance of the nearest cop whose height differs from the target by less than
1.5 m (a cop on a bridge above does not count). The busted speed limit in the table is in km/h and converted to
m/s; at heat 1 the player is busted when slower than 9 km/h, i.e. almost stopped, and at heat 5 and 6 at under 25 km/h.
The timer is a per-pursuit value that `SetAllBustedTimersToZero` can reset (used by the manager on restart paths).

### 5.2 Bust

When `busted_timer > 5.0` and the pursuit is not busted/bailed and the race is not an online race: **[decomp]**

- `PS_BUSTED`, `IsPerpBusted = true`.
- **Player pursuit:** every active cop gets its lights off and the in-position goal `AIGoalStopShort` (brake fully,
  hand brake, no throttle, no nitrous, until its "breaker" flag clears); message `MPerpBusted` (gameplay channel) is
  sent. 3 s of busted-HUD time later (`kBustedHUDTime = 3`) the same message goes to the cutscene channel; the cutscene
  system then plays the arrest animation (chooses a scene and camera track, places the car at an arrest marker). If no
  arrest location can be found a career-free-roam bust quits to the front end (`Infractions.fng`), a race bust pauses.
- **AI racer pursuit:** the pursuit is bailed (`BailPursuit`: flag set, `PursuitIsEvaded` called on the manager), the
  racer is marked busted and force-stopped, and a separate `AIRacerBusted` message is sent (speech uses it).
- While busted the perpetrator's controls are forced: no gas, full brake and handbrake, no steering, no nitrous.
- The evade level is forced to 0; the pursuit ends (`ShouldEnd`) once it has no cops left.

## 6. Evade (cool-down)

State kept per pursuit: `EvadeLevel` (0..1), `CoolDownTimeRemaining`, `CoolDownTimeRequired` (= table `evadetimeout`
each update while not in cool-down), `PursuitMeterModeTimer`, `CoolDownMeterDisplayed`. **[decomp]**

```
sum = time_since_any_saw + hidden_zone_time
if time_since_any_saw > 7:
    meter = -1
    evade_level = sum / required
    if not cooldown_displayed:
        evade_level = 0
        if mode_timer > 2.5:  mode_timer = 0; status = COOL_DOWN; displayed = true
                              spawn_timer = min(spawn_timer, TimeBetweenCopSpawn); backup_timer = 0
                              search_for_heli = true; if player pursuit: infraction "resisting arrest"
    else:
        evade_level = max(evade_level, 0.05)
        if evade_level >= 1: status = EVADED; evade_level = 1; if player pursuit: LockoutCops(true)
else:   # a cop saw the perp within 7 s
    if displayed: evade_level *= 0.93 (floor 0.05); if mode_timer > 2.5: displayed = false; status = INITIAL_CHASE;
                  mode_timer = 0; search_for_heli = false
    else: evade_level = 0
remaining = max(0, required - sum), clamped to GetCoolDownTimeRequired() = required - 7
if busted or bailed: evade_level = 0
```

`mode_timer` grows by dt each update; the 2.5 s gates stop the meter flickering between the chase and the cool-down
display. Because `sum` already includes the 7 s grace window, the cool-down shows `required - 7` seconds of progress
bar; this is why the HUD is given `required - 7` as the full length. Consequences in numbers (`evadetimeout`,
**[verified]**): heat 1 to 10 need 20, 45, 75, 90, 120, 120, 120, 220, 60, 220 s of "not seen" (hidden spots count
`HiddenZoneTimeMultiplier` times faster). The 0.93 decay applies once per pursuit update, so seeing the player again
drains the displayed bar within a few seconds.

**Pursuit end.** `ShouldEnd` is true when the target is gone, the evade level is at least 1, the status is `EVADED`,
or the pursuit is busted/bailed *and* it has no cops. The cop manager then (player case): counts a pursuit in a
row, records the pursuit length, notifies the gameplay manager (`NotifyPursuitEnded`), applies the career evade
adjustments (heat and impound count, [ai-pursuit-heat.md](ai-pursuit-heat.md) section 6), fires the evade
messages (`MPerpEscaped`, a music event, the milestone screen in free roam), commits the pursuit's bounty and
infractions to the career record (section 5 of the heat spec), then detaches the pursuit, makes every remaining
cop flee, and frees the activity. A busted pursuit resets the in-a-row counter and notifies the gameplay manager of a
failure instead. **[decomp]**

**Safehouse.** While in cool-down, pressing the HUD "engage event" action inside a *safehouse* trigger zone calls
`EndPursuitEnteringSafehouse`: evade level 1, status `EVADED`, flag "enter the safehouse when done". **[decomp]**

## 7. Bailing, fleeing, giving up

- `BailPursuit` is used only for AI racers (when an AI racer is busted): the pursuit stops chasing; every active cop is
  told to flee on each update while the flag is set.
- A cop's **flee** (`StartFlee`): lights off, goal `AIGoalFleePursuit` (actions race / too damaged / get unstuck /
  airborne, i.e. it simply drives off along its route); helicopters get `AIGoalHeliExit`, which un-spawns when done.
- Cops are told to flee when the contingent exceeds the wanted count (section 4 of the cops spec), when the collapse
  begins while heavy support cars exist, when the perpetrator is not in sight and heavy support is still active, and
  when the pursuit is detached.
- A cop whose car is destroyed runs the "too damaged" action: it ends its pursuit, releases all controls and brakes at
  25 % when moving faster than 2.5 m/s and not in shock. The pursuit removes destroyed and inactive cops from its list on the
  next manager pass, counts the destruction (rep points, section 5 of the heat spec) and the cop un-spawns once off
  screen long enough (cops spec section 5).

## 8. Interface the pursuit offers (what other systems read)

The `IPursuit` getters consumed by the HUD, music, speech and gameplay: `GetPursuitStatus`, `GetPursuitDuration`
(`TotalPursuitTime`, advanced only outside cool-down and only when stats may accumulate), `TimeUntilBusted`,
`IsPerpBusted`, `IsPerpInSight`, `GetEvadeLevel`, `GetCoolDownTimeRemaining`, `GetCoolDownTimeRequired`,
`GetBackupETA` (the backup countdown), `GetNumCopsFullyEngaged`, `GetNumCopsDestroyed`, `GetNumCopsDamaged`,
`GetTotalNumCopsInvolved`, `IsHeliInPursuit`, `IsCollapseActive`, `IsFinisherActive`, `GetFormationType`,
`GetMinDistanceToTarget`, `GetLastKnownLocation`, `GetRoadBlock`, `CalcTotalCostToState`, the roadblock and spike
counters and `GetCopDestroyedBonusMultiplier` / `GetMostRecentCopDestroyedRepPoints` / `...Type`. Stats accumulate only
when no race is running or when the race is a pursuit race (`AllowStatsToAccumulate`). **[decomp]**

## Constants (hard-coded, not in AttribSys)

| Name | Value | Use |
|---|---|---|
| out-of-sight grace | 7.0 s | `mIsPerpInSight`, start of cool-down |
| meter mode gate | 2.5 s | between chase and cool-down displays |
| bust timeout | 5.0 s | `kBustedTimeout` |
| bust reach | 15 m (x6 flashing) | `kBustedCopDistance` |
| bust fill / drain | 0.25 / -0.5 per second of dt (x4 flashing) | section 5.1 |
| busted HUD time | 3.0 s | second `MPerpBusted` |
| roadblock sight | 60 m | roadblock cop counts as "seeing" |
| pursuit-start speed | 65 mph, heat <= 3 | `CheckForPursuit` |
| jerk detector | lag rate 0.1 per second; on at 3.0, off at 1.75 | `UpdateJerk`: a target whose smoothed speed x10 / smoothed distance from its own lagging position is high (it circles or sits in place) is flagged "a jerk"; the formation code then raises the collapse speed to 125 kph so cops box it in at any speed |
| seconds per rep update | 10 s | pursuit rep points (heat spec 5.1) |

## Rust implementation notes

- One `Pursuit` struct per chased vehicle with the fields of section 1 (status, timers, contingent counts); step it at 4 Hz;
  integrate `busted_timer` every simulation tick.
- Keep heat on the perpetrator; resolve the three table rows by `int(heat) - 1` each time the integer part changes.
- The visibility test needs only distance, the cop's forward vector and one world ray (the collision library exists).
- The HUD needs from the pursuit: in-pursuit flag, `pursuit_meter`/`time_until_busted`, `busted` flag, `evade_level`,
  `cooldown_remaining` and `cooldown_required - 7`, `time_until_backup`, pursuit duration, cops engaged/destroyed/damaged,
  heli involved, pending rep, cost to state (heat spec section 7).
- Hidden zones need the track-path zone data (`TrackPathManager` zones of type HIDDEN and PURSUIT_START); until that data
  is read, treat the perpetrator as never hidden.

## How to check it

In the original: (1) at heat 1 drive past a parked/patrolling cop at 60 mph (nothing) and at 70 mph (chase starts);
(2) stop next to a cop: the busted bar fills in about 5 s and drains at twice the speed when you move off;
(3) break line of sight: the cool-down bar appears about 7 s later and fills in `evadetimeout - 7` s;
(4) drive into a hidden spot while unseen: the bar fills `HiddenZoneTimeMultiplier` times faster;
(5) enter a safehouse during cool-down: the pursuit ends at once.

## Open questions

- The exact meaning of `dT` for the variable-rate pursuit task (section 1.1); measure the busted-bar fill time in the game.
- Whether `TimeToHideInZone` is read anywhere (it is not in the decomp read).
- The infraction detection rules (speeding, reckless, racing): `GInfractionManager.cpp` is empty in the decomp.
- Whether `mCollapseActive` / finisher timings behave as written; the formation code is in [ai-pursuit-formations.md](ai-pursuit-formations.md).
