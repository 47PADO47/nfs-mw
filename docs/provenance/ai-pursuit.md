# AI pursuit (state, heat, cop management, busted and evade)

Planned module: the pursuit crate of milestone 7 (not created yet). Engine-generic parts (a pursuit state machine, a target visibility
test) would live in `libs/`; the Most Wanted specifics (the `pursuitlevels` tables, the cop car names, heat rules) in `crates/`.

- **Spec:** [docs/specs/ai-pursuit.md](../specs/ai-pursuit.md), [docs/specs/ai-pursuit-heat.md](../specs/ai-pursuit-heat.md),
  [docs/specs/ai-pursuit-cops.md](../specs/ai-pursuit-cops.md).
- **Sources read for the spec:** dbalatoni13/nfsmw (CC0, decompiled; GameCube build), `src/Speed/Indep/Src/`:
  `AI/AIPursuit.h`, `AI/Common/AIPursuit.cpp`, `AI/Activities/AICopManager.cpp`, `AI/AISpawnManager.h`, `AI/Common/AISpawnManager.cpp`,
  `AI/AIVehiclePursuit.h`, `AI/Common/AIVehiclePursuit.cpp`, `AI/AIVehicleCopCar.h`, `AI/Common/AIVehicleCopCar.cpp`,
  `AI/AIGoal.h`, `AI/Common/AIGoal.cpp`, `AI/Common/AIVehicle.cpp` (`AIPerpVehicle`, `AIVehicleHuman`), `AI/Actions/AIActionTooDamaged.cpp`,
  `AI/Actions/AIActionStopShort.cpp`, `AI/aireflectedtypes.h`, `Interfaces/Simables/IAI.h`, `Interfaces/SimActivities/ICopMgr.h`,
  `Sim/Common/Simulation.cpp` (task scheduling), `Gameplay/GInfractionManager.h`, `Gameplay/GRaceStatus.h`,
  `Frontend/Database/VehicleDB.cpp` (career records, impound), `Frontend/MenuScreens/Safehouse/career/uiInfractions.cpp`,
  `Frontend/HUD/{FePursuitBoard,FeHeatMeter,FeCostToState,FeInfractions,FeRadarDetector,FeBustedMeter,FePkg_Hud}.cpp`,
  `Animation/AnimChooseArrest.cpp`, `Generated/AttribSys/Classes/{pursuitlevels,pursuitsupport,pursuitescalation,infractions}.h`.
  Read for understanding only; no code, local-variable names or tweak-variable names copied. Field names of the attribute classes and the
  numeric tuning quoted in the specs are data (facts), read from the install's AttribSys vaults with a throwaway reader.
- **Data read from the install (not the decomp):** `pursuitescalation` (1 collection), `pursuitlevels` (21 collections), `pursuitsupport`
  (21), `infractions` (8), `aivehicle` cop collections, `gameplay` race-bin fields (`BaseOpenWorldHeat`, `MaxOpenWorldHeat`), `speechtune`
  from `GLOBAL/attributes.bin` and `GLOBAL/gameplay.bin` on 2026-10-09. Only small tuning tables are reproduced; no dumps.
- **Gaps in the sources:** `GInfractionManager.cpp` is empty (the speeding, racing and reckless detection rules and thresholds are
  unknown); the code that copies pursuit values into the HUD elements and that loads the player's starting heat is absent; the semantics
  of the "variable frame" task `dT` in the scheduler is marked unsolved in the decomp; the `AssignClosestOffsets` greedy loop is marked
  as not matching; the PC cop budget is unknown; `GRaceStatus.cpp` (race-bin accessors) is stripped. The specs mark these **[unconfirmed]**
  or list them under "Open questions".
- **Implemented:** not yet.
- **Checked against the game by:** pending. The specs list the measurements (busted-bar fill time, evade timer, wave size per heat, heat
  growth per second, spawn distances, rep points per kill).
- **Known differences from the original:** none yet. Expected: the rewrite may use a fixed 4 Hz pursuit step in real seconds (the original
  runs it at a fraction of its sim rate); the cop budget and the helicopter are decided by the rewrite's milestones.
