# Sphere object contacts

Evidence: the PC 1.3 geometry setup, sphere/box and sphere/sphere routines were
executed in isolated x86 memory with synthetic geometry. The executable hash is
recorded in [provenance](../provenance/sphere-contacts.md). These rules precede
their implementation; no original code or binary bytes are included.

## Inputs and conventions

Geometry records contain current centre, half dimensions, rotation and travel
since the primitive's previous prepared position. Travel includes rotation of
an off-centre primitive, not just the body's linear velocity times the tick.
Sphere geometry uses its radius on all three axes and ignores rotation.
The returned normal points from the second object to the first. Depth is positive
for penetration. Exact external tangency produces no contact.

## Sphere against sphere

For separated centres, reject distance greater than or equal to the sum of radii.
Otherwise normal is the normalized centre difference, depth is radius sum minus
distance, and the point is on the first sphere's surface toward the second.
This routine does not use the travel vectors. Coincident centres choose negative
Y and a depth equal to the first radius. The PC writes its contact x coordinate
three times and leaves y/z untouched in this case; tests zero-initialize geometry.
The rewrite uses the common centre as a deterministic contact point instead of
reproducing residual memory. This exceptional case is an explicit parity limit.

## Sphere against box

Subtract the box travel from sphere travel. Express current and previous relative
centres in box axes. For each axis choose a sign from the previous position
(positive only if strictly above zero), then project both positions along it.
Reject any current signed coordinate at or beyond half extent plus radius.
The signed penetration is half extent plus radius minus current coordinate.

If all current signed coordinates are inside the half extents, or none of the
previous coordinates was outside, select the face with the smallest previous
penetration. Only faces with positive area participate; ties keep the first axis.
Normal is that signed face axis. The point is the sphere centre translated by
the current penetration along the normal (the expanded box face).

Otherwise project the sphere centre onto the box features whose previous
coordinates were outside. This gives an offset and a candidate point on the box.
Project that candidate onto the relative travel segment, from previous to current
sphere position. A segment squared length at most 0.000001, or a projection
outside its open interval, uses the endpoint case:

- For a box with positive volume, normalize the offset toward the candidate.
  Reject distance at most 0.000001, or distance greater than radius plus the
  travel projected along that direction. Require positive radius-minus-distance.
  The contact normal is opposite the offset; the point remains on the box.
- For a zero-volume box, reject squared offset longer than radius squared.
  Select the first outside face with positive area. Depth is the positive exit
  distance of a ray from the candidate through the sphere along negative normal.
  Reject exit distance at most 0.000001.

For a projection within the travel segment, compute squared perpendicular distance
to the candidate. Reject radius squared minus that distance below 0.000001.
For a positive-volume box, find the earlier sphere/point intersection along the
segment: the normal runs from candidate to that intersection. Depth is radius
minus perpendicular distance. For a zero-volume box, use the first outside
positive-area face and its current signed penetration. Require positive depth.

This establishes sphere contacts only. Box/box manifolds, full contact dispatch,
separation-point adjustment and response ordering remain distinct parity work.
