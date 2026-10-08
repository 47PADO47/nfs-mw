# AI pursuit: cop management

How the original decides how many cops exist, which ones, where and when they appear, when they leave, how backup
and support cars are called, and how the chasers are arranged around the player. The pursuit status and the busted and
evade rules are in [ai-pursuit.md](ai-pursuit.md); the per-heat data is in [ai-pursuit-heat.md](ai-pursuit-heat.md).
The road network (cop lanes, `WRoadNav`, `WPathFinder`) is the parent spec `ai-road-network.md`; how a cop car steers,
rams, PITs, builds roadblocks or flies a helicopter is `ai-pursuit-tactics.md`. Only the interfaces they use are named here.

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled) under
  `src/Speed/Indep/Src`: `AI/Activities/AICopManager.cpp`, `AI/Common/AIPursuit.cpp`, `AI/AISpawnManager.h`,
  `AI/Common/AISpawnManager.cpp`, `AI/AIVehiclePursuit.h`, `AI/Common/AIVehiclePursuit.cpp`,
  `AI/Common/AIVehicleCopCar.cpp`, `AI/Common/AIGoal.cpp`, `AI/AIVehicleCopCar.h`, `Interfaces/Simables/IAI.h`,
  `Interfaces/SimActivities/ICopMgr.h`, `AI/aireflectedtypes.h`. Read for understanding; no code copied.
- **Data inputs:** `pursuitlevels` (`cops`, counts, timers), `pursuitsupport` (`AirSupportOptions`, `HeavySupportOptions`,
  `LeaderSupportOptions`, `MinimumSupportDelay`), `aivehicle` (`RepPointsForDestroying`, `DetachmentID`, speed
  multipliers), `pvehicle` (the cop cars are ordinary vehicles with the cop driver class).

Everything here is **[decomp]** unless tagged. Times are seconds, distances metres.

## 1. The cop manager

A singleton activity, ticked at 0.5 of the sim rate with the real elapsed time. Each run, in this order:

1. decrement the **lockout timer** (also on non-main ticks);
2. clear the "cop that wants a pursuit" slot;
3. apply **breaker zones** (message `MBreakerStopCops`: every non-destroyed cop within the zone's horizontal radius is
   destroyed until the zone's end time; used by the gameplay "breaker" power);
4. **update patrols** (section 2);
5. **update pursuits** (sections 3 to 6);
6. **update roadblocks** (remove empty ones, mark dodged ones);
7. **service queued spawn requests** (`SpawnCop` requests from scripts: a car is placed only if its spot does not overlap
   another vehicle's radius + 2 m; failed requests stay queued);
8. decrement the heavy-support delay.

State and limits: **[decomp]**

| Item | Value |
|---|---|
| max cop cars | 8 (`min(8, platform budget 8)`) **[unconfirmed]**: PC budget not read |
| max active cop cars | 8 |
| max helicopters | 1 |
| lockout timer at construction / restart | 60 s |
| lockout after an evade (player) | 60 s, or `TimeInactiveFor911` (120 s) in free roam |
| challenge races | restart lockout 0 |
| patrol limit field | `mMaxPatrolCopCars = 2` is stored but the patrol count comes from `NumPatrolCars` |
| cop car pool | cars are created on demand by name, kept un-spawned in a list and reused (`GetAvailableCopVehicleByName`) |

**Spawning enabled?** `VehicleSpawningEnabled(isDespawn)`: false when "no new pursuits or cops" was requested (only for
spawning), when the lockout timer is above 0 or cops are disabled (`MSetCopsEnabled`, a debug / mission switch), or when a
cutscene (NIS) is active (unless it is a "world moment" and this is a despawn check). While spawning is disabled,
every active cop is removed on the next patrol pass. `LockoutCops(false)` clears the lockout and the "no new" flag
(used by the 911 call and by pursuit-start zones).

## 2. Free-roam patrols

Each manager run: **[decomp]**

```
wanted = NumPatrolCars of the global pursuitlevels (the player's current row; 1 if none)
if more than one pursuit exists: wanted = 0
elif one pursuit exists:         wanted = 0, except 1 when the current race has at least one opponent (cops hunt the racers)
if any scripted spawn request is queued: wanted = 0
current = cops whose goal is AIGoalPatrol
if wanted > current: spawn one patrol car
```

**Patrol car spawn:** take an available cop car (class "car", only names that appear in the current `cops` list, else a
random valid name, else `copmidsize`), pick a random traffic-capable road segment from the spawn manager's candidate
list (section 3.1), a rotating lane and a random point along it, require that the spot passes the checks of 3.2, put
the car on the lane (`ResetVehicleToRoadNav`), mark it spawned and start `AIGoalPatrol` (actions: traffic-style cruising
at the default `SearchModeCityMPH` 50 / `SearchModeHwyMPH` 71 mph, and the "too damaged" action).

**Per active cop** (every run, in list order): decide whether to remove it: **[decomp]**

```
respawn_available = RespawnAvailable(cop position)          # section 3.3
can_respawn = (the cop was out of the respawn zone at some time, latched, and was spawned more than 8 s ago)
              or (it is in the respawn zone and was spawned longer ago than a "never visible" time-out)   # time-out constant not read
should_remove = can_respawn and respawn_available
if the cop is destroyed: should_remove = can_respawn and the cop has been off screen > 3 s
else if should_remove and the cop is in a pursuit:
      ahead = target position + 75 m along its velocity (or heading when slower than 1 m/s)
      if distance(cop, ahead) < 375 (heli 600): should_remove = false        # do not delete cops that are about to meet you
remove if should_remove or spawning is disabled (despawn variant)
```

A removed cop is detached from its pursuit or roadblock and un-spawned (back to the pool). The first non-destroyed cop that
reports a pursuit request (patrol cop that sees a perpetrator, ai-pursuit.md 4.2) is remembered and the loop stops, so only one new
pursuit starts per run. A cop off the road network (`IsOffWorld`, not a helicopter) is switched to a simple kinematic
physics mode until it is back on the world.

## 3. Where cops appear

### 3.1 Spawn manager (shared with traffic)

Constants for cops: `min spawn distance 150`, `max spawn distance 400` (measured from the **camera** position, horizontally).
**[decomp]** It keeps a ring of up to 50 candidate road segments refreshed 20 segments per call: a segment qualifies when it
is allowed for traffic, is not a decision segment, is at least 10 m long, and one of its end nodes is within
`max spawn distance + 50` of the camera position shifted by the player's velocity vector (horizontal). Candidates that drift
farther are dropped (5 per call). A random candidate is picked per request.

### 3.2 Position validity (`CheckSpawnPosition`)

A spot is acceptable when: it is within [min, max] of the camera when distance checking is requested; it is not inside a
`NO_COP_SPAWN` track-path zone; and no active vehicle is too close: another traffic-driven car in the same lane and node
within 15 m (squared 225), or in a different lane within 5 m, or any car (not lane-checked) within 40 m (squared 1600).

### 3.3 Out-of-view rule for removing and replacing (`RespawnAvailable`)

A position may be recycled when its horizontal distance to the camera is above `min` and either above the mid-point
`(min + max)/2 = 275` m, or between `min` and 275 m and outside the camera cone (the direction to it has a dot with
the camera forward in the range [-0.707, 0.8], i.e. to the side or behind but not straight ahead). In short: cops farther
than 275 m, or beyond 150 m and not in front of the camera, can be quietly removed and respawned elsewhere.

## 4. How many cops and which: the wave

Each pursuit update asks (`CopRequest`) for the next car type to spawn, subject to the spawn timer. **[decomp]**

```
if busted, bailed or spawn_timer >= 0: nothing
allow_heli = heli timer < 0  and  pursuitsupport.MinimumSupportDelay < pursuit time
if allow_heli and "search for heli" flag (set when cool-down begins): clear flag; if the perp is out of sight and no heli active:
       roll < SearchModeHeliSpawnChance -> force the next request to be a helicopter (and the speech system announces a quadrant)
if forced heli: return "copheli"
counts = AdjustedCopCounts()        # below
need[type] = counts[type].Count - number of that type already in the contingent
if total need == 0: nothing
weights: counts[type].Chance (0 -> 100), 0 if need is 0, 0 for the heli if not allowed
pick a type by weighted random; return its name
```

**Adjusted counts:** the nominal total of cars in the row (heli not counted) is scaled by how many cops the pursuit
still needs: **[decomp]**

```
max_cops = remaining_to_evade = NumCopsRequiredToEvade - NumFullyEngagedCopsEvaded     # for the player's pursuit
           (an AI racer's pursuit: at most 3, or 2 while a player pursuit is active)
min_cops = 0;   in cool-down: min_cops = max_cops = min(NumPatrolCars, max_cops)
want = clamp(nominal_total, min_cops, max_cops)
for each non-heli record in order: n = round(Count * want / nominal_remaining); if n>0 keep it with Count=n; want -= n; nominal_remaining -= Count
heli records are copied unchanged
```

So early in a chase the wave is the full row (for example 4 cars at heat 1) but never more than the number of
cops still needed to finish the wave (`FullEngagementCopCount`, 5 at heat 1); in cool-down only `NumPatrolCars` cops are wanted.
`NumCopsRequiredToEvade` also grows: when more cops are fully engaged than remain required it increases by the difference, and by one
when the remaining count would fall below 1, and by one more when a roadblock cop is pulled into the chase.

**Removing surplus:** every update `RemoveUnwantedVehicles` compares the contingent (count per car type in the chase) with
the adjusted counts. If a type has too many, a type is picked at random weighted by the surplus and `FleeCopOfType` sends at
most 2 of them away: the farthest cops of that type (distance +40 m when they cannot see the player) are told to flee, but
a cop that sees the player flees only while more than 2 cops can see the player. Support cars never flee this way.

**Spawn timer:** when a cop joins a pursuit the next spawn waits `TimeBetweenFirstFourSpawn` while fewer than 3 cops have been
involved (and not in cool-down), otherwise `TimeBetweenCopSpawn`; if 3 or more cops are still needed every second join
uses 0.2 s ("fast spawn"). The manager spawns at most one car per pursuit per run, only while fewer than 8 cop cars are active. The
running count of cops involved (`TotalCopsInvolved`), support cars deployed, helicopters spawned and cop cars deployed
feed the cost-to-state totals.

### 4.1 Where a pursuit car appears (`SpawnPursuitIVehicle`)

Three cases, in order: **[decomp]**

1. **Cool-down:** a random spawn-manager lane position (3.1, 3.2) off in the traffic, so the extra cops show up "around town".
2. **Free roam / pursuit race (not in a race event):** from the target's *seek-ahead* point on its route, take the vector to the
   target, rotate it by 60 degrees to a random side, walk that distance along the road network's **cop path** (cop-filtered
   segments, "should cops consider" segments only), require the 150 to 400 m window of 3.2, reverse the direction so the car faces
   the player, and put the car there. If that fails (or the game is warping): a point 200 m ahead of the target along
   its velocity plus a random point in a circle of radius 190 m, snapped to the cop road network, with the same checks.
   Failure returns nothing and the timer is not reset (it retries next run).
3. **During a race event:** 230 m ahead of the target on the cop path (`min distance + 80`), facing it.

The *forced start* and the roadblock builder use other paths. A forced start (`SpawnPursuitCar`) puts the first car 340 m
ahead of the target on the cop path (80 m when warping or when the start is a free-roam scripted pursuit); if no spot is
found it falls back to a car 15 m behind the player (`SpawnCopCarNow`). When no free cop car exists in the pool for those
two paths, an active cop that has been off screen for over 5 s and is more than 100 m from the view (the farthest wins) is
**stolen**: removed from where it is and reused. A helicopter spawns 250 m behind the target on the cop path, 20 m up. **[decomp]**

## 5. Despawn, wrecked cops, per-cop giving up

- A cop leaves the chase by: being destroyed (counted: destroyed total, rep points, roadblock stats), being removed as
  surplus (flee goal), being recycled by section 2 once out of view, or being removed by the over-budget rule below.
- **Over budget** (`UpdateCopPriorities`, called when spawning a request that would exceed the platform budget): the active
  cop with the lowest priority is un-spawned, priority = dot(direction cop-to-player, player forward) + 1 if the cop is in
  view; i.e. cops behind the player and out of view go first.
- Destroyed or inactive cops are removed from the pursuit each manager run (`RemoveVehicle`); a destroyed cop is counted as
  a kill only if it was still in the pursuit and stats are allowed to accumulate.
- A cop that reaches `AIGoalFleePursuit` just drives; it is un-spawned by the patrol rule when out of view and far enough.
- A helicopter whose fuel runs out (field `HeliFuelTime`) leaves with `AIGoalHeliExit` and un-spawns; the next one waits
  `TimeBetweenHeliActive`.

## 6. Support: heavy, leader and air

`pursuitsupport` has per-heat rows `support01..10` (and `supportrace_01..10` in races): **[verified]**

| Row | Min delay | Heavy options (strategy, chance %, duration s, big SUV %) | Leader options | Air options (strategy, chance %, s) |
|---|---|---|---|---|
| 1, 2 | 60 | ram 0 | none | pursuit 0 |
| 3 | 60 | roadblock 5 (40 s), ram 10 (20 s) | none | none |
| 4 | 45 | ram 25 (30 s, 100), roadblock 25 (20 s, 100) | none | pursuit 50 (60 s) |
| 5 | 35 | ram 25 (30, 100), roadblock 40 (20, 100) | cross follow 5% (2000 s), priority 10% after 120 s | skid-hit 60 (90 s) |
| 6 | 10 | ram 20 (30, 100) | cross follow 5 (2000 s), priority 100% after 60 s | skid-hit 100 (180 s) |
| 7 | 60 | ram 30 (30, 100) | none | skid-hit 0 |
| 8 | 10 | ram 40 (20 s, 80) | none | high patrol 75 (180 s) |
| 9 | 10 | none | cross follow 100% (2000 s), priority 100% after 30 s | skid-hit 100 (2000 s) |
| 10 | 10 | none | none | skid-hit 100 (999 s) |

Strategies: heavy `E_BRAKE 1`, `COORDINATED_E_BRAKE 2`, `RAM 3`, `HEAVY_ROADBLOCK 4` (only ram and roadblock are used);
leader `CROSS_FOLLOW 5`, `CROSS_BRAKE 6`, `CROSS_PLUS_V_BLOCK 7` (only follow is used); air `HI_PATROL 0`, `PURSUIT 1`,
`SKID_HIT 2`, `SPIKE_DROP 3`. The air strategies are consumed by the helicopter code; the pursuit only asks whether
`SKID_HIT` exists (`SkidHitEnabled`). Race rows are similar with heavy-ram chances 0 up to heat 7 and 40 to 60 at heat 8 to 10.

**Request protocol and vehicles:** `RequestGroundSupport` is polled by the manager each run (at most one roll per 10 s, only while the player is in sight and the pursuit
time exceeds `MinimumSupportDelay`); the priority leader option is tried once per heat level when its `PriorityTime` has passed, otherwise one weighted roll over the heavy options (when no
roadblock exists) and then the leader options. Heavy support = 2 SUVs (`copsuv` with probability `ChanceBigSUV`, else `copsuvl`) for ram, 4 for the roadblock option; leader support = the
`copcross` car (plus two `copsporthench` for the V-block option, unused in the data). The manager spawns them on the cop path (ahead of the player at 340 m, 330, 320 ... for heavy support;
behind at 150 m for leader support), marks them with a support goal (they never count as chase cops and never flee for surplus) and cancels the request when its duration ends,
the collapse starts or the player is out of sight. Details: [ai-pursuit-formations.md](ai-pursuit-formations.md) section 6. **[decomp]**

## 7. Roadblock requests (interface only)

`RequestRoadBlock` runs every manager run: refused if busted, bailed, a roadblock exists or the roadblock timer is above 0;
requires pursuit time over `MinimumSupportDelay`; sets the timer to 8 to 12 s; rolls `roadblockprobability` (player in
sight) or the search-mode probability `SearchModeRoadblockChance x (radius - d) / radius` (player out of sight) and, on
success, arms a request: the next call that passes the timer (8 to 12 s later) returns `4`, which the manager latches until a
roadblock was built (failure keeps retrying each run). Build parameters (distance 250 m ahead on the cop path, road width test, cars needed
by width `{2,2,3,3,4,4,5}+1`, spike chance, element layouts) belong to the tactics spec. A dodged roadblock (the player's direction
relative to it flips) counts in the stats; a roadblock with no cops left is released. Adding a roadblock ends the current
formation unless it is FOLLOW or its finisher is running.

## 8. Roles and formations

Cops in a chase are not given named roles in data; the state that matters is:

- **in formation / in position:** set by the pursuit each update for non-support, non-helicopter cops that can drive to the
  target, within `60 m + distance from the target to its road centre` of it. They are matched to target offsets in the target's
  local frame (x right, z forward) by a greedy assignment that repeatedly takes the farthest unassigned cop and gives it its
  nearest free slot; the *in position* flag is set when the cop is within 4 m of its slot. Slots beyond the formation size, and cops
  outside the radius, are given ring slots at x in {-3.5, 0, 3.5} and z = +/-(25 + 5 per row of 6), alternating front/back.
- **support**, **breaker** (set by gameplay), **chicken** (stored flag), **damaged by perp** (first hit in the chase).
- **helicopter:** always in the chase list, never in a ground formation; it is given its own goal (`AIGoalHeliPursuit`) unless
  a roadblock exists or it is exiting.

Formation choice (alternating stagger-follow and a weighted random record of `CopFormations`), the slot tables, the greedy slot assignment, the finisher
timer, the collapse ring and the support-car request protocol are specified in [ai-pursuit-formations.md](ai-pursuit-formations.md); the cop-side driving in
[ai-pursuit-tactics.md](ai-pursuit-tactics.md). Facts this spec relies on: formations are re-chosen when `ActiveFormationTime` runs out (random 0 to 3 s plus the record's
duration or `StaggerFormationTime`), only while no finisher runs, the pursuit is not busted or bailed, and 15 s have passed since the speech system left its opening
"pursuit flow" focus (skipped in pursuit races and with speech off); a roadblock that is up, undodged and undamaged forces FOLLOW; `IsCollapseActive` and
`IsFinisherActive` are read by the HUD and the speech.

## Rust implementation notes

- A `CopManager` resource with the limits and timers of section 1, a `Pursuit` per target (ai-pursuit.md), and a small `CopPool` of
  un-spawned cop vehicles keyed by car name; vehicles are created through the existing car-assembly path with the cop driver class.
- The spawn manager needs the road network's segment list (traffic flag, decision flag, length, nodes, lanes) and the track-path
  zone list; build the candidate ring once per frame slice as described.
- `CopRequest` and `AdjustedCopCounts` are pure functions of the `pursuitlevels` row, the contingent and the pursuit counters:
  unit-test them against the section 4 pseudo-code.
- Cop AI (steering, goals) is a separate crate; the manager only calls `start_patrol`, `start_pursuit(target)`, `start_flee`,
  `un_spawn` and `reset_to_road(pos, forward)` on it.
- HUD needs: cops fully engaged, cops destroyed/damaged, heli involved, backup ETA, roadblock present, formation type,
  collapse/finisher flags.

## How to check it

(1) Count cop cars and types for a long chase at each heat (use the section 2.7 table and the wave rule); (2) note that the
first four spawns arrive `TimeBetweenFirstFourSpawn` apart; (3) leave a cop behind: it should disappear once out of view and
beyond 150 m; (4) after an evade, no cops spawn for the lockout period; (5) at heat 5 and 6 watch for the cross car and SUV
support after the support delay.

## Open questions

- Platform cop-car budget on PC (the decomp is the GameCube build, budget 8 assumed).
- Exact weighting of the greedy slot assignment ties; the decomp marks `AssignClosestOffsets` as not matching.
- Roadblock cop recruitment rules (spike hit pulls up to the nearest roadblock cop into the chase) are summarised only.
- Whether `copcompact` (tether weight 100) is ever spawned (it is in `aivehicle` but in no `cops` row).
