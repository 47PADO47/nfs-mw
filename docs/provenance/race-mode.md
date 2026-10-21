# Race mode (`libs/blackbox-race`, `crates/nfsmw/src/scenes/world/race`, `ai/racer.rs`)

- **Specs:** [race-mode](../specs/race-mode.md), [ai-racers](../specs/ai-racers.md),
  [ai-racers-route](../specs/ai-racers-route.md), [ai-racers-catchup](../specs/ai-racers-catchup.md); data in
  [formats/gameplay-races.md](../formats/gameplay-races.md).
- **Restricted sources read:** `speed.exe` v1.3 in Ghidra (read only: no function was renamed, retyped or changed in the
  project), the install's `GLOBAL/gameplay.bin` through the Rust reader (**[verified]**), and the repository's
  existing specs. The decompiled class sources (`GRaceStatus.cpp` and friends) were **not** read in this pass:
  fetching them was refused by the environment, so nothing below is **[decomp]** unless a spec of this repository
  already carried it.
- **Evidence tags:** [docs/README.md](../README.md#evidence-tags). **[exe]** = read from the machine code or data of
  `speed.exe` (understanding only; no code or bytes are copied); **[verified]** = measured on the install.
- **Implemented:** 2026-10-20, from the specs, without the executable's code open.

## 1. How the executable was searched

1. `search_strings` for `Racer`, `Checkpoint`, `Countdown`, `Staging`, `Drag` and the `MMICON_` family.
2. The references to a string, found with a small inline Ghidra script (`ReferenceManager.getReferencesTo`), name the
   function that uses it. The inline scripts run as Java; Python scripts fail to compile in this server.
3. `get_functions` with `decompiled_code` for each function (only for reading; the output is not kept). The functions
   that Ghidra has not created (`LAB_` addresses) were read with `disassemble_bytes`.
4. The attribute-hash constants inside the functions were matched to field names by hashing candidate names with the
   repository's own `vlt_hash` (a throwaway test, not kept).

## 2. The script binding table, `FUN_0061e750` **[exe]**

One function registers every Lua-callable name with its native routine. The `GRaceStatus` group (the global race status,
`DAT_0091e000`) and the neighbouring game calls:

| Name | Routine | Name | Routine |
|---|---|---|---|
| `NotifyCheckpointReached` | `0x006117f0` | `NotifyLapFinished` | `0x00611850` |
| `NotifyRaceFinished` | `0x006119f0` (no function yet) | `NotifyRacePlacement` | `0x0060aa00` |
| `NotifySpeedTrapTriggered` | `0x0060a7b0` | `KnockoutRacer` and `TotalRacer` | `0x00611440` (the same routine) |
| `SetRacerGoal` | `0x00611700` | `SetRacerLapsLeft` | `0x00604db0` (no function) |
| `StartRace` | `0x0060dbd0` | `StartRaceTimers` | `0x00604ef0` (no function) |
| `SetAllStaging` | `0x0060df70` (no function) | `NotifyCountdownDone` | `0x0060e160` (no function) |
| `InitRacers` | `0x0061dbe0` (no function) | `CalculateRanking` | `0x006058b0` (no function) |
| `SetNextCheckpointPos` (method) | `0x005fe660` | `AwardBonusTime` (method) | `0x005fe600` |
| `AddCheckpoint` (method) | `0x005f5c20` (no function) | `ClearCheckpoints` | `0x005eeb40` |
| `AddRacer` | `0x00601f90` | `ClearRacers` | `0x005e30a0` |
| `EnterSuddenDeath`, `SetActivelyRacing`, `SetTaskTime`, `DisableBarriers`, `EnableBinBarriers`, `SetHasBeenWon`, `SetRoaming`, `SetIsLoading` | small routines `0x00604030` to `0x005dbf00` | `AbandonRace`, `JumpToCarLot`, `JumpToSafeHouse` | `0x0060deb0`, `0x00605250`, `0x006052b0` |
| `ShowRaceOverSummary` | `0x006120c0` | `EnterPostRaceFlow` | `0x00611f20` |
| `ShowWinningPostRaceScreen` / `ShowLosingPostRaceScreen` | `0x0060e240` / `0x0060e230` | `RestoreStartPositions` / `SaveStartPositions` | `0x0060aa50` / `0x00604e90` |
| `AllRacersDone`, `AllHumanPlayersDone`, `GetNumRacers`, `GetRacerIndex`, `GetRacerElement`, `GetRacerCharacter`, `RacerIsHuman` | `0x006051f0`, `0x0060e0a0`, `0x00604e00`, `0x0060dae0`, `0x0060db60`, `0x00604de0`, `0x00611e20` | `IsRaceCompleted`, `IsRaceUnlocked`, `UnlockRace`, `AddEngagedRace` | `0x006051b0`, `0x00605170`, `0x00612140`, `0x00604b50` |
| `IsActiveSpeedTrap`, `IsActiveMenuGate`, `EnterGateZone`, `ExitGateZone`, `EnterEngagableTrigger`, `ExitEngagableTrigger` | `0x00611e80`, `0x00604e40`, `0x00611fb0`, `0x00611ff0`, `0x00604f80`, `0x00604fd0` | `SetCopsEnabled`, `NoNewPursuitsOrCops`, `ForcePursuitStart`, `PreventPlayerBeingBusted` | `0x00604f40`, `0x0060aab0`, `0x0060aac0`, `0x0060ae00` |
| `SetPlayerStartPosition`, `WarpPlayerToTrigger`, `ResetTrigger`, `WarpToMarkerWhenRoaming`, `ShowTriggerIcon`, `HideTriggerIcon` | `0x00611670`, `0x00611570`, `0x00604d10`, `0x00605930`, `0x00604d20`, `0x00604d30` | `ForceAIControl` / `ClearAIControl` | `0x006123b0` / `0x00612420` |

The Lua state graphs call these; the bytecode was not decompiled (open item). The race type of a collection is
`EventIconType` looked up in the 11-entry table at `0x008f5cd8` (string pointer, type number) in this order:
`circuit` 1, `p2p` 0, `drag` 2, `knockout` 3, `tollbooth` 4, `speedtrap` 5, `cashgrab` 7, `checkpointrace` 6, `challenge`
8, `speedtrapjump` 9, `milestonejump` 10 (the strings sit at `0x008a3b0c` to `0x008a3b9c`) **[exe]**.

## 3. The race status object `DAT_0091e000` **[exe]**

Offsets inside the object (inferred from the routines above; names are this repository's):

| Offset | What |
|---|---|
| `+0x18 + n * 0x194` | racer record `n` (404 bytes); the racer count is at `+0x1958` |
| `+0x1960` | an event is underway (non-null while a race runs; the autosave reads it) |
| `+0x1964` | state flag; `1` is the idle value (`CalculateRanking` does nothing then) |
| `+0x1968` | the race parameters (`GRaceParameters`): `+4` a cached type byte (`+0x2b` of the record it points to), `+8` the attribute collection, `+0xc` and `+0x10` lazy-load stubs |
| `+0x1970` to `+0x1978` | the race clock: start time, elapsed share and a running flag |
| `+0x1980` | the bonus time total (`AwardBonusTime` adds to it) |
| `+0x1990`, `+0x1994` | the next-checkpoint marker objects (`SetNextCheckpointPos` moves them to the checkpoint) |
| `+0x19b8` | the next checkpoint's object |
| `+0x19bc ...` | per-racer, per-lap time tables (`NotifyLapFinished` fills `(racer * 16 + lap) * 4`) |
| `+0x1c3c ...` | per-racer, per-lap, per-checkpoint time table (`((lap * 16 + racer) * 16 + checkpoint) * 4`) |
| `+0x454c`, `+0x4550` | traffic density and traffic pattern hash set by `StartRace` |

Racer record fields used by the routines read **[exe]**: `+0x00` the vehicle key, `+0x04` an AI-control flag, `+0x10`
rank, `+0x14` rank among racers with a driver, `+0x18` percent complete (set to 100 by `NotifyRaceFinished`), `+0x1c`,
`+0x1d`, `+0x1e` the three eliminated flags (knocked out, totalled, busted), `+0x21` finished, `+0x28` laps completed,
`+0x2c` checkpoints reached this lap, `+0x120` points (the rank-by-points key), `+0x14c` to `+0x154` the finish time,
`+0x158` to `+0x160` the lap clock, `+0x164` to `+0x16c` the checkpoint clock, `+0x150` the stored finish time.

## 4. Functions read, with what each does (own words)

- `0x005fde80` (ranking): copies the racers into a pointer list, bubble sorts it with the comparison below or, when
  the race's bit-9 flag (`RankPlayersByPoints`, attribute `0x5EC1880F`) is set, by the points float (with a special case
  when the two points add up to at most 1), then writes each record's rank and the rank among racers that have a driver.
- `0x005e31d0` (the comparison): finished racers by finish time; a finished racer before an unfinished one; racers with
  an eliminated flag after the others (compared by the number `0x00565f20` returns, the time or distance of
  elimination); otherwise more laps first, then larger percent.
- `0x006117f0` (`NotifyCheckpointReached`): finds the racer, stores the checkpoint time and the lap clock in the tables,
  then calls `0x00600660`, which advances the racer's checkpoint counter only when the reported number is the counter
  plus one, restarts the checkpoint clock, and resets the intermediate flag.
- `0x00611850` (`NotifyLapFinished`): stores the lap time, sets the completed laps with `0x00602200`, re-ranks, and for a
  knockout race (type 3) applies the knockout rule of the spec section 7 using `0x005facd0` (the opponent count) and
  `0x005fc6e0` (attribute `0x181462DA`, `KnockoutsPerLap`). The knocked racers go through `0x00611440`.
- `0x006119f0` (`NotifyRaceFinished`, disassembled): sets the racer's percent to 100 (`0x42C80000`), re-ranks and, for a
  human racer that is not AI-controlled, takes the finish path.
- `0x0060dbd0` (`StartRace`): clears the race flag, starts the race timers, posts the `Game_StartRace` event
  (`0x00897200`, also posted from `0x004c2340`), reads the traffic density (`ForceTrafficDensity`, attribute
  `0x5E161BBA`; a drag race, type 2, sets it to 0), the traffic pattern name hash, sets the heat of every racer's car to at
  least `ForceHeatLevel` (attribute `0xE4211F4F`), turns the AI-control flag off for the racers and calls the helpers that
  place the racers.
- `0x005fe090` (the race clock): time left = `TimeLimit` (attribute `0x7585F041`, `0x005fc260`) + the bonus total
  (`+0x1980`) minus the elapsed time; floored at 0; 0 when the limit plus the bonus is not above 0.
- `0x005fe600` (`AwardBonusTime`): appends the current time left to each racer's bonus list and adds the seconds to the
  bonus total.
- `0x005fe660` (`SetNextCheckpointPos`): stores the checkpoint object and moves the two marker objects to its position;
  the markers are removed when the object is null or the race has no checkpoint markers.
- `0x00611700` (`SetRacerGoal`): gives a racer's AI a goal object: it asks the racer's vehicle for its AI interface,
  puts the goal's position and orientation into it, and, when the racer is the local player, moves the next-checkpoint marker.
- `0x0060a7b0` (`NotifySpeedTrapTriggered`): reads the speed (km/h), the stat `speedtrap_speed`, converts with the factor
  `0.27778` and stores the m/s value for the racer and the trap; for the human racer flashes `SPEEDTRAP_FLASHER_ICON` with
  the text `+%0.0f %s`, using one of two unit strings by the metric setting.
- `0x0060aa00` (`NotifyRacePlacement`): posts the event `0x20D60DBF` with the racer, rank and race.
- `0x005faa20` (race type): returns the cached type byte, or looks `EventIconType` up in the table of section 2.
- `0x0060dae0` (`GetRacerIndex`): the 1-based index of a vehicle in the racer list, -1 when it is not there.
- `0x005dc260` and `0x005dc2f0`: the two time table writers named in section 3.
- `0x005694e0` (HUD file by layout number): 2 `HUD_Drag.fng`, 3 `HUD_Player1.fng`, 4 `HUD_Player2.fng`, 5
  `HUD_Drag_Player1.fng`, 6 `HUD_Drag_Player2.fng`, any other `HUD_SingleRace.fng`.
- `0x0067f0d0` (vehicle behaviour names): for a human driver with the drag flag (`+0x144 == 1`) the input behaviour is
  `InputPlayerDrag`, else `InputPlayer`; the damage behaviour is `DamageDragster` for the drag flag; the engine
  behaviour is `EngineDragster` for the drag flag and one engine type; `SuspensionSimple` for the driver classes 3, 4
  and 6; `RBRemote` for class 6 (remote); `InputNIS` and `EngineSpline` for scripted cars.
- `0x00795290`: reads three comma separated lists of ten numbers, `CIRCUIT_TIER_POINTS`, `SPRINT_TIER_POINTS` and
  `DRAG_TIER_POINTS`, from a `conf` block into `DAT_009b3e20` (the online ranking's tier points).

## 5. Strings and their users **[exe]**

| String | Used by |
|---|---|
| `MMICON_AIRACER_%d` (`0x008a1f88`) | `0x0059d500` (the minimap's blip builder, at `0x0059d8b8`) |
| `MMICON_CHECKPOINT` (`0x008a0174`) | `0x0059d500` (at `0x0059d9bc`) |
| `MMICON_COPCAR_%d` (`0x0089d924`) | `0x0059d500` (minimap) and `0x00531060` (world map) |
| `MMICON_SPRINT_%d` (`0x008a0280`) | `0x0059d500` (minimap), `0x00530df0` (world map) and a table at `0x008f3ad4` |
| `MMICON_ROADBLOCK_%d` (`0x0089d938`) | `0x00531350` (world map only) |
| `MMICON_DRAG_%d`, `MMICON_SPEEDTRAP_%d`, the other race icons | no direct reference (built from the table at `0x008f3ad4`) |
| `HUD_Drag.fng` (`0x0089fbdc`), `HUD_Drag_Player1.fng` | `0x005694e0` |
| `EngineDragster`, `DamageDragster`, `InputPlayerDrag` | `0x0067f0d0` |
| `RACERNAME_%03d` | `0x005dc829` (the racer name lookup) |
| `scroll_racers_%d` | `0x00558b20`, `0x0055c450` (online lobby lists) |
| `SPEEDTRAP_FLASHER_ICON`, `+%0.0f %s`, `speedtrap_speed` | `0x0060a7b0` |
| `Checkpoint %d` (`0x008a38ec`) | no reference found |
| `RaceResultStat` | `0x00872c36` (inside a type descriptor table) |

## 6. Attribute names confirmed by hashing **[verified]**

`EventIconType` = `0x0F6BCDE1`, `KnockoutsPerLap` = `0x181462DA`, `TimeLimit` = `0x7585F041`, `RankPlayersByPoints` =
`0x5EC1880F`, `SpeedTrapList` = `0x822179D1`, `ForceTrafficDensity` = `0x5E161BBA`, `CashValue` = `0xD8BAA07B`,
`ForceHeatLevel` = `0xE4211F4F`.

## 7. Data findings that fix a rule **[verified]**

- A tollbooth race's `Checkpoint[]` list holds the `time_bonus_checkpoint*` triggers in line with the plain checkpoints;
  each carries `TimeBonus` as an `Int32` (25, 35 and 160 s in `2.4.3`).
- A speed trap is a `speedtrap` collection of class `radiustrigger` (a sphere): `Position`, `Radius` (20 to 30 m) and
  `ThreshholdSpeed` (88 km/h in `1.5.3`). It is not a box trigger.
- The stored `Dimensions` of a checkpoint are half sizes: the first checkpoint of `10.2.1` stands 22 m above the road it
  crosses with a stored height of 40, which a full-size box would miss, and the AI racers cross it as half sizes only.

## 8. Dead ends and what was not read

- The decompiled class sources (`GRaceStatus.cpp`, `GRace.h`, `AIVehicleRacecar.cpp`): fetch refused; the specs that
  carry them (`ai-driver-speed-skill.md`, `ai-driver-control.md`) were used instead.
- The Lua state graphs and their countdown timing; `InitRacers` (`0x0061dbe0`), `SetAllStaging` (`0x0060df70`),
  `NotifyCountdownDone`, `StartRaceTimers`, `CalculateRanking` and `AddCheckpoint` have no function in the project, so
  only their first instructions were seen.
- The inline script for string references printed old compile errors from other scripts in the user's
  `ghidra_scripts` folder; they are not from this work and were left alone.
- `MMICON_ROADBLOCK_%d` is **not** an object of `HUD_SingleRace.fng` (checked by hashing the name in the package), so
  the in-game minimap has no roadblock icon; the world map (`WorldMapMain.fng`) has `MMICON_ROADBLOCK_0` and `_1`.
- `HUD_Drag.fng` exists in the install, but none of the minimap race icon names is an object in it.

## 9. Countdown, start speed and HUD names **[exe]** **[install]**

- **`InitialPlayerSpeed`** (`vlt_hash` `0x3A0E4B19`) has one reader, `0x005fc4a0`, called only from `0x00600540`. That
  function fills a racer record from the vehicle's own state: the position, then the live speed (a virtual call that
  returns a float) into the same slot, and only when that speed is `0.0` it takes `InitialPlayerSpeed` instead. The
  attribute is therefore in the unit of the vehicle layer's speed, metres per second, and there is no conversion in
  between **[verified]** for the unit, **[inferred]** that the vehicle call is a speed in m/s (no scale factor is applied
  in this function). Other race attribute readers (`NumLaps`, `RollingStart`, `StartPercent`, `PlayerCarType`,
  `PlayerCarPerformance`, `PostRaceActivity`, `IsLoopingRace`, `RankPlayersByDistance`, `ThreshholdSpeed`, `StartTime`,
  `TOD`, `BustedLives`) sit in the same table of small getters at `0x005d9840` to `0x005da6e0`; `DoCountdown`,
  `RaceOverMessageDuration`, `TerminateRaceTime`, `SharedCheckpoints`, `TimeBonus`, `TokenValue` and `ResetsPlayer`
  have no constant in the executable, so they are read by name in the Lua graphs.
- **Countdown:** `StartCountdown` (used by `0x0044cc10`), `MCountdownDone` (`0x004b3a50`) and `NotifyCountdownDone`
  exist, but the cadence is in the Lua graphs. The FEng package `HUD_SingleRace.fng` carries the group
  `321_GO_GROUP` (fe-hash `0xA7B3AA16`) with the scripts `THREE`, `TWO` and `ONE` (1200 ms each: fade in 0 to 100 ms,
  hold, fade out 1100 to 1200 ms) and `GO` (3200 ms). Three number scripts started a second apart give the 3 s
  countdown **[inferred]**; `COUNTDOWN_SECS = 3.0` stays, now with this evidence.
- **Gate half sizes:** see section 7; no executable reader of the dimensions was found, the evidence is the install.
- **HUD objects** (fe-hash matches in `HUD_SingleRace.fng`): `321_GO_GROUP`, `POSITION_GROUP` (`0x0FC5B7A0`, two
  strings: place and racer count), `TIMER_ICON`, `CurrentLapTime` (`0x30D9802E`), the race information group
  (`0xB1C5ED0D`) with the lap group (`0x1A091045`, current and total lap strings) and the progress group
  (`0xE3DDCD09`, a percentage and `COMPLETE`). `TOLL_BOOTH_GROUP`, `SPRINT_GROUP`, `SPEEDTRAP_GROUP`, `RIVAL_GROUP`
  and the `*Stat` names are executable strings that are not objects in this package.
- **Post-race screens:** `PostRace_Results.fng` (`0x00642140` case 0, `0x00549460`, `0x0057ca60`),
  `PostRace_Pursuit.fng` (case 1) and `PostRace_MilestoneRewards.fng` (`0x00641fe0`) are pushed by name through
  `0x005257f0`; `EnterPostRaceFlow` and `ShowRaceOverSummary` (`0x006120c0`) are Lua-callable.

## 10. The post-race screens **[exe]** **[install]**

- **Packages** (`GLOBAL/InGameB.bun`): `PostRace_Results.fng` (149 objects), `PostRace_Pursuit.fng` (95),
  `PostRace_MilestoneRewards.fng` (51). `0x00642140` pushes the first or the second by its argument (0 or 1);
  `0x00641fe0` (`EShowMilestones`) pushes the third; both go through `0x005257f0`, which pushes a screen by name.
- **`PostRace_Results.fng`**: a title (`RACE RESULTS`), the racer's name between two arrows (`PLAYER`, with the shoulder
  glyphs `L1.tga` and `R1.tga`), a header (`Lap`, `Pos.`, `Time`) and 10 rows of three strings (the row `n` strings are
  the name hashes `0x38B136A7 + n`, `0x85AB7368 + n` and `0xD2A5B029 + n` for 1 to 9, and `0x4ED80BD8`, `0x3B19E0B9`,
  `0x275BB59A` for the tenth). The footer has Exit (Enter, `Cross.tga`), Restart (key 1, `Circle.tga`) and Stats (key 2,
  `Square.tga`). The package answers these to the game: `PAD_ACCEPT` (`0x406415E3`), `PAD_BUTTON0` (`0xC519BFBF`),
  `PAD_BUTTON4` and `PAD_BUTTON5`, `PAD_LTRIGGER` (`0x5073EF13`) and `PAD_RTRIGGER` (`0xD9FEEC59`); the roles of Exit,
  Restart and the arrows follow the glyphs and the labels, not a read of the screen class **[unconfirmed]**. What the
  columns count (a lap table of one racer, the arrows switching racer) follows the headers **[unconfirmed]**.
- **`PostRace_MilestoneRewards.fng`**: a milestone line (`0xE526D0D2`), a bounty line (`0xE1045A4F`), the total
  (`0x324F7792`), a big icon (`0xFCC8A`) whose script `0x5079C8F8` posts the messages that bring the lines in, and a row
  of ten small icons by milestone type. Its Continue button leaves through the package's own script, which ends with
  `EXIT_COMPLETE` to the game.
- **The flow** is the Lua graph's (`docs/specs/career-scripts-events.md` section 1.1): `raceover` (3 s), `postrace`
  (the native call that opens the result and reward screens), then `restartrace`, `enterfreeroam` or `flowreadyforoutro`.
  The strings `EnterPostRaceFlow` (`0x0061EA30`) and `ShowRaceOverSummary` (`0x006120C0`) are the native entries.
- **Not read**: the screen classes behind these (their handlers were not decompiled), the outro's cutscene call, and
  `PostRace_Pursuit.fng` (95 objects: the pursuit statistics, read by `0x00642140` case 1).
- **`InGamePhotoMaster.fng`** (91 objects): the groups the executable's race code names, `SPRINT_GROUP` (`0xE05794EB`),
  `RIVAL_GROUP` (`0x895E5289`), `TOLL_BOOTH_GROUP` (`0xC59184C1`) and `SPEEDTRAP_GROUP` (`0xFBE65EB3`), are objects of this
  package and of no other (`hud/install_tests.rs`). Each rests hidden and holds `WINNER`, a cash line and a time or speed
  line (the speed trap's: `CLOCKED IN AT:`, the speed and `BOUNTY: n`, with the camera frame `ROCKPORT POLICE VID CAM 01`).
  Its footer has Continue (accept), Race Stats (`PAD_BUTTON4`) and Restart (`PAD_BUTTON5`), with the PC keys Enter, 2 and
  1 drawn by the package. The Lua graph's `ShowWinningPostRaceScreen` hook is this screen **[inferred]** from the names.
  The rests-hidden groups are held at full alpha by the screen logic: their own fade script leaves the group's colour at
  0 (probed in a run of the package alone). The package's picture object (`photofinish_toobooth.tga`, 1024 x 512) is a
  polaroid with a transparent window; the game swaps its texture for the race's backdrop and shows the live world through
  the window from a replay camera. The reading is in [../specs/race-finish-effects.md](../specs/race-finish-effects.md)
  (provenance [race-finish-effects.md](race-finish-effects.md)).
- **`PostRace_Pursuit.fng`** (95 objects): rows of a label (`0x38B136A8 + n`) and a value (`0x85AB7369 + n`) reading
  `00:00.00 / 00:00.00`: Length, Escape Within, Cops Damaged, Cops Destroyed, Roadblocks Dodged, Cost to State
  Achieved, Unserved Infractions, Consecutive Pursuits Evaded, Unserved Infractions. The heading says `INFRACTIONS` (the
  package serves the infractions page too). The font has no `$` glyph (a string with one draws without its text).
- **`GameOver.fng`** (FrontB, 44 objects) is the career's game over (`All of your vehicles have been impounded...`), not a
  race's losing screen. **No other package** of the install is a race loss screen (the list of the 197 packages was read);
  the loss takes `PostRace_Results.fng`, as the Lua graph's stubbed loss hook leaves it.
- **The drag tachometer** (`0x0057C000`, called with the layout number): `DRAG_RPM_<n>_LINES` for the face and, by `MAX_RPM`
  band and red line, the mask angle: for a band below 7000: 39.25, 32.25, 26 and 19.25 from the red lines 6500, 6000,
  5500 and below; 7000 to 8000: 40, 34, 28.25, 22.5 and 17.25 (7500, 7000, 6500, 6000, below); 8000 to 9000: 42, 37, 32.25,
  27 and 22 (8500, 8000, 7500, 7000, below); 9000 and over: 41.5, 37, 31.5, 27 and 22.75 (9500, 9000, 8500, 8000, below).
  The race HUD's branch of the same function gives the angles `redline_rotation` has. The needle's update is a callback the
  decompiler does not resolve (the class at `0x008A261C` has a destructor and property setters only), and the constants
  `228.0` and `66.0` are in no function between `0x580000` and `0x5B0000`, so the sweep is still the race HUD's
  **[unconfirmed]**.
- **Chains and outros** (install, `nfsmw-data` test `post_race_activities_and_outros_are_read`): 29 races set
  `PostRaceActivity` (the five prologue races, and one or two races of each bin: `1_7_3_drag` goes to `4_2_1_sprint`, the
  last one of bin 20 names no race that exists); exactly one race sets `OutroNIS`, `16_nis_rog_razor` with `EndingNis03`.
  The NIS bundles are `NIS/Scene_*` (87 files: 26 arrests, `EndingNis03` and `EndingNis04`, 27 intros, replays, world
  map and FMV scenes). No code of the rewrite plays one.
