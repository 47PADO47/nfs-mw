# AI drivers: what is implemented

How the computer-driven cars of `view-world --drive` work today, which spec each part follows, where the
rewrite deliberately differs, and what is still open. The behaviour is specified in
[specs/ai-*.md](specs/README.md); this page is the map from those specs to the code.

## Try it

```sh
nfsmw view-world --drive BMWM3GTR      # then open the console (F12)
```

| Command | Effect |
|---|---|
| `traffic <count>` | keeps `<count>` traffic cars on the lanes 150 to 250 m ahead of you; `traffic off` removes them, `traffic status` lists them |
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
| Controllers: simple (traffic) and adaptive PID (cops), reverse, stuck | [`blackbox-driver`](../libs/blackbox-driver) | [ai-driver-control](specs/ai-driver-control.md), [pid](specs/ai-driver-control-pid.md) |
| Requested speed: cornering, skill, governor, pursuit cap | `blackbox-driver` (`speed`) | [ai-driver-speed-skill](specs/ai-driver-speed-skill.md) |
| Car performance (top speed, acceleration table, grip), car against car | [`blackbox-vehicle`](../libs/blackbox-vehicle) | [vehicle-rigid-body](specs/vehicle-rigid-body.md) §6 |
| Cop waves and `aivehicle` numbers from the attribute database | [`nfsmw-data`](../crates/nfsmw-data) (`pursuit`) | [ai-pursuit-heat](specs/ai-pursuit-heat.md) |
| Traffic and cop cars in the world | [`crates/nfsmw/src/scenes/world/ai`](../crates/nfsmw/src/scenes/world/ai) | [ai-traffic](specs/ai-traffic.md), [ai-traffic-spawning](specs/ai-traffic-spawning.md), [ai-pursuit-cops](specs/ai-pursuit-cops.md) |

## Checked against the install

Real-install tests (`NFSMW_GAME_DIR`, `-- --ignored`): the road records have the documented counts; stored
curve lengths match the Bézier arc length; lanes keep their side along a segment; a traffic cursor started in
every traffic lane runs 300 m without a dead end except at the dead-end nodes (16 of 6,740); routes are connected
walks with no two junction connectors in a row (mean length 1.44 times the straight distance); the cop waves
and the `aivehicle` multipliers match the tables of [ai-pursuit-heat](specs/ai-pursuit-heat.md) and
[ai-driver-control](specs/ai-driver-control.md) §10.

## Deliberate differences

- **A stopped traffic car on a free road may ask for a walking pace (1 m/s).** In the original the target speed
  is capped at the current speed plus `2 dT`, which is under the `0.5 m/s` the pedal logic treats as "stop", so a
  car that has come to a halt stays on the brake. Without the floor, junction jams never clear.
- **Stuck traffic is removed** (20 s standing, more than 40 m from the player), and cars with no wheel on the
  ground for 4 s (fallen off the streamed map) or farther than 350 m are removed.
- **Cars are spawned and removed by distance**, not by "out of view" time: the rewrite has no view test for
  cars yet.
- **Hole punching and the trail curvature** follow the structure of the specs, which mark both as unfinished or
  unsolved in the original; the cut-side and overlap tests are this rewrite's reading.
- **Car against car** uses a textbook impulse (the original's solver is not in the sources) between the
  collision boxes, once per frame.
- **Performance table** is measured by driving the car flat out (the estimator is not in the sources).

## Not implemented yet

- Traffic: lights and the scripted drag-race traffic, horns and drive-by sound, trailers and the tractor joint,
  collision reaction records (`collisionreactions`), the pattern zones (`TrafficPattern`) and pool reuse.
- Racers: the race action's nitrous, skill rubber banding, race routes and checkpoints, staging.
- Pursuit: heat growth, busted and evade rules, cop sight, formations, ram and PIT tactics, roadblocks and
  spike strips, support cars, helicopters, speech, the HUD values. Cops today only path to the player and
  drive at the speed of the race action in pursuit mode.
- Simulation level of detail for far cars, the GPS arrow, the shortcut and barrier flags of races.
