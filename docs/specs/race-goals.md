# Race goals: the checkpoint list, then the finish line

What a racer aims at in a race and what ends the race for it. The ranking, the clock and the types of race are in
[race-mode.md](race-mode.md); this page is the order of the gates and the exact moment a sprint ends. The Rust rules
are `libs/blackbox-race/src/course.rs` (`Progress::advance`) and the course the world builds in
`crates/nfsmw/src/scenes/world/race/setup.rs` (`course_of`). Method, offsets and addresses are in the
[provenance file](../provenance/race-goals.md).

- **Sources:** the compiled Lua of the `basicrace` state graph in `GLOBAL/gameplay.bin` (the `prerace` and `racing`
  handlers, decoded and read; nothing is copied), `speed.exe` v1.3 through Ghidra for the native calls, and the
  install's race data read with the repository's reader.
- **Evidence tags:** **[lua]** read in the compiled script. **[exe]** read in `speed.exe`. **[verified]** measured on the
  install's data. **[ours]** a choice of the rewrite. **[unconfirmed]** a guess.

## 1. Where the rule lives

The executable does not decide that a lap or a race is over. `NotifyCheckpointReached`, `NotifyLapFinished` and
`NotifyRaceFinished` only record what the script tells them ([race-mode.md](race-mode.md) section 3). The decision is
the `racing` state's `triggerenter` handler of the `basicrace` graph (every race inherits it), fed by the `prerace`
state's setup. **[lua]**

## 2. The rule

**Setup (`prerace`).** For every racer: a goal list, `GoalList[racer]`, is the race's `Checkpoint` array (or, for a
racer whose character has a `CannedPath`, that path); `GoalCount[racer]` is its length; `NextGoal[racer]` is 1, or 0
when the list is empty; `LapsLeft[racer]` is `NumLaps`. The checkpoint list the native side shows (the next-checkpoint
marker and the minimap) is built by adding each entry of `Checkpoint` in order and then `racefinish` as the last one.
**[lua]**

**The goal.** The goal of a racer is `racefinish` when `NextGoal` is 0, else `GoalList[racer][NextGoal]`. After a goal
is reached `NextGoal` becomes `NextGoal + 1` while that is not above `GoalCount`, else 0. So the sequence is the
listed checkpoints in order, then the finish line, and after the finish line of a lap the checkpoint 1 again. **[lua]**

**A trigger is entered** (every box trigger of the race raises the same event, with the trigger as `Sender` and the car
as `Element`):

1. A racer that is not in the race (`InRace` false) is ignored.
2. A trigger flagged `ResetsPlayer` (the `wrongway` child) entered by the player sends the race back to `prerace`
   (offline): the whole race starts again from its grid ([race-wrongway.md](race-wrongway.md)).
3. The trigger counts only if it **is the racer's goal**, or its `MasterCheckpoint` is the goal. Any other trigger is
   ignored, whatever it is called and wherever it stands: a racer that has not reached checkpoint 3 gains nothing from
   driving through checkpoint 4 or the finish line, and a trigger that is a child of the race but is not in
   `Checkpoint` is never a goal.
4. A counted trigger of the race's own checkpoint list calls `NotifyCheckpointReached(racer, NextGoal)`; the native side
   advances its counter only when the number is the counter plus one, so the finish line (number 0) does not advance it.
   The trigger's `TimeBonus`, if any, is added to the player's clock.
5. `NextGoal` advances and the native racer is told its new goal (`SetRacerGoal`).
6. If the trigger is `racefinish`: `LapsLeft` drops by one and `NotifyLapFinished(racer, NumLaps - LapsLeft)` runs. When
   `LapsLeft` is 0 the racer has finished: `NumberFinished` grows, the ranking is calculated, `NotifyRaceFinished` and
   `NotifyRacePlacement` run and the racer leaves the race (`InRace` false). **[lua]**

So **reaching the last listed checkpoint only makes the finish line the goal. A sprint ends for a racer when it enters
the finish line after the last checkpoint, and at no other moment** (apart from time running out, a destroyed car, a
bust and the other endings of `racing`, which are not goals). The rewrite's `Progress::advance` is the same rule
(`reached` is `NextGoal - 1`; a finish line crossed early is ignored; `close_lap` is the finish).

## 3. The two prologue sprints **[verified]**

The prologue races `16_1_0_partial_dday` (`16.1.0`) and `16_2_2_sprint` (`16.2.2`) have both a finish line and a
`Checkpoint` list, one lap each:

| Race | Listed checkpoints | Last listed checkpoint to the finish line | A trigger the list leaves out |
|---|---|---|---|
| `16_1_0_partial_dday` | 4 (`ai_cp1` to `ai_cp4`) | about 1,040 m | `ai_cp5`, about 400 m before the finish line |
| `16_2_2_sprint` | 6 (`checkpoint1` to `checkpoint6`, 80 m cubes) | about 590 m | `checkpoint7`, about 280 m before the finish line |

The extra trigger of each is a child of the race with the checkpoint's shape and `Dimensions` but is not in the
`Checkpoint` array, so under section 2 rule 3 it is never a goal: the original does not stop, count or end anything
there. Both races also have a `wrongway` child with `ResetsPlayer`, a plane trigger 78 m and 99 m behind the start grid
and 1.4 km and more from the last checkpoint ([race-wrongway.md](race-wrongway.md) section 4), so it cannot be what ended
a run at the last checkpoint. The gates' numbers are the install's; walking a
point through the centres of the gates in order (the straight lines, one metre per step) with the rewrite's rules gives
`Checkpoint 1 .. n`, then `Finished` and `Over` on the step that enters the finish line, in both races.

## 4. What the rewrite does, and the report that a race ends at the last checkpoint **[ours]**

The world builds the course from the `Checkpoint` list and `racefinish` (`course_of`; a missing trigger is skipped in
both lists), the engine raises `Checkpoint(n)` for each listed gate in order and `Finished` only for the finish line,
and the world ends the race on the engine's `Over`. Tests hold the rule in each layer: in `libs/blackbox-race`
(`the_last_checkpoint_of_a_sprint_does_not_end_the_race`, `standing_in_the_last_checkpoint_does_not_finish_the_race`
and `a_far_finish_line_after_a_wide_last_checkpoint_is_reached_on_its_own`) and for the course the world builds
(`setup_course_tests.rs`: a trigger the list leaves out is no goal).

A player report says that in both prologue races the race ended on reaching the last checkpoint and the finish line
should have come next. Reading the code and the data of this version finds no path for it: the finish line is the goal
after the last checkpoint, 280 m to 1,040 m on; the clock has no limit in either race; no time bonus is set; the
opponents cannot end a race while the player is racing; and the simulation above ends each race at its finish line.
The race now writes its gates at the info level, with where the car stands and the race clock, so the next log shows
what a run was doing when it ended:

```
race: checkpoint 6/6 (x, y, z) at 61.2 s
race: the player crossed the finish line (x, y, z) at 83.4 s
race: over (x, y, z) at 83.4 s, the player finished
```

A run that ends without a `crossed the finish line` line did not end through the engine's finish. **[unconfirmed]**: the
cause of the report; what the player took for the last checkpoint may be one of the unlisted triggers of section 3 (no
marker is drawn for any checkpoint, so a player has only the banner `Checkpoint n/m`, the minimap blip and the finish
line's flares to go by).

## 5. Not modelled

- **Trigger entry.** The original raises the event once, when a car goes from outside a trigger to inside it, and tests a
  sphere the size of the car ([race-wrongway.md](race-wrongway.md) section 2) **[exe]**; the rewrite's checkpoints and
  finish line test the segment a car moves along each tick against the box, which counts a gate the car already stands in
  (the reset zone is edge-triggered). The data never puts the next goal's box around the previous one's exit, so the two
  agree on the install's races **[unconfirmed]**. The extents of the checkpoint boxes are also read differently
  ([race-wrongway.md](race-wrongway.md) section 2).
- **`MasterCheckpoint`** (a trigger that counts for another), **`SharedCheckpoints`** (other racers aiming at the same
  checkpoint advance with the one that reached it), a **`CannedPath`** goal list and the
  scripted triggers of single races (cop spawns, the 911 call and the bail of `16.2.2`, the engine kill of `16.2.1`
  which has its own module, the cell calls) are not part of the goal rule here.
- **`NotifyLapFinished` on the last lap.** The script calls it before `NotifyRaceFinished`; the rewrite raises `Finished`
  for the last lap and `Lap` for the others.
