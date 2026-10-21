# Story flow, part 2: messages, rivals, the final race and the open facts

Continues [story-flow.md](story-flow.md) (sections 0 to 2: the race life cycle, the prologue and the first visit). This
part holds the text messages and calls (section 3), the rival bins and gauntlets (section 4), the final race (section 5),
the list of facts still missing (section 6) and the conflicts between the source specs (section 7). Source keys and
tags are as in story-flow.md.

## 3. Text messages and cell calls (machine F)

Tags: **[verified]** for the ids, senders and the per-bin table (CP 5.1, CS 3.3); **[exe]** for the delivery rules
(CP 3.1 to 3.6).

### 3.1 The two calls

| Call | Argument | Effect | Source |
|---|---|---|---|
| `EAddSMS(id)` | message id | Deliver at once or queue (3.2) | CP 2, CP 3.1 |
| `ECellCall(id)` | call id | A phone call: a voice take or, in a text language, a message (3.4) | CP 2, CP 3.3 |
| `EShowSMS(id)` | slot, or a negative number for the list | Opens the inbox. No script uses it. | CP 2 |

### 3.2 `EAddSMS` **[exe]**

1. If the id is 10 to 14, 95 or 96, deliver it now (step 4).
2. Else read the slot. If the slot is absent or in state 0 (never delivered, or deleted) and the id is not already in the
   pending list, append the id to the pending list, oldest first.
3. A slot in state 2 or 4 ignores the call. So a "wins = n" message cannot repeat.
4. Deliver: if the slot exists and is in state 0, set state 2, add 1 to the sequence counter and give that number to the
   slot. Then post `ECellCall(id)`.

### 3.3 The flush **[exe]**

Runs once per game update, after the race-status update.

1. If the pending list is empty, stop.
2. If the can-show test (table below) is false, wait. There is no timeout.
3. Take the pending ids oldest first. For each one whose slot exists: if it is a story message and none is chosen yet,
   choose it; set the slot to state 2; add 1 to the counter and give it to the slot.
4. Clear the pending list. If no story message was chosen, choose the oldest id.
5. Open the inbox at the chosen id and tell the HUD engage object.

| # | Can-show test (all must hold) | Tag |
|---|---|---|
| 1 | Game state is in game (6), and the disabling globals are zero | [exe] |
| 2 | The race status exists and its state is free roam (0) | [exe] |
| 3 | The front end may open an overlay (no photo screen and no fade screen up) | [exe] |
| 4 | The speech event `cellcall` (204) is neither queued nor playing | [exe] |
| 5 | The player car exists, is not wrecked or loading, and its interface reports no pursuit (the three car calls are not named: F-27) | [exe] |

For `ECellCall` in a text language only item 3 is tested (CP 3.5).

### 3.4 `ECellCall` by language **[exe]**

| Class | Languages (id) | Effect |
|---|---|---|
| Text | Dutch (5), Swedish (6), Danish (7), Polish (12), Finnish (13) | If item 3 holds: set the slot to state 2 with no sequence number (if the slot exists). Push a pause, then open `SMS_Mailboxes.fng` at the slot (or at the list for a negative id). If item 3 fails the call is lost and the slot stays in state 0. |
| Voice | English (0), French (1), German (2), Italian (3), Spanish (4) | If the id is 10 to 105: queue a `CellCall` speech request with its group and line, then flush the speech queue. The inbox is not touched. |

Consequence: the ids sent only by `ECellCall` (10, 11, 12, 13, 95, 96, see 3.5) reach the inbox only in text languages.
In voice languages they play as voice only. This is what the code does (CP 3.3). Whether it is intended is not stated.

### 3.5 Who sends what **[verified]**

| Id | Sent by | Call type | Condition | Source |
|---|---|---|---|---|
| 10 | Lua, 16.1.0 and 16.2.1, at `call_from_mia` | `ECellCall` | Player enters the trigger | CP 5.1, CSE 2.2 |
| 11, 13 | NIS `EndingNis03` (event 16.8.1) | `ECellCall` in the scene | Scene plays | CP 5.1 |
| 12 | NIS `IntroNis10` (intro of 16.1.1) | `ECellCall` in the scene | Scene plays | CP 5.1 |
| 15 | `drive_to_target`, initial | `EAddSMS` | Unless `SkipCareerIntro` | CSE 7.1, CF 6 |
| 121 | `drive_to_target`, `hurry` timer | `EAddSMS` | 120 s after the drive starts | CSE 7.1, CP 5.1 |
| 95 | Race 1.8.1, `ECellCall` | `ECellCall` | 10 s after the start, once | CSE 4, CP 5.1 |
| 96 | Race 1.8.1 | `ECellCall` | Over 300 s and within 500 m of the barrier area, once | CSE 4, CP 5.1 |
| 93, 94 | Bin 1 when its rival unlocks (script copy) | `EAddSMS` | Rival unlock | CS 3.3 |
| 120 | Native impound code | Native | The impound countdown of a car reaches 0 (car state 4), after a career finish | CP 5.1, F-26 |
| 122 | Never sent | | | CP 5.3 |
| Bin messages | Per-bin copies of the bin graph | `EAddSMS` | Table 3.6 | CS 3.3 |

Never sent in this build: 16, 21, 32, 47, 50, 74, 97, 98, 105, 110, 111, 122, 129 (CP 5.3). Message 120 and 121 are
conditional.

### 3.6 Per-bin messages **[verified]** (CS 3.3)

"Wins = n": the bin's races-won counter equals n right after a first place. "Challenges >= k": the challenges counter is at
least k; each such message is sent once (flags `DoneFirstSMS`, `DoneSecondSMS`, `DoneThirdSMS`). A message already in the
inbox is not sent again (3.2).

| Bin | Entry (queued at start) | Wins = n: message | Challenges >= k: message | Rival unlock | World heat (`SetWorldHeat`) |
|---|---|---|---|---|---|
| 15 | none | 1: 123 | 1: 124; 3: 137 | 20 | none |
| 14 | 125 and 126 (start copy) | 1: 18; 2: 27 | 2: 19; 3: 22 | 29 | none |
| 13 | 30 (and `SetHasRapSheet`) | 1: 23; 2: 24; 4: 31 | 1: 35; 2: 25; 3: 26 | 36 | none |
| 12 | 37 | 1: 127; 2: 17; 4: 40 | 1: 33 | 41 | none |
| 11 | 42 | 2: 128; 4: 130 | 2: 38 | 46 | none |
| 10 | 48 | 3: 49 | 2: 132 | 52 | none |
| 9 | 54 | 2: 53; 5: 55 | 2: 43 | 57 | none |
| 8 | 58 | 2: 59; 4: 60 | 3: 61 | 63 | none |
| 7 | 64 | 1: 66; 4: 65; 7: 134 | 2: 133 | 69 | 1 at 4 wins |
| 6 | 70 | 3: 135 | 2: 100 | 72 | none |
| 5 | 73 | none | 3: 101 | 76 | none |
| 4 | 77 | 4: 136 | 4: 102 | 79 | none |
| 3 | 80 | 2: 81; 4: 82 | none | 84 | none |
| 2 | 85 | 4: 86 | none | 88 | 5 at 4 wins |
| 1 | 89 | 4: 90; 6: 91 | none | 93 and 94 (script copy) | none |

Rival unlock is the bin's `SMSRivalChallenge`. The rival unlock is sent in `rival_unlocked` (section 4.1).

### 3.7 What the player sees and hears **[exe]** + **[verified]**

1. Two screens: `SMS_Mailboxes.fng` (the list) and `SMS_Message.fng` (one message). The list has two tabs, story and tips.
   Tips are the non-story ids: 110, 111, 120 to 130 and 132 to 137 (CP 1.2).
2. The list shows the slots in state 2 or 4, newest sequence first. Slots with sequence 0 sort last. An unread row shows
   its `NEW_MESSAGE_ICON` (CP 4.1).
3. Opening a message: if it is a story message and unread, its voice line plays. The state becomes 4 (read).
4. Button 4 replays the voice (story messages only). Button 5 asks for confirmation (`InGameDialog.fng`); "yes" sets the
   state back to 0 (CP 4.3). The key mapping is F-28.
5. Opening the inbox over gameplay pauses the game, sets the audio flag for SMS and hides the incoming widget (CP 3.6).
6. Voice: 75 of the 76 valid ids from 10 to 105 have a take. Id 16 has none. The group is the bin (16 for the prologue,
   0 for the post-game set). The selector rule is F-29 (CP 1.4).
7. The gameplay HUD has no new-message icon (CP 3.6).

## 4. The rival flow (machines G and H)

### 4.1 The bin graph (machine G) **[verified]** (CB 2.1 to 2.6, CS 3.2)

Each of the 15 rival bins runs this graph. A bin's graph starts when the bin is entered (CS 3.1). Per-bin copies run in
addition to the generic handler (CS 1.1).

| State | On enter | Transitions |
|---|---|---|
| initial | Bin 16 only: its entry activity runs and the state waits for its `NotifyFinished`. Other bins with no entry activity queue `EntryCellCallID` if it is not 0. Bins 4 to 15 also reset the SMS flags `DoneFirstSMS`, `DoneSecondSMS` and `DoneThirdSMS`. | unlock_races |
| unlock_races | Reset the bin's counters to 0. `NumWorldRaces` = length of `WorldRaces`; `NumRivalRaces` = length of `BossRaces`. Each `engage` trigger with `InitiallyUnlocked` set unlocks its race. | introduce_rival |
| introduce_rival | Call `IntroduceRival` (a stub in this build). | check_rival |
| check_rival | Recompute the three counters and apply the gate (4.2). | rival_unlocked if all three hold; else rival_locked |
| rival_locked | Re-check on entering gameplay, on each bounty change, on a first-place finish of a `WorldRaces` race, and on a challenge passed. A challenge passed adds 1 to the bin's counter. At `RequiredChallenges` queue `SMSCellChallenge` (if 0 or more) and unlock `CellChallengeRace` (bin 15: race `1.2.2`). The per-bin copies use the native counter instead (CS 3.3). | check_rival |
| rival_unlocked | Queue `SMSRivalChallenge` (if 0 or more). Unlock every `BossRaces` entry. With no boss races, go to done. A first-place finish of a boss race adds 1 to `NumRivalRacesWon`; at `NumRivalRaces` go to done. | done |
| done | Unlock every `WorldRaces` entry. The graph does not advance `CurrentBin`. | end (bin advance: 4.7) |

The bin's state is saved as the lookup2 hash of its name (CB 2.6). `done` is `0x32956993`, `rival_locked` `0x2ABB39C2`,
`rival_unlocked` `0x76F21CB8`, `initial` `0x904ECCE4`.

### 4.2 The gate **[verified]** (CB 2.5, 3.1, 3.2, 4.3)

A rival unlocks when all three counters meet the bin's values. All comparisons are "at least".

1. Races won: the count of the bin's `WorldRaces` whose record has flag `0x2`. Boss races do not count.
2. Total bounty: the stable's total (the owned cars' career records plus the banked amount, stable offset `0x8CA0`).
3. Challenges passed: the bin's milestones in state 4, plus its speed-trap entries with flag 4.

Counter 1 depends on F-10 (what sets flag `0x2`).

| Bin | Rival (alias) | RequiredRacesWon | RequiredChallenges | RequiredBounty | WorldRaces | BossRaces (count) |
|---|---|---|---|---|---|---|
| 15 | tony_ho (Sonny) | 3 | 3 | 20,000 | 3 | 2 |
| 14 | vince_kilic (Taz) | 4 | 3 | 50,000 | 6 | 2 |
| 13 | victor_vasquez (Vic) | 4 | 3 | 100,000 | 7 | 2 |
| 12 | isabel_diaz (Izzy) | 4 | 3 | 180,000 | 7 | 2 |
| 11 | lou_park (Big Lou) | 5 | 3 | 300,000 | 8 | 2 |
| 10 | karl_smit (Baron) | 5 | 4 | 500,000 | 8 | 2 |
| 9 | pierre_dupont (Earl) | 5 | 4 | 790,000 | 8 | 2 |
| 8 | jade_barrett (Jewels) | 5 | 4 | 1,180,000 | 7 | 2 |
| 7 | kira_nazakato (Kaze) | 7 | 4 | 1,680,000 | 10 | 2 |
| 6 | hector_domingo (Ming) | 7 | 4 | 2,300,000 | 11 | 2 |
| 5 | wes_allen (Webster) | 7 | 4 | 3,050,000 | 10 | 3 |
| 4 | joe_vega (JV) | 7 | 5 | 4,050,000 | 11 | 3 |
| 3 | ronnie_mccrea (Ronnie) | 8 | 5 | 5,550,000 | 12 | 3 |
| 2 | toro_sato (Bull) | 8 | 5 | 7,550,000 | 12 | 3 |
| 1 | razor_callahan (Razor) | 9 | 5 | 10,000,000 | 11 | 6 |

### 4.3 The challenge button **[exe]** (CB 3.3)

The rival challenge screen (`SafeHouseRivalChallenge.fng`) shows the selected bin's three counters against the gate.
Its button reads `RIVAL_CHALLENGE_ACTIVATED` when any of these holds, else `RIVAL_CHALLENGE_DEACTIVATED`:

1. Some `BossRaces` entry is unlocked (flag 8) and not completed (flag 2).
2. The career-completed flag (settings bit 14) is set.
3. The selected bin number is higher than `CurrentBin` (a beaten rival can be replayed).

Starting a boss race sets the pending node and mode and leaves the front end (CSH2 6.2). Gauntlet order: 4.4.

### 4.4 The gauntlets **[verified]** (CSE 3)

Boss races of a bin run back to back. Each boss race except the last names the next one in its `PostRaceActivity`
(section 1.5 of story-flow.md). The order per bin, in the gate table of 4.2:

| Bin | Boss races, in order |
|---|---|
| 15 | 15.1.2 circuit, 15.1.1.r circuit reversed |
| 14 | 14.2.4 sprint, 15.2.1.r sprint reversed |
| 13 | 13.2.3 sprint, 14.2.2.r sprint reversed |
| 12 | 12.2.1 sprint, 12.1.2 circuit |
| 11 | 11.1.2 circuit, 12.1.2.r circuit reversed |
| 10 | 10.2.1 sprint, 10.7.3 drag |
| 9 | 9.5.3 speed trap, 9.2.1 sprint |
| 8 | 8.2.1 sprint, 8.7.5 drag |
| 7 | 7.2.2 p2p, 7.2.3 p2p |
| 6 | 6.1.1 circuit, 6.5.2 speed trap |
| 5 | 5.2.2 sprint, 5.5.4 speed trap, 5.2.1.r sprint reversed |
| 4 | 4.5.2 speed trap, 4.7.4 drag, 4.5.1.r speed trap reversed |
| 3 | 3.1.2.r circuit, 3.2.2 sprint, 3.1.2 circuit |
| 2 | 3.1.1.r circuit, 9.2.2.r sprint, 2.2.1 sprint |
| 1 | 1.5.2 speed trap, 1.7.3 drag, 4.2.1 sprint, 1.1.2 circuit, 1.2.3 sprint, 1.8.1 (final race, section 5) |

Rules:

1. The last race of each gauntlet carries the reward set (4.5). Earlier races carry none.
2. The chain runs on a win. Whether a loss stops it: F-09.
3. Several bins borrow event ids of other bins. An event id does not give the bin.
4. Each boss race can name an intro NIS. The game data holds `IntroNisBL02` to `IntroNisBL07` and `IntroNisBL09` to
   `IntroNisBL16` (names, this pass). `IntroNisBL01` and `IntroNisBL08` have scene bundles on disk but no race names them.
   Which race uses which: F-22.

### 4.5 Rewards **[exe]** (CB 4.2, CET 6.5, SAVE 3.2, CSE 3)

| Kind | Reward | Where it goes |
|---|---|---|
| 0x11 | Get Out of Jail Free | Marker inventory, type 0x11, active flag 1 |
| 0x12 | Pink slip: the rival's car, added to the garage | Applied at once. Not stored in the inventory. |
| 0x13 | Cash bonus | Added to the profile cash at once. Not stored. |
| 0x14 | Extra impound strike | Marker inventory, type 0x14 |
| 0x15 | Impound release | Marker inventory, type 0x15 |
| 1 to 16 | Part unlocks: 1 brakes, 2 engine, 3 nitrous, 4 induction, 5 chassis, 6 tires, 7 transmission, 8 body kit, 9 hood, 10 spoiler, 11 rims, 12 roof scoop, 13 HUD, 14 vinyls, 15 decals, 16 paint | Marker inventory, that type |

Routing (CET 6.5): a non-cash, non-car kind goes to the inventory (first free slot, no duplicate check) when the apply flag
is non-zero. When the flag is zero it goes to the popup list (6 slots at `0x0091DC1C`), which the markers screen reads in
4.6. The flag: F-25.

The reward list is granted only on the first completion (CB 4.2, step 5). The install's counts (CSE 3): 15 pink slips,
13 out-of-jail-free, 16 back-room unlocks, 8 impound boxes, 6 impound releases, 8 cash bonuses.
The prologue race 16.2.3 grants three out-of-jail-free markers (CSE 2.1).

### 4.6 The arrival sequence after a rival is beaten **[exe]** (CSH 5.3, 5.4, CSH2 11, CB 5.4)

This is the corrected order (CSH 5.3). It replaces the order in CF 7.3, which put the autosave after the region screen
(conflict C-12). A stepper walks the steps below; the counter starts at -1 and adds 1 on each call.

| Step | Screen or action | Condition | Notes |
|---|---|---|---|
| 0 | `SafeHouseRivalChallenge.fng` | Always | Context bit 17 (`0x20000`, Blacklist sequence) is set. |
| 1 | `SafeHouseMarkers.fng` | Always | Marker choice (F-24). Reads the first six popup entries, keeps those whose reward type is in one of four categories, and shows up to four of them. |
| 2 | `SafeHouseRegionUnlock.fng` | Stage 8 or 12 only | Shows `RIVAL_09_BG` at stage 8, `RIVAL_13_BG` at stage 12. Changes no state. |
| 3 | `MC_Main.fng` autosave flow; `SafeHouseRivalBio.fng` opens when it ends | Autosave possible | Autosave possible: context bits 3 and 6 clear; memory-card layer present; profile autosave option (profile `+0x38`) on or the manual-save byte set; the current event not flagged `0xFF5EE5D6`; not the final event. |
| 4 | `SafeHouseRivalBio.fng` | Always. Reached at once when step 3 is skipped. | |
| 5 | Movie `blacklist_NN` in `FEAnyMovie.fng` (`WS_FEAnyMovie.fng` on wide screens) | Always | NN is the stage byte at this moment, two digits: the movie introduces the rival now faced (F-19). |
| 6 | By stage (below) | | |
| 7 | Broadcast on the gameplay subject; create a type-0 event; clear bit 17; end | | The player is in the safe house menu. |

Step 6 by stage:

| Stage | Action |
|---|---|
| 15 | Clear context bit 17. Set context bit 0 and settings bit 5 (`0x20`, "opening seen"). Open `SafehouseReputationOverview.fng`. End. |
| 13, rap sheet flag (settings bit 4, `0x10`) clear | Set the flag. Set the counter to 5. Play `storyfmv_rap30` (F-20). When it ends, run step 6 again; the flag is now set, so leave the front end. |
| Any other | Leave the front end (the Resume Free Roam path, CSH2 6.1). |

The front end starts the stepper (first call, counter -1 to 0) when the request carries the Blacklist-sequence request bit
(`0x400000`). **The first visit** (story-flow.md 2.6) opens the bio outside the stepper; on Accept, with context bit 17
set, the bio sets a counter of -1 to 4 and steps: step 5 plays `blacklist_15`, step 6 (stage 15) sets "opening seen"
and opens the reputation overview (F-18).

### 4.7 The bin advance (rewrite rule) **[unconfirmed]**

No script or native path was found that moves `CurrentBin` from N to N-1 (CB 5.3, CS 5). The rewrite uses:

1. When bin N reaches `done` (its last boss race won), enter bin N-1 at once.
2. Entering a bin (CB 5.2, **[exe]**): write `CurrentBin`; set each milestone of that bin from state 1 to 2; set the
   speed-trap entries' flags to `|= 3`; grant the bin's unlock list (unless a manager flag is set); mark the rival's
   `Boss` character; refresh the barriers and the race status bin.
3. Bin N-1's graph then starts at `initial` (4.1).
4. Bin 1 has no bin below it. Its last boss race is the final race (section 5).

Lookup: F-23.

### 4.8 Regions by stage **[verified]** (CB 6)

| `CurrentBin` | Barriers closed | Minimap file |
|---|---|---|
| 16 to 13 | 11, 12, 13, 14, 15 | `MINI_MAP_UNLOCK_1` |
| 12 to 9 | 10, 12, 13 | `MINI_MAP_UNLOCK_2` |
| 8 to 1 | none | `MINI_MAP` |

## 5. The final race 1.8.1 (machine I)

Tags: **[verified]** for the setup and triggers (CSE 4, CB 1, CB 9, RM 11); names **[verified]** (this pass); the
post-race return and screens **[exe]** (CB 7.1, CF 6, CSH 4, CR2 3).

### 5.1 Setup

| Field | Value | Source |
|---|---|---|
| Event id | 1.8.1, bin 1's last boss race | CB 1, CSE 3 |
| Vault and kind | `race_bin_01/epic_pursuit`, a `challengerace` boss race | CSE 4 |
| Forced heat | 6 (`ForceHeatLevel`) | CSE 4 |
| Start | Rolling start at speed 40 | CSE 4 |
| Car | `e3_demo_bmw` | CSE 4 |
| Cops | A scripted cop force | CSE 4 |
| Intro | Movie `Storyfmv_her136` (file `storyfmv_her136` exists, names this pass). No intro NIS is recorded. F-05. | CSE 4 |
| Outro | NIS `EndingNis04` (scene `Scene_EndingNis04_BundleB.bun` exists, this pass) | CSE 4 |
| Autosave | Off in this event (the final event) | CF 8 |
| Rewards | None recorded. F-25. | CB 1 |

### 5.2 Children (names, this pass)

`startgrid1`, `win_trigger`, `lose_trigger`, `barrier_area`, `win_nis_marker`, `taggedcarmarker1` to `taggedcarmarker6`
(all under `race_bin_01/epic_pursuit/`). The race's `Barriers` list can hold scenery group entries such as
`SCENERY_GROUP_DOOR` (GR 3.1); whether this race lists it: F-33. Roles of the children: F-32.

### 5.3 Timers and triggers

| Event | Condition | Effect | Source |
|---|---|---|---|
| Race start + 10 s | Once | Phone call 95 (`ECellCall`) | CSE 4, CP 5.1 |
| Race time over 300 s, and the car within 500 m of the barrier area | Once | Phone call 96. Open `SCENERY_GROUP_DOOR`. GPS guides the player to `barrier_area`. | CSE 4 |
| The player enters `barrier_area` | After the GPS is on | Hide the icon and the GPS | CSE 4 |
| The player enters `win_trigger` | Speed at least 180 km/h, and direction dot product at least 0.8 | The race is won (`PlayerHasWon`). Play `EndingNis04` as the outro. Call `EBailPursuit(0, 1)`. | CSE 4, CB 9 |
| The player enters `win_trigger` below either value | | No effect is recorded: F-31 | CSE 4 |
| `EndingNis04` completes | | `ChallengeComplete`, then `NotifyRaceFinished`. The race ends. The next state: F-11. | CSE 4 |
| The player enters `lose_trigger` | The race is not yet won | Detach the camera and wreck the racer | CSE 4 |

### 5.4 After the win

1. The post-race return (native) sets the career-completed flag: settings bit 14 (`0x4000`) (CB 7.1). It also unlocks
   the extra cars and lifts the shop tier limits (SAVE 3.3). Order and writers: F-35.
2. The front-end request for the final event, with its node complete, sets context bit 18 (`0x40000`, game won), platform
   2 and screen `GameWon.fng` (CF 6 step 5, CSH 4).
3. `GameWon.fng` closes into the end-game step routine. The routine's counter (global `0x0091CA54`) is set to 0 when the
   final event is won (CR2 3).

| Counter (read, then +1) | Screen pushed |
|---|---|
| 0 | `Credits.fng`. It rolls the credits while context bit 18 is set. |
| 1 | `RapSheetLogin_ENDGAME.fng` |
| 2 | `RapSheetLogin2_ENDGAME.fng`. Writes the profile name into its list when the counter is 3. |
| 3 | `RapSheetMain_ENDGAME.fng`. The warrant figures appear when the counter is 4. |
| 4 | `MainMenu_Sub.fng` |

4. Where bit 18 is cleared, and how `RapSheetMain_ENDGAME` ends (timeline or "SUBMIT"): F-34. **[exe]** (CR2 3).

### 5.5 Resume after the win

A load with race 1.2.3 complete and settings bit 14 clear activates 1.8.1 (story-flow.md section 2.7). Once bit 14 is set,
the rival challenges all open (CB 3.3) and the game-won path applies.

## 6. Facts still missing

Each row blocks a state machine above, or a screen the machines call. "Where" names the source to search: Ghidra on
`speed.exe` (addresses from the specs), the Lua disassembly of `gameplay.bin` (the 258 handlers of `gpcore`, not in the
repo), or the install data (the crate reader in `crates/nfsmw-data` or a throwaway reader).

| F-id | Blocks | What must be found | Where |
|---|---|---|---|
| F-01 | 1.2, 2.1 | The prologue races 16.1.0, 16.2.2, 16.2.3 and 16.2.1 each hold a movie and an NIS. Find which attribute holds each value (`IntroMovie`, `IntroNIS`), and whether the movie plays first or never plays. | Data: the four races' intro attributes. Lua intro_message handler of `basicrace`. |
| F-02 | 1.1 intromovie | The state that `intromovie` enters on `notifymoviefinished`: `intronis` (diagram, CSE 1.1) or `prerace`. | Lua: `basicrace` intromovie handler, the `ChangeState` target. |
| F-03 | 1.1 tutorial | What the native `PlayTutorial` does when no tutorial is due, and whether it posts the movie-finished message at once. | Ghidra: registry at `0x0061E750`, the `PlayTutorial` handler. |
| F-04 | 1.1 intro_message, outro_message | Which screen an `IntroMessageID` opens (example: 2 in 16.1.1), the outro message id, and the table of message ids to screens. The `IntroMessageID` value of each race. | Ghidra: the `EShowMessageScreen` handler and its table. Data: `IntroMessageID` of the races. |
| F-05 | 1.2, 5.1 | Per race: the values of `IntroNIS`, `IntroMovie`, `QuickRaceNIS` and `OutroNIS`, and the attribute hashes of these fields (not in the repo docs). Names known from this pass: `IntroNisBL02` to `07`, `09` to `16`, `IntroNisDD`, `IntroNis01` to `08`, `IntroNis10`, `GenericStart`, `IntroNisToll01`. | Data: attribute names hashed per `docs/formats/attributes.md`; read every race with a value. |
| F-06 | 1.2 step 2 | Whether the quick race test is the play mode (0 = quick race) of the running race, and whether it comes before or after the message screen. | Lua: `basicrace` intro_message. Race status play mode at `+0x1964` (RM). |
| F-07 | 1.1 countdown | The countdown length and cadence of the original. The rewrite uses 3 s. | Ghidra or the HUD package scripts (`321_GO_GROUP`); the Lua `countdowndone` path. RM 2 open question 1. |
| F-08 | 1.1 raceover, 1.4 | After a loss: does `raceover` go to postrace through the losing hook (CSE 1.1), or does a loss end at raceover (CSE 1.2)? RM 11 says a loss takes the same results screen. | Lua: `raceover` handler (the first-place test and the target). Ghidra: `ShowLosingPostRaceScreen` (a stub). |
| F-09 | 1.5, 2.3, 4.4 | Does `done` run `PostRaceActivity` after a lost race? The gauntlets and the prologue depend on it. RM 11 item 6 says a loss leaves the chain. | Lua: `basicrace` done handler, the test before the activity call. |
| F-10 | 1.6, 4.2 | **Answer [exe]:** only a human racer in first place (record `+0x10` = 1) sets `0x2` (career) or `0x1` (other modes); the career pays `CashValue` on every first place; rewards and unlocks only on the first win. Story-flow 1.6. | done |
| F-11 | 5.3, 5.4 | After `EndingNis04` completes in 1.8.1: the next state (`postrace` or `done`), and whether the post-race return opens `GameWon.fng` directly. | Lua: the `niscomplete` handler of the `epic_pursuit` copy. |
| F-12 | 2.1 step 4 | `16_1_1_circuit` holds an `outro_nis_marker` child (name, this pass), but CSE 2.1 and RM 11 say only `16_nis_rog_razor` sets `OutroNIS`. Is the outro of 16.1.1 set? | Data: `OutroNIS` and `OutroMessageID` of `16_1_1_circuit`. |
| F-13 | 2.1 | `CashValue` (attribute `0xD8BAA07B`) of 16.2.2 and 16.2.3. The CSE 2.1 text says steps 2 to 4 pay 10,000; its table shows it only for 16.1.1. | Data: `CashValue` of `16_2_2_sprint`, `16_2_3_sprint`. Lua done copies. |
| F-14 | 2.2 | **Answer [exe]:** settings bit 20 (`0x100000`, profile flags at `+0xAC`); the hook forces the `E3_DEMO_BMW` stable car (`0x03A94520`) for races with attribute `0x4393F69B` only while the bit is clear. The prologue's M3 GTR is the race's `PlayerCarType`. | done |
| F-15 | 2.4 | What blocks new pursuits at `bail_trigger` (CSE 2.2), and the arguments of `EDispIntroRace` and `EBailPursuit(1, 1)`. | Lua: the `bail_trigger` triggerenter handler in 16_2_1. Ghidra: the `E*` wrappers. |
| F-16 | 1.7, 2.4 | Which fade ends 16.2.1: the 8 s fade after the call starts (CP 5.2) or the 1 s fade after the engine blows (CSE 2.2). Both may hold. | Lua: `basicrace` racing handler for `notifycellcallstarted`; the 16_2_1 engine-blown handler. |
| F-17 | 2.5 | The behaviour of `drive_to_target` states `activate_gps` and `outro_movie` (CS 1.2 lists them; CSE 7.1 does not describe them), and whether `at_target` goes to `outro_movie`. | Lua: the `drive_to_target` handlers (17 in the class). |
| F-18 | 2.6 step 5, 4.6 | **Answer [exe]:** the bio's Accept (context bit 17) sets a counter of -1 to 4, so the first visit continues with step 5 (`blacklist_15`) and step 6 (stage 15: opening seen, reputation overview). No autosave step. | done |
| F-19 | 4.6 step 5 | **Answer [exe]:** `blacklist_%02d` of the stage byte (`profile + 0xB0`) at that moment. | done |
| F-20 | 4.6 step 6 | **Answer [exe]:** the stage 13 movie is `storyfmv_rap30`; `bounty_tutorial` is not played here. | done |
| F-21 | 4.6 (visual only) | The port draws nothing on the rival screens because their content groups carry definition flag `0x8` (CSH2 11.5). Whether the original clears the flag or only the alpha at run time. | Ghidra: the reveal scripts of `SafeHouseRivalChallenge` (groups `0x1162D4`, `0x10CF79`, `0x116248`) and `SafehouseRivalBio` (`0x116316`); the runtime flag word `+0x1C` bit 0. |
| F-22 | 4.4 | Which race names each `IntroNisBL` scene. `IntroNisBL01` and `IntroNisBL08` have bundles (names, this pass) but no race names them. Whether the later boss races of a gauntlet play an NIS. | Data: intro attributes of all 19 boss collections and the `*_r` events. Compare with the rival number. |
| F-23 | 4.7 | The path that enters bin N-1 after bin N is done. The callers of the bin-entry routine `0x0062C2D0` are the first-visit step, the Lua function `EEnterBin` (`0x0062C4C0`) and its event handler (`0x0062C470`). None of them is called from a script, NIS or world event (CB 5.3). | Ghidra: references to `0x0062C2D0`, `0x0062C470`, `0x0062C4C0` including data references (vtables, function tables, Lua constants). The `race_bin` done bytecode for a call to an entry function. |
| F-24 | 4.6 step 1 | The number of marker choices and the choice rule. CT2 says "a choice of two markers"; CSH 5.4 says up to four, shuffled, from the first six popup entries. | Ghidra: the `SafehouseMarkers` class `0x007B9A80` (count, shuffle, pick action). |
| F-25 | 4.5, 5.1 | The flag argument of the reward apply routine (`0x005A62D0`, called from `0x0062AA90`) and which caller sets it. Also which race holds `race_bin_01/rewards` (the collection exists, this pass; CB 1 says bin 1 has no reward list). | Ghidra: callers of `0x0062AA90` (data references included), `FUN_005FBE00`. Data: reward attribute `0x50104D90` of every race. |
| F-26 | 3.5 | The native code that sends message 120: the impound countdown writer that sets a car record to state 4, and when it runs. | Ghidra: the impound code (CP 5.1, CET). |
| F-27 | 3.3 | The three car calls and the pursuit-count call in the can-show test (virtual slots `+0xBC`, `+0x20`, `+0x5C`, `+0xAC`, then `+0xA0`). | Ghidra: the game manager's flush routine and the car object's virtual table (CP provenance). |
| F-28 | 3.7 | The keys behind hashes `0xC519BFC3` (replay), `0xC519BFC4` (delete) and `0x35F8620B` (open). | The input map of `SMS_Mailboxes.fng` and `SMS_Message.fng`. |
| F-29 | 3.7 | The voice take chosen from (group, line): the speech rule for the `cellcall` sentence (sentence 204, phrase 425). | Speech spec `speech.md` and the speech data. Not decoded in CP. |
| F-30 | 3.7 | The pending list is saved with every second id lost (writer steps by two, reader does not). Save-only; the list is almost always empty. | Ghidra: the save writer of the Game block (CP section 6). |
| F-31 | 5.3 | The direction reference of the 0.8 dot product (the car heading against the trigger's rotation, or the race route), the trigger's shape (box or sphere) and its dimensions. | Data: `win_trigger` (`Position`, `Rotation`, `Dimensions`). Lua: the win handler's arguments. |
| F-32 | 5.2 | The roles of `barrier_area`, `taggedcarmarker1` to `6`, `win_nis_marker`, `lose_trigger` and `startgrid1` (names, this pass). | Data: their attributes. Lua: the `epic_pursuit` copies. |
| F-33 | 5.3 | The native call that opens `SCENERY_GROUP_DOOR`, and the race's `Barriers` entry for it. | Ghidra: the barrier-group toggle called from the copy. Data: `Barriers` of `race_bin_01/epic_pursuit`. |
| F-34 | 5.4 | Where context bit 18 (`0x40000`) is cleared (CR2 3: not found), and how `RapSheetMain_ENDGAME` ends (timeline or "SUBMIT"). | Ghidra: writers of the context word bit 18; the class at `0x00532B90`; the exit of `RapSheetMain_ENDGAME`. |
| F-35 | 5.4 | Which code sets settings bit 14 (`0x4000`): the post-race return or the Game Won path (SAVE 3.3), and which comes first. | Ghidra: the post-race return `0x00623F80`; the writers of settings flags bit 14. |

Not blocking the machines (for the record): the consumer of the race `Reputation` attribute (`0x477EC5AA`, CB 4.2);
what "OPM" stands for (CB 1); whether the time-of-day index survives a reload (CSH2 7.6); the `Barriers` `*` prefix
(GR 8.4).

## 7. Conflicts in the source specs

Each conflict names the spec that disagrees and the F-item that settles it. Until settled, the rule in the section named
is the one to build.

| C-id | Topic | Conflict | Settled by |
|---|---|---|---|
| C-01 | The "won" flag | SAVE 3.2 sets node bit `0x2` on a win in career. CB 4.2 sets record flag `0x2` on every human finish with no placing test. | F-10 |
| C-02 | Cash per finish | CB 4.2: `CashValue` on every finish. RM 11: cash on first place. | F-10 |
| C-03 | Prologue intro | CSE 2.1 lists a movie and an NIS for the same race. CSE 1.1 plays the movie only when no NIS is set. | F-01 |
| C-04 | Settings bit 17 (`0x20000`) | CB 7.1: "first safe house visit processed". SAVE 3.3: "bonus-content popup already queued". CF 3 step 4 and CF 6 set it with the bonus probe. The context word's bit 17 is a different word (Blacklist sequence). | Writers of settings flags bit 17 (Ghidra) |
| C-05 | "Career flag 20" | CS 3.2 and CB 7.2 test flag 20 in `DoSpecialSetup`. SAVE 3.3 gives bit 20 as "E3 demo BMW given". | F-14 Settled by F-14: the same bit. |
| C-06 | Stage 13 movie | CB 5.4: `bounty_tutorial` and flag `0x10`. CSH 5.3: the rap sheet movie flow. | F-20 |
| C-07 | Marker choice count | CT2: "a choice of two markers". CSH 5.4: up to four shuffled. | F-24 |
| C-08 | Outro NIS | CSE 2.1 and RM 11: only `16_nis_rog_razor` has `OutroNIS`. The install's `16_1_1_circuit` has an `outro_nis_marker`. | F-12 |
| C-09 | Bin 1 rewards | CB 1: bin 1 has no reward list. The install has `race_bin_01/rewards`. | F-25 |
| C-10 | `drive_to_target` states | CS 1.2 lists six states, including `activate_gps` and `outro_movie`. CSE 7.1 describes four. | F-17 |
| C-11 | Settings bits 14 to 16 | CB 7.1: bit 14 career completed, bit 15 race 19.8.69 done, bit 16 bin 19 complete. SAVE 3.3: bit 14 unlocks bonus cars, bit 15 unlocks `BONUS_SL65`, bit 16 unlocks `BONUS_GT2`. Bit 14 gates the resume rule (story-flow.md section 2.7). | F-35 for bit 14; the others by Ghidra writers |
| C-12 | Arrival order | CF 7.3 puts the autosave after the region screen. CSH 5.3 (corrected) puts it at step 3, before the bio. | Follow CSH 5.3 (section 4.6). |
| C-13 | Loss path | CSE 1.2: a loss shows "the losing screen through raceover". CSE 1.1: `raceover` goes to postrace. | F-08 |
