# Vehicle engine, drivetrain, brakes and driver input

Modules: not yet implemented. Planned home: a vehicle-physics crate (engine, transmission, induction, NOS,
brakes, input shaping) fed by an AttribSys reader for the classes below.

- **Spec:** [docs/specs/vehicle-engine-drivetrain.md](../specs/vehicle-engine-drivetrain.md) (engine,
  clutch, torque loop, drivetrain split, gearbox) and
  [docs/specs/vehicle-input-induction-brakes.md](../specs/vehicle-input-induction-brakes.md) (induction,
  nitrous, brakes, driver input, special cases). Split in two to stay under 500 lines per file.
- **Sources read for the spec:** dbalatoni13/nfsmw (https://github.com/dbalatoni13/nfsmw, CC0-1.0,
  decompiled; GameCube build): `src/Speed/Indep/Src/Physics/Behaviors/EngineRacer.cpp`,
  `Physics/PhysicsInfo.cpp`, `Physics/Behaviors/PInput.cpp`, `Physics/Behaviors/SuspensionRacer.cpp` (drive
  split, brake torque, steering only), `Physics/Behaviors/Chassis.cpp` (control read-out only),
  `Interfaces/Simables/{IEngine,ITransmission,IInductable}.h`,
  `Generated/AttribSys/Classes/{engine,transmission,induction,nos,brakes,acceltrans,tires}.h`, `Sim/Util.h`,
  `Misc/Table.cpp`, `Tools/Inc/ConversionUtil.hpp`. Read for understanding only; no code copied. The field
  names and the unit conversion factors are facts. The tweak constants and tables quoted in the spec are
  tuning numbers recorded as data, not code.
- **Implemented:** not yet. To be done from the specs only, with the decompiled code closed.
- **Checked against the game by:** nothing yet. The specs are [decomp] only; "How to check it" in each spec
  lists what to measure on the PC build.
- **Known differences from the original:** none yet. Expected: the GameCube build was read, so values and
  some behaviours of the PC build may differ (see questions).

## Open questions

Tracked for the implementer; none blocks starting.

- **Q1** Torque curve spacing: samples are spread over `IDLE..MAX_RPM` but lookups clamp at `RED_LINE`, so the
  tail past the redline is never used. Is that the intended reading, and does `MAX_RPM` in the data really
  exceed `RED_LINE`?
- **Q2** `MaxInductedTorque` / `MaxInductedPower` (peak torque, its RPM, peak power) are declared in
  `PhysicsInfo` but their bodies were not in the sources read. The spec assumes "scan the curve with full
  induction boost over idle..redline". Needed for the torque-converter blend, perfect-launch and the UI hp.
- **Q3** The shift-point search calls the induction helper with `rpm` and `spool` swapped, which makes
  its factor equal 1. Is that also true in the PC build, or does it include boost there?
- **Q4** `PAD_DEAD_ZONE` (gas/brake snap thresholds) has no value in the sources read; also the steering
  remap table choice (`STEER_REMAP_MEDIUM`) and whether keyboard input uses the same path.
- **Q5** Fixed time step and the unit of `dT`; whether the 60 Hz averager windows match the actual
  simulation rate on PC.
- **Q6** Steering tables are indexed over 0..160 by car-frame forward speed: is that m/s (so 160 m/s is far
  beyond reach) or a different unit? It decides the whole speed sensitivity; measure in the game.
- **Q7** `SPEED_LIMITER[1]` is read as the ramp width (in mph); confirm against a car with a limiter. Also
  whether `GEAR_EFFICIENCY` and `TORQUE_CONVERTER` are non-trivial in the shipped data.
- **Q8** Brake units: `BRAKES` in ft·lb scaled by 4 (and `EBRAKE` by 10) look like tuning fudge; check
  real stopping distances. Also `FLYWHEEL_MASS` units and `FLOW_RATE` (appears display-only).
