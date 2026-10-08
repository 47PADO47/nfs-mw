# Vehicle engine, drivetrain, brakes and driver input

Modules: [`libs/blackbox-vehicle`](../../libs/blackbox-vehicle): `engine`, `drivetrain` (gearbox, differentials,
torque loop, `Powertrain`), `induction`, `nos`, `brakes`, `input`, and the steering shaping in `steering`.
Filled from AttribSys by `nfsmw-data`.

- **Spec:** [docs/specs/vehicle-engine-drivetrain.md](../specs/vehicle-engine-drivetrain.md) (engine,
  clutch, torque loop, drivetrain split, gearbox) and
  [docs/specs/vehicle-input-induction-brakes.md](../specs/vehicle-input-induction-brakes.md) (induction,
  nitrous, brakes, driver input, special cases). Split in two to stay under 500 lines per file.
  [docs/specs/vehicle-manual-shifting.md](../specs/vehicle-manual-shifting.md) adds the transmission setting and
  what manual mode changes (2026-10-08).
- **Read for the manual-shifting spec** (decompiled, CC0, understanding only): `EngineRacer.cpp` (`DoShifting`,
  `AutoShift`, `SportShift`, `OnGearChange`, `DoGearChange`, the limits at the end of the torque loop),
  `PInput.cpp`/`PInput.h` (`DoShifting`, `IsAutomaticShift`, `DoAutoReverse`), `World/RaceParameters.hpp`,
  `Frontend/Database/FEDatabase.cpp`, `Frontend/MenuScreens/Safehouse/options/uiOptionWidgets.cpp` and
  `quickrace/uiQRCarSelect.cpp`. The label hashes and the 0/1 values of the setting are facts.
- **Sources read for the spec:** dbalatoni13/nfsmw (https://github.com/dbalatoni13/nfsmw, CC0-1.0,
  decompiled; GameCube build): `src/Speed/Indep/Src/Physics/Behaviors/EngineRacer.cpp`,
  `Physics/PhysicsInfo.cpp`, `Physics/Behaviors/PInput.cpp`, `Physics/Behaviors/SuspensionRacer.cpp` (drive
  split, brake torque, steering only), `Physics/Behaviors/Chassis.cpp` (control read-out only),
  `Interfaces/Simables/{IEngine,ITransmission,IInductable}.h`,
  `Generated/AttribSys/Classes/{engine,transmission,induction,nos,brakes,acceltrans,tires}.h`, `Sim/Util.h`,
  `Misc/Table.cpp`, `Tools/Inc/ConversionUtil.hpp`. Read for understanding only; no code copied. The field
  names and the unit conversion factors are facts. The tweak constants and tables quoted in the spec are
  tuning numbers recorded as data, not code.
- **Implemented:** 2026-10-08, from the specs only (the decompiled code was not open while writing). Engine
  torque curve, engine braking, inertia, clutch, the torque loop, shift points, automatic and sport shifting,
  speedometer and limiter, centre and axle differentials, drive-torque split, induction, nitrous, brake
  torques and commands, input shaping and steering. Not implemented: perfect launch, drag shift quality,
  engine heat, sabotage, staging, the catch-up cheats and forced stops.
- **Checked against the game by:** nothing yet. The specs are [decomp] only; "How to check it" in each spec
  lists what to measure on the PC build.
- **Known differences from the original:**
  - The speed limiter tapers the throttle from the unclamped speedometer (the spec clamps it to the limit,
    which would make the taper unreachable).
  - Gear ratios are used as magnitudes; shift points are not computed for more than 10 gear entries.
  - The steering range coefficient uses the `[1, 1.05, 1.1, 1.2, 1.3, 1.4]` table of the chassis spec rather
    than the `[1, 1, 1.1, 1.2, 1.25, 1.35]` one listed here, and the input remap uses the "medium" set here
    rather than the milder one in the chassis spec.
  - The pedal dead zone defaults to 0.05 (Q4 has no value).
  - Peak torque (Q2) is found by scanning the curve with the full induction boost, as the spec suggests.
  - Expected: the GameCube build was read, so values and some behaviours of the PC build may differ.

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
