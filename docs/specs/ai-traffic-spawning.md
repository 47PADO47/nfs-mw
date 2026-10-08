# Traffic spawning: density, patterns, spawn points, removal

How the original decides how many ordinary road cars exist, which models they are, where they appear and when
they disappear. The driving of a spawned car is in [ai-traffic.md](ai-traffic.md); the world-side parts (lights,
horns, scripted drag traffic) are in [ai-traffic-world.md](ai-traffic-world.md). The road graph, lane lookups and
the navigator are in [ai-road-network.md](ai-road-network.md) and are only used here through the calls named in
[section 6](#6-where-to-spawn). Provenance: [provenance/ai-traffic.md](../provenance/ai-traffic.md). For the tag
meanings, see [evidence tags](../README.md#evidence-tags).

- **Sources read:** [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw) (CC0-1.0, decompiled; GameCube
  build), under `src/Speed/Indep/Src`: `AI/Activities/AITrafficManager.cpp`, `AI/Common/AIVehicleTraffic.cpp`,
  `AI/Common/AIVehicle.cpp` (spawn/unspawn parts), `AI/AISpawnManager.h`, `Interfaces/SimActivities/{ITrafficCenter,
  ITrafficMgr}.h`, `Misc/Table.cpp`, `World/TrackPath.{hpp,cpp}`, `World/Common/WRoadNetwork.cpp` (`CanTrafficSpawn`),
  `Sim/Activities/QuickGame.cpp`, `Frontend/Database/FEDatabase.cpp`, `Gameplay/GRace.h`, `Gameplay/GRaceStatus.h`,
  `Generated/AttribSys/Classes/{trafficpattern,gameplay}.h`. Read for understanding; no code copied.
- **Data inputs:** AttribSys class `trafficpattern` (10 collections, `GLOBAL/attributes.bin`), class `pvehicle`
  (the car list), class `gameplay` fields `TrafficPattern`, `TrafficLevel`, `ForceTrafficDensity`; the
  `TrackPathZones` chunk `0x3414A` of `TRACKS/L2RA.BUN` (zone type 9). Numbers marked **[verified]** were read from
  the install with throwaway scripts.

Everything is **[decomp]** unless marked. The decompiled sources are the GameCube build; the PC build is assumed to
share the logic, and no value below was measured in the running game.

## 1. The manager and its tick

One activity, `AITrafficManager`, is created when a game session loads (the quick-game state "loading activities",
next to the cop manager, the avoidable manager, the parked-car spawner, the path finder and the GPS). It is not
created when the debug option "skip front end, disable traffic" is set.

- It runs as a frame-variable task at **0.5 s** (offset 0.5 s). Call that tick `dT` (about 0.5 s).
- It owns a **pool** of traffic vehicles (`vehicles` below): every car it ever created, active or not. Despawning
  only deactivates a car; the car stays in the pool and can be reused for the same model.
- Per tick, in order: compute the density; if it is above zero and a pattern is valid, advance the type timers,
  advance the new-instance timer and make **one** spawn attempt; then check every active pool car for removal.
- One attempt per tick means at most one new car per 0.5 s, whatever the density.

State: the pattern in use, ten type timers `timer[0..9]` (seconds-equivalents, see section 5), a round-robin cursor
`cursor`, the new-instance timer, a scratch road cursor `nav` used for spawn probing.

## 2. Density

`density` is a number in `[0, 1]` computed every tick:

```
if a cutscene (NIS) exists or the cop manager has a cop request pending:  0
else if the debug option "skip FE" is on and traffic is not disabled:      clamp(SkipFETrafficDensity / 100, 0, 1)
else:
    d = 0
    if a race status exists:
        mode Racing:   d = clamp(race.traffic_density / 100, 0, 1)
        mode Roaming:  d = 1
    if at least one pursuit exists:  d = d * 0.75
```

- Free roam (Roaming) always asks for full density; a race asks for its own. Any other state gives 0, so nothing
  spawns while loading or in menus.
- The race value is set when the race is created: a quick race stores the menu choice Off / Light / Medium / Heavy as
  **0 / 10 / 30 / 50** (`FillCustomRace`); a career or challenge race takes it from its `gameplay` collection
  (`TrafficLevel`, an integer percentage, `race` default 100; `ForceTrafficDensity`, see section 3). The quick
  race's "random" setting picks one of the four levels. (The step that turns the `gameplay` fields into
  `traffic_density` is in a stripped source file: **[unconfirmed]**.)
- `TrafficLevel` values found in `gameplay.bin`: 0 (9 collections), 10, 15, 20, 25, 30, 35, 40, 45, 50, 60, 80 and
  100 (88 collections) **[verified]**.

Two piecewise-linear tables turn the density into rates. They are indexed by `density` over `[0, 1]` in 11 equal
steps (value at index `i` for `density = i/10`, linear in between, clamped at the ends):

| i (density) | 0 | 0.1 | 0.2 | 0.3 | 0.4 | 0.5 | 0.6 | 0.7 | 0.8 | 0.9 | 1.0 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| spawn-rate multiplier `R(d)` | 0 | 0.05 | 0.1 | 0.125 | 0.2 | 0.4 | 0.6 | 1 | 3 | 5 | 8 |
| off-screen distance `D(d)` (m) | 130 | 120 | 110 | 100 | 90 | 80 | 70 | 60 | 55 | 45 | 40 |
| off-screen time `T(d)` (s) | 12 | 10 | 9 | 8 | 7 | 6.5 | 6 | 5.5 | 5 | 4.5 | 4 |

The more traffic is requested, the faster new cars are allowed and the sooner unseen ones are recycled.

## 3. Which pattern is active

A **pattern** is one `trafficpattern` collection (section 4). The active pattern is re-evaluated every tick that
has `density > 0`:

1. **Race override.** If a race status exists, it is in Racing mode and the race has a non-zero traffic-pattern
   key, that collection is used and nothing else is looked at. The key comes from the race's `gameplay`
   field `TrafficPattern` (text): in the install only `drag` (12 collections: the drag races) and `cityhighway`
   (one tollbooth challenge) set it **[verified]**.
2. **Zone lookup.** Otherwise: take the mean position of all traffic centres (section 6), form the 2D point
   `(position.z, -position.x)` and ask the track-path manager for the first zone of type **9
   (TRAFFIC_PATTERN)** that contains it. The zone's `Data[0]` is the string hash (`h = h * 33 + c`, seed
   `0xFFFFFFFF`) of a pattern's collection name; the manager keeps a sorted map hash to collection and switches to
   the match. A hash with no collection gives key 0 (the pattern presumably becomes invalid, so nothing spawns:
   **[unconfirmed]**; all 11 zones in the install name an existing collection **[verified]**).
3. **No zone found:** the previous pattern stays. At start-up it is the `default` collection.

Switching to a different pattern resets all timers (section 5). Choosing a pattern also resets an unused
`oncoming chance` field to 0.5 (never read).

The install has **11 type-9 zones** (of 705 zones in the chunk) **[verified]**. The list is searched in file order and
the first polygon that contains the point wins; polygons overlap in places, so order matters:

| file index | pattern (`Data[0]`) | polygon points | bounding box (x0,y0)-(x1,y1) in zone space |
|---|---|---|---|
| 0 | `collegesouth` | 14 | (-173,1875)-(1968,3061) |
| 1 | `downtown` | 10 | (421,-655)-(2447,785) |
| 2 | `collegenorth` | 12 | (7,2970)-(2798,4720) |
| 3 to 6 | `collegehighway` (4 zones) | 7, 8, 7, 12 | (-658,1498)-(1479,2569), (-777,2125)-(129,4582), (-184,4138)-(2202,4819), (1166,1509)-(2829,3931) |
| 7, 8 | `cityhighway` (2 zones) | 5, 18 | (-287,-780)-(942,1087), (43,-1290)-(2764,1096) |
| 9 | `coastalsouth` | 10 | (1207,-499)-(5186,2176) |
| 10 | `coastalnorth` | 10 | (1009,1271)-(4587,4016) |

`default`, `drag` and `industrial` are never selected by a zone (`default` is the start-up pattern, `drag` is race
selected, `industrial` is unused in the data I read). Highway zones come after the neighbourhood zones in the file,
so a highway only wins where no earlier zone contains the point.

## 4. The `trafficpattern` data **[verified]**

Class `trafficpattern`, 10 collections: `default` and nine that inherit from it (`coastalnorth`, `collegenorth`,
`drag`, `downtown`, `cityhighway`, `collegehighway`, `industrial`, `collegesouth`, `coastalsouth`).

| Field | Type | Meaning | Value in the install |
|---|---|---|---|
| `CollectionName` | text | the name hashed for the zone lookup | the collection's own name |
| `SpawnTime` | float, s | minimum time between two **new vehicle instances** (see section 7); not the spacing of spawns | 4 in every collection |
| `SpeedStreet` | float, mph | cruising speed on roads with fewer than 4 traffic lanes | 35 |
| `SpeedHighway` | float, mph | cruising speed on roads with 4 or more traffic lanes | 55 |
| `Vehicles` | array of record, up to 6 in the data (the code reads up to 10) | the car list | see below |

A `Vehicles` record is 24 bytes: `Vehicle` (a reference to a `pvehicle` collection, 12 bytes), `Rate` (float),
`MaxInstances` (u32) and `Percent` (u32).

- `Rate`: how much density-weighted waiting time this type needs before it may be chosen (section 5). Small rate =
  frequent. It is also the range of the random head start given at start (section 5).
- `MaxInstances`: at most this many **active** cars of the type at once; 0 means no limit.
- `Percent`: this type may take at most this share of the traffic slots (section 5); 0 means no share limit.

The records (`rate / max / percent`):

| Pattern | Cars |
|---|---|
| `default`, `drag`, `industrial` (the same list) | `trafha` 3/0/40, `trafpickupa` 3/0/40, `trafnews` 10/1/0, `traftaxi` 6/0/20 |
| `coastalnorth` | `trafstwag` 3/0/30, `trafpickupa` 3/0/30, `trafcemtr` 60/1/10, `trafficcoup` 6/0/20, `traftaxi` 15/1/10 |
| `coastalsouth` | `traftaxi` 6/0/25, `trafficcoup` 3/0/20, `trafgarb` 60/1/10, `trafpizza` 6/0/20, `trafpickupa` 20/1/10, `traf4dseda` 6/0/25 |
| `collegenorth` | `traftaxi` 3/0/40, `trafha` 3/0/40, `trafvanb` 10/1/0, `trafstwag` 6/0/20, `trafminivan` 3/0/0 |
| `collegesouth` | `trafminivan` 3/0/30, `trafpizza` 3/0/30, `trafficcoup` 6/0/30, `trafnews` 3/0/10, `trafstwag` 6/0/30 |
| `downtown` | `traftaxi` 3/0/25, `trafgarb` 40/1/10, `trafnews` 15/1/10, `traffire` 60/1/10, `traf4dseda` 6/0/25, `trafcourt` 6/0/20 |
| `cityhighway` | `semib` 30/1/10, `semicrate` 30/1/10, `trafcourt` 10/0/20, `traf4dsedc` 6/0/20, `trafdmptr` 30/1/10, `trafnews` 20/1/20 |
| `collegehighway` | `semilog` 30/1/10, `semicon` 30/1/10, `trafminivan` 10/1/20, `traf4dseda` 6/0/20, `trafpickupa` 3/0/10, `trafficcoup` 3/0/20 |

Reading the table: the common cars (sedans, taxis, pickups) have rate 3 to 6 and a 20 to 40 % share; the special
vehicles (garbage truck, fire truck, cement truck, dumper, semis) have rate 30 to 60 and `MaxInstances = 1`, so they
are rare and never two at once. The highway patterns bring the semi trucks (tractor plus trailer). The car classes
(`street`, `van`, `truck`, `tractors`) are in [ai-traffic.md](ai-traffic.md#2-the-traffic-cars).

## 5. Choosing the next car type

Each tick with a valid pattern and `density > 0`:

```
for i in 0..9:  timer[i] += dT * R(density)
new_instance_timer += dT
spawn_one()
```

`timer[i]` therefore grows by `R` per second. At start and whenever the pattern changes or traffic is flushed,
`timer[i] = Rate_i * uniform(0, 1)` for each record (a random head start); entries beyond the pattern's record
count stay 0.

`next_type()` returns a `pvehicle` key or none:

```
n = min(record_count, 10)
repeat up to n times, starting at cursor (cursor wraps modulo n, advanced by one after every look):
    r = record[cursor]
    eligible if  timer[cursor] > r.Rate  and  r.Rate > 0
    count = number of ACTIVE pool cars of r's model
    ok_max = (r.MaxInstances == 0) or (count < r.MaxInstances)
    other = (number of pool cars) + 10 - (number of all vehicles in the world)     # unsigned
    ok_pct = (r.Percent == 0) or (count < max(1, other * r.Percent / 100))         # integer division
    if eligible and ok_max and ok_pct:  return r.Vehicle
return none
```

- `other` equals 10 minus the number of vehicles that are not traffic pool cars (the player, racers, cops, trailers
  **[unconfirmed]**). With only the player it is 9, so `Percent = 40` gives a cap of 3 cars of that type, 25 gives
  2, 10 and 20 give 1 (the `max(1, ...)` floor).
- The cursor keeps advancing after a hit, so consecutive spawns rotate through the list instead of always taking
  the first eligible entry.
- When a car is actually spawned, the `timer[i]` of every record whose model equals the spawned car is set to 0.
- Worked rate example: at density 1.0 the multiplier is 8, so a `Rate 3` entry is eligible 0.375 s after its last
  spawn and a `Rate 60` entry after 7.5 s; at density 0.5 (multiplier 0.4) they take 7.5 s and 150 s; at the quick
  race "Light" setting (0.1, multiplier 0.05) they take 60 s and 1200 s. Because only one car can spawn per
  0.5 s tick and the cap below applies, high densities mostly fill the 10-vehicle budget at the tick rate.

**Population cap (`needs_traffic`).** A spawn is only attempted while

```
(number of vehicles in the world) - (number of pool cars that are neither active nor loading)  <  10
```

That is, fewer than **10 active vehicles in total** (player, racers, cops, helicopters count as well as traffic;
cars still loading count). Racers and cops therefore squeeze traffic out.

## 6. Where to spawn

`find_spawn_point(nav)` is tried once per tick (before the type is chosen). It uses the **traffic centres**, a list
of objects that can report a basis matrix (position in the matrix's fourth row, forward in its third) and a
velocity. In the sources read there is no implementer, so this is **[unconfirmed]**, but they are clearly the
cameras / local players. The list is sorted into a pseudo-random order each call (alternating direction).

For each centre `c` (position `P`, forward `F`, velocity `V`), in that order, until one succeeds:

```
angle   = uniform(-1, 1) * 0.125 turns                     # = +-45 degrees (angles in the engine are turns)
dir     = F rotated about the vertical axis by angle
offset  = uniform(-1, 1) * 50
speed   = dot(V, dir)                                      # camera speed along dir
dist    = offset + 200 + max(speed, 0)                     # 150..250 m ahead, plus one second of camera speed
spawn   = P + dir * dist
t       = ramp(speed, 0, 50)                               # 0 at rest, 1 at 50 m/s (180 km/h)
oncoming_chance = lerp(1.0, 0.5, t)
if uniform(0, 1) <= oncoming_chance:  dir = -dir          # head towards the camera instead of away
nav.reset(); nav.init_at_point(spawn, dir, force_centre_lane = false, dir_weight = 1.0)
reject unless nav.valid and nav.can_traffic_spawn() and check_race(nav)
position = nav.position
reject if find_collisions(position)
accept
```

So traffic appears roughly 150 to 250 m (more when the camera is fast) in front, within a 90 degree wedge around the
view direction; a stationary camera always gets oncoming cars, a camera at 50 m/s or more gets half oncoming, half
same-direction. The scratch cursor has navigation type "none", lane type "racing" and **no filters**, so the closest
segment may be one that forbids traffic; `can_traffic_spawn` then rejects the attempt (no retry in the same tick).

- **`can_traffic_spawn` (road side, [ai-road-network.md §6](ai-road-network.md#6-queries)):** the cursor is valid, its
  segment is not a decision (junction connector) segment, traffic is allowed on it, it is not a one-way segment
  entered against its direction, and the lane set for the chosen direction has at least one traffic lane. A random
  traffic lane of that direction is then picked (`random_int(num_lanes)`), the cursor is moved to that lane's
  offset and its lane index is stored.
- **`check_race`:** true when no race status exists, or the mode is not Racing, or the race has no finish line;
  otherwise true only when the cursor's segment carries the **InRace** flag. In a point-to-point or lap race traffic
  therefore only appears on the race route.
- **`find_collisions` (true means "blocked"):**
  - the world-height query at the position fails (nothing solid below): blocked;
  - any traffic centre within **150 m** horizontally (squared distance below 22 500): blocked;
  - any **active** vehicle within **20 m** horizontally (squared distance below 400): blocked.
  The 150 m rule is the real minimum spawn distance. Two static values `225` and `300` (min/max spawn distance)
  exist in the class but are never read.

## 7. Creating, reusing and activating a car

`spawn_traffic()` after the type is chosen:

1. Look for a pool car with the chosen `pvehicle` key that is **inactive or still loading**. If one exists use it.
2. If none exists and `new_instance_timer > SpawnTime` (4 s), create a new pool car: a physics vehicle with driver
   class **traffic**, the chosen `pvehicle` collection, placed at the origin, attached to the manager and immediately
   unspawned (deactivated); `new_instance_timer` is reset to 0. If the timer is too small, give up for this tick.
   (So at most one new vehicle instance per 4 s; reusing pooled cars is not limited.)
3. If the car is still loading its assets, give up this tick (it is tried again on a later tick).
4. Place the car: copy the probe cursor into the car's own cursor and put the vehicle on the ground at the cursor's
   position, facing the cursor's forward vector. Failure gives up the tick.
5. `set_spawned`: reset the AI internals (spawn age 0), reset damage, restart the event sequencer, repeat for an attached
   trailer. Then **activate** the car.
6. `start_driving(speed)` with `speed = 0.75 * min(SpeedStreet, SpeedHighway)` converted to m/s (all patterns:
   0.75 * 35 mph = 11.7 m/s): clears the goal, sets the goal "traffic", resets damage and inputs, clears invulnerability and
   enables object collisions, sets the car's speed and drive speed, sets the first drive target and starts driving.
7. Post the message `SetTrafficSpeed(SpeedStreet, SpeedHighway, fixed = 0)` addressed to the car's driving action
   (it stores the two cruising speeds, section 3 of [ai-traffic.md](ai-traffic.md)).
8. Zero the timers of all records with this model (section 5).

The pool cars are also visible to the vehicle cache (the memory manager that may evict inactive vehicles): the manager
asks the cache to keep its **active or loading** cars and does not care about inactive ones, which can be evicted.
The cop manager asks the same for its own cars; none of the other callers (cutscene, race status, game manager) cares.

## 8. Removing cars

Every tick, every **active** pool car is validated; an invalid one is **unspawned** (the goal is cleared and the
vehicle is deactivated; it stays in the pool):

```
invalid = car.is_off_world()
if not invalid and car.offscreen_time > T(density):
    invalid = distance_to_camera(car.position) > D(density)
if invalid and car is a tractor with a trailer and the trailer is VALID by this same test:  keep
if invalid and any child part of the car's model (detached pieces) is in view:               keep
remove if invalid
```

- `offscreen_time` is the number of seconds the car has not been in view; `is_off_world` means the car has left the
  playable world (fell off, no ground). Their implementers are not in the sources read: **[unconfirmed]**.
- With `D(d)`, `T(d)` of section 2: at free roam (density 1) a car is removed once it has been out of view for 4 s and
  is more than 40 m from the camera; at density 0 (also used while a cutscene or a cop request makes the density 0)
  the limits are 12 s and 130 m. Cars that are always in view are never removed by distance.
- A traffic centre also blocks a spawn within 150 m (section 6), so a car cannot normally be removed and respawned in
  the same place.

**Flush.** `flush_all_traffic(release)` is exposed through the traffic-manager interface: with `release = false`
every active car is unspawned; with `release = true` every pool car is killed and the pool is emptied. In both cases
the timers get a new random head start (`Rate * uniform(0, 1)`). No caller is in the sources read (**[unconfirmed]**;
presumably race start, cutscenes and resets).

## 9. Constants

| Value | Meaning | Tag |
|---|---|---|
| 0.5 s | manager tick | **[decomp]** |
| 10 | maximum number of active vehicles in the world for a spawn; also the base of the percent cap | **[decomp]** |
| 10 | maximum record count read from a pattern (timers) | **[decomp]** |
| 4 s | pattern `SpawnTime`, all patterns | **[verified]** |
| 200 +- 50 m, plus camera speed | spawn distance ahead | **[decomp]** |
| +-45 degrees | spawn direction spread | **[decomp]** (the source has the value as 0.125 turns) |
| 1.0 to 0.5 over 0 to 50 m/s | chance of the car facing the camera | **[decomp]** |
| 150 m / 20 m | minimum spawn distance from a traffic centre / from an active vehicle | **[decomp]** |
| 0.75 | density factor while a pursuit exists; start speed factor | **[decomp]** |
| 0 / 10 / 30 / 50 % | quick-race traffic Off / Light / Medium / Heavy | **[decomp]** |

**Leftover settings that do nothing.** The AttribSys class `world` (one collection, `default`) still declares
`MAX_TRAFFIC`, `MIN_TRAFFIC_SPAWN_DISTANCE`, `MAX_TRAFFIC_SPAWN_DISTANCE`, `TRAFFIC_SPEED`, `TRAFFIC_LANE_CHANGES`,
`TRAFFIC_TYPES` and `CHECK_PLAYER_BEHIND_TRAFFIC`, inherited from an earlier game's engine. Only `TRAFFIC_TYPES` has
a value (`trafha`, `semilog`); no source read uses any of them, and a header constant `MAX_TRAFFIC_CARS = 19`
is also unused **[verified, decomp]**. The real limits are the hard-coded ones above (10 active vehicles, 150 m).
A rewrite may expose them as settings, but the originals are not tunable from data.

## 10. How to check it

In the original (PC build):

1. Free roam on foot-brake at a straight road: count cars that appear; they should appear 150 m or more away,
   mostly facing you; note the spawn rate for the first minute.
2. Drive at 180 km/h or more for a minute: about half of the new cars should be moving the same way as you.
3. Quick race with Traffic Off / Light / Medium / Heavy: with Off no traffic at all; compare the number of visible
   cars; with the percent caps expect at most 3 cars of the same common model (40 % types) when only the player is
   present.
4. Wait near the border of a neighbourhood zone: the car models should change (taxis and vans downtown, semis on the
   highways of the college and city areas, cement and dump trucks in the coastal areas).
5. Turn around and look away for 5 s: cars more than 40 m behind should be gone by the time you look back (free
   roam), which tells the removal limits.
6. Start a pursuit: expect the density to drop to 0.75 (more spawn gaps).

## 11. Open questions

- Who implements the traffic centres (all local cameras? the player cars?) and what `is_off_world` and
  `offscreen_time` measure exactly.
- How `TrafficLevel` / `ForceTrafficDensity` of a race become `traffic_density` (which wins, and the scaling): the
  code that reads them is stripped from the decompiled sources.
- Whether the vehicle count used by the cap (`Count(ALL)`) includes inactive cop/helicopter pool cars and trailers.
- Whether the first-match zone search (file order) is the same in the PC build.
- Who calls `flush_all_traffic`.
