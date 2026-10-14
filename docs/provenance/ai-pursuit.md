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

## Assault, property, spike strips and breakers (E8 subtask h)

- **Reversed from:** the existing `[decomp]` and `[verified]` notes in `docs/specs/ai-pursuit-heat.md` section 4 (the
  infraction table and its fines), `ai-pursuit-roadblocks.md` sections 1, 2.3, 4 and 6 (the spike chances, the spike
  layouts, the tire puncture) and `pursuit-breakers.md` section 7 (`EBreakerStopCops`). No new Ghidra reversal was run in
  this batch: the assault and property detection rules (`GInfractionManager`) are stripped in the decompilation, so the
  rules rest on the spec's table and remain **[unconfirmed]**.
- **Implemented as:** `blackbox_pursuit::rules::contact_infraction` (unchanged), fed by the player's cop hit, the prop
  contact, and the existing traffic hit; `blackbox_pursuit::roadblock::{spike_chance, takes_spikes}`; `blackbox_pursuit::spikes`
  for the strip geometry and tire test; `blackbox_pursuit::breaker::inside` for the zone test, now horizontal as the spec
  says.
- **Known differences:** a breaker zone brakes the cops inside it (`StopCops` to `set_breaker_stop`), it does not wreck them as
  the spec's `Destroy()` does. The sawhorse barriers are not placed. The spike strip's size is a design value.
- **Not done:** Ghidra cross-check of `GInfractionManager` and `AICopManager::ApplyBreakerZones`; both remain open.

## Wave re-pick on heat rise (E8 subtask d)

- **Reversed from:** the existing `[decomp]` notes in `docs/specs/ai-pursuit.md` section 2 and `ai-pursuit-cops.md` section 4 (the
  attribute instance is re-created when `int(heat)` changes and `CopRequest` reads the current row). A Ghidra cross-check was
  attempted but not completed: `speed.exe` is ambiguous in the open programs and the symbol-name search returned no hits.
- **Implemented as:** the chase holds all ten `heattable` rows and re-reads the wave each spawn decision (`nfsmw` `Pursuit::follow_heat`,
  `blackbox_pursuit::wave::pick_needed`). Race rows are read by `nfsmw_data::pursuit::race_heat_row`; the chase does not yet know it is
  inside a race event, so it still uses `heattable`.
- **Not done:** surplus removal when heat falls or a type is over its count; heat growth rate is unchanged (`TIME_PER_LEVEL` in `police.rs`
  is bin-independent and does not apply the career `ScaleEscalationPerBucket`).
