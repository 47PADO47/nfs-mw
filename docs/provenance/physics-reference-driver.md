# Physics reference driver

Implemented 2026-10-20 from
[the comparison interface specification](../specs/physics-reference-driver.md),
[PC contact response](../specs/pc-object-contact.md) and
[box contacts](../specs/box-object-contacts.md).

The portable examples are independently written numeric adapters over existing
production functions and typed data readers. No external harness code was
copied. Original executable execution, addresses, installed profile exports and
captured objects remain outside this source repository. The public source has
no emulation dependency.

The notification calculation was extracted unchanged from `resolve_bodies`
into `ContactMoment::notification_delta_v`, so the comparison adapter exercises
the same production method. Existing contact notification regressions continue
to cover physical versus callback response and both body orders.

Private validation executes original machine code afresh and compares output
classifications exactly, with explicit numeric tolerances for x87 versus Rust.
Generated response cases span all readable installed smackable records, approach
speeds, contact arms, breakaway thresholds, immobility and fixed centres.
Separate generated geometry cases include arbitrary 3D orientations, movement
and flat boxes. Private capture replay supports the full moment orientation and
principal inertia rather than reconstructing a body from an assumed shape.

These comparisons do not demonstrate complete world-dispatch or gameplay parity.
They isolate response math, geometry and scalar notification mass selection.
Live recordings, when supplied, are labeled separately from generated cases.
