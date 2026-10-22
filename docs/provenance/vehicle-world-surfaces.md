# Vehicle wall surface friction provenance

Spec: [vehicle-world-surfaces.md](../specs/vehicle-world-surfaces.md).

The CC0 dbalatoni13/nfsmw RigidBody::ResolveWorldCollision reference and the existing
vehicle-rigid-body specification both retain a world material multiplier on the two
wall friction coefficients. The data reader already exposes the installed WORLD_FRICTION
values; vehicle wall response omitted consuming them.

Original PC routine at0x699410: mapped executable instructions load the world surface
coefficient and multiply each already selected plane coefficient before reaction.
Targeted D/E mapped function bytes agree despite their different full executable hashes.
The saved Ghidra database has no function at this address; bounded file disassembly,
not a claim of successful decompilation or live execution, supplies this confirmation.
Original bytes and complete instruction records stay in private research. No original
code, installed assets or data dump is distributed.

This is a material-admission correction. It does not prove that a given rail uses a
larger multiplier, that every wreck should stop sooner, or that the simplified chassis,
world primitive selection and full OG trajectory are complete.

A second omission is the unconditional player head-on adjustment in the shared wall
adapter. PC RBVehicle constructor0x6B7AD0 selects its main behavior vtable0x8AC938;
world reaction's virtual slot0x34 targets0x68BD60. That routine explicitly branches on
owner IsPlayer before modifying inertia and elasticity, while both branches adjust COG.
Mapped0x18E bytes match D/E. This resolves the ambiguous symbol label using actual
constructor/call-slot linkage; the CC0 RBVehicle surface overload independently agrees.
Original isolated-role execution uses synthetic owner/interface responses and moment
setter stubs, so it demonstrates the role gate and resulting parameters, not a live wall
collision or complete moment solver.
