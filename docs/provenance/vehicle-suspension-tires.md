# Vehicle suspension, tires, steering and aero

Modules: none yet. Planned home: an engine-generic vehicle-dynamics library under `libs/` (corner springs,
tire model, steering geometry, aero) driven by AttribSys data read through `nfsmw-data`.

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
- **Implemented:** not yet. Date and implementer to be filled in when the first module lands.
- **To check against the game before calling it done:** see "How to check it" in the spec. Nothing in the
  spec is [verified] yet. Highest-risk items, recorded as open questions there: wheel index/side order
  versus car assembly, the PC build's extra lateral-force multiplier (the GameCube numbers may not match
  PC), the ground-probe semantics (`normal.w`), and the m/s versus mph indexing of the steering tables.
- **Known differences from the original (planned):** none implemented. Speed-break, nitrous and
  perfect-launch hooks are expected to be stubbed; AI-only and traffic suspension variants
  (`SuspensionSimple` and the others) are not specified.
