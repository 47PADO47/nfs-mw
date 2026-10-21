# Nearby world barrier list

Sources: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0,
revision `80993f35224c61d2f1b135beba76c6f8234c7940`,
`WCollisionMgr::GetBarrierList`, `GetInstanceListGuts` and
`WCollisionBarrier::DistSq` in `WCollisionMgr.cpp` / `WCollision.h`.
This specifies the original list producer separately from the primitive/barrier
narrow phase; no decompiled expression is copied into the implementation.

Inputs are a world point, a nonnegative finite radius, a surface exclusion mask,
loaded collision packs, and the world's current scenery-group membership.
Candidate instances come from every grid cell touched by the point's xz square
expanded by radius; repeated references are tested once. Without a grid, all
loaded instances are candidates. Supplemental ungridded packs are always tested.
Missing streamed packs are skipped, not replaced by guessed walls.

Grouped instances require their group enabled. Apply the exclusion mask to
instance and barrier flags. As in existing world queries, an enabled group clears
its baked group-exclusion bits for the eligibility check; retain raw output flags.
An instance's horizontal centre distance must be strictly below the squared sum
of the query radius and its broad radius.

Transform the point into instance-local coordinates. For each barrier, project
the local horizontal point offset onto its endpoint difference, multiplied by
the **authored inverse horizontal length squared**. Clamp that fraction to the
closed interval zero to one; measure squared horizontal distance to the selected
point. Do not recompute the inverse length from endpoints. Accept only distance
strictly below radius squared, point.y+radius strictly above the lower endpoint
height, and point.y-radius less than or equal to the upper endpoint height.

Copy accepted barriers and transform both endpoints into world coordinates,
preserving authored inverse length, raw flags and surface index. Return the
article's resolved surface hash, section and instance identity, and the local
squared distance used for ordering. Compute the world horizontal normal from
the transformed endpoint difference and retained authored inverse length; do
not normalize or flip it. Sort by ascending local squared distance, preserving
candidate discovery order on equal distance. This matches insertion after equal
elements in the original list.

The library query does not choose body radius constants or contact response.
The caller supplies its original body/primitive search radius. The query
collects actual barrier segments, not a handful of centre rays. It does not
collect terrain triangles, static world OBB objects, or dynamic prop bodies.
Invalid input clears the reusable output list and returns no candidates.
