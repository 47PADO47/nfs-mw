# AI road network: lanes, road geometry and the road navigator

How the original turns the road records of [formats/road-network.md](../formats/road-network.md) into
positions, lanes and "where do I drive next" for every AI car, the GPS arrow and the spawn code. The
navigator is a per-car cursor on the road graph (`WRoadNav` in the original); traffic, racers and cops all
steer towards the point it produces. Look-ahead trails and obstacle shaping are in
[ai-road-nav-trail.md](ai-road-nav-trail.md), path finding in [ai-pathfinder.md](ai-pathfinder.md).
Provenance: [provenance/ai-road-network.md](../provenance/ai-road-network.md). For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

- **Sources read:** dbalatoni13/nfsmw (CC0-1.0, decompilation): `World/WRoadNetwork.h`, `World/WRoadElem.h`,
  `World/Common/WRoadNetwork.cpp`, `World/Common/WGrid.h`, `World/Common/WGridNode.h`, `World/TrackPath.*`.
  Numbers verified on the install (PC v1.3) with throwaway scripts.
- **Data inputs:** `RNgp` records (nodes, segments, profiles, roads) and the grid's road-segment lists in
  `TRACKS/L2RA.BUN`; shortcut and barrier-exemption markers of the current race; the track path barriers.
- **Conventions:** physics space, metres, y up; angles are not used (vectors only). `T` is the segment
  parameter in 0…1 along the travel direction.

## 1. Model

### 1.1 Directions and sides **[decomp]**

A segment is stored from node `[0]` to node `[1]`. A navigator is always on one segment and heads towards one
end; it keeps `node_ind`, the index of the node it is **heading to** (1 = stored direction, 0 = against it).
`T` runs from the node it came from (0) to the node it heads to (1), so travelling against the stored
direction means `T = 1 − T_stored`. Leaving a segment through the end it heads to, at the node it just
reached, selects the next segment ([§4](#4-moving-along-the-graph)).

Lane offsets are signed distances from the road centre line, positive on the **right of the travel
direction**. A segment's local right vector at an end is `(forward.z, 0, −forward.x)` normalised, where
`forward` is the stored handle direction at that end; positions are `node + right × sign × offset` with
`sign = +1` when heading to node 1 and `−1` when heading to node 0, so the same lane offset describes the same
physical lane in both directions of travel.

### 1.2 Profiles and lanes **[decomp + verified]**

A node's profile lists its zones left to right in the stored direction. Zones `0 … middle−1` lie left of the
centre line, zones `middle …` right of it; zones right of the middle are the **forward** lanes (travel in the
stored direction), the others are **backward** lanes (right-hand traffic). The per-zone numbers are the
width, the offset (always positive in the file) and the zone type; the side is taken from the zone index
(`relative offset = −offset` for zones left of the middle).

Mirroring: the segment's `StartInverted` / `EndInverted` flags say the profile of that end node is stored for
the opposite orientation. When inverted, the zone order is reversed (`index' = zones − 1 − index`) and the
middle becomes `zones − middle`. A helper "invert xor backward" is used wherever the travel direction can flip
the reading order: for a navigator heading to node 0, forward and backward swap as well.

Counts used by the callers: `forward lanes = zones − middle`, `backward lanes = middle`. "Traffic lanes" are
the zones of type 1 on one side; "nth traffic lane" counts from the centre outwards, "nth from the curb"
counts from the outer edge inwards; both fall back to the nearest traffic lane (or the middle) when `n` is out
of range.

Lane position along a segment: the profile offset at the start node and at the end node are
interpolated linearly with `T`. The navigator keeps one lane index and the lane offset of that lane, so a lane
that exists at both ends stays the same lane across the whole segment.

### 1.3 Zone-type masks **[decomp + verified]**

Two 8-entry mask tables, indexed by the navigator's **lane type**, say which zone types a car may drive on
("drivable") and which it may choose as a target lane ("selectable"). Bit `n` is zone type `n`
(1 traffic, 2 sidewalk, 3 shoulder, 4 median, 5 curb median, 6 grass median, 7 barrier, 8 train, 9 parking,
10 road, 11 center, 12 drivable, 13 undrivable):

| Lane type (id) | Drivable zone types | Selectable zone types |
|---|---|---|
| Racing (0) | all except 7, 13 | 1, 10 |
| Traffic (1) | 1 | 1 |
| Drag (2) | all except 7, 13 | all except 2, 5, 7, 13 |
| Cop (3) | all except 2, 7, 13 | 1, 4, 5, 6, 10 |
| CopReckless (4) | all except 7, 13 | all except 7, 13 |
| Reset (5) | 1, 10 | 1, 10 |
| StartingGrid (6) | all except 7, 13 | 1, 10 |
| Any (7) | all | all |

Cops therefore never drive on sidewalks but may cross medians; racers drive anywhere except barriers but aim
for traffic or plain road lanes.

### 1.4 Curve geometry **[decomp + verified]**

For an uncurved segment the centre line is the straight line between the two nodes. For a `Curved` segment
it is a cubic with control points: node A, node A + handle A, node B + handle B, node B (confirmed to be a
Bézier by comparing arc lengths with the stored length, see the format page). A lane line is the same curve
displaced sideways: its end points are the nodes shifted by `right × offset` at each end, and its handles are
the node handles **scaled by the ratio** (distance between the shifted end points) / (distance between the
two nodes), so an outside lane line gets longer handles. The navigator re-evaluates this curve whenever the
segment, the lane offset or a lane change changes it.

Evaluating at `T` gives a position, a tangent (forward vector, not normalised) and for curved segments a
planar (xz) curvature. Straight segments report curvature 0 and use the end point difference as forward vector.

## 2. The navigator object

State **[decomp]**: `valid`; `segment` (index); `node_ind`; `T`; `lane_ind` and `lane_offset` (current), the
lane-change fields (`from`/`to` offsets, `change_dist`, `change_inc`); `position`, `forward`, `curvature`
at the cursor; the start/end points and handles of the current lane line; an optional cookie trail
([ai-road-nav-trail.md](ai-road-nav-trail.md)); an optional path (up to 510 segments, [ai-pathfinder.md](ai-pathfinder.md));
`dead_end` (set when the cursor could not leave a segment); `vehicle_half_width` (1.0 m unless an AI vehicle is
attached; then the car's half dimension; +2 m extra for tractor-class cars in drag races).

Three enumerations choose the behaviour:

- **Nav type:** none, **traffic** (follow the lane graph like a driver), **direction** (go where a
  target vector points), **path** (follow an explicit segment list).
- **Path type:** cop, none, racer, GPS, player, chopper, race route. It selects which segments are
  admissible and how barriers and shortcuts are treated (§5, [ai-pathfinder.md](ai-pathfinder.md)).
- **Lane type:** the eight values of [§1.3](#13-zone-type-masks-decomp--verified).

Four boolean **filters** restrict which segments are considered when initialising and when walking:
race filter (only segments on the race route, in race direction; effective only while a race route exists),
traffic filter (only segments where traffic is allowed), cop filter (only segments cops should consider:
`NoTraffic xor CopsXorTraffic` is false means allowed), decision filter (skip junction connectors when
initialising).

## 3. Placing a navigator

### 3.1 Closest segment **[decomp]**

Given a point, a heading and a direction weight `w`:

1. Ask the grid for cells within 32 m of the point; collect the road segments listed there (type 3), each
   once.
2. Skip segments excluded by the active filters.
3. For each, find the closest point on the segment: first project onto the straight line between the nodes
   (clamped to the ends); if the segment is curved, refine along the curve by walking in steps of 1.0 m and
   then 0.25 m in the better direction.
4. Plain score = distance to that point. With `w > 0` (callers use 1.0 for most; 0 for the goal of a path
   search) the distance is measured to the **edge of the road** instead: the lateral distance minus the
   half width to the left or right outermost zone edge (interpolated between the end profiles), so a car
   inside the road has zero side distance; then add `w × 10 × (1 − |dot(heading, segment direction)|)`.
   For one-way segments the dot is signed: driving against the flow scores up to 20 more.
5. Take the lowest score (initial threshold 20,000). Return the segment, the closest point and `T`.

### 3.2 Initialising on a segment **[decomp]**

`InitAtSegment(segment, T, position, heading, force_centre_lane)`:

- Direction: the cursor goes in the stored direction when `dot(heading, segment forward) ≥ 0`; with the race
  filter it follows the race direction flag instead. `T` is mirrored when going backward.
- Lane offset: 0 with `force_centre_lane`; otherwise the signed lateral distance of the given position from
  the centre line (cross product with the forward vector). For a **traffic** nav that offset is then snapped to
  the nearest traffic lane: for each traffic lane counted from the curb, interpolate its offsets at start and
  end by `T`, keep the closest, and remember its lane index.
- Then build the lane line, handles and curve, evaluate at `T`, and reset the cookie trail.

`InitAtSegment(segment, lane, T)` (used for traffic spawning) picks the direction from the lane: lanes right
of the profile's middle zone run in the stored direction, others against it; the exception is one-way
segments, which always run in the stored direction.
`InitFromOtherNav` copies another cursor, optionally reversed (`node_ind` flipped, `T = 1 − T`).
`InitAtPoint` = closest segment + `InitAtSegment`. `InitAtPath(position)` first finds the closest point on the
current path's segments (straight-line distance) and starts there, with the direction taken from the path order.

## 4. Moving along the graph

### 4.1 Advancing **[decomp]**

`advance(distance, target_direction, max_lookahead)`:

- The step changes `T` by `distance / chord`, where `chord` is the straight distance between the current lane
  line's start and end points (not the arc length; on curves the cursor therefore moves slightly differently
  from the metres asked). While `T` stays ≤ 1 the lane offset is updated for an active lane change and the
  curve is evaluated.
- Crossing the end: spend the distance up to `T = 1`, ask the next-segment function for the new segment,
  new `node_ind` and the lane offset to continue with, rebuild the lane line (starting at the old end point,
  ending at the new segment's far end at that lane offset), set `T = 0` and carry on with the remaining
  distance. If the function returns the same segment and direction the cursor sets `dead_end = 1`; a traffic
  cursor then stops where it is.
- With a cookie trail the step is split into pieces of at most `1.1 × cookie_gap` and a cookie is recorded
  after each piece. `cookie_gap` is 3 m, or `clamp(max_lookahead / 26, 1, 3)` when a look-ahead is given.

### 4.2 Choosing the next segment: direction / path navs **[decomp]**

At the node reached, with `toward` the desired direction (the target vector):

- **Path nav:** if the current segment is in the path list and has a successor, take the successor: if the
  successor shares the node just reached, continue on it (direction chosen so the cursor leaves through its
  other node); if it does not share it, the cursor turned around (flip `node_ind`, stay on the segment).
  At the end of the list: a GPS path stops; other paths drop to direction mode.
- **Direction nav:** let `next` be the plain (non-decision) segment at the node other than the current one.
  - Dead end node (one segment) or on a decision segment with no plain successor: turn around.
  - Otherwise, if a plain successor exists (the cursor left a junction or is on a chain node), take it.
  - Otherwise (the cursor entered a junction through its approach road) examine each decision segment of the
    node. Each candidate is walked forward up to 19 further segments or about 100 m, always continuing
    through the plain segment of the next node. A candidate is dropped when any walked segment: crosses a
    barrier that this path type respects; fails the active filters (race membership and direction, traffic
    allowed, cop consideration); is a shortcut the car may not take (§5); is one-way against the walking
    direction; or already deviates more than the best candidate so far. Candidates are scored by the **worst deviation** along the walk: the
    maximum over walked segments of `|dot(segment direction, toward) − target|`, with target 1.0 (straight
    to the target direction), or a uniform random in [0, 1) per call when the global "random turns" debug
    switch is on. The candidate with the smallest worst deviation wins, so the car prefers turning that keeps
    pointing along `toward` for the next ~100 m.
  - If nothing qualifies the cursor turns around.
- A direction nav with a zero target vector uses the current segment's forward vector as target.

After choosing, the lane offset to continue with is the nearest selectable lane of the new segment to the
current offset ([§4.4](#44-lane-snapping)).

### 4.3 Choosing the next segment: traffic navs **[decomp]**

The cursor tracks which traffic lane it is in, counted from the centre: `nth`. At the end node:

- If the node has a plain successor (chain node or leaving a junction): take it and keep the lane by taking
  its `nth` traffic lane for the new direction.
- Dead end (node with fewer than 2 segments): stay on the same segment and direction; the advance code then
  flags `dead_end` and the cursor stops (traffic never turns around, unlike direction navs).
- Junction, with a non-zero target vector: among the decision segments that allow traffic and whose far node
  has a traffic-allowed plain segment, pick the one whose outgoing direction (plain segment at the far node)
  has the largest dot product with the target vector. If the chosen junction exit is the **rightmost entrance**
  of the departure road (computed by a cross-product comparison among the other approaches), the car uses its
  lane counted from the curb so right turns end up in the curb lane; otherwise it keeps its `nth` lane.
- Junction, with a (near) zero target vector: build candidates of every decision segment whose destination
  allows traffic, is not a one-way entered from the wrong side and has at least one traffic lane in the exit
  direction; the lane is chosen as above (curb-relative for the rightmost entrance, `nth` otherwise);
  a candidate that would force a lane further from the curb than the car's lane (`nth_from_curb > 0` on a
  rightmost entrance) is "last resort". Pick one candidate uniformly at random; if it is a last resort, take
  the next one in the list.

The new lane's offset is read from the profile of the far node.

### 4.4 Lane snapping

`snap_to_selectable_lane(offset, segment, node_ind)` returns the lane offset to use on a segment:

1. Consider the forward-side lanes (those that run in the travel direction) whose zone type is selectable for
   the cursor's lane type; take the one whose offset is closest to `offset`.
2. For cop, racing, drag and starting-grid cursors, or when step 1 found nothing, also consider the
   opposite-side lanes (their offsets negated) and take the closer one overall.
3. For cop and racing cursors keep the lateral position inside the chosen lane: use the input offset clamped to
   `lane_offset ± lane_width / 2`, then clamp it to the drivable extent: from the outermost contiguous drivable
   zone on each side of the chosen lane, minus a safety margin of `vehicle_half_width + 1.5` m on each side.
   Other cursors use the lane centre.

## 5. Race filters, shortcuts, barriers

### 5.1 Race route marking **[decomp]**

When a race path is built the race code marks its segments `InRace` and sets `RaceRouteForward` per segment
by comparing consecutive segments' nodes (a segment is "forward" when it shares its start node with the
predecessor; the successor is forward when it shares a node with the end of the previous). A navigator with the
race filter initialises only on `InRace` segments, travels in the flagged direction, and `IsWrongWay` is true
when it heads against it. Marks are cleared at the start of each race; the filter is only active when a race
route exists.

### 5.2 Shortcuts **[decomp]**

Race setup writes a shortcut number into each road whose centre is nearest a shortcut marker of the race; every
segment of such a road gets `Shortcut`. The **shortcut decision** for a segment with number `s`:
the player's path type allows it only when the player is already on shortcut `s`; a racer decides **once per
number per search** and caches the result: allowed with probability `min + skill × (max − min)` where `skill`
is the AI vehicle's shortcut skill and `min`/`max` are the marker's chances (values above 1 are percentages and
are multiplied by 0.01); all other nav types allow shortcuts. Distance along a shortcut road is multiplied by
the road's scale when path length is summed.

### 5.3 Barriers

At setup the game tests every enabled track barrier against every road segment found by the grid near the
barrier: each segment is sampled along its curve at `max(ceil(length / 10), 4)` pieces and flagged
`CrossesBarrier` (or `CrossesDriveThroughBarrier` for "player barriers") when any piece intersects the barrier
line. Roads containing a race's barrier-exemption marker are skipped. Path types **cop** and **chopper**
ignore full barriers; **racer, player, GPS and race route** also respect drive-through barriers.
Barriers are enabled from scenery-group switches; none is enabled in the file.

## 6. Queries

- **Legal road:** the nav is on a segment where traffic is allowed (for a junction connector, the plain
  segment attached at the node it heads to).
- **Can traffic spawn:** valid, not a decision segment, traffic allowed, not (one-way and heading against),
  at least one traffic lane in the direction; then a random traffic lane is picked and the cursor is moved to
  it ([ai-traffic.md](ai-traffic.md) for the call site).
- **Road speech id:** the `RNrd` speech id of the current segment's road (0 when none), used for the cop
  dispatch street names.
- **Path distance remaining:** the sum, over the path segments from the current one to the goal segment, of
  `(T_end − T_start) × length`, times the road scale for shortcut segments.
- **On path:** the current segment is in the path list and its successor shares the node being approached.
- **Reverse:** flips `node_ind`, `T = 1 − T`, rebuilds; ignored for a racer or player nav that is on its
  race route in the right direction.
- **Pull over:** moves a traffic cursor sideways to the outer edge of its contiguous traffic lanes plus
  the car's half width (inward when the edge is a barrier or the last zone); the call sites are in the traffic spec.

## 7. Constants

| Value | Meaning | Tag |
|---|---|---|
| 32 m | radius for closest-segment search | **[decomp]** |
| 10 | weight of the direction term | **[decomp]** |
| 20,000 | initial best score | **[decomp]** |
| 19 segments / 100 m | junction exit walk-ahead | **[decomp]** |
| 3 m | default cookie gap; 1.1× per sub-step | **[decomp]** |
| 1.5 m | extra safety margin beside half width | **[decomp]** |
| 1.0 m | default vehicle half width | **[decomp]** |
| 510 | maximum segments in a path | **[decomp]** |
| 0.01 m | minimum chord used in divisions | **[decomp]** |

## 8. How to check it

- Walk the 731 junction groups: from each approach node, every exit chosen by a direction nav should leave the
  junction through the plain segment of another group node.
- Spawn a traffic cursor in every traffic lane of every non-decision, traffic-allowed segment and advance it
  300 m: it should never dead-end except at the 5 dead-end nodes and one-way violations should be zero.
- Compare lane line positions against the road mesh/collision under the cursor (tyre ray hit with a road
  surface type) to confirm the right-vector and offset sign convention (not yet done).

## 9. Open questions

- The `USpline` implementation is not in the decomp: the Bézier form is inferred from arc lengths. Tangent
  and curvature formulas (`EvaluateTangent`, `EvaluateCurvatureXZ`) need a disassembly of `speed.exe` or a
  numeric derivative; the cursor only needs them for the forward vector and curvature numbers.
- Segment flag 14 (`LaneMap`, 239 segments) has no reader in the decomp.
- Sign convention of lane offsets has not been confirmed against geometry in the world.
- `WRoad` length and width scales (16.38 and 8.5 per metre) are inferred from ratios.

## 10. Rust implementation notes

- Build a `RoadNetwork` value from the five records and a segment spatial index (reuse the grid's lists or
  bucket the nodes into 64 m cells); no per-section streaming is needed (340 kB).
- Keep the navigator a plain struct with no engine references; give it the car dimensions and a random source
  as parameters so it is testable. The race route marks and shortcut decisions belong to the race code.
- The look-ahead trail and path search are separate modules (see the two specs above).
