# Player immovable-plane response

This supplements [rigid-body contact](vehicle-rigid-body.md) and
[object contact](pc-object-contact.md). See [provenance](../provenance/player-plane-response.md).

The PC immovable-plane routine uses 16 moment iterations. Normalize the supplied
normal by summing squared X/Y components, then Z, taking the square root and
multiplying each component by its reciprocal. Component-wise division can
produce different low bits. Retain finite-input validation and reject a zero normal.

Project both iterative contact momentum and initial closing velocity by adding
the Z and Y products first, then X. Keep the separate object's X/Z-then-Y
projection and existing normalization path. Near-tangent cancellation can otherwise
change whether a plane contact is accepted.

Iteration zero records telemetry without applying an increment. Later iterations
scale full normal and friction vectors by `i / 16`. Restitution affects output
increments, not temporary velocities. Friction and material coefficients are not
retuned. Preserve the separate static and kinetic friction roles.

Synthetic instruction fixtures check contact acceptance and final velocities.
World candidate selection, support traversal, live FPU state and complete
vehicle-task timing require separate evidence. Do not treat the plane adapter
as proof of the full collision pipeline.
