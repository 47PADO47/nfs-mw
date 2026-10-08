# Vehicle rigid body

Modules: [`libs/blackbox-vehicle`](../../libs/blackbox-vehicle): `rigid_body` (integration, inertia, drag,
sleep, a plane-contact impulse) and the ground-contact glue in `vehicle`. Walls, props and car-versus-car are
left to the caller, who uses `RigidBody::react_plane` with the collision library.

- **Spec:** [docs/specs/vehicle-rigid-body.md](../specs/vehicle-rigid-body.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled), `src/Speed/Indep/Src/`:
  `Physics/Behaviors/RigidBody.{h,cpp}`, `RBVehicle.{h,cpp}`, `ResetCar.cpp`; `Physics/Dynamics/Collision.h`,
  `Inertia.h`; `Physics/Bounds.h`, `Physics/VehicleBehaviors.h`; `Sim/Common/Simulation.cpp`, `Util.cpp`;
  `Misc/Main.cpp`, `Misc/MWAttribUserTypes.h`; `World/WCollision.h`, `WCollider.h`,
  `World/Common/WCollisionMgr.cpp`, `WCollisionPack.cpp`, `WCollisionAssets.cpp`, `WWorldPos.cpp`;
  `Libs/Support/Miscellaneous/CARP.h`; `Generated/AttribSys/Classes/{rigidbodyspecs,collisionreactions,pvehicle}.h`.
  Read for understanding only; no code, names of locals or tweak variables copied. The attribute field
  names and struct layouts quoted in the spec are facts (data formats), not expression.
- **Gaps in the sources:** the impulse solver (`Moment::React`), the box/sphere overlap test
  (`Geometry::FindIntersection`), `Bounds::Collection::AddTo`, the construction of a car's rigid-body
  parameters and `SetVehicleOnGround` are empty or stripped in the decompilation. The spec marks those
  parts **[unconfirmed]** and gives a textbook rule; the implementation must be tuned against measurements
  of the running game, not against more decompiled code.
- **Implemented:** 2026-10-08, from the spec only (decompiled code not open). Fixed-step semi-implicit
  integration with forces applied one step late, rotation about the centre of gravity, box inertia and
  `SetMass`, quadratic linear and angular drag, the speed clamps, sleep and waking, the ground contact of the
  box corners with a textbook impulse and Coulomb friction. Not implemented: barriers and world objects,
  car-versus-car and `ModifyCollision`, the collision reaction records, invulnerability, reset-to-road.
- **Checked against the game by:** pending. The spec's "How to check it" lists the measurements (settling
  and sleep, wall rebound and head-on yaw damping, mass-ratio push-out in car-vs-car, reset timing).
- **Known differences from the original:**
  - The impulse solver is the textbook rule (the original is not in the sources): one normal impulse with
    restitution, then Coulomb friction that sticks or slides; no 16 sub-steps.
  - Ground contact casts the eight corners of the body box from 0.3 m above and pushes out along the deepest
    normal; no speed-dependent tolerance, no mesh points, no per-surface friction multiplier, no AI skip rule.
  - A car with any wheel on the ground never sleeps (as specified); the vehicle wakes a sleeping body on
    throttle or handbrake.
  - The source of the car collision half-dimensions is the caller choice (`VehicleSpec::dimension`).
