# Vehicle suspension, tires, steering and aero

Modules: [`libs/blackbox-vehicle`](../../libs/blackbox-vehicle): `suspension` (springs, dampers, anti-roll,
probe, geometry), `tires`, `steering` (Ackermann and the shaping), `aero`, and the per-step assembly in
`vehicle`. Driven by AttribSys data read through `nfsmw-data`.

- **Spec:** [docs/specs/vehicle-suspension-tires.md](../specs/vehicle-suspension-tires.md) (conventions,
  per-step order, springs and dampers, tire model) and its companion
  [docs/specs/vehicle-steering-assists-aero.md](../specs/vehicle-steering-assists-aero.md) (assists, drive
  split, burnout, drift, aero, steering, stabilisers, tunings, field table). The split is only for the
  500-line file limit.
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled, mostly the GameCube build):
  `Physics/Behaviors/SuspensionRacer.cpp`, `Behaviors/Chassis.{h,cpp}`, `Physics/Wheel.h`,
  `Physics/Common/Wheel.cpp`, `Physics/PhysicsInfo.cpp`, `Physics/PhysicsTunings.h`, `Misc/Table.cpp`,
  `Misc/MWAttribUserTypes.h`, `Tools/Inc/ConversionUtil.hpp`, `Interfaces/Simables/ISuspension.h`,
  `Generated/AttribSys/Classes/{chassis,tires,brakes,simsurface,transmission}.h`;
  `Behaviors/SuspensionSimple.cpp` skimmed only. Read for understanding; the Rust code must be written from
  the spec, without the decompiled files open. Facts taken as data: the unit factors, the lookup-table
  values (load-sensitivity, steering, traction/grip-versus-speed, drift and burnout tables) and the
  hard-coded tuning constants, all listed in the spec. No code, comments or identifiers are copied.
- **Implemented:** 2026-10-08, from the specs only (the decompiled files were not open). Centre of gravity,
  state snapshot, corner springs with progression, digression, blow-off and the sway bar, the ground probe
  and compression, the tire update (slip, lateral table, friction ellipse, brake lock, surface multipliers,
  wheel spin), the per-step grip and traction scales, the rear stability boost, brake commands, drive split
  (inside the drivetrain module), aero, steering and Ackermann, landing and creep damping.
  Not implemented: burnout, drift, the axle traction control, wall steer, the airborne stabilisers,
  speed-break, blown tires.
- **To check against the game before calling it done:** see "How to check it" in the spec. Nothing in the
  spec is [verified] yet. Highest-risk items, recorded as open questions there: wheel index/side order
  versus car assembly, the PC build's extra lateral-force multiplier (the GameCube numbers may not match
  PC), the ground-probe semantics (`normal.w`), and the m/s versus mph indexing of the steering tables.
- **Known differences from the original:**
  - The tire update blends the gripping and the sliding force by the last traction instead of choosing
    one: choosing made a tire at its limit alternate between them every step and lose half its force.
  - A gripping wheel sheds its leftover slip (follows the extrapolated ground speed); sliding friction ramps up
    linearly below 0.5 m/s of skid speed; the low-speed viscous brake is capped at a quarter of the body
    mass times the speed per step; the `|torque| / r` cap on sliding force skips brake-locked wheels; a wheel
    that was airborne forgets its road speed on touchdown.
  - Creep damping does not scale the pending force and torque.
  - Speed-break, nitrous and perfect-launch hooks are stubs or missing; AI-only and traffic suspension
    variants (`SuspensionSimple` and the others) are not specified.
  - Ground probe: a downward ray from 0.5 m above the wheel contact point; penetration is measured along the
    surface normal (open question 3).
  - Open questions 1 (wheel order), 2 (PC lateral multiplier), 6 and 7 (units) are unresolved and listed in
    the library README under "What needs calibration".
