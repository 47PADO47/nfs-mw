# Persistent target-vehicle road tracking

Sources: the original PC executable inspected privately, and the CC0
[AIVehicle reference](https://github.com/dbalatoni13/nfsmw/blob/main/src/Speed/Indep/Src/AI/Common/AIVehicle.cpp),
specifically `UpdateRoads`, `GetSeekAheadPosition`, `path_spot`, and `road_walker`.
These establish behavioral facts. No decompiled expression is copied.
See [provenance](../provenance/ai-road-target-tracker.md).

## Ownership and public interface

**[verified native, static]** A target vehicle retains current and future road
cursors, a farther road prediction, previous future-segment identity, two road
update clocks, and a separate seek-ahead cache. Chasing cars query this shared
vehicle state. They do not each reconstruct it from a fresh closest-road query.

**[ours]** The renderer-free road library exposes `RoadTracker`, `Default + Clone`:

```text
update(network, index, Option<Body>, dt)
current() -> Option<&RoadNav>
future() -> Option<&RoadNav>
snapshot() -> Option<RoadObservation>
seek_ahead(network, speed, prediction_seconds, cop_filter) -> Option<Vec3>
visit_state(&mut FnMut(u32))
```

`RoadObservation` is a `Copy` value containing `current: Vec3` and `future: Vec3`.
Coordinates are physics-space metres, Y up; speed is metres per second.
The caller supplies prediction time and segment filtering. The first adapter
supports free roam; racing callers retain their previous implementation until
the navigator can enforce complete race membership and direction. The generic library
does not depend on game heat, pursuit goals, or the game crate.

The tracker owns a monotonic simulation clock. `update(None, dt)` advances this
clock and suppresses observation output while preserving road history. Thus an
inactive interval cannot leave a cached prediction artificially fresh. Invalid
or nonpositive `dt` does not advance the simulation. A new tracker starts with
expired update clocks and no valid road cursors. Accessors return no observation
while inactive or without the required valid cursors.

## Incremental tracking

**[verified native, static]** Let `P` be body position, `V` velocity, and `C` and
`F` the retained current and future road points. State is locally valid when
both road cursors are valid and `length(P - C) < 20 m`.

When locally valid and less than 0.02 s has elapsed since the last increment,
keep the current state. Otherwise read current motion and race-route policy.
When locally valid and less than 0.33 s has elapsed since the full rethink,
perform an increment and update the increment clock:

- Normalize current road forward. Its planar right vector is `(forward.z, 0,
  -forward.x)`, normalized separately.
- Project `P - C` onto forward and right. When forward displacement exceeds
  0.05 m, snap the requested lateral offset to a selectable lane and advance
  the retained current cursor by that forward displacement, with its own
  forward direction as the junction hint.
- Compute `deficit = length(V) - distance(F, C)`. This represents a one-second
  future-distance target. When the deficit exceeds 0.05 m, use the direction
  from `F` toward the retained farther prediction. Project the increment onto
  future-road forward; adjust its lane offset using the velocity's lateral
  component times `deficit / speed`, then advance the retained future cursor.

Do not replace the current or future cursor with a weighted closest-road query
on every caller's think. See [road-network section 3.1](ai-road-network.md)
for why that navigation cost is distinct from physical nearest-road distance.

## Full route rethink

**[verified native, static]** Reconsider the route when local validity fails or
the 0.33 s rethink clock expires. Update that clock before searching. Below
1 m/s, use body forward as the motion vector and use its magnitude as speed.
Normalize the resulting direction. Search for a future point at
`max(speed, 2 m)` and a farther target at `max(2 * speed, 90 m)`.

The bounded road walker receives the prior future segment and its travel node.
On success, install its best start and future cursors, retain the future segment
and node for the next rethink, and store the farther position and direction.
On failure, leave existing road state in place. Failed searches do not invent a
new weighted closest-road fallback or clear a previously valid prediction.

### Candidate starts

Gather unique segment candidates from nearby grid cells within 25 m; optionally
restrict them to the active race route. Initialize each candidate at the body
position and motion direction, honor one-way direction, and snap to a selectable
cop lane. For its resulting point and planar unit forward, score:

```text
max(XZ distance from body - 2 m, 0)
+ 3 * max(abs(body height - road height) - 2 m, 0)
+ 2 * (1 - dot(motion direction, planar road forward))
```

Evaluate starts in increasing score order. **[ours]** Equal-score order must be
deterministic and must not consume a world random stream.

### Connected path search

Follow the attached directional plain road where one exists. Otherwise explore
connected junction exits, excluding backward one-way travel, respected barriers,
and roads outside an active race route. Accumulate whether a traversed segment
is unsuitable for cops. Distances along these paths use the stored segment
length and segment parameter; do not substitute navigator chord distance.

Retain the spot reached at the future distance. At the farther target distance,
evaluate an end candidate. Bound the entire rethink to 32 walk calls and 10 end
evaluations. Once a start's score cannot improve the best result, stop considering
its later starts. **[ours]** Also bound malformed cycles and degenerate segments
so a corrupt network cannot cause an unbounded inner traversal.

### End score

Start with the candidate-start score and add 5 if any traversed segment is
unsuitable for cops. Snap the target spot toward the straight predicted target.
Let `ratio = future_distance / target_distance`. Interpolate from the start
point to the snapped target using `ratio`, then interpolate from the straight
future prediction to that result using `ratio`. Snap the stored future spot
toward this blended point. Add:

- 16 times the directional disagreement between motion and the planar unit
  displacement from the start to the snapped farther target.
- 24 times the corresponding disagreement for the snapped future point.
- Twice the disagreement between motion and the target spot's planar forward.
- 2 when the future segment differs from both the previous future segment and
  its attached directional successor.

Each disagreement is `1 - dot(direction_a, direction_b)`. Keep the best start,
future, and target spots. Prune candidates once their partial score cannot beat
the best score. No arbitrary preference for road labels or captured coordinates
belongs in this algorithm.

## Cached seek-ahead point

**[verified native, static]** The getter first ensures road tracking is current.
Refresh its separate cache only after more than 0.33 s. Clone the retained current
cursor, retain the supplied race policy, select cop lanes and directional
navigation, and apply the caller's conditional cop-segment filter. If valid,
set the seek clock and advance by `min(500 m, speed * prediction_seconds + 8 m)`
with a zero direction hint. Save the resulting point. Otherwise retain the
previous cached point. No observation is returned while the tracker is inactive.

The game adapter supplies 7.8 s ordinarily and 3.12 s for its relevant pursuit
flag. Its cop filter is enabled only outside racing and when the current
segment satisfies the cop-consideration predicate. These game decisions remain
outside the generic tracker. Call the shared getter once per update; do not
request different horizons for individual chasing cops within one cache period.

## Reset, determinism, and integration boundaries

**[verified native, static]** The inspected generic driving-state reset does not
clear target-road cursors or clocks. This history belongs to the target vehicle,
not an individual pursuing car's action. Large displacement naturally triggers
the validity failure and route rethink.

**[ours]** The game world owns one primary-target tracker across pursuit restarts.
It updates once before the car loop, using the target body already authorized
by the existing visible/last-seen selection. It can update the same player's
tracker in free roam. Missing observations still age clocks; a hidden live pose
must not leak through a cached snapshot. Secondary-target tracking remains an
explicit separate boundary until those vehicle identities own equivalent state.

`visit_state` includes active/valid discriminants, all simulation
clocks, cached seek point, current/future navigation state, farther prediction,
and previous future identity in stable order using raw scalar bits. The game
also hashes the copied observation supplied to car environments. No pointers,
render state, hash iteration order, or random consumption may affect results.

Existing lane geometry remains the adapter boundary: `RoadNav` stores lane
indices rather than the original continuous offsets. A new tracker must use
the existing lane representation consistently and must not claim exact lane
offset parity. Broader race-route and dynamic-barrier support retains the
limitations of the supplied road network.

## Acceptance

Synthetic coverage must distinguish retained target tracking from a fresh
weighted initializer that chooses a distant collinear endpoint. Check the
increment/rethink clocks, absent-observation cache aging, invalid displacement,
bounded fork search, deterministic route continuity, and serialized state.

Installed acceptance uses an explicit private target trace without embedding
its data. Geometry and the initial accepted-cursor handoff must pass before
scoring the unchanged route. Measure original-unit gap, actual/requested speed,
real sight, search, and retirement throughout. Passing a straight corridor alone
does not establish junction behavior or original-game parity. This specification
does not assert that road tracking is the sole cause of a failed chase.
