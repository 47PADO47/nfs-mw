# Race mode: start, checkpoints, laps, finish, ranking and results

How a race runs once it has been chosen: the state of one race, what a checkpoint and a lap do, how the racers
are ranked, what ends a race for each racer and what the result is. The stored race data is in
[formats/gameplay-races.md](../formats/gameplay-races.md); the opponents are in [ai-racers.md](ai-racers.md),
[ai-racers-route.md](ai-racers-route.md) and [ai-racers-catchup.md](ai-racers-catchup.md); the career side of a
result is in [career-blacklist.md](career-blacklist.md) and [career-economy-flows.md](career-economy-flows.md).
The Rust rules are in `libs/blackbox-race`; the world wiring is in `crates/nfsmw/src/scenes/world/race/`.

**Sources read.** `speed.exe` v1.3 through Ghidra (machine code only; nothing is copied), listed with addresses in
[provenance/race-mode.md](../provenance/race-mode.md), the existing specs of this repository
([career-scripts-events.md](career-scripts-events.md) section 1 describes what the Lua states do), and the install's data. The decompiled
class sources (`GRaceStatus`) could not be fetched in this pass, so every rule below is either read from the
executable (**[exe]**), measured on the install's data (**[verified]**), taken from an existing spec of this
repository, or a choice of this rewrite (**[unconfirmed]**). Evidence tags as in
[docs/README.md](../README.md#evidence-tags).

## 1. The pieces

The game keeps one global race status (`GRaceStatus`) with a list of racers (each a record of about 404 bytes), the
active race's parameters (`GRaceParameters`, a view of one `gameplay` collection) and the race timer state
**[exe]**. The Lua state graphs of the `gpcore` vault drive it through these calls **[exe]** (names are strings in
the executable): `InitRacers`, `ClearRacers`, `AddRacer`, `SetAllStaging`, `StartRace`, `StartRaceTimers`,
`NotifyCountdownDone`, `AddCheckpoint`, `ClearCheckpoints`, `SetNextCheckpointPos`, `NotifyCheckpointReached`,
`NotifyLapFinished`, `NotifyRaceFinished`, `NotifyRacePlacement`, `NotifySpeedTrapTriggered`, `KnockoutRacer`,
`TotalRacer`, `SetRacerGoal`, `SetRacerLapsLeft`, `AwardBonusTime`, `EnterSuddenDeath`, `CalculateRanking`,
`AllRacersDone`, `AllHumanPlayersDone`, `ShowRaceOverSummary`, `EnterPostRaceFlow`, `ShowWinningPostRaceScreen`,
`ShowLosingPostRaceScreen`, `AbandonRace`, `RestoreStartPositions`, `SaveStartPositions`.

The rewrite does not run the Lua graphs (their bytecode is not decompiled). It reproduces what the calls do as a
small state machine, section 2, and the rest of this page.

## 2. The race phases

| Phase | What happens | Evidence |
|---|---|---|
| Idle | free roam; the race zone offers its card | [world-markers.md](world-markers.md) |
| Staging | `SetAllStaging(true)`: every racer is on the grid, held; the AI cars creep at 2.5 to 3.0 mph ([ai-driver-speed-skill.md](ai-driver-speed-skill.md) section 1) | **[exe]** call, **[decomp]** speed |
| Countdown | only when `DoCountdown` is True (217 of 255 races); the 3, 2, 1, GO board; the cars stay held | **[verified]** flag |
| Racing | `StartRace` and `StartRaceTimers`; staging off; the lap and race timers run | **[exe]** calls |
| Finished | the player has finished or is out; the post-race flow runs while the others finish | **[exe]** |
| Post-race | the results card and the career glue; the world returns to free roam | **[exe]** call names |

- **Rolling start** (`RollingStart`, 38 races: bosses, challenges): no staging and no countdown. Every racer is
  placed on the grid already moving at `InitialPlayerSpeed` (40 or 60) **[verified]** field values; the unit is
  assumed to be m/s **[unconfirmed]**.
- **Countdown length** is not in the executable data read here (the board's timing is in the HUD package); this
  rewrite uses 3 s, one second per number **[unconfirmed]**.
- **Grid.** The player stands at the race's `racestart` marker facing its heading. Opponents stand in two columns
  behind and beside it, [ai-racers-route.md](ai-racers-route.md) section 3.

## 3. Checkpoints and laps

A race has an ordered list of checkpoints (`Checkpoint[]`, box triggers) and a finish line (`racefinish`).

- **Reached in order only.** The racer keeps a counter of the checkpoints it has reached this lap. Reporting
  checkpoint `n` counts only when `n` equals the counter plus one; any other report does nothing to the counter
  **[exe]** (`NotifyCheckpointReached` stores the time and sets the counter only for the next index). Checkpoints are
  numbered from 1.
- **A report also stamps the checkpoint time** of that racer (per racer, per lap, per checkpoint) and restarts the
  racer's split timer **[exe]**.
- **Laps.** A circuit-type race (`IsLoopingRace`, `NumLaps` 2 to 10) runs its checkpoints again each lap and
  reports a lap finished when the racer crosses the finish line after the last checkpoint. `NotifyLapFinished`
  sets the racer's completed-laps number to the lap just finished, resets its checkpoint counter to 0, stamps the lap
  time, and re-ranks everybody **[exe]**. A sprint (`NumLaps 1`) has one lap, and the finish ends the race.
- **Finish.** After the last lap, `NotifyRaceFinished(racer)` sets the racer's progress to 100 percent, ranks the
  racers again, and, for a human racer, starts the end of the race **[exe]**.
- **The trigger test.** The rewrite crosses a gate when the segment a racer moves along in one tick touches the
  checkpoint's box, and the checkpoint is the next one expected **[unconfirmed]**. The stored `Dimensions` are taken as
  **half sizes**: the first extent along the heading, the second across, the third up (the evidence is in
  [world-markers.md](world-markers.md) section 3). Races that leave out `finish` end when the last checkpoint is
  reached.
- **What the rewrite must accept.** A sprint's last checkpoint is often at the finish line; a gate the car skips
  (fast cars, 60 Hz steps) is caught by testing the segment between two positions against the box, not just the
  point **[unconfirmed]**.

## 4. Progress and ranking

Each racer has a percent complete (0 to 100, the first float of its record; 100 on finish) and a completed-laps
number **[exe]**. After every checkpoint, lap or finish the status ranks all racers by this order **[exe]**
(`FUN_005e31d0`, the less-than used by the bubble sort):

1. A racer that has finished ranks ahead of one that has not. Two finished racers: the smaller finish time first.
2. A racer that is out of the race (knocked out, totalled, or busted: three flags in the record) ranks behind
   every racer still racing. Among those the order is by the time or distance they were eliminated at.
3. Otherwise more completed laps first, then larger progress first.

If `RankPlayersByPoints` is on (speed-trap races), racers are ordered by their points, larger first
**[exe]** (the second comparison branch reads a float that the speed-trap notification adds to); a tie falls back
to the order above.

Each racer's record then gets its rank (1-based) and a second number: its rank among racers that have a driver
(the first is rank minus the racers without one) **[exe]**; the rewrite has no racer without a car, so both are
the same.

**Progress.** The distance along the route is measured against the route length (`RaceLength` or the length the
route builder returns, [ai-racers-route.md](ai-racers-route.md) section 2); percent is
`100 * (lap + along_lap) / laps` for a circuit and `100 * along` for a sprint **[unconfirmed]**, with
`StartPercent` as the HUD offset when it is set (the D-Day race).

## 5. Timers and the time limit

- The race timer starts at `StartRaceTimers` (the end of the countdown, or the start of a rolling race).
- **Time left** is `max(0, TimeLimit + bonus_total - elapsed)`, and it is 0 (no limit) when `TimeLimit + bonus_total`
  is not above 0 **[exe]** (`FUN_005fe090`). `AwardBonusTime(seconds)` adds `seconds` to `bonus_total` and, for
  every racer, appends the current time left to that racer's bonus list **[exe]** (the checkpoints with extra time).
- 181 races have no limit; the tollbooth races use 40 to 70 s and a speed-trap race uses 30 to 60 s
  ([formats/gameplay-races.md](../formats/gameplay-races.md) section 3.2).
- **Tollbooth bonuses.** The `Checkpoint[]` of a tollbooth race holds `time_bonus_checkpoint` triggers among the plain
  ones; each has a `TimeBonus` (an `Int32`: 25, 35 and 160 s in `2.4.3`) that the player's clock gains when it is
  reached, through `AwardBonusTime` ([career-scripts-events.md](career-scripts-events.md) section 1.2) **[verified]**.
- **Time out.** When the time left reaches 0 in a race with a limit, the player has lost the race (the rank list
  puts everyone who has not finished behind): time expiry ends the race like a knock-out or a destroyed car
  ([career-scripts-events.md](career-scripts-events.md) section 1.2).

## 6. The race types

| Type | Rule beyond the common ones |
|---|---|
| Sprint (`p2p`, 0) | one lap; finish ends the race |
| Circuit (`circuit`, 1) | `NumLaps` laps, the checkpoints repeat; the finish line counts the lap |
| Drag (`drag`, 2) | [section 8](#8-drag-races) |
| Knockout (`knockout`, 3) | circuit laps; at the end of each lap the last racer is knocked out, [section 7](#7-lap-knockout) |
| Tollbooth (`tollbooth`, 4) | a sprint against the clock with time bonuses; opponents none or few |
| Speed trap (`speedtrap`, 5) | ranked by points, [section 9](#9-speed-traps) |
| Challenge (`challenge`, 8) | pursuit challenge: rolling start, cops on; its own result flow ([career-blacklist.md](career-blacklist.md) section 6) |
| `checkpointrace`, `cashgrab`, jump types | in no race of this install; not implemented |

## 7. Lap knockout

On every lap finish the status does this **[exe]** (`FUN_00611850`, for a race of type 3): count the racers whose
completed laps are at least the lap just finished. If that count equals
`(opponent_count + 1) - KnockoutsPerLap * lap` (the number of racers still allowed after this lap), every racer
that has not completed the lap, and is not a human racer, is knocked out. (`opponent_count` is the length of the
race's `Opponents`; the player makes the `+ 1`.) With 4 racers and `KnockoutsPerLap 1` the last of the 4 is out
when 3 have crossed the line at the end of lap 1, the last of the remaining 3 at the end of lap 2, and so on.
`KnockoutRacer(car)` totals the car's progress, takes it out of the ranking, and the car stops racing (its goal is
cleared). A human racer who falls behind is not knocked out by this rule **[exe]** (the check skips human
racers); the post-race flow decides his loss when the race ends **[unconfirmed]**.

## 8. Drag races

11 races (`TrafficPattern "drag"`, `EventIconType drag`, a sprint-shaped route on one straight road).

- **Staging and start.** The countdown is the same board as other races, and the cars are held until GO. The player
  car has a drag steering helper: its steering is taken by the autopilot controller along the drag lane (target =
  nav position) unless the car is under 1 m/s, the game-breaker is active, or the car faces the wrong way; the
  left and right inputs only change lane ([ai-driver-control.md](ai-driver-control.md) section 6) **[decomp]**.
- **Drag tuning** of the controller and the AI opponents: the PID set of [ai-driver-control-pid.md](ai-driver-control-pid.md)
  (human drag 0.6 to 1.2 / 0.05 to 0.5 / 0.02 to 0.6), the look-ahead `[8, 40]` for a human and `[30, 100]` for AI,
  the drag speed table `[0.85, 1.0]` and the acceleration table `[0.82, 1.0]` of
  [ai-driver-speed-skill.md](ai-driver-speed-skill.md) **[decomp]**.
- **HUD.** The layout number 2 selects `HUD_Drag.fng` (3 and 4 are the split-screen `HUD_Player1/2.fng`, 5 and 6 the drag
  split-screen files, anything else `HUD_SingleRace.fng`) **[exe]** (`0x005694e0`); the drag HUD has a drag tachometer
  (`Hud_DragTachometer`, `DRAG_RPM_%d_LINES`) and turbo lines **[exe]** strings. The package objects are not bound
  yet (`HUD_Drag.fng` is loaded in a drag race only through the same readout); see section 10.
- **Vehicle behaviours.** The behaviour chosen for a car depends on its driver class and its drag flag: a human
  driver with the drag flag gets `InputPlayerDrag` (otherwise `InputPlayer`), the damage behaviour is `DamageDragster`
  and the engine `EngineDragster` for the drag flag **[exe]** (`0x0067f0d0`). What the dragster behaviours change in
  the physics is not read; a drag car in the rewrite uses the stock engine, damage and transmission.
- **Traffic.** `TrafficPattern "drag"` selects a pattern with no cross traffic. The rewrite turns normal traffic
  off for the drag race and restores it after **[unconfirmed]**.
- **Result.** First across the line wins, as a sprint. The elapsed time and the speed at the line are shown.

## 9. Speed traps

A speed-trap race (31 races) has a `SpeedTrapList` of 4 to 10 triggers **[verified]**, ranks by points
(`RankPlayersByPoints`), and has a time limit with overtime penalty `OvertimePenaltyPerSec 10`.

- A speed trap is a **sphere** (class `radiustrigger`: `Position`, `Radius` 20 to 30 m) with a `ThreshholdSpeed` in km/h
  (88 in `1.5.3`), not a box **[verified]**.
- A speed trap fires when the racer enters its sphere. `NotifySpeedTrapTriggered(racer, trap, speed_kmh)` converts
  the speed to m/s with the factor 0.27778, stores it in the racer's slot for that trap (the trap's index in the
  `SpeedTrapList`), and, for the human racer, flashes `+%0.0f` with the unit on the HUD **[exe]**.
- A trap pays only when the speed beats both its `ThreshholdSpeed` and the racer's previous best at that trap
  ([career-scripts-events.md](career-scripts-events.md) section 1.3).
- A racer's points are the sum of its trap speeds; the largest sum ranks first. Points are in the unit the HUD
  prints (km/h or mph) **[unconfirmed]**.
- Going over the time limit costs `OvertimePenaltyPerSec` points per second **[unconfirmed]**.
- The career's per-bin speed traps (the 39 milestone traps) are a different thing: they are free-roam
  challenges ([career-milestones.md](career-milestones.md)).

## 10. What the player sees (rewrite)

The race HUD is the package's own (`hud/race.rs`, `HudState::race`) **[exe]** names and **[install]** objects
(`docs/provenance/race-mode.md` section 9):

- `POSITION_GROUP` shows the place and the number of racers (two strings), `CurrentLapTime` the clock (`mm:ss.cc`: the
  time left under a time limit, else the time since GO **[unconfirmed]** for which one the original shows), the lap group
  the lap now and the laps in all for a lapped race, the progress group (`100% COMPLETE`) the share of the gates done
  for a sprint **[unconfirmed]** (which race kinds use it).
- `321_GO_GROUP` plays its `THREE`, `TWO`, `ONE` and `GO` scripts when the countdown step changes; its two strings take
  the digit or `Go!`.
- Not bound, because the package has no object of that name: `TOLL_BOOTH_GROUP`, `SPRINT_GROUP`, `SPEEDTRAP_GROUP`,
  `RIVAL_GROUP`, the `*Stat` strings and the generic message item (`Hud_GenericMessage` is a message-queue item of the
  executable, not an FEng object). The checkpoint, lap, bonus and speed trap lines, the points of a points race and the
  result stay on the egui race card (`hud/race_card.rs`).

## 11. Results

At the end of the race the player's placing (1 for first) and the time go to the post-race flow. In career mode the
result is applied to the Blacklist ([career-blacklist.md](career-blacklist.md) section 4.2): the event id is the
race's name without the vault prefix (for example `1.5.2`), the placing is the player's final rank (a player who is
out of the race, or whose time ran out, counts as last), and cash is paid on a first place
([career-economy-flows.md](career-economy-flows.md) section 7). Quick race has no career effect.
A won boss race in career opens the next step of the bin; the bounty and SMS glue is in the career crate.

## 12. Abandoning and restarting

`AbandonRace` ends the race for the player without a result; `RestoreStartPositions` puts every racer back on
the grid for a restart **[exe]** names. The rewrite supports `race stop` (abandon, no result) and `race restart`.
The player's reset key keeps working in a race.

## 13. What the rewrite runs (status)

| Part | Where |
|---|---|
| Gates, course, laps, ranking, the clock, knockout, speed-trap points, tollbooth bonus, catch-up | `libs/blackbox-race` (pure, tested) |
| The race parameters and the opponents' characters from the install | `crates/nfsmw-data/src/gameplay/params.rs` |
| Starting a race (from a zone, `race start`, or a quick race), the grid, loading the opponents' cars | `scenes/world/race/start.rs`, `load.rs` |
| The running race: positions, events, goals, catch-up, far racers | `race/flow.rs`, `emulate.rs` |
| The drag race's steering assist and lanes | `race/drag.rs`, `drive/grid.rs` |
| The result, the career glue and the clean-up | `race/finish.rs`, `career/events.rs` (`career_race_finished`) |
| The card (place, lap, clocks, banner, result) | `hud/race_card.rs` |
| The minimap blips (next checkpoint, racers, speed traps, race starts) | `race/blips.rs`, `hud.rs` |
| The console | `race list|start|status|stop`; `race start <race> demo` puts the player's car at each gate in turn (a test aid, not original behaviour), so a whole race, its result card and its career glue can be run offscreen. The install test `every_install_course_can_be_finished_by_walking_through_its_gates` walks all 212 courses (laps included) through the same rules |
| Quick Race | `frontend/quick_race.rs`, `track_select.rs`, `track_options.rs` |

Differences from the original that the rules above call out: the countdown is 3 s; the lines above that have no package object are an egui card; the
post-race flow is a result card, with no reward screens, no outro and no retry choice; a player who is busted in a race
is not handled (the pursuit's own bust flow runs); `ResetsPlayer` (wrong way) triggers, `SharedCheckpoints`,
`TokenValue` (cash grab), `Shortcuts` and `BarrierExemptions` are not used; the sudden-death, cash-grab, checkpoint-race
and jump types do not occur in this install.

## Open questions

1. The countdown board's timing and the exact post-race flow (the Lua graphs).
2. The unit of `InitialPlayerSpeed` and the amount of a tollbooth time bonus.
3. The exact progress formula (`+0x18` is written by the AI routing code, not read here).
4. Whether the player's loss on a knockout is decided at the end of the race.
5. The trigger test's axes ([formats/gameplay-races.md](../formats/gameplay-races.md) open question 1).
