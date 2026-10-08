# AI simulation level of detail

Planned module: a `SimLod` part of the game crate (the kinematic stand-in, budgets, spawn and removal
radii) plus small additions to [`libs/blackbox-vehicle`](../../libs/blackbox-vehicle) (a modelled/frozen
switch, chassis sleep).

- **Spec:** [docs/specs/ai-simulation-lod.md](../specs/ai-simulation-lod.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0-1.0, decompiled), `src/Speed/Indep/Src/`:
  `Sim/Common/Simulation.cpp` (scheduler, budgets, `CanSpawnRigidBody`), `Sim/Simulation.h`,
  `Sim/SimTypes.h` (limits, `PhysicsMode`, driver classes), `AI/Common/{AIVehicle,AIVehicleRacecar,
  AIVehicleCopCar,AIVehicleTraffic,AIVehiclePursuit}.cpp` (update rates, simple physics, respawn timers),
  `AI/Activities/{AITrafficManager,AICopManager,AvoidableManager}.cpp`, `AI/AISpawnManager.h`,
  `Physics/Behaviors/{RigidBody,RBVehicle,RBCop,Chassis,ResetCar,SuspensionRacer,SuspensionSimple,
  SuspensionTraffic,EngineRacer,EngineTraffic,BehaviorSpecs}.cpp`, `Physics/{Behavior,PhysicsObject,
  VehicleBehaviors,PVehicle}.h`, `Interfaces/Simables/{IVehicle,ICollisionBody}.h`. Read for understanding
  only; no code, local names or tweak-variable names copied.
- **Data read from the install** (throwaway AttribSys reader, nothing committed): every `pvehicle`
  collection's `BEHAVIOR_MECHANIC_RIGIDBODY/ENGINE/SUSPENSION/INPUT` and `aivehicle` reference, grouped
  (56 racer-chassis collections, 14 cop collections with `RBCop` and the simple chassis, 24 traffic cars with
  the traffic engine and chassis, 8 tractors, 7 trailers, 2 helicopters); `system.SimTasks`;
  `rigidbodyspecs` (`SLEEP_VELOCITY`, `GRAVITY`); `trafficpattern` speeds and spawn time.
- **Gaps in the sources:** `IVehicle::IsOffWorld`, `SetPhysicsMode` and the vehicle creation code (empty
  `PVehicle.cpp`); behaviour ordering (`PhysicsObject.cpp`, `Behavior.cpp` empty); the variable-rate task
  argument. The spec marks the meaning of "off world" (no collision data loaded under the car) and the freezing
  of the body during emulation as **[unconfirmed]**.
- **Negative finding recorded in the spec:** no code path changes a vehicle task's rate by distance; the only
  per-car level of detail is the model choice by car type, the emulated mode while off the loaded world, the
  traffic world-test throttles and the AI skip of ground tests.
- **Implemented:** not yet implemented.
- **Checked against the game by:** pending ("How to check it" in the spec: physics-mode log across the
  edge of the loaded world, speed continuity and the 1 s invulnerability, traffic world-test counts, chassis
  sleep thresholds, traffic budget).
- **Known differences from the original (planned):**
  - Cheaper traffic and cop models are not required at first; the full vehicle model is used with the AI controller
    until the budget needs them.
  - `IsOffWorld` is defined from the rewrite's own streaming state.
  - Think periods are tick counts, not render-frame counts.
