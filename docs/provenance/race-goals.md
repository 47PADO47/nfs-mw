# Provenance: race goals

How [race-goals.md](../specs/race-goals.md) was read, and the region-gate facts of
[career-regions.md](../specs/career-regions.md) section 3.5. Image base `0x00400000`; program `/speed.exe` (Black Edition
PC v1.3). Ghidra auto names (`FUN_*`) appear here only. Nothing was renamed, retyped or saved in the Ghidra project, and
no decompiled text or script bytecode is kept in the repository.

## Method

1. **The scripts.** The 259 `bytecode` blobs of the `gpcore` vault in `GLOBAL/gameplay.bin` are Lua 5.0 chunks
   ([formats/attributes.md](../formats/attributes.md), "Blobs: Lua bytecode"): 253 are `HUFF`, 5 `JDLZ`. A throwaway
   Python script outside the repository (a port of the documented HUFF stream in [formats/huff.md](../formats/huff.md),
   about 70 lines) decompressed the 253 `HUFF` blobs, found by their `HUFF\x01\x10\x00\x00` header, and a second
   throwaway script read the Lua 5.0 chunk format and listed each function's constants and instructions. Neither script
   nor its output is kept.
2. **Lua 5.0 layout, as it was found here.** The 32-bit instruction has the opcode in the low 6 bits, then `C` (9 bits),
   `B` (9 bits) and `A` (8 bits) at the top; `Bx` is the 18 bits from bit 6; a constant operand is a value of 250 or
   more (index minus 250). The opcode order is the one of Lua 5.0 (`MOVE` first, `CLOSURE` last). Float constants are
   4 bytes. A function is: source name, line, upvalue count, parameter count, vararg flag, stack size, line table, locals,
   upvalue names, constants, nested functions, code.
3. **Which chunk is which.** Each chunk sets one global named `stategraph_<graph>_<state>_handler_<event>` (a variant
   number is appended to the name when several collections define the same handler). The chunks read for this page, with
   the file offset of their compressed blob in `gameplay.bin`:

   | Handler | Offset | What it shows |
   |---|---|---|
   | `stategraph_basicrace_racing_handler_triggerenter` | 1,376,964 | the goal test, the checkpoint call, the advance of `NextGoal`, the finish line, the laps, the end of a racer |
   | `stategraph_basicrace_prerace_handler_stateenter` | 1,395,688 | the per-racer tables (`GoalList`, `GoalCount`, `NextGoal`, `LapsLeft`, `InRace`) and the native checkpoint list |
   | `stategraph_basicrace_done_handler_stateenter` | 1,371,396 | `DisableBarriers` and `EnableBinBarriers`, the only chunk of the 253 that names either |
   | the other `racing` `triggerenter` variants (offsets 1,358,144 to 1,430,380) | | scripted triggers of single races: cop spawns, traffic, phone calls, speed traps, cash bags, the epic win and lose triggers; none touches the goal list |
   | `stategraph_basicrace_racing_handler_notifytimer` | 1,387,076 | the `killrace` timer after a race: fade, `ChangeState('postrace')` |

4. **The native side.** `NotifyCheckpointReached` `FUN_006117f0`, `FUN_00600660` and `NotifyLapFinished` `FUN_00611850`
   are described in [race-mode.md](race-mode.md)'s provenance. `AddCheckpoint` (`0x005F5C20`, no function created) pushes
   its argument onto a vector at `race status + 0x19A8` and does nothing else; the script is what fills it (every entry
   of `Checkpoint`, then `racefinish`).
5. **Data.** The races, triggers and `Checkpoint`/`racefinish` fields of `16_1_0_partial_dday` and `16_2_2_sprint`
   were read with the repository's reader through a throwaway program outside the repository; the distances of
   [race-goals.md](../specs/race-goals.md) section 3 are measured between trigger centres in physics space. The same
   reader and a copy of the rewrite's rules produced the walk through the gates of both courses.

## Region gates and races (career-regions.md section 3.5)

| Routine | Address | Finding |
|---|---|---|
| `StartRace` `FUN_0060DBD0` | `0x0060DBD0` | at `0x0060DCF2` to `0x0060DD09`: event id text of the race (`FUN_005FBA70`) compared with `e3demo` (`0x008A06FC`) by the byte-wise compare `FUN_00460D20`; `FUN_00659660` (clear all barriers) only when equal; then `FUN_005FE330` (enable the race's list) |
| `FUN_00659660` | `0x00659660` | callers: `FUN_00602BB0` (set current bin) and `StartRace` only |
| `FUN_005FBA70` | `0x005FBA70` | returns the race's text field `0xA78403EC` (the event id, `19.8.18`) or the cached copy at `+4` |
| race status constructor `FUN_00603720` | `0x00603720` | applies the stage's bin list (`FUN_00602BB0` then `FUN_005FE3E0`) only when `[DAT_0091E004 + 0x2C] == 0`; with an event current the world starts with no region gate |
| race status reset `FUN_00603500` | `0x00603500` | enables the stored bin's list again only when that same slot is 0 |
| race status update `FUN_00602C80` | `0x00602C80` | the stage re-evaluation runs only in the roaming state (`+0x1960 == 0`) |

The straight-line check of the routes against the gates (an in-memory program outside the repository) used the six
barrier lines of [career-regions.md](../specs/career-regions.md) section 1.3 and, for each race, the start, the
`Checkpoint` list and the finish line in map coordinates `(z, -x)` of the physics positions, with the gates a race's bin
closes (stages 13 and above: 11 to 15; 9 to 12: 10, 12, 13). Only `16_2_2_sprint` crosses a line.

## Dead ends

- The routine that sets the gameplay manager's current event (`+0x2C`) before the world is built was not followed, so
  that the front end's hand-over of a career race sets it is inferred from the constructor's test and from the data.
- The trigger system's own enter test (point or swept, edge or level) was not read; the script only receives the event.
- The five `JDLZ` Lua chunks were not read.
