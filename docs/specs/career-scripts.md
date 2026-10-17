# Career scripts: the Lua state graphs in `gameplay.bin`

What the 258 compiled Lua handlers of vault `gpcore` do, written from a disassembly of every distinct body. This file
covers the runtime model, the inventory, the Blacklist bin graph with its phone-message table, and the answers to the
open questions of [career-blacklist.md](career-blacklist.md). The per-event scripts (prologue, final race, race kinds,
engage triggers, challenge ladder, OPM) are in [career-scripts-events.md](career-scripts-events.md). Container and
compression: [attributes.md](../formats/attributes.md#blobs-lua-bytecode-verified).

- **Sources:** `speed.exe` (Black Edition PC v1.3, base `0x00400000`) through the Ghidra MCP and a linear
  disassembly; `GLOBAL/gameplay.bin` and `GLOBAL/ATTRIBUTES.BIN` read with the repo's `blackbox-attrib`; the two
  local save files for structure only. No script text, bytecode or decompiler output is copied; rules are restated.
- **Evidence tags:** **[verified]** read from the install data (scripts, collections, saves). **[exe]** read in
  `speed.exe`, not tested in game. **[unconfirmed]** a guess. Addresses are `speed.exe` addresses; Ghidra names are
  only in [the provenance file](../provenance/career-scripts.md).

## 1. Runtime model

### 1.1 Graphs, states, handlers **[verified]**

Each script class (`race_bin`, `basicrace`, `drive_to_target`, ...) is a state graph. A `state` collection names a
state of one class (71 states). A `messagehandler` collection holds one compiled function for one (class, state,
event) triple, named `stategraph_<class>_<state>_handler_<event>` with an optional `_N` suffix for override
copies. The handler fields: owner collection, state, an optional list of target states (documentation only), and the
bytecode blob. The 259 blobs are 258 handlers plus the empty template `messagehandler`. Among the 258 handlers there
are 203 distinct function bodies; one of them, an empty function, is shared by 31 handlers.

Every handler is one function with three arguments: the owning collection's attributes (`self`), the incoming message
(`msg`), and the instance's persistent table (`st`, a Lua table saved with the game). The handler may call
`ChangeState("name")`, which does not run at once: it posts a request and the state changes afterwards. **[exe]**

A copy with a `_N` suffix belongs to a specific collection (a bin, a race) through its owner field. Such copies run
**in addition to** the class handler, not instead of it: the bin copies only queue phone messages, and the generic
copy is the only one that moves the graph on. **[unconfirmed]** (not seen in the code, but the graph would stall
otherwise; the generic `initial` handler is what loads `unlock_races`, and bin 14 has an `initial` copy.)

### 1.2 Events (handler names) **[verified]**

`stateenter`, `stateexit`, `triggerenter`, `triggerexit`, `notifysimtick`, `notifytimer`, `notifyraceplacement`,
`notifyfinished`, `notifychallengepassed`, `notifyplayerrep`, `notifymilestoneprogress`, `notifymilestonereached`,
`notifyspeedtrap`, `notifymoviefinished`, `notifymessagedone`, `niscomplete`, `enteringgameplay`,
`entersafehouse`, `acceptentercareerevent`, `declineentercareerevent`, `perpbusted`, `perpescaped`,
`restartrace`, `loadingcomplete`, `countdowndone`, `flowreadyforoutro`, `enterfreeroam`, `careerintrodone`,
`notifycellcallstarted`, `notifycellcallcomplete`, `notifyspeechstatus`, `notifyengineblown`,
`notifyknockedout`, `notifyvehicledestroyed`, `notifyraceabandoned`, `notifyonlineraceover`, `icecamerafinished`,
`notifyracetimeexpired`, `notifyracetimesectick`. The C++ side raises them as messages (for example
`MNotifyRacePlacement`, `MNotifyPlayerRep`, `MNotifyChallengePassed`, `MNotifyFinished`,
`MNotifyMilestoneProgress`, `MNotifyMilestoneReached`, `MAcceptEnterCareerEvent`, `MDeclineEnterCareerEvent`).

### 1.3 Script API, by Lua table **[exe]**

The registry at `0x0061E750` binds names to native handlers; the Lua table each name lives in:

| Table | Names (handler address in section 1.4 where career-relevant) |
|---|---|
| `Activity` | `Run`, `Suspend` |
| `Audio` | `SetFlag`, `IsCopSpeechPlaying` |
| `Bin` | `GetNumChallengesPassed` (`0x00604A10`), `GetNumRacesWon` (`0x00604A30`) |
| `MiniMap` | `AddEngagedRace` (`0x00604B50`) |
| `Math`, `Camera`, `NIS`, `Movie`, `Debug`, `Demo`, `HUD`, `Table` | utilities; `Demo.SetRaceCompleteForFE` and `Demo.StorePursuitRepForFE` are one-instruction stubs |
| `Game` | race, trigger, cop, traffic and career calls: `UnlockRace`, `IsRaceUnlocked`, `IsRaceCompleted`, `GetPlayerBounty`, `AwardPlayerBounty`, `AwardCash`, `AwardPoints`, `ChallengeComplete`, `ChallengeCompleted`, `NotifyRaceFinished`, `NotifyRacePlacement`, `CalculateRanking`, `SetWorldHeat`, `SetCopsEnabled`, `ForcePursuitStart`, `NoNewPursuitsOrCops`, `JumpToCarLot`, `JumpToSafeHouse`, `DoSafeHouseIntro`, `DoZoneMenuAction`, `EnterGateZone`, `ExitGateZone`, `IsCareerMode`, `SetHasRapSheet`, `SkipCareerIntro`, `IntroduceRival` (stub), ... |
| `RaceStatus` (method calls) | `ClearRacers`, `AddCheckpoint`, `SetRoaming`, `DisableBarriers`, `EnableBinBarriers`, `SetHasBeenWon`, ... |
| global `E*` functions | the event library: 161 names, of which about 100 have a Lua wrapper (`EAddSMS`, `ECellCall`, `EBailPursuit`, `EPlayRaceMovie`, `EJumpToStrategyFlow`, ..., `EEnterBin`) |

Event names are hashed with lookup2 (initial value `0xABCDEF00`); the sorted hash table is at `0x008A5418`, the Lua
wrappers at `0x008F6450`, the event constructors at `0x008F61C8` and the native event handlers at `0x008F5F40`.

### 1.4 What the career-relevant calls do **[exe]**

| Call | Effect |
|---|---|
| `Game.UnlockRace(r)` | race record flag 8, and flag 4 unless the race has attribute `0xA4E6FCFD`; refresh map icons |
| `Game.IsRaceUnlocked(r)` / `IsRaceCompleted(r)` | record flag 8 / flag 2 (career completion) |
| `Bin.GetNumRacesWon(n)` / `GetNumChallengesPassed(n)` | the two u16 counters of bin `n` |
| `Game.GetPlayerBounty()` | stable bounty total |
| `Game.AwardPlayerBounty(x)` | adds `x` (scaled) to the current car's career bounty; sends `MNotifyPlayerRep(RepPoints = total)` |
| `Game.AwardCash(racer, x)` = `AwardPoints` | adds `x` to the racer's in-race total; never touches the profile |
| `Game.ChallengeCompleted()` | broadcasts `MNotifyChallengePassed` |
| `Game.ChallengeComplete(racer)` | marks the racer's race-status entry as challenge complete |
| `Game.NotifyRaceFinished(racer)` | records the finish: best result, completed flag, cash, rewards, unlock list (blacklist spec 4.2) |
| `Game.NotifyRacePlacement(race, racer, place)` | broadcasts `MNotifyRacePlacement` with `Activity`, `Racer`, `Placing` |
| `Game.SetWorldHeat(n)` | sets the world heat object's value (`0x00612660`); meaning of the number **[unconfirmed]** |
| `Game.SetHasRapSheet()` | career flag 0x10 |
| `Game.DoSafeHouseIntro(marker)` | stores the respawn marker, sets CurrentBin to 16, opens the safe-house front end |
| `Game.DoZoneMenuAction(zone)` | stores the respawn marker; zone `safehouse` opens `MainMenu_Sub.fng`, `carlot` opens `Car_Select.fng`, `customshop` opens `CustomizeMain.fng` |
| `Game.EnterGateZone` / `ExitGateZone` | show or hide the "engage" prompt icon for the zone |
| `EAddSMS(id)` | adds phone message `id` to the inbox |
| `ECellCall(id)` | starts a phone call scene |

## 2. Inventory

| Class (graph) | Handlers | States | Instances in this install |
|---|---|---|---|
| `basicrace` (every race, with per-event copies) | 116 | abandoned, countdown, done, initial, intro_message, intromovie, intronis, onlineover, outro_message, outromovie, outronis, postrace, prerace, raceover, racing, tutorial | 276 collections inherit it |
| `race_bin` | 62 | initial, unlock_races, introduce_rival, check_rival, rival_locked, rival_unlocked, done | 20 bins |
| `drive_to_target` | 17 | initial, activate_gps, driving, at_target, outro_movie, done | 1 (`race_bin_16/drive_to_safe_house`) |
| `pursuit_mini_game` | 19 | initial, racing, busted, escaped, survived, postpursuit, outronis, restarting, done | 1 (OPM) |
| `bin_race_engage` | 8 | initial, wait, prompt, reset | 15 (one `engage` child per bin 1..15) |
| `world_engage` | 8 | initial, wait, prompt, reset | 4 (OPM test races) |
| `menu_gate` | 6 | initial, nis | 3 (`menu_gate_safehouse`, `_carlot`, `_customshop`) |
| `speedtrap_group_monitor` | 2 | initial, monitor | 1 (`speedtraps`) |
| `milestone_monitor` | 0 | initial | 1 (`milestones`), no handler |
| `milestone_challenge` | 3 | initial, done | none |
| `speedtrap_activity`, `speedtrap_challenge_engage` | 3, 2 | | none |
| `cell_engage` | 7 | initial, ring, delay, disable | none |
| `pursuit_zone`, `reputation_unlock`, `completion_unlock` | 1, 2, 2 | | none |

Classes with no instance are dead code in this install: milestones are not script driven (they are a native table,
blacklist spec 4.4); the speed-trap pass is handled by the group monitor; `reputation_unlock` and `completion_unlock`
(unlock activities after a bounty level or after n races of a list) are unused. **[verified]**

## 3. The Blacklist bin graph

Mechanics and gate formula are in [career-blacklist.md](career-blacklist.md) section 2; this section adds the
details read from every override.

### 3.1 When a bin's graph exists

A graph starts when its bin is entered. Entering loads the bin's collection and starts its `initial` state. The
saved table for a bin that was never entered does not exist. The two local saves agree: the save at CurrentBin 15
holds a table for bin 15 only (state `rival_locked`); the other save (CurrentBin 1) holds tables for all bins, with
bins 2..15 in `done`, bin 1 in `rival_unlocked` and bin 16 in `initial`. **[exe]** + **[verified]**. That a graph is
started by entering its bin is inferred from this and from the entry messages (they would all arrive at once
otherwise); the loading code (`0x0062C2D0` loads the bin's instance when it is not yet loaded) supports it.

### 3.2 State-by-state rules

- **initial:** if the bin has an `EntryActivity` and that activity auto-starts (bin 16), do nothing and wait for the
  activity's `NotifyFinished`; if it does not auto-start, run it and wait. With no entry activity: queue
  `EntryCellCallID` if it is not 0, then go to `unlock_races`. When the entry activity finishes: go to
  `unlock_races`, and queue `EntryCellCallID` if it is not 0. (This corrects the earlier text of the other spec.)
- **unlock_races, introduce_rival, check_rival:** as in the other spec (the introduce step calls the stub).
- **rival_locked:** re-check on entering gameplay, on any bounty change, on a first-place finish of one of the bin's
  `WorldRaces`, and on `NotifyChallengePassed`. The generic challenge handler increments its own counter and, when it
  reaches `RequiredChallenges`, queues `SMSCellChallenge` (if 0 or more) and unlocks `CellChallengeRace`. The
  per-bin copies use the native counter instead (section 3.3). Bin 15 has `CellChallengeRace` = event `1.2.2` and
  `SMSCellChallenge` -1.
- **rival_unlocked:** queue `SMSRivalChallenge` if 0 or more, unlock every `BossRaces` entry, finish at once if
  there are none. On a first-place finish of a boss race count it; when all are won go to `done`.
- **done:** unlock every `WorldRaces` entry. Nothing else; in particular the script does not advance CurrentBin and
  does not start the next bin (section 5).

### 3.3 Phone messages and world heat per bin **[verified]**

Per-bin copies fire at these points. "wins n" = the bin's native races-won counter equals n right after a first
place (any first-place finish in the game triggers the check, so repeated wins at the same count may repeat the
message **[unconfirmed]**). "challenges >= k" = native challenges-passed counter at least k, each message once
(flags `DoneFirstSMS`, `DoneSecondSMS`, `DoneThirdSMS`).

| Bin | Entry message | Race wins -> message | Challenges -> message | Rival unlocks |
|---|---|---|---|---|
| 15 | none | 1: 123 | >= 1: 124, >= 3: 137 | 20 |
| 14 | 125 and 126 at start of the bin | 1: 18, 2: 27 | >= 2: 19, >= 3: 22 | 29 |
| 13 | 30 | 1: 23, 2: 24, 4: 31 | >= 1: 35, >= 2: 25, >= 3: 26 | 36 |
| 12 | 37 | 1: 127, 2: 17, 4: 40 | >= 1: 33 | 41 |
| 11 | 42 | 2: 128, 4: 130 | >= 2: 38 | 46 |
| 10 | 48 | 3: 49 | >= 2: 132 | 52 |
| 9 | 54 | 2: 53, 5: 55 | >= 2: 43 | 57 |
| 8 | 58 | 2: 59, 4: 60 | >= 3: 61 | 63 |
| 7 | 64 | 1: 66, 4: world heat 1 and 65, 7: 134 | >= 2: 133 | 69 |
| 6 | 70 | 3: 135 | >= 2: 100 | 72 |
| 5 | 73 | none | >= 3: 101 | 76 |
| 4 | 77 | 4: 136 | >= 4: 102 | 79 |
| 3 | 80 | 2: 81, 4: 82 | none | 84 |
| 2 | 85 | 4: world heat 5 and 86 | none | 88 |
| 1 | 89 | 4: 90, 6: 91 | none | 93 and 94 (script copy) |

"Entry message" and "Rival unlocks" for bins 2..13 and 15 are the data fields `EntryCellCallID` and
`SMSRivalChallenge`. Bin 13 also calls `SetHasRapSheet` when it starts. The message ids index the phone table, which
was not read; the text and sender are therefore unknown. Bin 14 has no `EntryCellCallID` but its start copy queues
two fixed messages. The bin 16 prologue queues message 15 when the drive to the safe house starts and message 121
after 120 s without arriving (section 6 of the events file).

## 4. Safe-house entry **[exe]** + **[verified]**

The three `menu_gate` collections (safe house with four zones, car lot, custom shop with eight) each run one graph:
a zone trigger entered by the player raises `EnterGateZone` (shows the prompt) if menu gates are allowed (and for a
safe house if safe-house engagement is allowed); leaving it raises `ExitGateZone`. The message `EnterSafehouse`
(sent by the front end when the player confirms) calls `DoZoneMenuAction`: the respawn marker of the zone is stored
(a second marker slot is stored for safe houses) and the menu opens: `MainMenu_Sub.fng`. If the gate has an intro
NIS the graph plays it first and calls the menu action when it ends.

There is **no heat or cop reset in any script** on safe-house entry. Whether native code clears the pursuit on
opening the menu is not part of the scripts and was not traced here. **[unconfirmed]**

## 5. Answers to the earlier open questions

1. **What advances CurrentBin after bin 15.** No script does. The bin-entering routine (`0x0062C2D0`) has three
   callers: the first-visit step of the safe-house front end (16 to 15), the Lua function `EEnterBin(n)`
   (`0x0062C4C0`) and the handler of the event of the same name (`0x0062C470`); the event constructor is a
   one-instruction stub. `EEnterBin` appears in none of the 258 handlers, in no NIS scene, no world event list and no
   other install file (lookup2 hash `0x2AF7332B`, also checked in the other byte order, as `EnterBin`, as a
   multiplicative hash and as plain text). No native code writes the CurrentBin byte except that routine and the
   three initialisers (new career 16, debug start 15, `DoSafeHouseIntro` 16). The result is a gap: the original
   progression through bins 14..1 must reach that routine from somewhere that is not in the executable's call graph
   or in the shipped data **[unconfirmed]**. A rewrite should enter bin N-1 when bin N reaches `done`.
2. **OPM.** `race_bin_opm` (index 20) is a developer test bin: events `20.1.1` circuit test, `20.1.2` "MaxLoad",
   `20.2.1` rival race against `mr_x` (car `rx8`), `20.2.2` a "habib test", `20.5.1` speed traps, `20.8.1` "escape
   the cops" (30 s), `99.1.1` online circuit, with `world_engage` triggers, a `pursuit_mini_game` after the rival
   race, and the front-end names `OPM_SafehouseRaceSheet.fng` and `OPMRivalIntro`. Nothing unlocks or enters it:
   no race lists its events as an unlock, and its bin is never entered. The expansion of "OPM" is unknown.
   **[verified]** (data) **[unconfirmed]** (purpose).
3. **Safe-house entry effects:** section 4.
4. **Bin messages:** section 3.3.
5. **Career flags set from Lua:** only `Game.SetHasRapSheet` (flag 0x10, bin 13). All other flags are native.

## 6. Corrections to the first spec

See the list at the end of [career-blacklist.md](career-blacklist.md); the main ones are the `initial` handler
(section 3.2), the missing state-by-state SMS ids, the finished challenge series ladder and the prologue chain.
