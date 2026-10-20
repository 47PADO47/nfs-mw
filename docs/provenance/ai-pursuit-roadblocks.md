# Roadblocks, spike strips and the helicopter's spawn (police parity pass)

- **Specs:** [../specs/ai-pursuit-roadblocks.md](../specs/ai-pursuit-roadblocks.md), [../specs/ai-helicopter.md](../specs/ai-helicopter.md),
  [../specs/ai-pursuit-support.md](../specs/ai-pursuit-support.md).
- **Sources read for understanding (no code copied):** the CC0 reconstruction (dbalatoni13/nfsmw, GameCube build):
  `AI/Activities/AICopManager.cpp` (`CreateRoadBlock`, `UpdatePursuits`, `UpdateRoadBlocks`, `SpawnPursuitHelicopter`),
  `AI/Common/AIPursuit.cpp` (`RequestRoadBlock`, `AddRoadBlock`, `CopRequest`, the pursuit task), `AI/Common/AIRoadBlock.cpp`,
  `AI/Actions/AIActionStaticRoadBlock.cpp`, `AI/Common/AIVehicleHelicopter.cpp`, `Physics/Behaviors/SpikeStrip.cpp`.
- **Binary:** `speed.exe` (PC v1.3) in Ghidra, image base `0x400000`. Names below are ours.

## Checked in the PC binary

| What | Address | Result |
|---|---|---|
| Roadblock builder | `FUN_0043dd30` | Same procedure as the reconstruction. The nav starts at the target, pointing the way it faces, and advances `250.0`. The segment must exist, be a plain (non-decision) segment, take traffic, be at least `40.0` m long (immediate compared with the segment length) and have a profile with zones. `cr_width = |left - right| + 1`. Cars asked: `{2,2,3,3,4,4,5}[min(int(cr_width / 4), 6)] + 1`. The spike roll reads the level's `roadblockspikechance` (key `0x5a318af6`) against a 0 to 99 percent roll and is forced off when the pursuit status is 2 (cool-down). Fails when fewer than `asked - 1` cars are found. The layout pick, the centre (`anchor + across * min_width / 2 / crW`), the x stretch (clamped to `1.14`) and the element loop (at most six elements) match. Element kinds: 1 car, 2 sawhorse (`XO_Sawhorse_1b_00`, smackable key `0x9663ad06`), 3 spike strip (`XO_SpikeBelt_1b_DW_00`, key `0xca89ef8f`, plus the `Position` speech message). |
| Cop manager tick | `FUN_0043e8d0` | The latch: the pursuit's roadblock request (interface slot `+0x84`) is called only while nothing is latched, and a positive result stores the cop count and the pursuit (`this+0xb8`, `this+0xbc`). While a pursuit is latched the cop request (slot `+0x50`) is not asked. The builder is called every run for the latched pursuit when the pursuit's roadblock accessor (slot `+0x3c`) returns 0, with a `Created` message carrying 1 or 0. The latch is cleared when no latched pursuit has a roadblock to build. |
| Support executor | `FUN_0043e770` | The `HEAVY_ROADBLOCK` branch (strategy 4) calls the builder with count 4 and the request's car list and **treats a non-zero return as success** (state 2, timer = the option's duration, `Created` with 1). This settles the spec's open question about the return convention: success is true in both callers. |
| Helicopter spawn | `FUN_004269a0` | Already recorded in the helicopter spec: 250 m behind, +20 m, backward nav. |

## Not found in the PC binary

`RequestRoadBlock` (the 8 to 12 s timer and the two-step lottery), `CopRequest` (the helicopter's turn among the wave's
types and the search-mode roll) and `UpdateRoadBlocks` (the dodge test) were not located: the pursuit object's interface
vtable that the manager calls through could not be resolved (the label `PTR_FUN_008928ec` is string data, see
[ai-pursuit.md](ai-pursuit.md)), and the attribute accessors for `roadblockprobability` and the other level fields are
reached through a table, not by immediate hash (only the accessors of `SearchModeHeliSpawnChance` at `0x00403270` and
`MinimumSupportDelay` at `0x004036e0` have the hash as an immediate, and they have no recorded callers). Those rules are
therefore **[decomp]** from the reconstruction only, which agrees with the PC binary wherever the PC binary could be read.

## Minimap strings (Ghidra)

`MMICON_ROADBLOCK_%d` (`0x0089d938`) is referenced only by `FUN_00531350`, `MMICON_COPCAR_%d` by `FUN_00531060` and the minimap
constructor `FUN_0059d500`; the constructor also loads `HELICOPTER_ICON_GROUP` and `HELICOPTER_LINE_OF_SIGHT` and has no roadblock
icon. The first two functions build the full-screen map's option icons (`OPTION_ICON_`, via `FUN_00530df0`).

## Install data

`pursuitlevels` rows (heat 1 to 10) were compared cell by cell with the tables in code by
`installed_level_rows_match_the_roadblock_and_helicopter_tables`: roadblock probability, spike chance, search chance and **radius**,
`roadblockhelichance`, `SearchModeHeliSpawnChance`, `HeliFuelTime`, `TimeBetweenHeliActive`, `heliLOSdistance`. Only the radius disagreed
with the earlier spec (it is 1000 m only at heat 1 and 2).

The prop bounds of `XO_SpikeBelt_1b_DW_00` (one box, half extents 0.378 / 3.265 / 0.254 m in model axes, centre 0.013 m up) and
`XO_Sawhorse_1b_00` (four boxes) are in the track pack (`prop_bounds`), and both models (`XO_SPIKEBELT_1B_DW_00`,
`XO_SAWHORSE_1B_00`, the four `SAWHORSE_FRAG` pieces) are in the track's shared sections. Read by the installed test
`installed_roadblock_props_have_bounds_and_models` with `NFSMW_GAME_DIR` set.

## Known differences from the original

- A roadblock is built whole or not at all (the original never fails after the cars are found). The occupancy and on-road
  checks are the rewrite's.
- The cars of a heavy roadblock are ordinary cop cars of the SUV models, not pooled vehicles that were un-spawned first.
- "Cops needed to evade + 1" when a car is recruited has no effect: the rewrite's escape is a timer, not a count of cops.
- A sawhorse is a body in the prop world that sleeps until reached; it does not break into its four fragments (the
  smackable's event sequence is not run for runtime props).
