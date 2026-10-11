# Articulated trailer constraints

Sources: isolated PC 1.3 articulation routines and installed vehicle bounds.
The original named female/male `5THWHEEL` positions are body-relative anchors.
Matching female constraint and male post children supply the angular rules;
neither visual pivots nor vehicle-name constants determine hitch geometry.

Each resolution computes the common fulcrum from the world anchors, weighted by
stored body mass (an immobile body's anchor wins). For a mobile body, rotate its
anchor arm toward the fulcrum-to-origin direction, then translate its origin so
the rotated anchor coincides exactly with the fulcrum. Keep existing velocities
during this geometric correction. Zero-length arms require translation only.

The velocity reaction uses 32 moment iterations. At iteration zero it measures
relative point velocity without applying an impulse. At iteration `i > 0`, oppose
the full relative momentum, scaled by `i / 32`, using reduced mass and each body's
point inertia. The point tensor adds the diagonal parallel-axis contribution
`mass * (arm.y² + arm.z², arm.x² + arm.z², arm.x² + arm.y²)` in the body frame
to the stored principal tensor, then transforms its inverse to world space.
There is no contact approach test, friction or restitution. An
immobile partner uses the mobile body's mass and changes only that body.
This is not a timestep-scaled closing-speed spring.

Angular constraints run after the velocity reaction. A female child establishes
a local constraint frame and symmetric angular allowance
`atan2(half.x, 2 * half.y)`; a matching male post supplies a vector of length
`2 * half.y`. The bounds frame conversion rotates local axes by -90 degrees about
X. Conical and prismatic flags select the original frame adaptation modes.
An exceeded limit rotates both bodies about the fulcrum, distributing correction
by mass, and resolves motion against the limiting plane. Immobile bodies do not
rotate. The caller must run articulation after world and car contacts, before
publishing current poses, so collision separation cannot leave a one-tick gap.

All resolution, braking and hitch release decisions run on the shared 60 Hz
clock. Frame-rate tests must compare complete position, orientation and velocity
traces during turning, braking and a collision. Installed checks must cover every
named constraint and tractor/trailer variant. Live original-game comparisons,
spawn/suspension equivalence and singular rotations remain separate checks.

The rewrite uses native `atan2`; the original's lookup approximation differs by
about 0.00011 radians in measured cases. Exact antiparallel arms use a finite
rotation fallback. The immobile angular-limit velocity projection is not yet
validated against the PC special branch. The unused cement trailer stores a
rotated body/kingpin frame and does not settle with the current vehicle spawn
adapter; resolving its named constraints does not establish driving parity.
