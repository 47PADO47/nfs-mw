# Race in progress and restart (pause menu)

What the pause menu asks the world about a race, and what a restart does. The race itself is in
[race-mode.md](race-mode.md); the pause menu's items are in [career-parity-menus.md](career-parity-menus.md). The
executable's side, with addresses, is in [provenance/race-restart.md](../provenance/race-restart.md). Evidence tags as in
[docs/README.md](../README.md#evidence-tags): **[exe]** read from the machine code of `speed.exe` v1.3, **[ours]** a choice
of this rewrite where the executable does not say, **[unconfirmed]** not read.

Code: `crates/nfsmw/src/scenes/world/race/restart.rs` (`race_in_progress`, `restart_race_now`), forwarded by the `Scene`
trait methods `race_active` and `restart_race` (`scenes/world/scene.rs`, `frontend/pausable_scene.rs`); the front end calls
`race_active` when it opens the pause menu (`frontend/flow.rs`, `pause`) and `restart_race` when the player confirms Restart
(`frontend/flow_career.rs`), which resumes play when it returns true.

## 1. What `race_active` means

| Situation | `race_active` | Why |
|---|---|---|
| Free roam, no race session | false | no race object |
| A race asked for (`queue_race`, a quick race, a post-race Restart or a chained `PostRaceActivity`) and not started yet | true **[ours]** | the race is staged; it starts on the next frame |
| Loading: waiting for the grid and the racers' models | true | staged |
| Countdown, rolling start, running | true | the race is on |
| Race-over message (`Stage::Results`, 3 s) | false **[ours]** | the result is fixed and the career has been told |
| Post-race screen's wait (`Stage::PostRace`) | false | the post-race screen owns the race; its own Restart answers it |
| Pursuit challenge (`challenge` race) running | true | it is a race session, see below |
| Free-roam speed trap, milestone trap, "event" that is no race | false | no race session; nothing to resume or restart |

**The original's rule [exe].** The pause menu builder (`FUN_0052b4d0`) has two families of menus. In quick-race mode
(play mode word `+0x1964` not 2) the race menu (Resume Race, Restart, a Quit item, Options) is always built. In career
mode (`+0x1964 == 2`) it asks the race status object's racing flag (`+0x1960`): 0 builds the free-roam menu (Resume Free
Roam, Jump to Safe House or Quit to Career Main Menu, Tuning, Options), non-zero the race menu (Resume Race, Restart, then
the quits). The flag is set to 1 by the start of a race (`StartRace`, `FUN_0060dbd0`, through `FUN_00602010`) and cleared by
`SetRoaming` (`FUN_005fe3e0`), which also zeroes the race parameters pointer `+0x1968`. So the menu does not look at the
parameters pointer to choose between the two menus; it looks at "a race was started and roaming has not been set again".
The pointer is read for the details (race kind, the event name `1.8.1`, flags).

- **Pursuit challenges** are races in the original: they go through the same `StartRace` (the challenge-series flag, bit 15
  of the definition's flags, only picks the Quit item of the quick-race menu **[exe]**), so the career menu shows Resume Race
  and Restart for them. The rewrite does the same: a challenge is a `RaceSession` (`race/challenge.rs`), so
  `race_active` is true while it runs and Restart starts it again.
- **Speed traps in free roam** are not races in the original: they are `AddSpeedTrapToMap` / `IsActiveSpeedTrap` entries of
  the roaming graph, and no `StartRace` runs for them **[exe]** (the Lua functions are registered separately). In the
  rewrite they have no session (`career/speedtraps.rs` does nothing while a session exists). A speed-trap **race** (race
  kind 5) is a session and counts as a race.
- **Race kind 4 (tollbooth)** and **the final pursuit (event `1.8.1`)** only change which items follow Restart (Tuning is
  left out for kind 4; `1.8.1` takes the Quit to Career Main Menu variant) **[exe]**. They do not change whether the race
  menu shows. The final pursuit is not run by this version (`challenge.rs`).
- **Results.** The original keeps the flag set until the post-race flow calls `SetRoaming`, so its menu would still show
  Resume Race during the race-over message. The rewrite shows the free-roam menu from the moment the result is fixed
  **[ours]**: the career has already been given the result (`career_race_finished`), and a Restart then would start a
  second race that could pay the same win again. The post-race screen has its own Restart for that.

## 2. What a restart does

`restart_race_now` runs the race in progress again from its grid with the parameters it was started with. It returns false
when there is no race (free roam), or when the result is already fixed; the front end then does not resume.

The original: Restart asks for a confirmation (label `0x4D3399A8`), and the race graph's restart path puts every racer back
on the grid and clears the racer records (`RestoreStartPositions`), resets the cops (`ResetCopsForRestart`) and goes back to
the pre-race state, which counts down again **[exe]** for the two native routines; **[unconfirmed]** for how the pause item
reaches the graph (provenance, dead ends).

| State | Original **[exe]** | Rewrite |
|---|---|---|
| Race clock, countdown | the three racer timers are reset; the graph counts down again | a new `blackbox_race::Race` engine: elapsed 0, countdown again (or a rolling start again) |
| Checkpoints, laps, progress, percent, place, finished/out flags | the racer record is zeroed, the 16 bonus slots go to "none" (`RestoreStartPositions` -> record reset) | new engine: no checkpoint, lap 0, place from the grid; nothing is carried over |
| Time bonuses already earned | in the record's bonus slots, cleared | in the new engine's clock, so cleared |
| Player's car position and speed | the saved start pose of the record (`SaveStartPositions`) | `drive.place_exactly(start, speed)`: a respawn on the race's start marker, standing (0 m/s) or at `InitialPlayerSpeed` for a rolling start; a respawn also snaps the chase camera (`cut`); the car is repaired first (`repair_player_car`) so the abandoned run's damage does not carry over **[ours]** |
| Grid hold, drag steering assist | back to staging | `hold_on_grid` set again by the new start; the assist cleared (`release_race`) and set again by the drag race |
| Opponents | placed on their saved poses | removed (`clear_racers`) and spawned again on their grid slots from the same plan |
| Opponents' goals, catch-up, emulated far racers, lane | per record | new session: goals unset, catch-up state fresh, `emu`, `glue`, `lane` fresh |
| Cops, chase, roadblocks, spike strips | `ResetCopsForRestart` | `stop_pursuit` (ends the chase, removes cops and roadblock cars, clears spikes), then the start stops a pursuit again unless the race has `CopsInRace` |
| Traffic count | restored by the graph | `traffic_manual` is given back before the new start reads it, so the new start sets its own level from the race's `ForceTrafficDensity`/`TrafficLevel` and the options (drag races: none) |
| Banner, race HUD, minimap blips | rebuilt from the status | the banner is new; the HUD and the checkpoint/racer blips are derived from the session each frame, so they follow the new one |
| Camera | the graph's | `view` is set to the chase view by the start; the respawn is the camera cut any respawn gives |
| Challenge state | (a challenge restarts as a race) | a new `Challenge`: statistics, dodged roadblocks and spikes, "value reached" flag at 0; the chase begins again once the car stands |
| Mods | not applicable | the mods' race snapshot reads the new session (clock 0); no outcome is recorded for the abandoned run, as for `race stop` |

**Same parameters.** The session keeps the `Race` it was started from and the course a mod gave it (`Origin`), so a restart
needs no lookup by name. A race a mod registered has no collection in the install's data, and the old queue-by-name
(`RaceAnswer::Restart` in `race/report.rs`) cannot start it again. The options (`RaceOptions`: laps, opponents, difficulty,
traffic, cash, career, demo) are copied unchanged, so a career race stays a career race with the same cash value.

**Mechanism.** `RaceAnswer::Restart` ends the race and re-queues `(name, options)`; the queued race starts on the next frame.
The pause Restart does the same in one call: `release_race` (what `end_race` gives back, without the career hook), then
`stop_pursuit`, a repair of the player's car, then `begin_race` with the kept race, options and course. The start is then the normal one
(`start.rs`): the race is in `Stage::Loading` until the car stands on the grid and the models are ready (they are already
loaded), then the engine is made. Nothing else of the old session survives.

**The intro is not played again.** The original's restart enters the pre-race state of the race graph, which comes after the
intro states (tutorial, message, movie, NIS); the rewrite's restart marks the new session (`RaceSession::restarted`) so that
`intro_queue` gives no steps and the grid goes straight to the countdown **[lua]**
([race-wrongway.md](race-wrongway.md) section 3). Before this, a restart replayed the tutorial and the story movie of the
races that have them.

**The wrong-way trigger** restarts the race through the same call: when a human racer enters the reset zone behind the
start, the engine raises `RaceEvent::Reset` and `race_tick` runs `restart_race_now` at the end of that tick
([race-wrongway.md](race-wrongway.md) section 5). Nothing is shown and nothing is paid, as for the pause menu's Restart.

## 3. The career: nothing is charged, paid or entered twice

A career race touches the career at three places, none of which a restart repeats:

1. **Entering** the race zone (`career_enter_race`, `entry.rs`): the Blacklist judges the race (open or locked) and raises the
   event-start autosave. A restart does not go through the zone, so it neither re-judges the race nor saves again.
2. **Finishing** (`finish_race` -> `career_race_finished`): the result goes to the Blacklist, which counts the race, pays the
   cash on a win, opens rivals and queues the messages. This happens once, when the player finishes. A race that is restarted
   has not finished, so no result is applied for the abandoned run. Restarting after the result is fixed is refused (section 1).
3. **Leaving** (`end_race` -> `career_race_left`): the roaming-start autosave. A restart is not leaving: it does not call
   `career_race_left`, so no roaming autosave runs between the two runs. (The post-race Restart does leave and autosave, as the
   original's graph passes through roaming there **[unconfirmed]**.)

The rewrite has no entry fee or stake for a race (`grep` for fee/stake in `scenes/world` and `career/` finds none), and the
original's career has none that this repository documents ([career-economy-flows.md](career-economy-flows.md)); a restart
therefore charges nothing. First-completion rewards (bin entry, rival unlock, side-race ladder `side_won`) come only from
`career_race_finished`, which a restart does not reach.

## 4. Edge cases

- **Asked for but not started.** `race_active` is true and Restart returns true without doing anything: the race starts from
  its grid on the next frame anyway.
- **Loading stage.** Restart works: the old session is dropped and a new one begins loading. The world does not wait twice for
  the models that are already loaded.
- **A restart that cannot start** (the car is gone, the race is not runnable): the old race is already down, so the world is
  in free roam; the call still returns true so the front end resumes play, and the reason is logged.
- **Pause while a bust ticket is up**: not a race pause; not handled here.

## 5. Not verified

- How the pause menu's Restart reaches the Lua race graph (the item's handler opens the confirmation dialog; the accepted
  message is handled in code the disassembly does not define as a function). The rewrite restarts directly. **[unconfirmed]**
- Whether the original repairs the player's car on a restart: the native routine only resets the pose and the racer record.
  A respawn in the rewrite does not repair by itself (`place_moving` only places and sets the speed), so `restart_race_now`
  repairs explicitly. **[unconfirmed]**
- Whether the original's pause menu shows Resume Race during the race-over message. **[unconfirmed]**
- The object behind the two other calls of the restart path (`0x0092C674` slot 1, `0x0092C67C` slot 2, called with 0 and 1):
  the second is the cop manager's reset by its Lua name; the first is a manager called with 0 before the racers are reset,
  probably traffic or the AI vehicles. The rewrite leaves the traffic as it is.

## 6. Tests

- Unit (`race/restart.rs`): which stages are live; a restart keeps the race, options and the mod's course; a restart has no
  intro; a fixed result has nothing to restart.
- Installed (`scenes/world/race_restart_tests.rs`, `#[ignore]`, set `NFSMW_GAME_DIR`): free roam is not a race; a queued race is
  active; once running and past the countdown, `restart_race_now` makes a fresh session with the same name and options at the
  loading stage, the clock back before the old elapsed time, the countdown running again and no checkpoint reached;
  `abandon_race` ends it and a second restart returns false.
