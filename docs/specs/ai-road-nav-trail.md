# AI road navigator: the look-ahead trail ("cookies"), obstacle shaping and steering target

The part of the road navigator that gives an AI car something to steer at: a trail of cross-sections of the
drivable corridor ahead ("cookies"), a pass that cuts holes in it for other cars, and a visibility sweep that
turns the trail into an "occluded" target point and a curvature number. The graph, lanes and cursor movement
are in [ai-road-network.md](ai-road-network.md). How the driver consumes the target point and curvature
(steering, speed choice) is in [ai-driver-control.md](ai-driver-control.md). Provenance:
[provenance/ai-road-network.md](../provenance/ai-road-network.md). For the tag meanings, see
[evidence tags](../README.md#evidence-tags).

- **Sources read:** dbalatoni13/nfsmw (CC0-1.0): `World/Common/WRoadNetwork.cpp`, `World/WRoadNetwork.h`,
  `Misc/CookieTrail.h`; call sites in `AI/Common/AIVehicle.cpp`, `AI/Actions/AIActionRace.cpp`,
  `AI/Actions/AIActionTraffic.cpp` (only to see when the functions run).
- **Data inputs:** the navigator state, the car's body (position, velocity, axes, half dimensions), the list
  of other cars to avoid ("avoidables", [ai-driver-control.md](ai-driver-control.md)).
- **Conventions:** all trail maths is planar (x and z of physics space); a 2D vector `(x, z)` is written
  `(x, y)` below as in the engine's `bVector2`. `cross(a, b) = a.x·b.y − a.y·b.x`. Heights are only used to
  ignore cars more than 5 m above or below.

## 1. The cookie **[decomp]**

A cookie is a 64-byte cross-section of the corridor:

| Field | Meaning |
|---|---|
| Centre (3D) | the navigator's position when the cookie was recorded |
| Left, Right (2D) | the left and right corridor bounds at that point (lane line bounds, [§2](#2-corridor-bounds)) |
| Forward (2D, unit) | perpendicular to the Left→Right line: `(−right.y, right.x)` of `Right − Left`, normalised |
| LeftOffset, RightOffset | signed lateral distances of Left and Right from Centre, `cross(bound − centre, Forward)`; left is normally negative |
| Length | distance from the previous cookie (0 for the first) |
| Curvature | the navigator's curve curvature (xz) at that point |
| Flags | bit 0: the bounds were cut by an avoidable; bit 1: the avoidable is at or behind the car |
| Segment number, node index, parameter | where on the road graph the cookie lies (parameter as 16-bit fraction) |

The trail is a ring of **32** cookies (oldest first when read). With the default 3 m gap that is ~96 m of road
ahead. The first cookie is also copied as the "current cookie".

**Recording.** After each sub-step of an advance ([ai-road-network.md §4.1](ai-road-network.md#41-advancing-decomp))
the navigator calls the trail update with the gap `g` (3 m, or less with a look-ahead): if the trail is empty,
or the cursor is at least `g` metres (3D) from the newest cookie, add a cookie; if the displacement from the
newest cookie points almost exactly against its Forward (dot < −0.99, the cursor reversed) the whole trail is
cleared first. When the ring is full the oldest cookie is overwritten and the "current index" moves one back.
`reset` clears the trail and records one cookie at the cursor.

## 2. Corridor bounds **[decomp]**

The left and right points of a cookie come from two more lane lines built with the cursor's lane line
(`SetBoundPos`), displaced by `left_offset` and `right_offset` instead of the lane offset:

- **Traffic cursor:** `offset ∓ half_width / 2`: a corridor as wide as the car's half width centred on its lane.
- **Other cursors:** default `offset ± 2 m`. If the node's profile has zones: find the drivable zone (per the
  cursor's lane type) whose centre is nearest to `offset`; extend left and right over contiguous drivable
  zones; `left = left-most zone centre − its width/2 + margin`, `right = right-most zone centre + its width/2
  − margin` with `margin = half_width + 1.5`. If `right − left` is smaller than `2 × margin`, widen both
  sides by half the shortfall. The cursor's own offset is clamped to `[left + 0.1, right − 0.1]` when a
  trail is present.

So on a racer's trail the corridor is the whole contiguous drivable road width minus a car-sized margin, and
on a traffic trail it is a narrow strip: racers may use the road, traffic stays in lane.

## 3. Finding the car in the trail

`closest cookie ahead(position)`: scan the cookies oldest first; compute for each `dot = Forward · (position −
Centre)`; the answer is the first index where the sign flips between consecutive cookies (the car is between
them) with the smallest squared distance, or index 0 if the car is in front of the oldest cookie (`dot < 0`).
The trail keeps a persistent `current index`, advanced by the occlusion update.

## 4. The occlusion update (steering target) **[decomp]**

Runs after every cursor advance for every AI car with a trail (`update occluded position`, flag
"occlude avoidables" true except for drag racing and a few probes). Inputs: car position and velocity, the
trail, the cursor position. Outputs: current cookie (interpolated), `out_of_bounds`, `occluded` flags,
`apex position`, `occluded position`.

1. **Reset** occlusion, apex and trail speed. Nothing to do without a car body or an empty trail.
2. **Locate the car.** Constants: `look_min = 2`, `look_max = 4` (traffic) or `8` m, `out_scale = 2`,
   `out_bounds = 1.5` (traffic) or `1.0`. Walk from the stored index: for each cookie with `dot ≥ 0` store it
   as current and widen the look-ahead by `out_scale × max(0, out_of_bounds)` where `out_of_bounds = out_bounds
   + max(lateral − RightOffset, LeftOffset − lateral)` (capped at `look_max`); stop at the first cookie with
   `dot < −look_ahead`. `n` is that stop index.
3. **Current cookie.** Blend the current and the next cookie with `blend = 1 − dot_current / (dot_current +
   |dot_next|)`: Left, Right, Forward (renormalised), Centre, offsets, and the segment parameter (choosing the
   segment of whichever cookie the interpolated distance falls on). Without a next cookie extrapolate along
   Forward. `out_of_bounds (final) = half_width + max(lateral − RightOffset, LeftOffset − lateral)` against
   the current cookie, where `lateral = cross(car − centre, Forward)`. The driver uses it to decide to reset
   the cursor when the car is far off its corridor.
4. **Hole punching** (only if `occlude_avoidables`), on a local copy of the cookies from `n` on
   ([§5](#5-hole-punching-around-other-cars)).
5. **Visibility sweep** ("funnel"): from the car's xz position, start with the left and right limit vectors to
   the first cookie's Left and Right bounds. For each later cookie (and finally the cursor position itself as
   a sentinel): tighten the left limit if the cookie's Left bound is further inside (`cross(left_limit,
   car→Left) < 0`), likewise the right limit; test the cookie's Centre against the two limits; if it is
   outside the left limit remember "occluded on the left" with that limit as the occluder, outside the right
   limit "occluded on the right", else it is `last visible`. Stop if the limits cross
   (`cross(left_limit, right_limit) > 0`). This is the classic "string pulling" through the corridor.
6. **Result.** If everything up to the cursor is visible the occluded position is the cursor position itself.
   Otherwise the apex is the occluder bound point (at the apex cookie's height); `road occlusion` = ±1
   (left/right) when the apex cookie was not cut by an avoidable, `avoidable occlusion` = ±1 when it was
   (cookie flag bit 0), and `occluded from behind` when also flag bit 1. The aim point is reflected through
   the apex: `desired = apex + dir(apex→cursor) × dist(car→apex)`, projected on the bisector of the
   directions apex→car and apex→cursor, with the projection length limited to the apex cookie's width.
   For avoidable occlusion the projection is scaled by `2 × ratio`, `ratio = clamp(closing_speed / (2 ×
   own_speed_along), 0, 1)` where `closing_speed` is the car's speed along the car→apex direction minus the
   occluder's speed along the trail ("occluding trail speed"); then limited again to the width. The result is
   the **occluded position**, the point the driver steers at; a car that is not occluded steers at the cursor.

## 5. Hole punching around other cars **[decomp]**

For each avoidable within the AI's list (up to 32; the car's own trailer and, for cops in formation, the
pursuit target and other formation cops are skipped) the corridor of the cookies near it is narrowed so the
trail passes on one side. The routine is marked unfinished in the decomp and the constants below come from it
as is.

For avoidable `a` and own car `m`, with `F` the closest cookie's Forward:

- Ignore it if its height differs from the closest cookie's by more than 5 m.
- `his_extent = a.half_width + 0.5 + |F·a.forward| × a.half_length + |F·a.right| × a.half_width`;
  `my_extent = m.half_width + 0.5 + m.half_length`; `dist_ahead = (a − m) · F`;
  `extent_side` and `dist_side` are the same test along the car's own right axis.
- Racers (and the player) treat a crossing traffic trailer (axis within 45° of crosswise, vehicle class
  trailer) as a block 6 m back with 1.8 m half extents.
- `cut_flags = 2` (the car is "inside" the avoidable's footprint) when: for traffic or own speed < 20 m/s,
  `dist_ahead + his_extent < my_extent`; otherwise `dist_ahead − his_extent ≤ my_extent`. Ignore avoidables
  more than `my_extent + his_extent` behind, and those behind with a clear side gap.
- Time to closest approach of the two **nose points** (own nose `min(my_extent, gap)` ahead, his nose the same
  amount back, `gap = max(dist_ahead − my_extent − his_extent, 0)`) from the relative position and velocity;
  returns 3 s when relative speed squared is below 1e-4. Cars that are not closing are ignored when farther than
  `my_extent + his_extent` (traffic: plus `0.5 × speed + 2 × my_extent`), or when traffic is clearly beside.
  Otherwise, unless blocked traffic, ignore when approach time ≥ 3 s.
- The point of impact is the avoidable's position advanced along `F` by `trailing_speed × approach_time −
  (my_extent + his_extent)`; the closest cookie to it is the cut start. Its lateral velocity component times the
  clamped approach time (0…1) shifts the cut position by `0.8 ×` and widens the avoidable by `0.2 ×`.
- Its half width in the trail is the larger absolute projection of the footprint's two diagonals on the
  cookie's right axis plus the widening; `close_factor = ramp(avoidable_ahead, −6, 6)` (0 when 6 m or more
  behind the car's own cookie distance, 1 when 6 m or more ahead) scales a safety margin
  (`close_factor`, or `0.8 × close_factor` in drag races).
- Decide the passing side: the gaps right and left of the avoidable must be larger than `margin +
  vehicle_half_width`; if only one fits take it, otherwise pass on the side opposite to the avoidable's
  lateral position relative to the car's predicted offset (`own offset + approach_time × close_factor × 0.2 ×
  lateral velocity + 2 × cross(nav forward, cookie forward)`).
- For the cookies from the cut start onwards while the cookie still overlaps (the loop ends when a cookie is
  more than `2 × (my_extent + his_extent)` before the avoidable's tail): move the bound on the pass side to
  `avoidable lateral ± (half_width + vehicle half width + margin)`, keeping at least 1 m (0.1 m for traffic)
  of corridor, and set cookie flag bit 0 (plus the `cut_flags` bit 1 when applicable).
- The closest cut that is more than `my_extent + his_extent` ahead records the avoidable's trail speed
  (`trailing_speed`) for the occlusion step.

After all avoidables, cookies cut this way have their centres moved to the middle of the remaining corridor
(`offsets = ±half remaining width`).

## 6. Curvature for speed choice **[decomp, unsolved]**

`trail curvature(car position, car velocity)` returns the larger of two numbers in 1/m:

- **Road term:** the length-weighted mean of `|curvature|` of consecutive cookies from the current index on,
  with each cookie's curvature clamped to ±0.01. (Zero with fewer than two usable cookies.)
- **Apex term** (only with more than 2 cookies, occluded and not occluded from behind): let `d` be the
  distance from the car to the occluded position (> 1 m) and `a` the apex cookie's width (at least `d`, at
  least 1); let `sina` be the angle between the directions car→occluded position and apex→cursor
  (via the arcsine of the planar cross product; pi minus it when they point opposite ways);
  `apex = sina × sin(min(sina, pi/2)) / a`, clamped to [0, pi] before dividing; for avoidable occlusion
  multiplied by `ratio²`, `ratio = ramp(closing_speed / own_speed_along, 0, 1)`.

The driver uses this to limit corner speed ([ai-driver-control.md](ai-driver-control.md)). The decomp marks
the function "unsolved" (the structure above was reconstructed from a non-matching decompile), so treat the
apex term as approximate.

## 7. Schedule at the call sites **[decomp]**

- A cursor with a trail is advanced so that its distance from the car equals the **look-ahead distance**
  (a speed-indexed table; separate tables for the human/AI-controlled player and drag races): if `dist(cursor,
  car) < look_ahead`, advance by the difference, with the car's forward vector as target direction and the
  look-ahead as `max_lookahead`; if the cursor is farther than a "too far" threshold, or the car is out of
  bounds by more than a threshold and a fresh cursor would be better, the cursor is reset to the car.
- The occlusion update follows each advance; `trail curvature` is read where the driver picks its speed.
- Without a trail (probes, cop road-block finders) the cursor is advanced in big steps (up to 250 m) with
  `max_lookahead = 0` and no cookies are kept.

## 8. How to check it

- Drive a straight road: the occluded position equals the cursor, `out_of_bounds ≈ half width` when centred.
- Place a parked car in a racer's lane: the cut bound should sit one car width plus margins from the car on the
  side with the larger gap; the occluded position should move to that side while the car is within ~3 s.
- Compare the trail length: 32 cookies × 3 m on a long straight; shrinks with `max_lookahead / 26` at low speed.

## 9. Open questions

- Exact form of the spline tangent and curvature used by `Curvature` (see
  [ai-road-network.md §9](ai-road-network.md#9-open-questions)).
- The look-ahead tables (speed → metres) and the "nav too far / out of bounds" thresholds are tuning data
  outside this module ([ai-driver-control.md](ai-driver-control.md)).
- Hole punching is unfinished in the decomp; where this section is vague, the behaviour of the shipped
  binary needs a disassembly of `HolePunchAvoidables`.

## 10. Rust implementation notes

- Implement as a pure function over a `Vec<Cookie>` (ring buffer of 32) plus the car body and an avoidable
  slice; no engine types needed. Keep the 2D maths in the xz plane.
- Cap work: the trail has at most 32 cookies and 32 avoidables.
