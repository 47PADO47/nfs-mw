# Driving history and assists corrections

The 2026-10-14 [final audit corrections](../specs/vehicle-final-audit.md) record
reverse slip, compression-based wheel feedback, maneuver tire drag/body direction,
wall/landing snapshot ordering, grouped ground rays, cached peak torque, collision
mass/inertia overrides, locked-wheel grip,
wheel sign crossing and per-tick drift charge. These were
checked against the original PC routines and the same behavioral references;
the implementation was written from the specification. Full live driving parity
remains unverified.

- Date: 2026-10-13.
- Specs: [steering history](../specs/vehicle-steering-history.md),
  [engine/drivetrain](../specs/vehicle-engine-drivetrain.md) §3 and
  [chassis assists/aero](../specs/vehicle-steering-assists-aero.md) §4–7 and
  [tire profiles](../specs/vehicle-tire-profiles.md).
- Sources: [dbalatoni13/nfsmw](https://github.com/dbalatoni13/nfsmw), CC0-1.0,
  decompiled reference, and locally inspected original PC v1.3 tire, wheel-feedback,
  body begin-frame and sleep routines. `SuspensionRacer.cpp` steering/assists was inspected to
  resolve history ordering. No source expression copied; implementation follows
  the written specifications after reference inspection.
- Changes: remove the custom peak-engine-torque cap on the re-engaging clutch;
  pass the original 0.8 airborne downforce retention from the game adapter;
  correct previous-input history, coefficient averaging and fixed-wheel range;
  restore axle traction control and drift friction/yaw stabilization. Add a reference tire profile
  with original branch/history/friction, sign-dependent longitudinal force limits, reported wheel
  speeds and explicit drivetrain wheel writes, original net-force sleep and asymmetric burnout.
  Caller-supplied maneuver coefficients provide extra downward acceleration, reduced drag,
  front-wheel grip and enlarged steering without adding game-specific library defaults.
- Validation: synthetic deterministic vectors and whole-car stability tests.
  Vehicle regressions pass without relaxing prior expectations. Six longitudinal-force and
  twelve wheel-feedback vectors match isolated original PC machine-code execution. Installed
  stock/maximum packages for all 48 selectable racing/police cars passed launch, gear progression,
  braking and steering road-following checks. Those usability checks use driver feedback to counter
  the intended asymmetric burnout, rather than requiring zero-input launches never change heading.
  Complete sustained PC-driving comparison is still required. Drag launch/shift bonuses remain
  specific to race-start/drag-race contexts that free roam does not activate. Wall steering follows the original PC contact callback
  and yaw consumer, with twelve isolated instruction replay cases and contact-sequence regressions;
  the host's outline collision geometry remains a separate approximation. Jump stabilization and its cached terrain dependency are
  documented in [jump stabilizers](vehicle-jump-stabilizers.md). The generic regularized profile
  retains its documented stabilizations; the reference profile removes the tire/sleep deviations.
  Ground query and world-response differences remain independent parity limits.
