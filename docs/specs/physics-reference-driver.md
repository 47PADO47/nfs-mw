# Routine-level physics comparison interface

This interface extends the observed response contract in
[pc-object-contact.md](pc-object-contact.md) and geometry contract in
[box-object-contacts.md](box-object-contacts.md). Those documents identify the
original executable and reference sources. No executable bytes, addresses,
captured game objects, or installed tuning tables belong in the public driver.

The portable `blackbox-vehicle` example accepts whitespace-delimited numeric
cases on stdin and returns one result per line. `moment` cases provide each
body's mass, principal inertia, orientation columns, inertia scales, local CG,
position, linear and angular velocity, elasticity, immobility, breakaway impulse
and fixed-centre flag. A contact provides point, normal, static/kinetic friction
and iteration count. This admits real captured moments without substituting
synthetic box inertia or yaw-only orientation. Conversion applies the existing
specification: inverse scaled principal inertia rotates to world space and CG
transforms by the same orientation. The solver and notification method are the
production implementation, not an independent test translation.

`box` cases provide both boxes' centre, orientation columns, half extents and
travel. Results include hit classification, point, normal, overlap and point
ownership. No epsilon or tuning can be supplied to the driver to hide failures.

The separate `nfsmw-data` `physics_profiles` example exports each readable
installed smackable profile's contact parameters through existing typed readers.
It reads the user-selected install through `game-install`; output is private
user data, never checked in. These parameters generate controlled response
cases, not whole-scene parity. Shape sizes in generated response cases are
explicit synthetic inputs, not purported installed collision geometry.
The optional `--vehicles` export reports vehicle/source-body keys, physical mass
and the linked body's object-contact immobility flag for construction audits.

The external comparison harness owns original machine-code execution, exact
executable identity, captured object decoding, case generation and tolerance
reporting. It must fail on missing/zero cases and categorical mismatches, report
numeric tolerances and maxima, and retain the first divergent case and field.
Fresh original executions are compared with the current compiled driver;
previous reference JSON alone must not be presented as a new original run.

Output includes callback intensity independently of physical velocities. An
immobile body's callback can be nonzero. Releasing attachment changes the mass
selection after the solve, and equality at the strict breakaway threshold must
be tested separately from either side of it.

This interface establishes component comparisons only. It does not execute
world queries, full dispatch callback order, event sequencers, traffic
suspension, camera scheduling, acoustic mixing or a complete driving step.
