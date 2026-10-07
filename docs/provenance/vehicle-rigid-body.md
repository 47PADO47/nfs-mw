# Vehicle rigid body

Modules: none yet. Planned home: an engine-generic rigid-body/collision library under `libs/` plus the
vehicle glue in the game crate (see the code-organisation rules).

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
- **Implemented:** not yet. Record the date, and that the code was written from the spec only
  (decompiled code not open), when the first module lands.
- **Checked against the game by:** pending. The spec's "How to check it" lists the measurements (settling
  and sleep, wall rebound and head-on yaw damping, mass-ratio push-out in car-vs-car, reset timing).
- **Known differences from the original:** none recorded yet. Expected: the solver internals above, and the
  source of the car's collision half-dimensions, are reconstructed rather than read.
