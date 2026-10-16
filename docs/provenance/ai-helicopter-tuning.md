# Police helicopter tuning file and hidden-zone types (epic E9, tuning part)

- **Restricted source read:** [docs/mods/nfsmwhelicopteroptions.md](../mods/nfsmwhelicopteroptions.md), a behaviour summary of
  the NFSMWHelicopterOptions mod (MIT). Used for the file's key names (`[Helicopter:<Name>]`, `default`, `heatNN`,
  `raceNN`) and the list of tunables. **Its numbers are not used as defaults.** Its FuelTime defaults for heat 7 to 10
  are 180, 180, 180, 180 s, while the install-verified table in [specs/ai-helicopter.md](../specs/ai-helicopter.md)
  section 2.3 gives 10, 75, 120, 400 s. Heat 4 to 6 agree (60, 90, 180 s). The install-verified table is the default.
- **Defaults and rules:** [specs/ai-helicopter.md](../specs/ai-helicopter.md) section 2.3 (`pursuitlevels` per heat and race
  level, tagged [verified] there), section 3 (the hidden-zone rule), section 8 (constants). Also the format record of HIDDEN
  zones in [formats/pursuit-data.md](../formats/pursuit-data.md) section 1, which states that every installed HIDDEN zone has
  `Data[0] = 0` (so the install's zones hide from both).
- **Mod numbers not used:** the mod's FuelTime row (10, 10, 10, 60, 90, 180, 180, 180, 180, 180) and its FlySpeed, SearchSpeed,
  Leading, ClosingSpeed, SearchRadius, Turning, Acceleration and Brake defaults were not taken as game truth. Where the spec
  has a value (fly and search speed, attack duration, visual sphere), the spec's is the default.
- **Ghidra:** not run in this pass. The `ghidra-mcp` tools were deferred and not loaded, so `CanSeeTarget`, the per-level
  `heliLOSdistance` and `HeliFuelTime` reads, `MAX_SPEED_MPS` and the hidden-zone data words were **not** re-checked in
  `speed.exe`. The values are the spec's, as tagged there. The install reader `nfsmw-data` has no field reader for
  `pursuitlevels` helicopter fields yet, so there is no install-gated test of them either.
- **Clean-room statement:** the file's schema, reader and clamping are our own code (`crates/nfsmw/src/settings/helicopter.rs`,
  built on `libs/ini-tuning`). No code of the mod was read or copied.
- **Hidden-zone reversal:** the perpetrator's test, as recorded in [formats/pursuit-data.md](../formats/pursuit-data.md): a
  HIDDEN zone whose first data word is the name hash of `"Car"` hides the target from cars only; `"Heli"` from helicopters
  only; any other word from both. Implemented in `crates/nfsmw/src/scenes/world/ai/police/world.rs`
  (`hidden_from_cars`, `hidden_from_helicopters`).
- **Tunnel test:** the spec's `CanSeeTarget` checks the main game view for a tunnel. We test the player's place against the
  TUNNEL zones (kind 3). The camera is not in the police state, so this is a proxy and is noted as such in the code.

## Pass 2: helicopter constants in speed.exe (Ghidra, PC v1.3)

Pass 1 above did not run Ghidra. Pass 2 did, on `speed.exe` (image base `00400000`, 26 079 functions, no symbols). The
results are in [specs/ai-helicopter.md](../specs/ai-helicopter.md) under "Constants in speed.exe (pass 2)".

**Tool limits this session.** `get_xrefs_to`, `get_xrefs_from` and `get_assembly_context` were listed as callable by
`search_tools`, but the calls failed with "No such tool available", and the deferred lookup never returned their schemas.
So no cross-reference search was possible. Substitutes used: `get_functions` with `fields=callers` (who calls a function)
and `fields=refs` (every address a function reads or writes, including strings and pool words); `force_decompile`;
`read_memory` for pool words and packed strings. A byte-pattern search (for immediates like `0x437A0000`) was not
available either.

**Addresses confirmed** (all `[decomp]`):

| Address | What | Note |
|---|---|---|
| 0x004269A0 | spawn helper (`SpawnPursuitHelicopter`) | 250 m push is the immediate `0x437A0000`; height `+20` is pool word 0x00890560; backward is 0x00890DA8 (-1.0); increments the heli count at `this+0x9C`; calls the pool lookup 0x426610 |
| 0x00426610 | pool lookup (a vehicle of a type, not spawned) | filters by a type word passed in a register; the decompile shows it as a return address, not resolved; callers: spawn, ground-cop placement, roadblock, pursuit |
| 0x0043E8D0 | cop manager update (the only caller of the spawn found) | `caller_count` 0 in the function list, so it is entered indirectly (not traced) |
| 0x0042EA90 | ground-cop spawner | not the heli: 60 m (`0x008EB1AC - 60`) or 80 m road distance; counter `this+0x94` |
| 0x0042EBF0 | pursuit cop loop | builds the `AIPursuit` factory name (0x008915E4); not the heli |
| 0x00430D90 | ground-cop placement on a road node | not the heli; counter `this+0x94` |
| 0x0043DD30 | roadblock spawner | uses `AIRoadBlock`, `XO_Sawhorse_1b_00`, `XO_SpikeBelt_1b_DW_00`; pool 0x008A2FB4 (250.0) and 2 pi (0x00890510); not the heli |

**Name blocks (strings, no pointers).** `AIVehicleEmpty`, `AIVehicleHuman`, `AIVehicleCopCar`, `AIVehicleTraffic`,
`AIVehicleHelicopter` at 0x0089302C to 0x00893070; `AIGoalHeliPursuit` 0x00891160, `AIGoalHeliExit` 0x00891174,
`AIActionHeliExit` 0x0089110C, `AIGoalHeliRoadBlock` 0x00891550, `AIActionStrafe` 0x00893008, `AIHeli` 0x008B487C;
`CHOPPER` 0x008AAA2C with `HOVER` 0x008AAA50 in a packed vehicle-class list (from `RigidBody` at 0x008AAA10);
`SimpleChopper` 0x008ADDFC, `DamageHeli`, `DrawHeli`, `SoundHeli`, `chopperspecs` 0x008ADD28 in a packed behaviour list
from 0x008ADD20; `HeliWash` 0x008AD6DC, `heliwash` 0x008AD6E8. The behaviour and class lists are packed without
pointers, so they give no xref to the code that reads them.

**Float pool.** Read with `read_memory` in blocks of 128 to 448 bytes and decoded with `BitConverter.ToSingle` in
PowerShell (decoding by hand gave errors). The whole range 0x00890500 to 0x00890E8F was read. The helicopter-like cluster is
0x00890828 (0.1, 0.125, 0.125, 0.25, 0.25, 0.0625), 0x00890840 (0.45), 0x00890844 (0.65), 0x00890848 (0.3), 0x0089084C (0.4),
0x00890850 (0.3), 0x00890854 (250), 0x00890858 (7), 0x0089085C (0.2), 0x00890860 (1.2), 0x00890864 (0.2), 0x00890868 (0.01).
Other values, with addresses, are in the spec's table. Words at 0x00890CE0 to 0x00890D8F are code pointers (0x0040xxxx)
and mixed strings; they are not decoded as floats.

**Dead ends (do not repeat).**
- Class namespaces: `list_class_members` for `AIVehicleHelicopter` and `SimpleChopper` returns nothing; `list_program_items`
  for classes lists only the DLL names, exception types and switch labels. The binary has no RTTI names for these classes.
- The behaviour name list at 0x008ADD20 and the vehicle-class list at 0x008AAA10: packed strings, no pointer words.
- The roadblock spawner 0x0043DD30 and the ground-cop functions 0x0042EA90, 0x0042EBF0, 0x00430D90 looked like the heli
  spawn at first (they call the same pool lookup). They are not the helicopter: they use counter `+0x94` or `AIRoadBlock`.
- Pool constants without an xref: the cluster matches the spec's controller numbers, but no function that reads it has been
  found. The use sites are not established, so the controller links stay `[unconfirmed]`.

**Next leads.** See the spec's "Next leads". The main one is an xref tool: rerun the cluster search (refs containing
0x00890828 to 0x00890868) and the factory-caller search (callers of `0x005CC240` with `0x00893070`) once the xref group loads.
