# Loose props against world barriers

After object contacts, apply drag, then world collisions, then ground support and
sleep. An awake detached prop that has moved can collide with the world unless
its linked rigid-body record sets `NO_WORLD_COLLISIONS`. Preserve the separate
`NO_OBJ_COLLISIONS` gate for object pairs.

Find resident eligible barriers near the body's current position, using the
larger of three metres and the primitive collection radius. Keep original
instance/group exclusion and order barriers by local point-to-segment distance.
Each enabled `PRIM_VS_WORLD` primitive is tested in authored order against each
barrier, represented by its full-height, zero-thickness vertical box. Use the
original displacement-aware box/sphere intersection, not outline ray probes.
The primitive travel is current minus previous primitive centre.

A barrier accepts a contact from its front unless marked two-sided. A primitive
with runtime `ONESIDED` still requires the front side even against a two-sided
barrier. Original sphere construction always sets this runtime flag; it is not
the serialized bounds mesh flag at the same numeric bit.
Translate the body out by the contact overlap before responding. Translate the
contact point too only when it belongs to the moving primitive. Subsequent
barriers for that primitive see its corrected geometry.

World response uses the linked `WALL_FRICTION`, `WALL_ELASTICITY` and
`WORLD_MOMENT_SCALE`. Multiply both friction coefficients by the contacted
`simsurface.WORLD_FRICTION`; kinetic friction cannot exceed static friction.
Restitution is the length of the body-local normal multiplied per axis by wall
elasticity. Use the existing sixteen-iteration plane reaction. Positional
correction still applies when separating and no impulse is produced.

This closes the missing barrier path for loose props, including parked cars.
Full instance triangle collisions, free-standing world OBBs, world-contact
effects and collision coverage outside resident packs remain separate gaps.
The installed PC world contains no free-standing world OBB records; this does
not establish parity for custom packs that supply them.

The original box intersection can also reject an axis-aligned zero-depth barrier
when its incident point lies outside the small body's vertical span, despite
apparent volume overlap. Controlled original-machine-code checks reproduce this
rejection. Preserve it in the OG path; do not disguise an enhanced intersection
fallback as an original behavior correction.
