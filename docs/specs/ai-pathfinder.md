# AI path finding (A* on the road graph) and the GPS arrow

How the original finds a route between two points on the road network, for race routes, cops, the player's
guidance and the GPS arrow. The road records are in [formats/road-network.md](../formats/road-network.md); the
navigator that follows a found path is in [ai-road-network.md](ai-road-network.md). Provenance:
[provenance/ai-road-network.md](../provenance/ai-road-network.md). For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

- **Sources read:** dbalatoni13/nfsmw (CC0-1.0): `World/Common/WPathFinder.cpp`, `World/WPathFinder.h`,
  `World/Common/WRoadNetwork.cpp` (path functions), `AI/Gps.cpp`, `AI/Common/AIVehicle.cpp` (one call site).
- **Data inputs:** the road graph; the requesting navigator (position, direction, path type, filters); goal
  position and optional goal direction; for race routes an array of "shortcut allowed" flags per shortcut
  number.
- **Conventions:** metres; costs are segment lengths along the road (chord-independent: the stored length).

## 1. The search service **[decomp]**

A single `PathFinder` activity owns a queue of searches (at most **16** at once, **3,072** search nodes shared)
and a sim task that calls `service(1 ms)` every frame. Each call works on the head of the queue until the time
budget runs out and removes finished searches. `service_all` loops with 1,000 ms budgets until the queue is
empty (used for blocking requests such as the GPS and race setup).

- `submit(nav, goal position, goal direction?, shortcut flags?)`: cancels any search already running for the
  same navigator, refuses (returns nothing) if either pool is exhausted, else enqueues a new search.
- `pending(nav)`, `cancel(nav)`: the navigator is the key. Destroying a navigator cancels its search.
- A navigator asks via `find_path` (queued) or `find_path_now` (submits then services everything and succeeds
  only if the path has at least one segment).

## 2. Creating a search **[decomp]**

1. Make a scratch navigator of direction type with the requester's path type and filters and place it at the
   goal: closest segment to the goal position with the goal direction as heading (or a fixed fake heading
   `(1, 0, 0)` if none), direction weight 0, not forced to the centre lane. If the goal cannot be placed the
   search ends as **no way**.
2. Tell the requester its path goal: the goal segment and `T`. (`distance remaining` uses it.)
3. If the goal segment is already in the requester's current cookie trail (ignoring the already-passed part),
   the search ends as **full way** immediately with no path array: the requester can just drive on.
4. When a goal direction was given the goal also fixes the **goal node** (the end the goal navigator heads to):
   the path must arrive through that node. Without direction any node of the goal segment counts.
5. Shortcut handling: if the requester is on a shortcut road (number `s`), mark `s` as cached and allowed; for
   the *race route* path type, if the caller's flag array disallows `s` the search ends as no way.
6. Seed the open list with the state "(node the requester is heading to, current segment)" at cost 0 and
   heuristic = straight distance from that node to the goal position. Unless the path type is race route,
   also seed the opposite node of the current segment, so the route may turn around immediately.

## 3. The search loop **[decomp]**

A state is `(road node, segment)`: the segment that was used to reach the node. Costs: `g` = sum of the lengths
of the segments used (shortcut roads' scale is **not** applied); `h` = straight distance from the node to the
goal position; both are stored in quarter-metre units (16-bit). The open list is kept sorted by `g + h` and
the best state is taken each step.

Take the best state; if its segment is the goal segment (and its node is the goal node, when one is fixed) the
search succeeds. Otherwise expand: for each segment `e` of the node except the one just used:

- Skip `e` if both the used segment and `e` are junction connectors (decision segments), except for cop
  paths. So normal routes pass a junction as approach → one connector → exit; cops may chain connectors.
- Skip `e` if it is not admissible for the path type ([§4](#4-admissibility)).
- `g' = g + e.length`; the next state is the node at the other end of `e`, reached by `e`.
- If a state with the same (node, segment) is already open or closed with `g ≤ g'`, drop the new one;
  otherwise remove the worse old state and insert the new one by `g' + h'`.

Finished states go to the closed list. The search stops when:

- the open list empties: **no way** (no solution);
- the node pool cannot hold the next expansion: race routes end as **no way**, other path types end as
  **half way** with the current state as a partial solution;
- the time budget of this service call is used up: the search simply continues next frame.

(Replay/capture hooks of the original, which store the iteration count of every service call so a recorded
session replays deterministically, are not needed in a rewrite.)

## 4. Admissibility

| Path type | Rules for taking segment `e` (forward = leaving through its start node) |
|---|---|
| Cop | not one-way against its direction |
| GPS | not one-way against; no barrier crossing (either kind) |
| Racer | shortcut roads only if the racer's shortcut decision allows ([road-network §5.2](ai-road-network.md#52-shortcuts-decomp)); not one-way against; no barrier crossing |
| Player | shortcut roads only if the number equals the player's current shortcut number; not one-way against; no barrier crossing |
| Race route | shortcut roads only if the caller's flag array allows; no barrier crossing; not one-way against |
| Chopper, none | everything |

"Barrier crossing" means the segment flags `CrossesBarrier` or `CrossesDriveThroughBarrier`
([road-network §5.3](ai-road-network.md#53-barriers)). Note cops and helicopters never test barriers.

## 5. Delivering the result **[decomp]**

When a search is finished (full way, half way or no way):

1. Count the segments from the solution back to the start via parent links. If there are more than the 510
   the navigator can hold, drop segments from the goal end.
2. If there is at least one segment: set the navigator's type to **path**. If the navigator is already on the
   goal segment and the goal lies behind it (dot of goal offset with the nav forward < 0), reverse the
   navigator.
3. Write the segments in order, start first. When the written segment is the navigator's own segment and the
   path enters it through a node other than the one the navigator heads to, reverse the navigator.
4. GPS paths that did not reach the goal (half way) are emptied.
5. Race-route sanity: when the request was a race route and a reversal was needed, or the same segment appears
   twice, the result is rejected: the navigator goes back to direction type with no path and the state becomes
   no way.

Following the path is done by the navigator ([ai-road-network.md §4.2](ai-road-network.md#42-choosing-the-next-segment-direction--path-navs-decomp)).
A navigator that reaches the end of its path falls back to direction mode, heading along its last forward
vector.

## 6. Who asks **[decomp]**

- Racers: a race-route path from the start to the next checkpoint, with the shortcut flags
  ([ai-racers.md](ai-racers.md)).
- The player's own driver object, while roaming or in an active race and not yet on a path: a path to the
  race/target marker (feeds the wrong-way and guidance logic, not the steering).
- Cops, helicopters and the GPS arrow with their own path types ([ai-pursuit.md](ai-pursuit.md)).

## 7. GPS arrow **[decomp]**

A world-space arrow model (`MARKER_DIRECTION_AID`) shown in the drive cameras only (not during cutscenes or
pause) that points along the route to a destination marker.

- **Engage(target, max_deviation):** state down; set deviation limit (200 m if the argument is below 0.1);
  place a GPS-type, path-type, racing-lane navigator at the player's car, force the centre lane, and
  `find_path_now` to the target (no goal direction). Fails if the car cannot be placed or no path exists;
  on success state is **tracking** and the look-ahead distance is 20 m.
- **Update (every frame):** re-place the navigator at the car (centre lane, direction weight 1). If it is not
  on the path, state becomes **searching** and the navigator is placed on the closest point of the path
  instead; if that fails, raise a "GPS lost" event and go down. Look-ahead target =
  `clamp(speed along path × 3 s, 20, 80)` m; the stored look-ahead moves towards it at 10 m/s. Advance the
  navigator by the look-ahead; the arrow aims at that position. Within 30 m of the target raise "GPS finished"
  and go down. While searching, deviation = distance to the arrow point / max deviation; above the maximum, raise
  "GPS lost".
- **Drawing:** the arrow is placed in front of the camera at a fixed distance, below centre, turning with
  `0.125…1.0` turns/s (more the larger the remaining angle), pulsing (scale 1 ± 0.1, rate 1.5 to 3.5 per second
  by deviation) while searching, and fading its pulse out at 1.5 per second when tracking.

## 8. Constants

| Value | Meaning | Tag |
|---|---|---|
| 1 ms | per-frame search budget | **[decomp]** |
| 16 / 3,072 | searches / search nodes | **[decomp]** |
| 0.25 m | cost quantisation | **[decomp]** |
| 510 | maximum path segments | **[decomp]** |
| 20 / 80 m, 3 s, 10 m/s | GPS look-ahead min / max / time / catch-up | **[decomp]** |
| 200 m, 30 m | GPS max deviation, arrival distance | **[decomp]** |

## 9. How to check it

- Route between two far nodes: the result must be a connected walk (each segment shares a node with the next),
  with no two consecutive decision segments (non-cop) and the sum of lengths close to the straight distance
  times ~1.3 on average.
- Cop paths: a route crossing a junction may use two connectors in a row; GPS and racer paths never do.
- A race-route request with a disallowed shortcut number must fail without a path.

## 10. Open questions

- The scale of A* "straight distance" vs. road length is consistent (metres); whether the opposite-node seed
  can produce a path starting with a U-turn on the same segment for racers was not verified in game.
- Whether the shipped PC build matches the GameCube-derived listing for the half-way fallback (decomp matching
  for this function on PC is partial).

## 11. Rust implementation notes

- A* over `(node, segment)` states with a binary heap; the 1 ms slicing is optional in Rust but keep the
  interface (`submit`, `poll`) so the work can be spread over frames. Quantising costs to 0.25 m affects only
  tie-breaking.
- Return `PathResult { segments, state }`; the navigator owns the path list (capacity 510).
