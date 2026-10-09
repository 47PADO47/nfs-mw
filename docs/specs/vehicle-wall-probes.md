# Vehicle wall probe eligibility

This specifies the rewrite's outline-probe wall query, not a replacement for the original game's
box/sphere contact algorithm. The original barrier sidedness, face normals and free-roam group
exclusion are specified in [collision.md](../formats/collision.md#query-semantics); body primitives
and ground versus wall response are in [vehicle-rigid-body.md](vehicle-rigid-body.md#4-world-collision).
No additional decompiled source was read for this correction.

## Inputs and selection

The body has eight outline probes at its local mid-height: four corners and four side midpoints.
For each segment from the body centre to a transformed probe:

1. Apply the existing free-roam `GROUP_EXCLUSION` mask to instances and surfaces.
2. Reject a face whose normal has `abs(y) > 0.7`: it belongs to the ground path, irrespective of which
   side the segment starts on. Segment queries flip normals towards their origin, so checking only
   positive `y` incorrectly treats a road seen from below as a wall.
3. Reject a barrier approached from behind unless its two-sided flag is set.
4. Select the nearest **eligible** contact, applying these checks to every candidate before choosing
   a winner. A rejected floor or rear-facing barrier must not hide a farther wall in the same article,
   another instance or another pack.
5. Keep the existing penetration bounds, response parameters and prop contacts. Prop overlap normals
   are not filtered by the floor threshold.

The collision library exposes a generic candidate predicate. It receives the world-space normal,
sidedness, flags, surface and section/instance identity. Its default ray cast still accepts both sides
of faces and barriers, including shallow faces; camera and ground ray semantics are unchanged.
The predicate runs during nearest-hit selection without collecting every hit into another list.

## Validation and limits

Synthetic fixtures cover candidates in the same and different instances, two-sided barriers, shallow
faces hiding walls, flipped floor normals, steep surfaces and preservation of unfiltered queries.
Ignored tests use the installed track's real grid and collision packs, selecting road height before
checking narrow pairs of barriers. No assets or raw dumps belong in the repository.

The report of invisible collisions near leaves, curbs and sidewalks has no reproducible location yet.
This correction fixes identified query defects; it does not establish that those defects explain that
report. Scenery is not disabled by name or visual category. The original uses swept boxes/spheres,
whereas this rewrite still uses outline segments and a root body box for props; that broader parity
gap remains explicit.
