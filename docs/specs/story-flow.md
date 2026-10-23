# Story flow: the career state machines for story mode

The contract the story mode is built from. It sets out the state machines of the career's story: the race life
cycle, the prologue chain and its scripted chase, the drive to the first safe house, and the first safe house visit.
The rest (text messages and calls, the rival bins and gauntlets, the arrival sequence and the final race) is in
[story-flow-2.md](story-flow-2.md). Every rule names the repo spec it comes from.

This file states rules, not code. Each machine gives its states, the entry actions, the transitions with their exact
conditions and constants, and the outputs. A row tagged **[unconfirmed]** is not closed by the sources. Section 6 of
[story-flow-2.md](story-flow-2.md) lists each open point as an F-item with the lookup that closes it.

- **Sources:** the specs in the key below, read in full. In this pass the install was searched by name only
  (`GLOBAL/gameplay.bin`, `MOVIES/`, `NIS/`). The names found are tagged **[verified]** with "(names, this pass)". The
  community decompile was not read, so no **[decomp]** tags appear.
- **Evidence tags**, as in the sources: **[verified]** checked in install data or saves; **[exe]** read in `speed.exe`,
  not tested in game; **[community]** from a community summary; **[unconfirmed]** a guess or a rule not closed.
  See [docs/README.md](../README.md#evidence-tags).

### Source keys

| Key | Spec |
|---|---|
| CSE | [career-scripts-events.md](career-scripts-events.md) |
| CS | [career-scripts.md](career-scripts.md) |
| CF | [career-flow.md](career-flow.md) |
| CP | [career-phone.md](career-phone.md) |
| CSH | [career-safehouse.md](career-safehouse.md) |
| CSH2 | [career-safehouse-2.md](career-safehouse-2.md) |
| CB | [career-blacklist.md](career-blacklist.md) |
| RM | [race-mode.md](race-mode.md) |
| SAVE | [../formats/career-save.md](../formats/career-save.md) |
| GR | [../formats/gameplay-races.md](../formats/gameplay-races.md) |
| CET | [career-economy-flows.md](career-economy-flows.md) |
| CT2 | [career-text-2.md](career-text-2.md) |
| CR2 | [career-rapsheet-2.md](career-rapsheet-2.md) |

"CSE 1.1" means section 1.1 of the named spec.

## 0. Summary

1. The story is nine machines. A: the race life cycle, one per race (section 1). B: the prologue chain of six races
   (section 2.1). C: the scripted chase 16.2.1 (section 2.4). D: the drive to the safe house (section 2.5). E: the first
   safe house visit (section 2.6). F: text messages and calls (story-flow-2 section 3). G: the rival bin graph
   (story-flow-2 section 4). H: the boss gauntlets and their rewards (story-flow-2 section 4). I: the final race
   (story-flow-2 section 5).
2. The rules come from the Lua state graphs in `gameplay.bin`. The screens and native hooks in `speed.exe` supply the
   rest. **[verified]** + **[exe]**.
3. Settled in this pass from the Lua: the prologue passes its movie to the NIS player, then plays its NIS (F-01); the intro movie exits to the
   NIS (F-02); a loss stays in raceover and never reaches the post-race chain (F-08, F-09). Still open: a race finish may
   set the "won" flag on any placing (F-10, see 1.6); the stepper's resume point after the first visit (F-18). Section 6
   of story-flow-2 holds the answers.
4. Nothing in the code moves the stage from bin N to bin N-1 after a rival is beaten (CB 5.3). The rewrite enters bin
   N-1 when bin N is done. This is a choice, **[unconfirmed]**.

## 1. Race life cycle (machine A)

Tags: **[verified]** for the state graph (CSE 1.1, CSE 1.2, RM 2, RM 11); **[exe]** for the screens and hooks.

### 1.1 States

| State | Entry actions | Exits |
|---|---|---|
| initial | Set the career flag from the play mode. In career, if the race's time of day differs, ask for the change: reload the world and stop; the same state runs again after the load. The special-setup hook may also reload the world (section 2.2). Cops on or off (`CopsInRace`). Chance of rain. Clear racers. Create racers on the start grid and count them. Clear the busted flag and the "has been won" byte. | tutorial (always) |
| tutorial | D-Day races skip this state. Any other race calls `PlayTutorial` and waits for a movie-finished message. What the native call does when no tutorial is due: **[unconfirmed]** F-03. | intro_message on the message |
| intro_message | If `IntroMessageID` is not -1, show that message screen first (screen: **[unconfirmed]** F-04). The choice of rule 1.2 runs when that message is done (or at once when there is none). | intromovie or intronis |
| intromovie | Play `IntroMovie`. | on movie finished: intronis, always **[verified]** (F-02) |
| intronis | Play the NIS (`IntroNIS`, or `QuickRaceNIS` in a quick boss race) at the start marker, passing `IntroMovie` as its movie argument (empty in a quick boss race). Warp the player to the start trigger when the NIS name is empty, or in split screen. On exit save the start positions. **[verified]** (Lua) | prerace when the NIS completes (`niscomplete`), unless `NISShell` is set |
| prerace | Reset cops, the per-racer goal list, laps left and in-race flag. Reset knockout allowances. Checkpoint list: each checkpoint, then the finish. Online flag off. Restore start positions. Wait until the world has loaded. | countdown |
| countdown | If `RollingStart`, the first two racers go under AI control. If `DoCountdown`, staging is held and the 3-2-1 board runs. Otherwise go straight on. | racing on `countdowndone`, or at once with no countdown |
| racing | Section 1.4. | raceover when every human player is done or the race ends |
| raceover | Show the race-over message for `RaceOverMessageDuration` (3 s offline). Each tick: when the pause flag is set, call the losing-screen hook (a stub in this build) and clear the flag. Go to postrace only while the race's `ShowPostRace` flag is set. A loss clears it, so the race waits here **[verified]** (F-08). | postrace when `ShowPostRace` is set; restartrace to prerace; abandon to done |
| postrace | Call the native post-race flow once (results and rewards screens). Set the audio flag. | flowreadyforoutro to outronis; restartrace to prerace; enterfreeroam to done |
| outronis | If `OutroNIS` is set and its marker exists, play the outro NIS. Otherwise skip. | outro_message |
| outro_message | Optional outro message screen. | outromovie |
| outromovie | If an outro movie is set, play it. | done |
| done | Call the special-finalisation hook. Disable the race's barriers and re-enable the current bin's barriers. If the race was abandoned, return to roaming. In career, if `PostRaceActivity` is set and the race was not abandoned, run that activity (section 1.5). It tests no win (F-09). Otherwise `SetRoaming` (free roam, autosave). | end |
| abandoned | Reached from any state. Mark abandoned, stop racing, cops on, `AbandonRace`. | done |

Online states (`onlineover`) do not occur in career.

### 1.2 Intro choice

Tags: **[verified]** (CSE 1.1, CSE 2.1, CSE 4). The boss test is the node's `BossRace` field (hash `0xFF5EE5D6`, CF 8).

1. At intro_message, after the message in its `IntroMessageID` (if any), test in this order. An empty text counts as not
   set. The test reads `IntroNIS` first, then `IntroMovie`, then `BossRace` and `IsCareerMode`. Nothing compares
   `IntroNIS` with `QuickRaceNIS`. **[verified]** (Lua intro_message, F-01)
2. If `BossRace` is set and the session is not career (`Game.IsCareerMode()` false), this is a quick boss race. The NIS is
   `QuickRaceNIS` and the movie is empty. Go to intronis. If `QuickRaceNIS` is empty, intronis still runs and only warps
   the player. Stop. **[verified]** (F-06)
3. Else, if `IntroMovie` is not set, go to intronis with `IntroNIS` (if that is empty too, it only warps the player).
4. Else, if `IntroNIS` is set, go to intronis with `IntroNIS` and `IntroMovie`. Intronis passes the movie to the NIS player
   as its movie argument, so the movie is part of the same intro. **[verified]** (the argument position). The order
   inside the NIS player is native **[unconfirmed]**.
5. Else (movie set, NIS empty), go to intromovie: `EPlayRaceMovie(IntroMovie)`, then intronis on the movie-finished
   message. Intronis then passes the movie to the NIS player again with an empty NIS name. Whether that replays the movie
   is native **[unconfirmed]**. The 1.8.1 final race is this case.

**Settled (F-01):** the prologue races 16.1.0, 16.2.2, 16.2.3 and 16.2.1 each set a movie and an NIS. Rule 4 sends them
to intronis, which plays the NIS with the movie as its argument, so the movie plays. The NIS completion goes to prerace.
An earlier reading (rule 3 skips the movie) was wrong.

### 1.3 Race kinds at start and end

Tags: **[verified]** (RM 1.3, RM 2, RM 5, RM 6, GR 3.2, GR 9).

| Kind (`EventIconType`) | Start | Ends |
|---|---|---|
| p2p (sprint, type 0) | Countdown unless `RollingStart`. One lap. | First over the finish line. |
| circuit (type 1) | `NumLaps` laps (2 to 10; template 3). Checkpoints repeat each lap. | The finish line counts a lap; the last lap ends it. |
| drag (type 2) | Countdown with cars held. Drag steering assist. `TrafficPattern "drag"`: no cross traffic. | First over the line; the time and speed at the line are shown. |
| knockout (type 3) | Circuit laps. | Last racer is knocked out at each lap end (`KnockoutsPerLap` 1). |
| tollboothrace (type 4) | `TimeLimit` 40 to 70 s (template 90). | Time bonus checkpoints add 25, 35 or 160 s. Time out is a loss. |
| speedtraprace (type 5) | Ranked by points. `OvertimePenaltyPerSec` 10. | Points decide; time over the limit costs 10 points a second. |
| challengerace (type 8) | `RollingStart` True, `InitialPlayerSpeed` 40, `DoCountdown` False, `ForceHeatLevel` set. | A win is an escape after the value is reached (RM 14). |
| nis_shell (story) | None. No racing. | Plays its NIS only (16.8.1). |

Counts: 217 races have a countdown, 38 roll (RM 2, GR 3.2).

### 1.4 Racing: checkpoints, finish, placings, ends

Tags: **[verified]** (CSE 1.2, RM 3, RM 4, RM 5, RM 7, RM 12).

1. A checkpoint counts only when it is the next one in order. A report of any other checkpoint changes nothing.
   Checkpoints are numbered from 1. A tollbooth bonus checkpoint pays its `TimeBonus` at once.
2. A circuit lap is finished when the racer crosses the finish line after the last checkpoint. The lap is stamped and
   the racers are ranked again.
3. On the last lap a finish records `NotifyRaceFinished` for the racer, then `NotifyRacePlacement` with the rank. The
   racer leaves the race. The finish record is section 1.6.
4. Ranking, best first: a finished racer before an unfinished one (finish times compare); a racer out of the race
   (knocked out, totalled or busted) behind every racer still in; then more completed laps; then more progress.
   Speed-trap races rank by points first.
5. The race ends for the human when every human player is done. The race goes to raceover.
6. The race also ends at once on: time out (a race with a limit whose time left reaches 0); a destroyed car (the
   raceover message then lasts 1 s or 2 s); a blown engine (after a 1 s timer); a knock-out; a bust (the native pursuit
   flow handles it).
7. Time left is `max(0, TimeLimit + bonus - elapsed)`. With `TimeLimit + bonus` not above 0 there is no limit.

### 1.5 PostRaceActivity chain

Tags: **[verified]** (CSE 1.1 done, CSE 3, RM 11 item 6, CB 3).

1. In career, `done` runs the race's `PostRaceActivity` when it is set. The game stays out of free roam until the chained
   race ends.
2. A career race that is won (and every prologue race) starts its `PostRaceActivity` at once. 29 races of the install
   chain this way: the prologue, and the boss approaches.
3. A lost race never reaches `done`, so the chain stops there (F-08, F-09). **[verified]**
4. The gauntlets of section 4 (story-flow-2) use this chain: each boss race names the next one. The last race of the
   gauntlet carries the reward set.
5. Chain end: `SetRoaming`.

### 1.6 What a finish writes (career)

Tags: **[exe]** (CB 4.2, SAVE 3.2, and the 2026-10-21 pass in [provenance/story-flow.md](../provenance/story-flow.md)) with **[verified]**
for the lists. Conflicts C-01 and C-02 are settled by F-10: every step below except 2 runs only for a **human racer that
finishes first** (the racer record's place, `+0x10`, equals 1).

| Step | Output | When | Source |
|---|---|---|---|
| 1 | Record flag `0x2` ("completed in career") set. Node state bit 1 (value `0x2`), "won in the career", the completion test for story nodes 16.2.1 and 1.2.3. | First place in career (bit 0, value `0x1`, outside the career). | CB 4.2, SAVE 3.2 |
| 2 | Best result kept if it beats the stored one (time for sprint, drag and tollbooth; lap value for circuit and knockout; points for speed trap and cash grab; integer for type 6). | Each finish that beats it | CB 4.2 |
| 3 | Profile cash += `CashValue` (attribute `0xD8BAA07B`, truncated). | Every first place in career, also on a repeat win. | CB 4.2, RM 11 |
| 4 | Reward list granted (attribute `0x50104D90`). Reward routing: section 4.5 (story-flow-2). | First place, only when the node did not already carry `0x2` | CB 4.2 |
| 5 | For each race in the unlock list (attribute `0xFC8995C8`): flag `0x4` set; flag `0x8` too unless attribute `0xA4E6FCFD`. | First place, only when the node did not already carry `0x2` | CB 4.2 |
| 6 | Trigger icons refreshed. | Always | CB 4.2 |

### 1.7 Constants used by the race life cycle

Tags: **[verified]** unless marked.

| Name | Value | Source |
|---|---|---|
| `RaceOverMessageDuration` | 3 s; 0.5 s online; 1 s or 2 s after a destroyed car | CSE 1.1 |
| Countdown length | 3 s, one second per number (the rewrite's choice) **[unconfirmed]**; the Lua sets no length | RM 2, F-07 |
| Blown engine timer | 1 s, then the race ends (`eblown` timer, then postrace) | CSE 1.2, F-16 **[verified]** |
| Call-start fade (prologue) | 8 s after the call starts, then 3 s more (`killrace`), then postrace; the engine-blown timer may end it first | CP 5.2, F-16 **[verified]** |
| `InitialPlayerSpeed` | 0 (217 races), 40 (36), 60 (2); unit taken as m/s **[unconfirmed]** | GR 3.2 |
| `TimeLimit` template | 0 (no limit); tollbooth 90 with races at 40 to 70 | GR 1 |
| Time bonus values | 25, 35 and 160 s | RM 5, GR 3.1 |
| Overtime penalty | 10 points per second | RM 9 |

## 2. Prologue and the first safe house visit

### 2.1 The chain (machine B)

Tags: **[verified]** (CSE 2.1; CF 3; CB 9; the race and trigger names, this pass, names only).

| Step | Event | Vault | Intro movie | Intro NIS | Outro | Cash | Notes |
|---|---|---|---|---|---|---|---|
| 1 | 16.1.0 | `16_1_0_partial_dday` | `storyfmv_rac01` | `IntroNisDD` | none | 0 | One lap. Explosion on leaving prerace. Children include `call_from_mia` and `boss_spawn`. Razor is the rival. |
| 2 | 16.2.2 | `16_2_2_sprint` | `storyfmv_cro06_coh06a` | `GenericStart` | none | 10,000 per CSE text (F-13) | Children include `cop_spawn_trigger`, `ronnie_mccrae`. |
| 3 | 16.2.3 | `16_2_3_sprint` | `storyfmv_raz08` | `IntroNis05` | none | 10,000 per CSE text (F-13) | Rewards `reward_out_of_jail_free1` to `3` (three out-of-jail-free markers). Cops at `cop_spawn_trigger`, bail at `bail_pursuit`, 911 call at `trigger_911`. |
| 4 | 16.1.1 | `16_1_1_circuit` | none | `IntroNis10` | outro_nis_marker child (F-12) | 10,000 | Two laps. The NIS scene sends call 12. |
| 5 | 16.8.1 | `16_nis_rog_razor` | none | none | `EndingNis03` | none | NIS shell; no racing. The scene sends calls 11 and 13. Razor wins the car. |
| 6 | 16.2.1 | `16_2_1_d_day_intro` | `storyfmv_pin11` | `IntroNis04` | `storyfmv_bus12` | 0 | The scripted chase (section 2.4). |

Every prologue race forces the preset car `M3GTRCAREERSTART` (section 2.2). The reputation attribute is 0 for all six.
The movie and NIS columns follow CSE 2.1. Each row's movie is passed to the NIS player (section 1.2, F-01).

### 2.2 Forced car and the flag test

Tags: **[exe]** (CSE 2.3, CB 7.2, CF 3; F-14 answered in [provenance/story-flow.md](../provenance/story-flow.md)).

1. The special-setup hook (`DoSpecialSetup`) runs for a race whose node carries attribute `0x4393F69B`. When settings
   flag bit 20 (`0x100000`, "the E3 demo BMW has been given") is **clear**, it finds the stable record of preset
   `E3_DEMO_BMW` (`0x03A94520`) and makes it the current car, and answers true; otherwise it does nothing. So this hook
   is the final race's BMW, not the prologue car.
2. A new career sets the current car to `M3GTRCAREERSTART` (hash `0x38A3C42C`) at the start (CF 3, step 2).
3. Every prologue race forces the preset (CSE 2.1). Boss races force their own car where their data says so: the final
   race forces `e3_demo_bmw` (CSE 4); other forced cars are in GR 3.2 (`PlayerCarType`, 177 races leave it empty).

### 2.3 Chain rules

Tags: **[verified]** (CF 3 step 6, CSE 2.1, CB 9, CS 3.2).

1. A new career starts at stage 16, with 0 cash and the preset car. Its first event is `16.1.0`.
2. Each prologue race names the next one in its `PostRaceActivity`: 16.1.0, 16.2.2, 16.2.3, 16.1.1, 16.8.1, 16.2.1.
3. The done copy of 16.2.1 calls `JumpToCarLot`. The car lot opens (`Car_Select.fng`, platform 5) and the player picks the
   first real car.
4. Then race bin 16 runs its entry activity, `drive_to_safe_house` (section 2.5).
5. Prologue races are children of no bin. Stage stays 16 until section 2.6.
6. Whether a lost prologue race stops the chain: F-09.

### 2.4 Scripted chase 16.2.1 (machine C)

Tags: **[verified]** (CSE 2.2, CP 5.2); names **[verified]** (this pass). Hooks **[exe]**.

Trigger names in the install (this pass): `call_from_mia`, `kill_engine`, `cop_spawn_trigger`, `cop_spawn_1`,
`cop_spawn_2`, `bail_trigger`.

| Event | Condition | Actions |
|---|---|---|
| Player enters `call_from_mia` | The marker is triggered | Request phone call 10 (`ECellCall`). Set `WaitForCallLoad`. Start a 15 s backup timer. |
| Backup timer reaches 15 s | Engine not yet sabotaged | `SabotageEngine` at level 5. |
| Player enters `kill_engine` | Before the backup timer ends | `SabotageEngine` at once. |
| Call started (`notifycellcallstarted`) | `WaitForCallLoad` set | Start an 8 s `fade` timer. When it fires, fade the screen on and start a 3 s `killrace` timer; `killrace` fades off and goes to postrace (11 s in all). **[verified]** (F-16) |
| Engine blown (`notifyengineblown`) | Any | Start a 1 s `eblown` timer. When it fires: kill the timer, fade out, go to postrace. **[verified]** (F-16) |
| Player enters `cop_spawn_trigger` | Any | Cops on. Spawn two mid-size cops (`cop_spawn_1`, `cop_spawn_2`). |
| Player enters `bail_trigger` | Any | Call `EDispIntroRace()`, call `EBailPursuit(1.0, 1.0)`, then `Game.NoNewPursuitsOrCops()` (the block; its effect native). **[verified]** (F-15) |
| Done, `PostRaceActivity` empty | Race finished | `JumpToCarLot` (section 2.3). |

### 2.5 Drive to the safe house (machine D)

Tags: **[verified]** (CSE 7.1; CS 1.2 state list; CF 6; Lua for the states, F-17).

Race bin 16 has an entry activity `drive_to_safe_house` (names, this pass: `race_bin_16/drive_to_safe_house` with
child `gpstarget`). The bin waits for the activity to finish (CS 3.2).

| State | Entry actions | Transitions |
|---|---|---|
| initial | Cops off. If GPS is set to auto, go to activate_gps; else driving. If `SkipCareerIntro` is false, send message 15 (`EAddSMS`). | activate_gps or driving |
| activate_gps | Show the GPS; navigate to `target` with the minimap item. **[verified]** (F-17) | driving |
| driving | Timer `cellusage` 0.25 s: show the GPS to the target, and a second hint with a 2,000 m offset. Timer `hurry` 120 s: send message 121. | at_target on entering `gpstarget` |
| at_target | Stop both timers. Call `DoSafeHouseIntro(RespawnMarker)` with `menu_gate_safehouse/zone_safehouse_1/respawn` (CurrentBin 16). Sends no state change in the Lua; the safe house intro takes over (F-17, F-18). **[verified]** | none in the graph |
| outro_movie | Empty handlers. No handler sends a change to this state, so this build never enters it. **[verified]** (F-17) | none |
| done | Cops back on. GPS off. | end |

`SkipCareerIntro` is true when the debug flag is set or settings flag bit 5 (`0x20`, "opening seen") is set (CF 6).

### 2.6 Safe house first visit (machine E)

Tags: **[exe]** (CSH 3.6, CSH 4, CSH 5.3, CSH2 11.1; CF 6). Order of the stepper after the bio: F-18.

1. `DoSafeHouseIntro(marker)` runs. The marker key goes into the respawn fields `+0x108` and `+0x10C`. The field `+0x110`
   is set to 1. Stage is set to 16. The time-of-day index is 0.
2. The intro movie is `storyfmv_saf25` (a file of this name is in `MOVIES/`, this pass). Its next screen is
   `SafeHouseRivalBio.fng`.
3. The front end request has platform 2 and screen `FEAnyMovie.fng` (`WS_FEAnyMovie.fng` on wide screens). Because the
   screen is the default front end screen, context bit 17 (`0x20000`, the "Blacklist sequence") is set (CSH 4).
4. `SafeHouseRivalBio.fng` opens. Its constructor sees bit 17 and stage 16. It posts the enter-bin event for bin 15
   (routine `0x0062C2D0`). The stage byte goes from 16 to 15. The bin's milestones and speed traps are set, and bin 15's
   unlock list is granted. The bio package is sent message `0x0B21A45F`.
5. The stepper continues from the point F-18 names. Stage 15 is the stage that the arrival sequence (story-flow-2
   section 4.6) reads at its step 6.

### 2.7 Resume rules

Tags: **[exe]** (CF 4, CSH2 6.1, CSE 2.3, CB 9). These run when a save loads.

| Save state on load | Result | Source |
|---|---|---|
| Stage 16 and node 16.2.1 not complete | Current car set to `M3GTRCAREERSTART`. The saved prologue node is re-activated and the world starts. The prologue restarts from its event. | CF 4, CSE 2.3 |
| Resume Free Roam chosen while the prologue is unfinished | The first unfinished event in the order 16.1.0, 16.2.2, 16.2.3, 16.1.1, 16.2.1 becomes pending (mode 2). The world loads straight into it, with no free roam and no autosave. | CSH2 6.1 |
| Race 1.2.3 complete and settings bit 14 (`0x4000`, career completed) clear | Node 1.8.1 is activated and the world starts (the final race, story-flow-2 section 5). | CF 4, CB 9 |
| Otherwise | Platform 2 (safe house). Context bit 8 cleared, bit 0 set. The safe house front end shows. | CF 4 |
| Debug byte `0x00926126` set | The prologue check is skipped. | CSH2 6.1 |

Stage 16 with `16.2.1` complete goes to the safe house menu. A save at CurrentBin 16 with 16.2.1 unfinished restarts the
prologue.

### 2.8 Rewrite choices in the prologue **[ours]**

What the rewrite decides where the spec above is silent. All of it is open to Ghidra checks.

1. The call, kill, cop and bail triggers of `16.1.0` and `16.2.1` are **plane triggers** (`planetrigger`), not boxes. A plane
   is a slab 1 m thick, so it is tested along the player's step, not by where the step ends. `16.1.0` has only
   `call_from_mia` (and `wrongway`); the chase script runs on it as on `16.2.1`.
2. The voice load of call 10 is a fixed 10 s (`WaitForCallLoad`), so the backup sabotage (15 s after the call trigger) comes
   before the call's own fade (8 s after the call starts). `notifyengineblown` is the sabotage plus 6 s, or the car coasting
   under 3 m/s after 1.5 s. Then the 1 s timer ends the race (F-16). The screen fades to black over 3 s and stays black until
   the next race or movie takes the screen.
3. A scripted end counts as a win of the event (the chain and the resume read it) and goes through the race outro: `16.1.0`
   leads to `16.2.2`; `16.2.1` plays `storyfmv_bus12` and then opens the car lot.
4. The flashback races open on a days-ago card: `16.2.2` "Six Days Ago..." (message 6), `16.2.3` "Four Days Ago..." (4),
   `16.1.1` "Two Days Ago..." (message 2, from the data). The texts are the install's (`0x4CBBA984`, `0x2117EECC`,
   `0xA461D64A`; `^` is a line break). Messages 6 and 4 are not in the race data **[unconfirmed]**.
5. The car lot is platform 5: its room is `FRONTEND/PLATFORMS/CAR_LOT.BIN`, the camera `angle_entry_car_lot` onto
   `angle_final_car_lot` and the screen pose `carlot_car_select`, the car stood at 340 degrees. It replaces the world; leaving
   a lot zone starts the world again at the zone.
6. Until the first safe house is reached (stage 16) the map shows only that safe house, and the career row says "Start Career".
   A career that won `16.2.1` resumes in the lot while it owns no car, and on the drive to the safe house after that.
