# Pursuit driving integration provenance

Date: 23 October 2026.

The implementation follows the existing specs `ai-pursuit-tactics.md`,
`ai-pursuit-goals.md` and `ai-driver-control.md`. Additional behavior is recorded
in `../specs/ai-pursuit-driving-integration.md` before implementation corrections.
No original source or binary artifacts are included in this change.

References read for behavior:

- [AIActionRace.cpp](https://github.com/dbalatoni13/nfsmw/blob/main/src/Speed/Indep/Src/AI/Actions/AIActionRace.cpp):
  raw-target catch-up speed vectors and the pursuit turn-around braking guard.
- [AIActionPursuitOffRoad.cpp](https://github.com/dbalatoni13/nfsmw/blob/main/src/Speed/Indep/Src/AI/Actions/AIActionPursuitOffRoad.cpp):
  action eligibility, target-speed resampling, pursuit seek, limiter, avoidance,
  affinity and nitrous delay.
- [AISteer.cpp](https://github.com/dbalatoni13/nfsmw/blob/main/src/Speed/Indep/Src/AI/Common/AISteer.cpp):
  capsule dimensions, closest-axis separation, centre-based predictive weighting
  and per-vehicle contribution cap.
- [AIVehicle.cpp](https://github.com/dbalatoni13/nfsmw/blob/main/src/Speed/Indep/Src/AI/Common/AIVehicle.cpp):
  targeting ray semantics, predicted drive-nav ray origin and road updates.

These are reconstructed GameCube sources, used as evidence, not copied code.
Original-PC scheduler findings are recorded separately in the core parity spec.
The speed-history resolution remains 0.125 seconds; the action scheduling rate
selects every eighth simulation tick. They are distinct quantities.

The world implementation now connects goal target points to steering, actual
player performance, jerk state, vehicle separation and per-think limiter state.
It also selects the close action when the documented range, relative velocity
and drivability conditions allow it. Road fallback remains active otherwise.

Synthetic regressions cover history resampling across different update rates,
eligibility boundaries, wall strength, nitrous delay, capsule separation and a
physical driver consuming a ram command to the side of its raw road slot.
Execution results belong in the run's validation evidence; listing tests here
does not claim that they passed.

Still unverified: the original-PC persistent target-road walker and lane/cursor
state, exact collision masks, full pitched/rolled capsule axes, original spawn
speed handoff, cumulative airborne timer behavior and generic far-chase nitrous.
The existing measured performance estimator also remains a separate approximation
of the original car performance model.
