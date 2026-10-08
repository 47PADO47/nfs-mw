# AI driver control

Planned modules: an AI driver layer (controllers, speed governor, action selection helpers) beside
[`libs/blackbox-vehicle`](../../libs/blackbox-vehicle) and the game-specific goals and actions in `crates/`.

- **Spec:** [docs/specs/ai-driver-control.md](../specs/ai-driver-control.md) (composition, schedule, simple
  controller, reversing, stuck recovery, output interface, `aivehicle` data),
  [docs/specs/ai-driver-control-pid.md](../specs/ai-driver-control-pid.md) (PID throttle and adaptive PID
  steering) and [docs/specs/ai-driver-speed-skill.md](../specs/ai-driver-speed-skill.md) (speed target,
  performance matching, skill, catch-up, nitrous decision).
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0-1.0, decompiled; mostly the GameCube build),
  `src/Speed/Indep/Src/`: `AI/AIVehicle.h`, `AI/Common/{AIVehicle,AIVehicleRacecar,AIVehicleTraffic,
  AIVehicleCopCar,AIVehiclePursuit,AIGoal,AIAction,AISteer,AIMath,AITarget,AdaptivePIDController}.cpp`,
  `AI/{AIGoal,AIAction,AISteer,AIMath,AITarget,AIAvoidable,AdaptivePIDController,aireflectedtypes}.h`,
  `AI/Actions/{AIActionRace,AIActionTraffic,AIActionGetUnstuck,AIActionAirborne,AIActionNone,AIActionSpline,
  AIActionStopShort,AIActionTooDamaged}.cpp`, `AI/Actions/{AIActionRam,AIActionPursuitOffRoad}.cpp` (only the
  use of the `aivehicle` multipliers and the shared limiter), `AI/Activities/AvoidableManager.cpp`,
  `Misc/Table.{hpp,cpp}`, `Sim/Common/Simulation.cpp` (task scheduler), `Sim/SimTypes.h`,
  `Interfaces/Simables/{IINput,IVehicle,ICheater}.h`, `Physics/Behaviors/{PInput.cpp,PInput.h,
  SuspensionRacer.cpp,SuspensionSimple.cpp,SuspensionTraffic.cpp,EngineRacer.cpp,EngineTraffic.cpp}` (only
  where they read the AI's output or the catch-up cheat), `Physics/PhysicsInfo.{hpp,cpp}`,
  `World/Common/WRoadNetwork.cpp` (`FetchAvoidables`, `CookieTrailCurvature` to see the interface only; the
  navigator itself is specified in [ai-road-nav-trail.md](../specs/ai-road-nav-trail.md)). Read for
  understanding only. No code, local names or tweak-variable names copied; the numeric constants and
  attribute field names quoted are facts (data).
- **Data read from the install** (this machine, `D:\Need For Speed Most Wanted Black Edition`, with a
  throwaway Python AttribSys reader, nothing committed): the `aivehicle` collections and their
  multipliers; `collisionreactions` records and the goal-name hash (Jenkins lookup2, case-sensitive: `AIGoalRacer`
  = 0x08D0D8A7, `AIGoalStaticRoadBlock` = 0x9E55B2E3, `AIGoalPit` = 0xEC619CB2); `system.SimTasks` order;
  `rigidbodyspecs.GRAVITY` (-9.8128 for racers, cops and traffic); `tires.STEERING` and `STATIC_GRIP`;
  `chassis.AERO_COEFFICIENT`.
- **Gaps in the sources** (empty or stripped in the decomp): the body of `ComputeAccelerationTable` and
  `EstimatePerformance` (how the AI's top speed, 10-sample acceleration table and the performance ratings are
  computed); `GRaceStatus::ComputeCatchUpSkill` (the rubber band proper); the spawn code that picks the AI
  behaviour class per driver class (`PVehicle.cpp`); `PhysicsObject.cpp` and `Behavior.cpp` (the order of
  behaviours inside the Physics schedule); the variable-rate task handler argument (marked unsolved). These
  parts are marked **[unconfirmed]** in the spec with a design that must be tuned on recordings of the
  running game, not on more decompiled code.
- **Implemented:** not yet implemented.
- **Checked against the game by:** pending. Each spec ends with a "How to check it" list: speed target
  versus a known curve radius, launch governor against real acceleration, pedal step responses, steering
  heading error rate, nitrous timing, and think intervals.
- **Known differences from the original (planned):**
  - Think timers for the traffic class run in ticks (10), not render frames, for determinism.
  - The adaptive controller's "term in turn" uses sim time rather than the wall clock.
  - The acceleration table is built by a headless full-throttle run (the original's builder is not available).
  - The catch-up cheat consumers inside the simple (cop) chassis are not ported (inactive in the shipped data).
