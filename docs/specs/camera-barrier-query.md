# Original camera barrier segment query

The camera query is a separate contract from vehicle collision rays. It tests
barriers only, with exclusion mask zero. Scenery-group membership remains an
independent gate: group zero is present; a nonzero group must be enabled by
the world owner. Free-roam currently supplies all such groups disabled.

Use the shared segment preparation and global segment-cell traversal. The
instance broad phase uses its stored horizontal radius, including tilted
instances; it does not replace that radius with the largest box extent.
Upright instances also require strict vertical overlap between the segment
height range and the instance center plus/minus half height.

Transform the segment to instance-local space. For ray endpoints a,b and
barrier endpoints p,q, define the horizontal differences:

    dx1 = b.x-a.x; dz1 = b.z-a.z
    dx2 = q.x-p.x; dz2 = q.z-p.z
    ex = a.x-p.x; ez = a.z-p.z
    denominator = dz2*dx1-dx2*dz1
    ray_numerator = ez*dx2-ex*dz2
    wall_numerator = ez*dx1-ex*dz1

Reject a zero denominator. Before dividing, each numerator n must satisfy:

    (n >= -0.0001 && n <= denominator)
    OR (n <= 0.0001 && n >= denominator)

The tolerance is on the raw determinant numerator, not a normalized segment
fraction. Evaluate t=ray_numerator/denominator without clamping. Interpolate
the local hit point at t; its height must lie strictly between the barrier's
endpoint heights. Select the nearest accepted local squared distance from a.
Preserve extended native intermediates where they affect the numerator test
and interpolated point; stored geometry and output components remain floats.

Transform the winning barrier endpoints and hit point into world space.
The normal is rebuilt horizontally from the transformed endpoints, scaled
by the barrier's stored inverse horizontal length:

    normal = ((q_world.z-p_world.z)*inverse_length, 0,
              -(q_world.x-p_world.x)*inverse_length)

Flip this normal when it faces away from the segment's start. Do not normalize
it or transform a separately normalized local normal. Its length may differ
from one on tilted geometry, which is observable in camera wall weighting.

The returned hit retains section, instance, surface and facing metadata.
The dedicated camera method leaves existing vehicle ray behavior unchanged.
Validation covers actual native barrier-producer execution for height limits,
directions, sloping rays, tilted transforms and endpoint tolerance, plus native
candidate-gate execution for inactive and active scenery groups. These finite
fixtures do not prove exhaustive streamed-world or group-lifecycle equivalence.
