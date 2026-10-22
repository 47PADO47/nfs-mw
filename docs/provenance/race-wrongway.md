# Provenance: the wrong-way trigger

How [race-wrongway.md](../specs/race-wrongway.md) was read. Image base `0x00400000`; program `/speed.exe` (Black Edition PC
v1.3). Ghidra auto names (`FUN_*`, `DAT_*`) appear here only. Nothing was renamed, retyped or saved in the Ghidra project,
and no decompiled text or script bytecode is kept in the repository. The Lua method is the one of
[race-goals.md](race-goals.md): the `bytecode` blobs of the `gpcore` vault of `GLOBAL/gameplay.bin` were decompressed and
read by throwaway scripts outside the repository.

## The script (Lua chunks, file offsets of the compressed blobs in `gameplay.bin`)

| Handler | Offset | What it shows |
|---|---|---|
| `stategraph_basicrace_racing_handler_triggerenter` | 1,376,964 | the only chunk of the 253 that names `ResetsPlayer`: for a player element of a racer that is in the race, a trigger with `ResetsPlayer` runs `ChangeState('prerace')` offline and `WarpPlayerToTrigger(racestart)` online; every other element goes on to the goal test |
| `stategraph_basicrace_racing_handler_restartrace` | 1,401,324 | `SetActivelyRacing(false)`, then `ChangeState('prerace')`: the pause menu's restart enters the same state |
| `stategraph_basicrace_racing_handler_stateexit`, `_2`, `_3` | 1,426,488, 1,413,200, 1,401,560 | nothing, or kill the timers `killrace`, `fade`, `eblown`, `bloweng`, `backup` |
| `stategraph_basicrace_prerace_handler_stateenter` | 1,395,688 | the per-racer tables, `ClearCheckpoints` and `AddCheckpoint`, `WarpPlayerToTrigger(racestart)` unless `CameFromNIS`, `RestoreStartPositions`, `NotifyScriptWhenLoaded` |
| the per-race `prerace` `stateenter` variants (offsets 1,365,004 to 1,432,676) | | scripted cops, a phone call, cash bags, speed-trap icons, the barrier area and door; run on every entry of the state |
| `stategraph_basicrace_prerace_handler_loadingcomplete` | 1,404,468 | `RestoreStartPositions`, `StartRace(race)`, `SetActivelyRacing(true)`, then `countdown` (`DoCountdown`) or `racing` |
| `stategraph_basicrace_prerace_handler_stateexit_2` | 1,390,016 | a per-race effect on leaving the state (`ENISNukeSmack` at one fixed place near the start, the `16.1.0` explosion of [career-scripts-events.md](../specs/career-scripts-events.md)) |
| `stategraph_basicrace_countdown_handler_stateenter`, `_countdowndone` | 1,383,344, 1,385,488 | the audio flag, `ForceAIControl` for a rolling start, `SetAllStaging(true)` and the countdown screen, else `racing` |
| `stategraph_basicrace_racing_handler_stateenter` | 1,369,272 | `StartRaceTimers`, the post-race flags (`RaceOverMessageDuration` 3 s) |
| `stategraph_basicrace_intronis_handler_stateexit` | 1,429,780 | `SaveStartPositions`, `CameFromNIS = true` |

Neither `Directional` nor `Width` nor the collection names `wrongway` and `resetplayertrigger` occur in any chunk: the script
reads one field of the trigger, `ResetsPlayer`. No chunk has a `triggerenter` handler outside the `racing` state of the
`basicrace` graph (the other graphs with one are the pursuit zone, the engage triggers, the speed-trap monitors, the drive to
a target and the menu gates).

## The trigger system (`speed.exe`)

| Role | Address | Finding |
|---|---|---|
| Gameplay trigger constructor | `0x00601790` | reads `Rotation` (`0x5A6A57C6`), `Position` (`0x9F743A0E`), `Dimensions` (`0x6D9E21AD`), the sphere radius (`0x39BF8002`), `Width` (`0x5816C1FC`) and `OneShot` (`0xCE4261AC`). No `Directional` (`0x6B37E124`) or `ResetsPlayer` (`0x6CCD5819`) constant appears anywhere in the executable. With a radius: a sphere. Otherwise a box; with no `Dimensions`: extents `Width`, 50 and 1 stored as the across extent, twice the vertical extent and the depth; with `Dimensions (a, b, c)`: `b`, twice `c` and `a`. The bounding radius is the half diagonal of the horizontal footprint from those extents. Flags written: shape box, `0x4810D01`; bit `0x200` for `OneShot` |
| Trigger fire routine | `0x005F6A30` | argument 1 (enter): when the element is not in the trigger's inside list (`this + 0x7C`), add it and post `MTriggerEnter` (`0x008A3844`); always post the inside message. Argument 2 (exit): remove it; post the exit message only when `FireOnExit` (`0xB2AC32C7`) is set |
| Trigger manager update | `0x00781C90`, `0x00781840`, `0x00781A00` | per element, the triggers around it from the spatial grid, each tested once per update (a stamp on the trigger), then a second pass over the elements already inside to detect the exit |
| Enter/inside tests | `0x007669F0`, `0x00766CB0` | `0x00766CB0`, the box case: a sphere of the element's bounding radius moved along its velocity, against the box built from the stored extents (half sizes taken as stored/2 across and in depth, stored/2 of the doubled value vertically); before the test the box case rejects an element that moves against the trigger's normal when flag `0x80000` is set |
| Race status, `StartRace` | `0x0060DBD0` | the native start the script calls from `loadingcomplete` (barriers: [career-regions.md](../specs/career-regions.md) section 3.5) |
| `AddCheckpoint` | `0x005F5C20` | pushes onto the checkpoint vector at `race status + 0x19A8`; `ClearCheckpoints` is the matching clear |

## Data

- The `resetplayertrigger` family was listed with the repository's attribute reader through a throwaway program outside the
  repository: every `gameplay` collection whose `ResetsPlayer` resolves to True (167 collections: the template and 166
  triggers), each with its lineage, `Position`, `Rotation` and `Width`. All 166 inherit `resetplayertrigger`, none has a
  `Dimensions`.
- The distances of section 4 are measured in map coordinates between the trigger and the race's start, `Checkpoint` list
  and finish line (the route as straight lines); "behind the start" is the component along the start's heading.

## Dead ends

- The routine that creates the trigger object from the collection (the class's factory) and the layout of the trigger
  record were not read beyond the constructor, so which of the two horizontal extents is the thin one is not read.
- `Directional` was looked for by constant, by string and in the scripts; nothing reads it. It may be read through an
  in-layout accessor that does not use the key constant; the test's flag `0x80000` is the candidate.
- The online branch (`OnlineRace`) was read in the script only.
