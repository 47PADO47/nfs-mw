# Jump stabilizers and persistent ground reference

Sources: original PC 1.3 executable inspected locally with Ghidra on
2026-10-13; [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0,
`Chassis.cpp`, `RigidBody.cpp`, `RBVehicle.cpp`, `WWorldPos.cpp` and their
headers. All behavior below is **[decomp]**. This extends
[vehicle steering/assists/aero](vehicle-steering-assists-aero.md) §7.3.
No decompiled expression is included in an implementation.

## Ground reference lifecycle

The jump stabilizer reads the rigid body's persistent ground normal and signed
plane separation. It does not average wheel contact normals. The normal starts
as `(0, 1, 0)` with separation zero. The owner keeps a world-position cache
containing a selected collision triangle, its validity and surface.

At an awake body's end of frame, refresh its collision candidate list around
the body position using the larger of the primitive radius and three meters.
Then update the owner's world-position cache at the body position, permitting
cached-face reuse and preserving its prior valid face if a fresh search misses.
Sleeping bodies also update their world-position cache, without the body
collider list. Enabling a previously disabled body forces a fresh search.
The next chassis update consumes this saved reference.

For the body-collider triangle-list path:

- If the cached face remains valid under the body's horizontal position and
  reuse is allowed, retain it.
- Otherwise inspect triangles in the supplied candidate list whose horizontal
  projection contains the body query point, lifted by the cache's tolerance.
- Select the smallest vertical separation that is greater than minus one
  meter, subject to the original triangle-list ordering and surface continuation
  flag. Non-continuation surfaces terminate the search after a face is found.
- Orient the face normal upward. The original caps its vertical component at
  0.9999 for a nearly horizontal plane; it does not subsequently normalize it.
- If no new face is valid but a prior valid face exists and preservation is
  enabled, retain that triangle and recompute separation against the body.
- If neither condition supplies a valid face, return failure and set separation
  to minus one, retaining the previous normal components.

For a selected face with any point `p`, current body position `x` and saved
normal `n`, the saved separation is `(p - x) dot n`. The stabilizer's altitude
input is its negation. It is signed perpendicular plane distance, not vertical
ray length and not distance from the wheels. Surface and cached-triangle
selection remain world-query responsibilities.

The generic collision-instance search uses a different candidate acquisition
path, including a vertical segment for transformed geometry. That path and the
configured world-position tolerance need separate verification when recreating
the complete world query. A new center ray or a wheel-contact average must be
labeled an approximation instead of original ground-reference parity.

## Jump state and force

Inputs are simulation delta, body mass, inertia in body axes, body frame,
linear/angular velocity, speed magnitude, wheel contact count, body contact
state and the persistent ground reference. Persistent jump state holds airborne
time and greatest altitude attained since the last wheel contact.

When no wheel touches, advance airborne time and maximum altitude. Add a
world-down force of `mass * 3` newtons. Add landing acceleration when all of
these hold: airborne time above one second, body up aligned with ground normal
above 0.96, maximum altitude above two meters, falling world vertical velocity,
and current altitude below six meters. The additional acceleration is
`(1 - ramp(altitude, 0, 6)) * 10 * ramp(speed, 0, 20)` in meters per second
squared. Touching any wheel clears airborne time and maximum altitude.

## Passive angular damping

Require fewer than two touching wheels, body up's world vertical component
above 0.8, and no rigid-body ground contact. The vehicle ground-contact query
also returns true when any suspension wheel touches, so normal vehicle use
typically requires fully airborne wheels for this branch.

Let `w` be body-space angular velocity and `I` the diagonal body-space inertia.
Interpolate the damping graph with knots `(0,0)`, `(0.4,0.15)`, `(2,5)` against
`length(w)`, then multiply by `ramp(speed,0,5)`. The body-space damping torque is
`-w * I * damping`, except its yaw component is zero. Rotate that torque into
world space and add it to the pending torque.

## Active alignment

Require zero touching wheels, ground normal's world vertical component above
0.9, body up's vertical component above 0.1, up/ground alignment above 0.8 and
positive altitude greater than the engine epsilon. Define altitude weight
`1 - ramp(altitude,0,15)` and speed weight `ramp(speed,0,5)`.

For pitch, normalize the cross product of body right and ground normal to obtain
the ground-aligned forward direction. Compare it with body forward. When their
dot product is below 0.99, use their normalized cross product as torque axis;
the magnitude is `mass * speed_weight * altitude_weight * 40 * (dot - 1)`.
For roll, derive the ground-aligned right direction from ground normal crossed
with body forward; compare with body right and use the same rule with a factor
of 20. Add both world-space torques to the passive result.

These torques consume the supplied saved plane. They can be implemented
independently of the world query, with synthetic plane/vector comparisons,
without claiming the query or complete original jump behavior is reproduced.

## Arithmetic boundaries

Vector rotations and alignment dot products add the Z and Y products before
the X product. Passive damping converts the saved world angular velocity with
that order, then rotates its body-space torque with the same order. Generic
matrix multiplication can change the recovered local angular velocity.

Unit cross products compute the cross vector, then sum squared components as
`(x*x + y*y) + z*z`. A squared length at or below 0.000001 returns zero.
Otherwise multiply by the reciprocal square root of that squared length.
Preserve these single-precision product, sum and reciprocal boundaries.

For each alignment correction, form the scalar in this order:
`((((dot - 1) * mass) * speed_weight) * altitude_weight) * axis_gain`.
Accumulate passive torque first, then pitch, then roll. Combining pitch and
roll before adding passive changes rounding. These are arithmetic contracts,
not additional tuning or changes to the physical force model.

## Validation boundary

The game selects the reference tire integrator and supplied racer jump tuning.
The consumer adds the verified airborne acceleration and passive damping.
The game supplies the [cached body terrain reference](vehicle-ground-reference.md)
to landing acceleration and active pitch/roll alignment. It refreshes before
the next chassis step at the preceding tick's final body position, including
the previous world and vehicle contact corrections. Placement clears the cache;
hardware changes preserve it. No wheel-average plane or new center ray is used.
The current free-roam adapter disables nonzero scenery groups and uses the
original per-driver collision masks: zero for player/racer, 0x80 for cops and
0x40 for traffic. Original dynamic group activation still requires a producer
from scenario/event state and live comparison.

The PC jump routine and world-position update agree with the recorded formulas
and cache lifecycle by static inspection. Required runtime comparisons include
bridge edges and stacked road geometry, fresh-search misses, first placement,
sleep/wake, landing from above six meters, falling while banked, and pitch/roll
correction versus ground slope. The current simple body-corner probes do not
establish original world-position cache parity.
