# Reset object collision grace

Specification: [reset-object-collision.md](../specs/reset-object-collision.md).

Read-only Ghidra identity confirmed original executable SHA-256
`80774c2e5d619b4f120b48d4462896fd504c263399d203a238769cffde1d253c` before querying.
PC `RBVehicle::DoPenetration` at `0x6B7CE0` rejects a nonzero reset state and records the simulation
time. `ResetCar::ResetTo` at `0x6A45F0` assigns duration `2.0` after successful placement;
`ResetVehicle` at `0x6B08C0` checks the active state for ordinary manual-reset admission.
PC `CanCollideWithGround` at `0x69AF50` and `CanCollideWithWorld` at `0x69B030` do not consult it.

The saved project lacks a function definition at PC `OnTaskSimulate`, `0x68BC80`. A bounded read of
its 221 bytes confirms the conditional branch around the timer subtraction: the subtraction executes
only after the simulation-time comparison against last penetration plus one second succeeds. This
agrees with the CC0 reference and corrects the older specification's unconditional countdown wording.
The CC0 `RigidBody::ResolveObjectCollision` calls both penetration hooks before constructing moments.

An isolated execution of the original PC timer and penetration routines with synthetic vehicle state
confirms the conditional countdown: at time `1.0`, a `1.0` tick leaves `2.0` remaining; at `1.25`, a
`0.25` tick leaves `1.75`; a refused contact then records `1.25`. At time `2.25`, a `1.0` tick still
leaves `1.75`; at `2.5`, a `0.25` tick leaves `1.5`; a final `1.5` tick at time `4.0` clears the state.
The unit regression uses those independent scalar results. No game process was launched or attached.

Offline mapped-byte comparison against executable SHA-256
`bde12bdd158b7f861078ad4527f5a656f34c6068e4a2120f28044f92f0fa158c` confirms all six described
ranges match: timer 221 bytes, penetration gate 36, reset placement 253, manual reset 249, ground
admission 124 and world admission 204. This does not establish equivalence of injected live hooks.

Reference read for understanding: dbalatoni13/nfsmw (CC0),
`Physics/Behaviors/{RBVehicle.cpp,RBVehicle.h,RigidBody.cpp,ResetCar.cpp}`. No decompiled implementation
or original executable bytes are included. Implementation follows the preceding behavioral spec.
The modified game installation's live hooks and complete original reset trajectories were not tested.
