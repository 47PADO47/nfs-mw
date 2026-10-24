# Police acceleration handoff evidence

- Specification: [police acceleration handoff](../specs/ai-pursuit-acceleration-handoff.md).
- Sources read on 2026-10-23: the CC0-1.0
  [dbalatoni13/nfsmw reference](https://github.com/dbalatoni13/nfsmw),
  `AI/Common/AIVehicle.cpp`, `Physics/Behaviors/EngineRacer.cpp` and
  `Physics/Behaviors/SuspensionSimple.cpp`. Existing installed mechanic findings
  are recorded in [AI simulation modes](../specs/ai-simulation-lod.md).
- Source audit: the remake's simple chassis adapter changes contact state and
  steering but does not select a distinct live powered simple tire solver.
  No physical speed correction is inferred from this source audit alone.
- Runtime evidence: a private original-game pursuit captured vehicle lifecycle,
  requested direction and speed, actual body pose and velocity, controls and
  simulation context. The modded installation and mixed heat/vehicle sample
  prevent treating aggregate speeds as a stock midsize acceleration target.
- Implementation: an independently written installed physical diagnostic follows
  the full-throttle, driver and governor paths on the same flat-road fixture.
- Fresh native evidence: the selected PC 1.3 executable identity was confirmed
  before inspecting the AI constructor's acceleration-table call, the analytical
  estimator, the inducted-torque averaging helper and curve interpolation.
  Attribute layouts identify mass, rear tire dimensions, ratios, efficiencies,
  engine RPM fields and chassis drag. Arithmetic constants were read from that
  same executable; no machine instructions or original source are included here.
- Analytical implementation: a generic optional performance estimate follows the
  written mathematical specification. Physical full-throttle measurement remains
  a separate API. The game adapter can select the analytical estimate for action
  planning without changing the player-controlled physical vehicle.
- Validation limits: the new installed diagnostic must be run against the
  selected installation; a passing handoff test does not prove original chassis
  force parity or close the reported integrated pursuit problems.
- Baseline installed result: one selected stock midsize handoff diagnostic passed
  before production estimator wiring. Direct PID closely follows full physical
  throttle; the measured-curve governor asks for less speed. The observed
  one/three/five-second values are recorded in the specification. No torque or
  live tire-force constants were changed from that comparison.
