# Cached body terrain reference

Sources: original PC 1.3 executable, locally inspected on 2026-10-13; and
[dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0,
`WCollider.cpp`, `WCollisionMgr.cpp`, `WWorldPos.cpp`, `WGrid.cpp`,
`WCollision.h`, `WCollisionSTL.h`. Behavioral reference only, no code copied.
This specifies the body-collider triangle-list path. **[decomp]**

## Candidate order and predicates

Enumerate grid cells overlapping the region's horizontal square, columns before
rows, then their instance references in stored order. Deduplicate while retaining
the first occurrence. Point-to-cell coordinates truncate toward zero before
clamping. The original collapses an axis's range to its first cell when its span
exceeds twenty cells. Missing resident instances contribute no geometry.

An instance needs a resident article, an enabled scenery group, no matching
instance exclusion bit, and strict horizontal-circle overlap using the sum of
region and instance radii. Upright instances use their stored horizontal radius;
tilted/dynamic instances use the maximum stored radius and half extents.

The original vehicle behavior setup supplies its mask by driver class: human,
racer and remote drivers use zero; cops use `0x80`; traffic, inactive and NIS
drivers use `0x40` (also the fallback for other values). This is separate from
the scenery-group enabled state. The collider's mask filters instances while
its awake triangle-list builder does not add surface-flag exclusions.

For each article strip in stored order, transform the region center into the
instance's local coordinates and require strict horizontal overlap with its
bounding sphere, using the sum of strip and region radii. Body-cache face
construction in the inspected PC path adds `strip.center - instance.translation`
to packed vertices. It does not rotate those face vertices. This observed body
cache path is distinct from general instance ray transformation; dynamic runtime
article transformation remains a separate comparison requirement.

For each face in strip order, require horizontal bounding-box overlap with the
region square. If the center lies outside a face edge, its horizontal distance
to that edge's infinite line must be strictly less than the region radius.
The projection is not clamped to the segment's endpoints. These are horizontal
predicates: no body-height or vertical-distance cutoff is applied at this stage.
The resulting list preserves acquisition order. Unlike general ray queries,
this triangle-list builder does not apply additional strip or triangle flag
exclusion. Only the supplied instance exclusion and scenery-group state apply.

## Predictive region cache

Keep the prepared region center/radius and previous requested position. Reuse
candidates when the requested sphere lies strictly inside that region:
distance squared is below `(prepared_radius - requested_radius)^2`.

For the first preparation, or when last-frame displacement exceeds the movement
threshold, set center to the request and radius to request radius times the
initial expansion. Otherwise normalize the displacement from the previous
requested position, extend the center in that direction by last-frame travel
times the prediction multiplier, and set radius to request radius plus the
extension length plus padding, capped at the maximum prepared radius.
Update the previous request every predictive refresh, including reuse.

Original body settings: minimum request radius three meters; initial expansion
1.1; movement threshold five meters; prediction multiplier 2.5; padding 0.1;
maximum prepared radius 25 meters. The owning physics-object terrain tolerance
is 0.025 meters, confirmed in its PC constructor.
The body's primitive-list radius is the maximum primitive half-dimension length
plus its body-relative center length. Disabled primitives still contribute to
this construction-time radius; the authored bounds root radius is not used.

## Face selection and retained plane

Reuse a valid cached face when its horizontal projection contains the body point
and reuse is requested. Otherwise search the prepared face list at the body's
position lifted by tolerance. Select only containing faces whose vertical
separation is strictly above minus one meter and smaller than the current best.
Continue after an accepted face only when its surface flags contain bit four;
otherwise stop immediately. Thus order matters on ordinary stacked faces.

The horizontal containment predicate is winding sensitive at boundaries:
when the first oriented edge value is positive, the other two must be
nonnegative; otherwise both must be nonpositive.
There is no added edge epsilon. Normalize the face normal, orient it upward,
and cap its vertical component at 0.9999 without renormalizing. A zero cross
product supplies world-up before the cap.

Evaluate the retained plane's signed separation by subtracting the query from
the selected face's first vertex, multiplying each coordinate by its normal,
then adding Z/Y before X. Preserve this order in both prepared-list and global
query paths. The generic vector dot product and normal normalization retain
their existing arithmetic. Exact original-instruction fixtures and a live
candidate/cache replay verify this scalar order; no terrain tolerance changes.

Cache whether the prior face was valid. If a search misses, preserve that face
when requested and previously valid, but clear the active surface reference.
Otherwise mark the face invalid and write separation minus one while retaining
the old normal. For a valid face, separation is `(face_point - body_position) dot
saved_normal`. A reused face keeps its normal; a search with the prior body point
still inside the previous face also keeps the prior normal, matching the inspected
update's recalculation return flag even when a search was explicitly forced.

## Fresh point height over a prepared region

A caller may prepare the same ordered triangle region around one center while
querying height at another point. Region prediction and streaming invalidation
are retained; selected-face history starts invalid for each such query.
Apply the existing horizontal containment, vertical tolerance, minimum
separation and continuation-mask rules. Return the selected face's plane
height at the query's horizontal coordinates, or no height on a miss.
There is no finite downward-ray endpoint and no vertical-distance cutoff in
candidate collection. All region sizing and face-selection constants remain
caller supplied. The query does not mutate the body's selected-face history.

The native driving-camera floor consumer uses this form: collider collection
is centered between geometry anchor and eye, then a fresh world-position
query samples the eye's horizontal location. Its request radius is half the
anchor-eye distance plus half a meter, capped at 24.9 m. The shared predictive
region policy still applies. Camera tolerance is 0.025 m and minimum accepted
separation is -1 m, permitting faces up to 1.025 m above the unlifted point.

## Global point and sleeping-body path

The no-collider path queries only the point's grid cell, with instance radius
tests at zero query radius and collision exclusion mask zero. It reuses a valid
containing saved face when requested; otherwise it lifts the point by tolerance
and searches resident enabled instances. Upright instances transform the point
to local space and require inclusive horizontal box bounds and strict strip
circle containment. A strip supplies its first eligible containing face. Normal
strip winding alternates each triangle; unknown-winding strips accept either
winding but require minimum face height below query height plus one. Upright
queries exclude face bit eight. The chosen face must have positive vertical
gap and minimum height minus 0.5 below the query. Choose the smallest gap across
strips and instances and transform the selected face fully into world space.

Tilted or animated instances use a segment frame from the lifted point plus
tolerance to the lifted point minus 750 meters. Require strict 3D strip-sphere
overlap with that finite segment. Each strip selects its closest containing
transformed face with gap greater than minus one; the instance then requires
that selected gap be positive. This branch applies the supplied surface mask
and does not additionally exclude bit eight. Subtract the extra tolerance from
the distance used to compare instances. Unlike the awake prepared-list path,
this global path rotates vertices through the full current instance transform.

Fresh global searches always recalculate the saved normal, including a retained
old face after a miss. Otherwise validity, null surface and signed separation
follow the shared cache rules. Sleeping rigid bodies use this no-collider path.

## Adapter integration

Each original wheel has its own selected-face and saved-normal history over the
same body collider's prepared triangle list. Its world point is body position
plus rotated local arm; point velocity includes angular velocity crossed with
the arm relative to center of gravity. Query tolerance is
`min(max(-velocity.y * dt, 0) + wheel_radius + length_xz(velocity) * dt,
vehicle_height / 2)`. Reuse is allowed only when the previous compression is
positive. Misses preserve the prior valid face and clear its surface, just as
the body path; a fresh invalid wheel still consumes the saved normal and
separation minus one. Wheels do not apply an extra face-bit-eight exclusion on
this triangle-list path. The library's ordinary ray probe remains the generic
fallback when no prepared wheel-plane provider is supplied.

The collision library exposes this as a persistent cache with caller-provided
settings and scenery-group predicate. It does not substitute a ray or averaged
wheel contacts. A world revision triggers candidate reconstruction after pack
changes; the copied valid face may still be retained under the specified policy.
Without a grid, loaded instances are visited deterministically by section/index;
that fallback has no original-grid ordering claim.

The game adapter supplies the original settings and refreshes at the start of
the next vehicle tick from the prior tick's final body position, including its
body/world and car-pair contacts. That chassis tick consumes the saved plane.
Sleeping bodies use the separately implemented global instance query above.
Runtime comparisons still need original world streaming,
animated article transforms and scenery-group transitions.

The current adapter supplies the original mask for player/racer, cop or traffic
roles and admits only group-zero geometry. It has no runtime group activation
state. Original initial/unload state enables only group zero; the group routines
can enable or disable named groups for scenarios. Those scenario/event-driven
transitions still require a game-side producer and live comparison.
The query consumes the prior end-of-tick body position at the start of the next
vehicle tick. Placement clears the cache; hardware changes retain it.
