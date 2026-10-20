# Ground contact response

Sources read before implementation: PC 1.3 executable hash recorded in
[pc-object-contact.md](pc-object-contact.md), isolated single-moment plane
response at `0x488830`, and CC0 reference revision `80993f35224c61d2f1b135beba76c6f8234c7940`
(`RigidBody.cpp`, `Collision.h`). Original bytes and inspection remain private.

Plane response uses the object moment iteration rule against a stationary,
immobile surface, with 16 iterations. Iteration zero records telemetry; later
iterations apply the fraction `i / 16`. Restitution affects output increments,
not temporary velocities. Report the accumulated normal impulse before
restitution. Reject a separating or stationary initial contact without waking
or changing the body. Temporary inertia uses `GROUND_MOMENT_SCALE`; restitution
is the length of the local normal multiplied by `GROUND_ELASTICITY`.

Ground friction is the body's `GROUND_FRICTION` multiplied by the actual
`simsurface.GROUND_FRICTION`. Cap kinetic friction at static before scaling.
Surface grip used by tyres is independent of body friction.

Visit enabled ground boxes before enabled point clouds, retaining at most 16
positive-penetration contacts in source order. Box corners visit the lower ring
(-x,+z), (+x,+z), (+x,-z), (-x,-z), then the upper ring in the same order.
The primitive's orientation and offset apply to each corner.

For root half dimensions h and body rotation R, define vertical extent
`depth = length((transpose(R) * up) * h)` and ceiling `position.y + depth`.
At world arm r, rotated centre c, angular velocity w and linear velocity v,
query tolerance is horizontal speed times fixed dt, plus
`max(w.x*(r.z-c.z) - w.z*(r.x-c.x) - v.y, 0) * dt + depth`.
Clamp first to at most ceiling minus point height, then to at least 0.25.
Query beneath that ceiling; a negative or zero tolerance supplies no contact.
Penetration must exceed 0.000001. Sort contacts by penetration, attempt the
deepest first and stop after the first successful response. Correct position
by the deepest retained positive penetration along its normal.

`NO_GROUND_COLLISIONS` suppresses queries without suppressing integration.
Ground contact count feeds sleep, so omitted clouds or changed traversal order
can affect resting orientation and cleanup eligibility.

Measured synthetic PC cases include centred impact, sliding, off-centre rotation,
tilted normals/inertia, elasticity, stationary and separating contacts.
The height-query adapter still uses streamed collision rays; cached world-face
selection and full live comparisons require further validation.
