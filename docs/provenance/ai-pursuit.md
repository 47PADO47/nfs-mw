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

## Support vehicles: ram SUVs and cross cars (D1)

- **Spec:** [../specs/ai-pursuit-support.md](../specs/ai-pursuit-support.md). Roll and gates: [../specs/ai-pursuit-formations.md](../specs/ai-pursuit-formations.md) section 6.
- **Binary:** speed.exe (PC), Ghidra program `/speed.exe`, image base `0x400000`. Decompiled code and assembly were read in
  the Ghidra MCP, not copied into the repo.

### Addresses

| What | Address | Note |
|---|---|---|
| Support executor (per pursuit, per tick) | `FUN_0043e770` | calls the acquire and place pairs; state 1 = pending, 2 = active |
| Cop manager tick | `FUN_0043e8d0` | walks the pursuit list, calls the executor, spawns waves, pursuit creation |
| Heavy acquisition (strategies 1 to 4) | `FUN_0041f130` | `copsuv` / `copsuvl` roll at `0041f1b0`..`0041f1c3`; ram timer `0x40400000` (3.0) at `0041f256` |
| Heavy placement | `FUN_0041f2e0` | `400 - 60 - 10k`; direction negated at the placement call |
| Cross acquisition (strategies 5 to 7) | `FUN_0041f480` | names `copcross`, `copsporthench`; goal `AIGoalPursuit`; pursuit offsets table |
| Cross placement | `FUN_0041f650` | `-(150 + 10k)`; state and duration set at the end |
| Spawn search (point at a signed distance on the cop road) | `FUN_0041eed0` | tries; distance window flag |
| Spawn validity | `FUN_00419fc0` | 5, 15, 40 thresholds (unit unsettled); no-cop-spawn zone query |
| Car finder (pool reuse or new car) | `FUN_0041ecd0` | name in a register, read from the disassembly |
| Roadblock builder (also used by the chase) | `FUN_0043dd30` | `AIRoadBlock`; sawhorse and spike-belt parts |
| Goal constructor `AIGoalHeadOnRam` | `FUN_0043bd10` | action list; in-position `(0,0,-3)`; vtable `PTR_FUN_00892cd8` |
| Goal factory wrapper | `FUN_0043d780` | `new` of the goal, calls the constructor |
| Action creator `AIActionHeadOnRam` | `FUN_00421480` | registration node: hash `0x90d748`, creator `0x90d74c`, list head `0x90d8ec` |
| Action constructor (score 0.1) | `FUN_00421360` | primary vtable `PTR_FUN_008913b8`; secondary `PTR_LAB_008913b0` |
| Action start (speed cap) | `FUN_004214f0` | `min(26.82, desired)` for a just-placed car |
| Action update (intercept, 100 km/h, `DoDriving(7)`) | `FUN_00421560` | vtable slot 7 |
| Intercept formula | `FUN_00418200` | the tactics spec's `Ram` |
| Pursuit constructor | `FUN_00442d00` | the two `AIPursuit` tasks (0.25 s, 1.0 s); starting values |
| Name hash | `FUN_005cc240` | used for every goal, action, car and message name |
| Random percent source | `FUN_00627320` (on `FUN_006ed200`) | `(r * 100) >> 16` |

Strings and data: `AIGoalHeadOnRam` `0x89091c`, `AIActionHeadOnRam` `0x892cc4`, `copsuv` `0x891238`, `copsuvl` `0x891230`,
`copcross` `0x891240`, `copheli` `0x8915dc`, `Request` `0x891228`, `Created` `0x892ee8`, `ReqDenied` `0x892edb`, `AIRoadBlock`
`0x892ecc`, `MReqRoadBlock` `0x890c80`, `MReqBackup` `0x890c90`. Goal name table: `0x8eb1b0` onward (pointers to the goal
name strings, indexed by the strategy). Float constants: `0x8eb1a8` = 150.0, `0x8eb1ac` = 400.0, `0x8eb1a0` = 225.0,
`0x8eb1a4` = 300.0 (the last two not used in the support code read).

### Method

1. Searched the strings for the goal and action names, then xrefs to the strings. The `DATA` xrefs on the goal names
   led to the name tables (`0x8eb1b0`) and to the goal constructor's caller; the `AIActionHeadOnRam` string led to the
   action creator through a static initializer.
2. Followed the goal factory to the executor (`FUN_0043e770`) by its callers, and read its three branches.
3. Read the executor's callees (acquire, place, search, check, finder) one by one, then the call-site register for the
   name of each car from the disassembly.
4. Read the action's constructor and vtable; the update slot is 7.
5. Decoded the float constants by hand from `read_memory` hex (`0x41DE38E4` = 27.78; `0x3dcccccd` = 0.1).

### Lessons

- The decompiler drops arguments that a `__fastcall` or `__thiscall` call passes in a register or through a vtable
  slot. Two cases here: the car name in `FUN_0041f130` (the `EDX`-style push at `0041f1cc`) and the goal name passed to
  the vtable slot `+0x84`. Read `disassemble_function` at the call site before trusting a decompiled call's arguments.
- A static initializer that does `push "name"; call hash; mov [global], eax; mov eax, [list]; ...; mov [node], imm32; ...`
  is a factory registration. The creator is the `imm32`. Find the initializer through the `DATA` xref of the name string.
- The `DATA` xref on a string is often an initializer's `push`, not a table: check the bytes after the xref before
  reading it as data.
- Hand-decode float constants from the hex with the mantissa bits. A quick decimal estimate was off by 0.02 for 27.78;
  the careful decode matched 27.7778.
- The 16-byte stride in a constant table can hide a `vec3` layout: `(x, 0, z)` entries at `0x0`, `0x10`, `0x20` read
  as `(0,0,-20)`, `(-4,0,-24)`, `(4,0,-24)`, which the decompile printed as a run of locals.
- Some vtable labels do not hold the methods. For the pursuit (`PTR_FUN_008928ec`) the bytes after the label are string
  data, so the label is not the interface the executor calls with slot `+0x44`. Resolve the subobject first.
- `get_function_call_graph` on a big manager function returns more than 100 edges; ask for a depth of 1 or use the
  decompile instead.

### Dead ends

- **"Leader" goal.** No `AIGoalLeader` or any leader goal exists: the string search for `Leader` returns only
  leaderboard and HUD strings. The cross car is an ordinary `AIGoalPursuit` with a pursuit offset. Followers are not
  directed to it.
- **`pursuitsupport` as a string.** Not in the binary: AttribSys names are hashed. Found the support tables by their
  consumers instead (the option's fields at `+0x00`, `+0x08`, `+0x0c`).
- **Pursuit support request timer.** The countdown and the reset were not found: the executor calls the pursuit through
  an interface (slot `+0x44`) whose vtable label holds string data. Not resolved; see the spec's open question Q1.
- **Pool budget check** (`manager+0x54`, slot `+4`): called, not traced.
- **Roadblock return value.** The executor and the manager read `FUN_0043dd30`'s return in opposite senses (spec Q3).

### Next leads

- Pursuit interface subobject for slot `+0x44` and its update (the timer and reset).
- The caller of `FUN_0041f130` that drops a denied request (spec Q2).
- The spawn manager constructor for its `+4` and `+8` distances (spec Q5).
- The action vtable slots 2 to 6 (attempt, start, finish) for the ram action.
- `FUN_00413320` (desired speed) and `FUN_00418200` (intercept).
