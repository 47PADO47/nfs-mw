# AI drivers: what is implemented and what is not

How the computer-driven cars of `view-world --drive` work today, which spec each part follows, where the
rewrite deliberately differs, and what is still open. The behaviour is specified in
[specs/ai-*.md](specs/README.md); this page is the map from those specs to the code.

## Try it

Traffic is on by default: drive (`nfsmw view-world --drive BMWM3GTR`) and cars appear ahead of you and go
when they are left behind. Around one car in twenty is a patrol cop. Open the console with F12.

| Setting | Config key | Environment | Command line | Default |
|---|---|---|---|---|
| Cars kept on the road around you (0: off) | `traffic` | `NFSMW_TRAFFIC` | `--traffic <n>`, `--no-traffic` | 10 |
| Share of them that are patrol cops, percent | `cop_share` | `NFSMW_COP_SHARE` | `--cop-share <0-100>` | 5 (one in twenty) |
| Cars stop at red lights at junctions (a rewrite extension, see below) | `traffic_lights` | `NFSMW_TRAFFIC_LIGHTS` | `--traffic-lights`, `--no-traffic-lights` | on |

In the console: `get traffic`, `set traffic 16`, `set cop_share 10`, `set traffic_lights off`. The defaults are the
named constants `DEFAULT_TRAFFIC`, `DEFAULT_COP_SHARE` and `DEFAULT_TRAFFIC_LIGHTS` in [`settings/mod.rs`](../crates/nfsmw/src/settings/mod.rs).

| Command | Effect |
|---|---|
| `traffic <count>` | a count that replaces the setting until it changes; `traffic off` removes the cars, `traffic status` lists them |
| `pursuit <heat>` | cops of that heat level (1 to 10) spawn 150 to 400 m away and chase you; `pursuit off`, `pursuit status` |
| `traffic warmup <s>` | for `--screenshot` runs: simulate `<s>` seconds first (a screenshot has no frame time) |

Headless: `--exec "traffic 24" --exec "traffic warmup 25" --screenshot out.png`.

## Layers

| Layer | Where | Spec |
|---|---|---|
| Road network records, lanes, curves, closest segment | [`blackbox-roads`](../libs/blackbox-roads) | [road-network](formats/road-network.md), [ai-road-network](specs/ai-road-network.md) |
| Navigator: lane cursor, traffic and direction rules, path following | `blackbox-roads` (`RoadNav`) | [ai-road-network](specs/ai-road-network.md) §2 to §4 |
| Path finding (A*) | `blackbox-roads` (`find_path`, `RoadNav::find_path_to`) | [ai-pathfinder](specs/ai-pathfinder.md) |
| Look-ahead trail, corridor cuts, steering target, curvature | `blackbox-roads` (`Trail`, `update_occluded_position`) | [ai-road-nav-trail](specs/ai-road-nav-trail.md) |
| Traffic lights: junction groups, stop lines, phases, fixed-time cycle (rewrite extension) | `blackbox-roads` (`SignalController`) | none: the original has none |
| Controllers: simple (traffic) and adaptive PID (cops), reverse, stuck | [`blackbox-driver`](../libs/blackbox-driver) | [ai-driver-control](specs/ai-driver-control.md), [pid](specs/ai-driver-control-pid.md) |
| Requested speed: cornering, skill, governor, pursuit cap | `blackbox-driver` (`speed`) | [ai-driver-speed-skill](specs/ai-driver-speed-skill.md) |
| Car performance (top speed, acceleration table, grip), car against car | [`blackbox-vehicle`](../libs/blackbox-vehicle) | [vehicle-rigid-body](specs/vehicle-rigid-body.md) §6 |
| Cop waves and `aivehicle` numbers from the attribute database | [`nfsmw-data`](../crates/nfsmw-data) (`pursuit`) | [ai-pursuit-heat](specs/ai-pursuit-heat.md) |
| Traffic and cop cars in the world | [`crates/nfsmw/src/scenes/world/ai`](../crates/nfsmw/src/scenes/world/ai) | [ai-traffic](specs/ai-traffic.md), [ai-traffic-spawning](specs/ai-traffic-spawning.md), [ai-pursuit-cops](specs/ai-pursuit-cops.md) |

## What traffic does

- **Spawning follows the original's manager** ([ai-traffic-spawning](specs/ai-traffic-spawning.md)): twice a second
  it advances the type timers (faster with density), picks the next car type by the rules of the pattern of the
  neighbourhood under you (`Rate`, `MaxInstances`, `Percent`), and adds one car 150 to 250 m ahead in a wedge in
  front of you, oncoming when you stand still and half and half when you are fast. The number of cars is the
  `traffic` setting; a pursuit takes a quarter of the density away.
- **The pattern** (which cars, 35/55 mph speeds) comes from the `trafficpattern` records of the install and the
  pattern zone under the player (`TrackPathZones` type 9). Models load on demand, one per frame.
- **Patrol cops** are cop cars of the heat 1 wave that drive like traffic at the search-mode speeds of the install
  (50/71 mph) and take bends at 1.6 g. Each new car is a patrol cop with the probability `cop_share`.
- **Cars are removed** when they are far away (350 m), when nobody has seen them for the density's off-screen
  time and they are beyond its off-screen distance (4 s and 40 m at full density, as in the original), when they
  fell off the map, or when they stand stuck away from you.

## Traffic lights: a rewrite extension

**The original has no traffic lights** ([ai-traffic-world §1](specs/ai-traffic-world.md#1-traffic-lights-and-stop-signs)):
its cars never slow down for a junction, and the signal scenery is only props. Lights here are an addition of this
rewrite, asked for by the player, and the `traffic_lights` setting turns them off (`--no-traffic-lights`) to get the
original's behaviour back. They are not a reading of the game and nothing in the specs depends on them.

- **Junctions and approaches** (`blackbox_roads::SignalController`): the nodes joined by decision segments form
  731 junction groups ([road-network](formats/road-network.md#junctions-intersections-without-records)). Every plain
  segment that can be driven into a junction node is an approach, with a stop line 8 m before the node along the
  segment (`STOP_LINE_DISTANCE`) and its heading there. A junction with three or more approaches (T and four-way
  junctions) gets signals: 718 of the 731 junctions, 2,560 approaches. The other 13 are left as they are.
- **Phases:** the approaches whose heading lies within 45 degrees of the first one's axis (either way, so the two
  opposite roads) share phase 0, the rest are phase 1. One phase is green for 14 s, amber for 3 s, then both are red for
  2 s before the other phase starts (`GREEN_TIME`, `AMBER_TIME`, `ALL_RED_TIME`). Each junction's cycle is
  shifted by 5.3 s times its index (`OFFSET_PER_JUNCTION`) so that the city does not change in step. The clock is the
  simulated time of `TrafficWorld`.
- **What a car does** (`world/ai/stop.rs`, `traffic.rs`): it looks for the stop line of the junction it is heading to
  (the ends of the segment under its cursor, plus the line it last saw) if it points the same way as the approach
  and is within 80 m. A red light, or an amber the car can still stop for (`stopping_distance`), is a stationary
  obstacle at the line: the wanted speed is `sqrt(2 * 4.5 * distance)`, so the car brakes at about 4.5 m/s squared
  and stops with its front at the line, and the walking-pace floor is dropped for the last metre so it stays on
  the brake. A car already past the line, or too close to stop on amber, drives through. A car held by a light is
  not counted idle by the 20 s stuck rule. Cops chasing the player ignore the lights; patrol cops obey them.
- **Not modelled:** turn priority and gap acceptance (cars still do not look at cross traffic), a junction jammed
  by queues (a car enters on green even if the exit is full), right turn on red, pedestrian phases, flashing amber at
  night, detection loops.

## Checked against the install

Real-install tests (`NFSMW_GAME_DIR`, `-- --ignored`): the road records have the documented counts; stored
curve lengths match the Bézier arc length; lanes keep their side along a segment; a traffic cursor started in
every traffic lane runs 300 m without a dead end except at the dead-end nodes (16 of 6,740); routes are connected
walks with no two junction connectors in a row (mean length 1.44 times the straight distance); the cop waves
and the `aivehicle` multipliers match the tables of [ai-pursuit-heat](specs/ai-pursuit-heat.md) and
[ai-driver-control](specs/ai-driver-control.md) §10; the traffic patterns match the car tables of
[ai-traffic-spawning](specs/ai-traffic-spawning.md) §4; the 705 track zones, the 11 pattern zones and the
pattern each resolves to match its §3.

## Implemented

| Area | State |
|---|---|
| Road network, lanes, curves, closest segment, navigator (traffic and direction), A* paths | done |
| Look-ahead trail, car-ahead avoidance, steering point, curvature | done (the spec marks hole punching and curvature as unfinished in the original: this is a reading of it) |
| Simple (traffic) and adaptive-PID (cop) controllers, reverse, stuck recovery | done |
| Traffic: patterns by zone, type timers, caps, density, 10 Hz think, posted speeds, accident and shock | done |
| Automatic spawn and removal around the player, `traffic` and `cop_share` settings | done |
| Traffic lights at junctions of three or more roads, `traffic_lights` setting | done, a rewrite extension (the original has none) |
| Patrol cops among the traffic | done (they cruise; see below) |
| Pursuit: wave per heat level from the data, spawn ring, path to the player, pursuit-mode speed | basic |
| Car against car | done (box impulse) |
| Measured car performance for matching cops to the player's car | done |

## Not implemented yet

- **Traffic:** stop signs, priority at junctions, trailers and the tractor joint (the semi patterns are skipped
  until trailers exist), horns and drive-by sounds, `collisionreactions` records, the scripted drag-race
  traffic and the drag pattern, pool reuse of cars, parked cars.
- **Patrol cops:** a patrol cop does not start a pursuit when it sees you (no infractions, heat or sight yet) and
  never reacts to the player. Use `pursuit <heat>` to start a chase by hand.
- **Racers:** the race action's nitrous, skill and rubber banding, race routes and checkpoints, staging,
  shortcuts and barriers, the GPS arrow.
- **Pursuit:** heat growth, busted and evade rules, cop sight and hearing, formations, ram, PIT and other
  tactics, roadblocks and spike strips, support cars, helicopters, speech, the HUD values, bounty.
- **Simulation level of detail** for far cars, and a test of the original's behaviour in the running game: the
  spec constants are from the decompilation and have not been measured against the PC build.

## Deliberate differences

- **Traffic lights exist** (`traffic_lights`, on by default) although the original has none; see
  [above](#traffic-lights-a-rewrite-extension). With the setting off the traffic is as in the original.
- **A stopped traffic car on a free road may ask for a walking pace (1 m/s).** In the original the target speed
  is capped at the current speed plus `2 dT`, which is under the `0.5 m/s` the pedal logic treats as "stop", so a
  car that has come to a halt stays on the brake. Without the floor, junction jams never clear.
- **Stuck traffic is removed** (20 s standing, more than 40 m from the player), and cars with no wheel on the
  ground for 4 s (fallen off the streamed map) are removed.
- **"In view" is a cone** (60 degrees either side of the player's heading, or within 15 m): the rewrite has no
  per-car visibility test yet.
- **The population cap is the `traffic` setting** where the original budgets 10 vehicles in all.
- **Hole punching and the trail curvature** follow the structure of the specs; the cut-side and overlap tests are
  this rewrite's reading.
- **Car against car** uses a textbook impulse (the original's solver is not in the sources) between the
  collision boxes, once per frame.
- **Performance table** is measured by driving the car flat out (the estimator is not in the sources).
