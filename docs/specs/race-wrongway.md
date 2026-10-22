# The wrong-way trigger: back to the grid

Almost every race has a `wrongway` trigger (or `resetplayertrigger1`) behind its start. When the player enters it while the
race runs, the race starts again from its grid. This page pins down the trigger's data, its shape, what the race script
does with it, which races have one and where it stands, and what the rewrite does. The order of the goals and the end of
a race are in [race-goals.md](race-goals.md); the pause menu's restart, which this trigger reuses, is in
[race-restart.md](race-restart.md). Method, offsets and addresses are in the [provenance file](../provenance/race-wrongway.md).

- **Sources:** the compiled Lua of the `basicrace` graph in `GLOBAL/gameplay.bin` (decoded and read; nothing is copied),
  `speed.exe` v1.3 through Ghidra for the trigger system, and the install's data read with the repository's reader.
- **Evidence tags:** **[lua]** read in the compiled script. **[exe]** read in `speed.exe`. **[verified]** measured on the
  install's data. **[ours]** a choice of the rewrite. **[unconfirmed]** a guess or not read.
- Not to be confused with the HUD's wrong-way indicator (`Hud_WrongWIndi`, a string of the executable's tables with no
  object in any HUD package), which this page does not cover.

## 1. The data **[verified]**

The trigger is a collection of class `resetplayertrigger`, child of `planetrigger`, child of `trigger`. It is named
`<race collection>/wrongway` (116 races), `<race>/resetplayertrigger1` (47, drag races, the tollbooth challenges, and a few
others) or `resetplayertrigger2` and `wrongway2` (3 more, in four races that have two). 166 triggers in 161 races.

| Field | Value | Meaning |
|---|---|---|
| `Position`, `Rotation` | per trigger | the slab's centre (map coordinates, as every marker, [formats/gameplay-races.md](../formats/gameplay-races.md)) and its heading in degrees |
| `Width` | 150 on the `resetplayertrigger` template (a bare `planetrigger` has 50); 200 on one trigger | metres across the road |
| `ResetsPlayer` | True | read by the race script, section 3 |
| `Directional` | True on `planetrigger` | **no reader**: neither the executable nor any of the 253 scripts names it, section 2 |
| `OneShot`, `FireOnExit`, `ParticleEffect` | False, False, empty | the trigger options of `trigger`; nothing is drawn |
| `Dimensions` | none | a plane trigger has no `Dimensions`; its box comes from `Width` |

There is no message, delay, target marker or sound field: the trigger only says "entering me resets the player". Where the
player goes is the race's own `racestart`, section 3.

## 2. The shape **[exe]**

The constructor of a gameplay trigger (a routine of the race status code) builds a trigger with no `Dimensions` and no
`Radius` as a **box**: `Width` across the road, **1 m** deep along the trigger's heading and **50 m above and 50 m below**
its position. A box trigger built from `Dimensions (a, b, c)` gets `b` across, `a` along and `c` above and below. The
stored numbers are therefore **full sizes for the two horizontal axes and a half size for the height** (read from the
constructor's bounding radius, `sqrt((b/2)^2 + (a/2)^2)`, and from the box test's half sizes, `b/2`, `a/2` and `c`). This matches the
height already found for the checkpoints (a checkpoint's `40` is a half height, [race-mode.md](race-mode.md) section 3) and
**contradicts the rewrite's reading of the horizontal extents as half sizes** (`setup::gate_of` doubles them): a checkpoint
`(2, 40, 40)` is 2 m deep and 40 m wide, not 4 m and 80 m. Which of the two horizontal axes is the thin one is the
rewrite's reading of the data (the cosine test of the format page), not read here **[unconfirmed]**. The gates of
checkpoints and finish lines are left as they were; the reset zone uses the numbers as the executable reads them.

**How the trigger system tests a car** (`FUN_00766cb0`, the box case) **[exe]**: a sphere of the element's bounding radius,
moved along its velocity for the tick, against the oriented box. So a thin slab cannot be jumped, and a car is
caught when its body, not its centre, touches the slab. The test has a flag bit (`0x80000`) that makes it
require the element to move along the trigger's normal, which would be `Directional` **[unconfirmed]**; the constructor never
sets it for a gameplay trigger and the field has no reader, so the zone catches a car from either side.

**The enter event** is raised once per entry **[exe]**: the trigger keeps the list of the elements inside it; an element
that is tested inside and is not on the list is added and raises `MTriggerEnter`; an element that is tested outside and is on
the list is removed (and raises the exit message only when `FireOnExit` is set). A car that stays inside raises nothing more.

## 3. What happens when the player enters it **[lua]**

The handler is the `racing` state's `triggerenter` of the `basicrace` graph (every race inherits it), the same one that
counts the checkpoints ([race-goals.md](race-goals.md) section 2). In order:

1. The racer must be in the race (`InRace`). A trigger entered before GO or after the player finished does nothing.
2. If the element is a **player** (`Element:IsPlayer()`) and the trigger has `ResetsPlayer`:
   - **Offline** (every race of this version): the script prints `reset trigger` and `jump to pre race` to its debug log and
     calls `ChangeState('prerace')`. Nothing else: no message, no fade, no sound, no penalty, no statistic.
   - **Online** (`OnlineRace`): the player is warped to the race's `racestart` (`WarpPlayerToTrigger`) and the race goes on.
     This version has no online races.
3. Any other element (a computer-driven racer, traffic) falls through to the goal test, where the trigger is no goal, so the
   **AI racers are not affected**.

**The handler runs only in the `racing` state.** The graph has no `triggerenter` in `prerace` or `countdown`, so the zone is
unarmed while the cars stand on the grid and count down. The zone is behind the grid (section 4), so it is not entered at the
start.

**`ChangeState('prerace')` is a full restart**, the same state the pause menu's Restart enters (`racing` handler
`restartrace`, which first calls `SetActivelyRacing(false)`; the wrong-way handler does not, and `loadingcomplete` sets it
again). The `racing` exit handlers only kill the race's timers. Then `prerace` runs again:

- `stateenter` resets the cops (`ResetCopsForRestart`, `SetCopsEnabled`), the weather chance, and builds the per-racer tables
  again (goal list, `NextGoal`, `LapsLeft`, `InRace`, `Element`), the knockout allowances, the checkpoint list (every
  checkpoint, then the finish line, `ClearCheckpoints` first), the online flag, then **warps the player to `racestart`** unless
  the race came from its intro NIS (`CameFromNIS`, cleared the first time), restores the racers' saved start positions
  (`RestoreStartPositions`), and waits for the world (`NotifyScriptWhenLoaded`). The per-race copies of `stateenter`
  (scripted cops, calls, the `DoMiaSMS` of a race, the barrier area and door of the final race) run again too.
- `loadingcomplete` restores the start positions again, runs `StartRace` (the native start: the race's barriers are switched
  on again, the clocks and the racers are set up, [race-barriers.md](race-barriers.md)), sets `SetActivelyRacing(true)` and
  goes to `countdown` when `DoCountdown`, else to `racing`.
- `countdown` holds the cars (`SetAllStaging`) and shows 3, 2, 1, GO when the race has a countdown, else the race goes
  straight to `racing`; `racing`'s `stateenter` calls `StartRaceTimers`, so the race clock starts from 0.

So the player is back on the grid with a fresh clock, the checkpoints from the first, the AI racers regridded, the cops reset,
and the countdown again (or the rolling start). The **intro is not played again**: the intro states (tutorial, message, movie,
NIS) come before `prerace` and nothing in the wrong-way or restart path enters them. A per-race effect that sits on leaving
`prerace` runs again (`16.1.0` spawns its explosion there, `ENISNukeSmack` at a fixed place near the start).

Nothing in the handler or in `prerace` calls `AbandonRace`, reports a result or touches the career's counters, so a
wrong-way reset is **not** counted as an abandoned or lost race **[lua]**. (The checks of the autosave and the career are
outside the script; this version's restart does not enter them either, [race-restart.md](race-restart.md) section 2.)

## 4. Which races have one, and where it stands **[verified]**

Measured on the install with each race's route taken as the straight lines from its start through its `Checkpoint` list to
its finish line:

- **166 triggers in 161 races**; four races have two (`16_2_1_d_day_intro`, `16_2_3_sprint`, `2_2_1_r_sprint`,
  `challenge_burgerking`). They belong to sprints, tollbooths, speed traps, drags, the tollbooth challenges and a circuit.
  All are plane triggers, 150 m wide but one of 200 m.
- **161 of the 166 lie 0 to 150 m behind the start grid** (along the start's heading), the median 50 m, the median
  1 to 2 m off the grid's own line (the prologue's two sprints 39 m and 19 m): a player who drives backwards from the
  start crosses it. In a circuit the start and the finish line are one place, so its zone is near the finish line too
  (`6_1_1_circuit`: 211 m).
- **Five do not:** `8_2_1_sprint` (352 m behind), `16_2_1_d_day_intro` (a second trigger, 1,077 m behind in the start's
  frame but 58 percent along the route and 259 m off it), `4_5_2_speedtrap` (87 m ahead of the start, 59 m off the route),
  `2_2_1_r_sprint` (90 percent along the route, 102 m off it) and `challenge_burgerking`'s `resetplayertrigger2` (91
  percent along the route, 478 m off it). Their purpose (a shortcut or a gate the route must not take) is not read.
- **The prologue's races:**

| Race | Trigger | Distance behind the start | Sideways of the grid | To the last listed checkpoint | To the finish line |
|---|---|---|---|---|---|
| `16_1_0_partial_dday` | `wrongway`, 150 m | 78 m | 39 m | about 1,390 m | about 1,390 m |
| `16_2_2_sprint` | `wrongway`, 150 m | 99 m | 19 m | about 1,830 m | about 2,060 m |
| `16_2_3_sprint` | `resetplayertrigger1` and `2` | 23 m and 8 m | 15 m and 70 m | about 1,420 m | about 1,250 m |
| `16_2_1_d_day_intro` | `wrongway2`, `resetplayertrigger1` | 67 m and mid-route | | | |

None of the prologue's zones stands near the end of its route: the two sprints' are 1.4 km and more from their last
checkpoint and from their finish line.

## 5. What the rewrite does **[ours]**

- `nfsmw_data::gameplay::PlaneTrigger` reads every `planetrigger` collection (`GameplayData::planes`);
  `GameplayData::reset_zones(race)` gives the ones with `ResetsPlayer` that are children of a race.
- `race/wrong_way.rs` turns each into a `blackbox_race::Gate` (1 m deep, `Width` across, 100 m high) in `Course::resets`; the
  world adds them when it builds a stock race's course. A mod's race has none.
- The engine raises `RaceEvent::Reset { racer }` when a **human** racer that is racing moves into a reset zone from outside
  it (a move that touches the zone while its start was not inside; the segment test cannot be jumped). Before GO, after a
  racer is done, and for the computer's racers it raises nothing.
- The world answers a reset with the pause menu's restart (`restart_race_now`): the racers, the clock, the checkpoints, the
  walls and the cops are made again, the player's car goes back to the grid and the countdown runs. No message and no fade.
  The restart goes **past the intro** (`RaceSession::restarted`): the tutorial, message, movie and NIS are not played again,
  which also corrects the pause menu's Restart for the races that have an intro. A split-screen race restarts for both players
  when either enters the zone (the script's `IsPlayer` is true for every human).
- The race logs the entry at the info level (`race: racer 0 entered a wrong-way zone ...: restart`).

**Differences from the original:** a car is a point to the engine, where the original tests a sphere of its bounding radius
(the slab is caught a metre or two earlier) **[unconfirmed]**; the online branch (a warp, no restart) is not modelled;
the explosion that `16.1.0` fires on leaving `prerace` is not part of the rewrite's restart; the player's car is repaired on
a restart, as the pause menu's restart does **[ours]**, while the original's `RestoreStartPositions` only places the cars.
