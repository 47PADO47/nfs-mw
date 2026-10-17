# Career: the Blacklist, rival gating, race records and story progression

How the original tracks career progress: the 15 Blacklist bins, what unlocks a rival challenge, what a won race
changes, how the player moves down the list, and what is written to the save. The bounty, cash and infraction rules
are in the other career specs (bounty is only referenced here as an input). The race fields are in
[gameplay-races.md](../formats/gameplay-races.md); the post-chase sequence is in
[busted-sequence.md](busted-sequence.md).

- **Sources:** `speed.exe` (Black Edition PC v1.3, image base `0x00400000`) through the Ghidra MCP and a linear
  disassembly; the install at `D:\Need For Speed Most Wanted Black Edition` (`GLOBAL/gameplay.bin` read with the
  repo's `blackbox-attrib` reader, the 259 Lua 5.0 chunks of vault `gpcore` disassembled with a throwaway script,
  `GLOBAL/ATTRIBUTES.BIN`); the two save files in the user's Documents folder (layout check only, no values quoted
  except the structure). The community decompile was not re-read.
- **Evidence tags:** **[exe]** read in `speed.exe`, not tested in game. **[verified]** checked in install data or in
  a save file. **[community]** from the existing repo docs. **[unconfirmed]** a guess or an unproven rule.
- **Addresses** are `speed.exe` addresses. Ghidra names are only in
  [the provenance file](../provenance/career-blacklist.md).

## 0. Summary

1. Career progress is a stack of 15 **race bins** (`race_bin_01` .. `race_bin_15`, rival number = bin number, 15 is
   the first, 1 is Razor), plus bin 16 (the prologue) and four side bins (18 tollbooth challenges, 19 challenge
   series, 20 "OPM", 21 collectors). Each bin is a Lua state graph: `initial`, `unlock_races`, `introduce_rival`,
   `check_rival`, then `rival_locked` or `rival_unlocked`, then `done`. **[verified]** (scripts) + **[exe]**.
2. The player has one number, **CurrentBin** (a byte, 16 at the start, counting down to 1). It chooses the minimap
   variant, the closed-off city regions, which rival the rap sheet shows, and which milestone sets are live.
   **[exe]**
3. A rival challenge opens when three counters of the bin are met: races won >= `RequiredRacesWon`, challenges
   passed >= `RequiredChallenges`, total bounty >= `RequiredBounty`. All three are re-checked on every race result
   and every bounty change. Table in section 3. **[verified]** (script) + **[exe]**.
4. The counters are not stored per rival as a flag. Races won and challenges passed are recomputed from the race
   records and the milestone and speed-trap tables; the bin's script state is saved by name hash. **[exe]**
5. Winning a race for the first time (career mode) sets its record flag "completed in career", pays its cash value,
   grants its reward list and makes the races named in its unlock list available. **[exe]**
6. The final race is event `1.8.1` (Razor's pursuit). Winning it sets the career-completed flag and opens
   `GameWon.fng`. **[exe]**
7. Several script hooks that the names suggest do work are stubs in this build: `IntroduceRival`,
   `SetRaceCompleteForFE`, `StorePursuitRepForFE`, `ShowWinningPostRaceScreen`, `ShowLosingPostRaceScreen`.
   `AwardCash` and `AwardPoints` are the same function and only change the racer's in-race total. **[exe]**
8. **Not found:** the code path that moves CurrentBin from bin N to bin N-1 after a rival is beaten (section 5.3).
   The only routines that change it are the first-visit step (16 to 15) and a "enter bin" function reachable from a
   Lua function and an event, and no shipped script calls either. **[unconfirmed]** how real play reaches it.

## 1. The bins in the data **[verified]**

`gameplay.bin` holds one `race_bin` collection per bin (parent template `race_bin`, itself a `progression`).
Fields (names read from the scripts and the code; four fields are still unnamed, see the provenance file):

| Field | Type | Meaning |
|---|---|---|
| `BinIndex` | int | 1..16, 18..21 |
| `RequiredRacesWon` | int | races of the bin to win (section 3) |
| `RequiredChallenges` | int | milestones plus speed-trap challenges of the bin to pass |
| `RequiredBounty` | int | total bounty needed |
| `Boss` | key | the `boss` collection (the rival) |
| `WorldRaces` | key[] | the free races of the bin (counted for "races won") |
| `BossRaces` | key[] | the rival's races (what the challenge button starts) |
| `Children` | key[] | every race, trigger and unlock collection under the bin |
| `EntryCellCallID` | int | phone call or message id queued when the bin starts (0 = none) |
| `SMSRivalChallenge` | int | message id sent when the rival unlocks (-1 = none) |
| `SMSCellChallenge` | int | message id sent when the last challenge is passed (-1 = none) |
| `CellChallengeRace` | key | race unlocked at that moment (bin 15 only: `1.2.2`) |
| `EntryActivity` | key | activity run on entry (bin 16 only: drive to the safe house) |
| `Barriers` | text[] | track barriers closed while this bin is current (section 6) |
| unlock list (`0xBAF89280`) | key[] | `unlocks/upgrade_*` collections granted on entering the bin (section 5.2) |

Per-bin values (collection names are the developers' names; the shown names come from the language table
`BLACKLIST_RIVAL_NN_NAME`, not read here). Skill and MinAI are the `boss` collection's `SkillLevel` and
`MinimumAIPerformance`; the boss's car is the preset ride `BLn`.

| Bin | Rival collection | Skill | MinAI | Races won | Challenges | Bounty | World | Boss | Entry call | Rival SMS | Unlock parts |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 15 | tony_ho | 0 | - | 3 | 3 | 20,000 | 3 | 2 | 0 | 20 | 11 |
| 14 | vince_kilic | 15 | 0.0 | 4 | 3 | 50,000 | 6 | 2 | none | 29 | 6 |
| 13 | victor_vasquez | 20 | 0.05 | 4 | 3 | 100,000 | 7 | 2 | 30 | 36 | 7 |
| 12 | isabel_diaz | 25 | 0.1 | 4 | 3 | 180,000 | 7 | 2 | 37 | 41 | 8 |
| 11 | lou_park | 30 | 0.2 | 5 | 3 | 300,000 | 8 | 2 | 42 | 46 | 6 |
| 10 | karl_smit | 40 | 0.3 | 5 | 4 | 500,000 | 8 | 2 | 48 | 52 | 6 |
| 9 | pierre_dupont | 45 | 0.4 | 5 | 4 | 790,000 | 8 | 2 | 54 | 57 | 6 |
| 8 | jade_barrett | 50 | 0.4 | 5 | 4 | 1,180,000 | 7 | 2 | 58 | 63 | 7 |
| 7 | kira_nazakato | 55 | 0.5 | 7 | 4 | 1,680,000 | 10 | 2 | 64 | 69 | 5 |
| 6 | hector_domingo | 60 | 0.5 | 7 | 4 | 2,300,000 | 11 | 2 | 70 | 72 | 2 |
| 5 | wes_allen | 65 | 0.6 | 7 | 4 | 3,050,000 | 10 | 3 | 73 | 76 | 5 |
| 4 | joe_vega | 75 | 0.6 | 7 | 5 | 4,050,000 | 11 | 3 | 77 | 79 | 2 |
| 3 | ronnie_mccrea | 80 | 0.7 | 8 | 5 | 5,550,000 | 12 | 3 | 80 | 84 | 0 |
| 2 | toro_sato | 80 | 0.8 | 8 | 5 | 7,550,000 | 12 | 3 | 85 | 88 | 2 |
| 1 | razor_callahan | 80 | 0.9 | 9 | 5 | 10,000,000 | 11 | 6 | 89 | none | 0 |

Notes:

- Bin 15 has no bounty gate in practice beyond 20,000 and its three world races; the player has `RequiredRacesWon`
  3 but only 3 world races, so every one must be won.
- Bin 1 lists six boss races: `1.5.2` speed-trap race, `1.7.3` drag, `4.2.1` sprint, `1.1.2` circuit, `1.2.3`
  sprint, and `epic_pursuit` (event `1.8.1`, a `challengerace` with forced heat 6).
- `race_bin_16` has only `RequiredRacesWon` 4 and an entry activity. It is the prologue: events `16.1.0`
  (partial D-Day), `16.1.1`, `16.2.1` (D-Day intro), `16.2.2`, `16.2.3`, `16.8.1` (NIS shell).
- Side bins: `race_bin_challenge` (index 19, 36 children, 70 world races across both challenge bins),
  `race_bin_challenge_tollbooth` (18), `race_bin_opm` (20, boss `mr_x` with car `rx8`, races `20.1.1`, `20.1.2`,
  `20.2.1`, `20.8.1`), `race_bin_collectors` (21, three races). What "OPM" stands for is **[unconfirmed]**.
- The script wide defaults of a bin (template `race_bin`): `RequiredBounty` 0, `RequiredChallenges` 0,
  `RequiredRacesWon` 0, `BinIndex` 0; bins 15 and 14 set the two unnamed floats `0xE10FB7A3` and `0x1823B89E`
  (bin 14: 2000.0) whose use is unknown.

## 2. The bin state machine **[verified]** (scripts) **[exe]** (hooks)

Every bin collection runs the same graph. Handlers are in vault `gpcore`; per-bin override handlers differ only in
phone and message ids (section 2.7).

### 2.1 States

```
initial --(stateenter / activity finished)--> unlock_races --> introduce_rival --> check_rival
check_rival --> rival_locked   (a counter is short)
check_rival --> rival_unlocked (all three counters met)
rival_locked --(race placed, challenge passed, rep changed, entering gameplay)--> check_rival
rival_unlocked --(every boss race won)--> done
```

### 2.2 `initial`

On enter: if the bin has an `EntryActivity` whose `AutoStart` is true, run it (bin 16 only). If `EntryCellCallID` is
not 0, queue that message. Then go to `unlock_races`. If the entry activity finishes (message `NotifyFinished` with
the sender equal to the entry activity) go to `unlock_races`. Bins 13, 14, 15 have override handlers that reset the
SMS bookkeeping flags (`DoneFirstSMS`, `DoneSecondSMS`, `DoneThirdSMS`), and bin 13 sets the "has rap sheet" flag
(section 7.2). Bin 14's override also queues messages 125 and 126.

### 2.3 `unlock_races`

Reset the bin's script table: `NumRacesWon`, `PlayerBounty`, `NumChallengesPassed`, `NumWorldRaces`, `NumRivalRaces`,
`NumRivalRacesWon` = 0; `NumWorldRaces` = length of `WorldRaces`, `NumRivalRaces` = length of `BossRaces`. For every
race trigger of the bin's `engage` child whose `InitiallyUnlocked` is true, call `UnlockRace(TargetActivity)`. Go
to `introduce_rival`.

### 2.4 `introduce_rival`

Calls the script function `IntroduceRival` (a stub in this build, section 7.3) and goes to `check_rival`. A movie
handler (`NotifyMovieFinished`) exists and does nothing visible.

### 2.5 `check_rival`

```
NumRacesWon         = Bin.GetNumRacesWon(BinIndex)            # recomputed, section 4.2
NumChallengesPassed = Bin.GetNumChallengesPassed(BinIndex)
PlayerBounty        = Game.GetPlayerBounty()                  # total bounty of the stable, section 3.2
unlocked = NumRacesWon >= RequiredRacesWon
       and PlayerBounty >= RequiredBounty
       and NumChallengesPassed >= RequiredChallenges
ChangeState(unlocked ? "rival_unlocked" : "rival_locked")
```

### 2.6 `rival_locked`, `rival_unlocked`, `done`

- `rival_locked` re-enters `check_rival` on: `EnteringGameplay`; `NotifyPlayerRep` (bounty changed, section 3.2);
  `NotifyRacePlacement` when the placing is 1st and the race is one of the bin's `WorldRaces`;
  `NotifyChallengePassed` (below).
- `NotifyChallengePassed` in the generic handler: `NumChallengesPassed` + 1; when it equals `RequiredChallenges`,
  queue `SMSCellChallenge` if it is 0 or more, unlock `CellChallengeRace` if set (bin 15 unlocks event `1.2.2`),
  then `check_rival`. The per-bin override handlers instead read `Bin.GetNumChallengesPassed` and queue the
  "first" and "second" challenge messages once each (flags `DoneFirstSMS`, `DoneSecondSMS`).
- `rival_unlocked` on enter: queue `SMSRivalChallenge` if 0 or more; call `UnlockRace` for every `BossRaces`
  entry; if the bin has no boss races go straight to `done`. On `NotifyRacePlacement` with the placing 1st: if the
  race is one of the `BossRaces`, `NumRivalRacesWon` + 1; when it equals `NumRivalRaces` go to `done`.
- `done` on enter: `UnlockRace` for every `WorldRaces` entry (so all the bin's free races stay available).

The state name hash of each bin is stored in the save (section 8.2): the first four bytes of the bin's saved table
are the lookup2 hash of the current state name (`done` `0x32956993`, `rival_locked` `0x2ABB39C2`,
`rival_unlocked` `0x76F21CB8`, `initial` `0x904ECCE4`). **[verified]** in both save files.

### 2.7 Messages and phone calls

`EAddSMS(id)` appends a text message to the in-game inbox; `ECellCall(1, 14)` places a call. The ids above are
indexes into the phone table, not read here. Bins 4..15 have per-bin handlers that send one or two challenge
messages when 1 or 3 (bins use their own thresholds, for example `2` for bin 13) challenges are passed.
Bin 1 queues messages 93 and 94 when its rival unlocks. **[verified]**

## 3. The gate for a rival challenge

### 3.1 Inputs

| Input | Source | Rule |
|---|---|---|
| Races won | record flags of the bin's `WorldRaces` | count of races with the "completed in career" flag (section 4.1) |
| Challenges passed | milestone table and speed-trap table | count of entries of this bin that are completed (section 4.3) |
| Bounty | stable | total bounty (section 3.2) |

All three are compared with `>=`; there is no pursuit-count or time gate in the bin data. **[verified]** The
coordinator's "pursuit count" gate does not exist as a field; the bounty total already sums pursuits.

### 3.2 Bounty

`GetPlayerBounty` returns the stable's total bounty (the sum over the owned cars' career records plus the banked
amount at stable offset `0x8CA0`). `AwardPlayerBounty` adds to the current car's career record, divides the amount
by a global scale first, and broadcasts `MNotifyPlayerRep` with the new total as `RepPoints`. The rival scripts
listen to that message. See the bounty spec for the rest. **[exe]**

### 3.3 The challenge button (rival challenge screen) **[exe]**

The screen `SafeHouseRivalChallenge.fng` shows the selected bin (a global holds the bin shown). It reads the bin's
counters and requirements and draws each as met or not met (races: won vs required; challenges: passed vs
required; bounty: player total vs required). The button reads `RIVAL_CHALLENGE_ACTIVATED` when either:

- some entry of `BossRaces` is unlocked (record flag 8) and not yet completed (flag 2), or
- the career-completed flag is set (section 7.1), or
- the selected bin number is higher than CurrentBin (`CurrentBin < selected bin`): the rival is already beaten and
  its races can be replayed.

Otherwise it reads `RIVAL_CHALLENGE_DEACTIVATED`. This is the replay rule: nothing in the bin data is called
"ReplayBlacklistRaces" in this build (no such string exists). **[exe]**

## 4. Race records and the counters

### 4.1 The race record table **[exe]** + **[verified]**

One 16-byte record per race event, up to 300, keyed by the hash of the event id text (for example `"16.1.0"`):

| Offset | Type | Meaning |
|---|---|---|
| 0x00 | u32 | event key (lookup2 hash of `EventID`); 0 = free slot |
| 0x04 | u32 | flags (below) |
| 0x08 | f32 | best result (time, lap time, points or cash by race type) |
| 0x0C | u16 | derived average speed (set at finish) **[unconfirmed]** |
| 0x0E | u16 | derived top speed (set at finish) **[unconfirmed]** |

Flags:

| Bit | Meaning |
|---|---|
| 0x01 | completed outside career mode (quick race and the like) |
| 0x02 | **completed in career mode** (read by `IsRaceCompleted` and by the bin counters) |
| 0x04 | available (set at start from `InitiallyUnlocked`, or when an earlier race lists it as unlocked) |
| 0x08 | **unlocked by script** (`UnlockRace`; read by `IsRaceUnlocked`) |
| 0x10 | revealed on the map **[unconfirmed]** |

A fresh table fills flags from the race attributes: for every race that is not flagged by attribute `0x8CB01ABF`,
flag 4 when `0xB39ED8C3` or `InitiallyUnlocked` is true (and not a boss race), flag 0x10 when `0x39509746` is true.
The meaning of these three attributes is **[unconfirmed]**. In the install, `InitiallyUnlocked` is true for five
collections.

`UnlockRace(race)` sets flag 4 if the race's attribute `0xA4E6FCFD` is false, then flag 8, then refreshes the map
trigger icons (an icon is shown only when flag 8 is set and flag 2 is clear). **[exe]**

### 4.2 Recording a finish **[exe]**

When the local player finishes (the race-finished hook, once per race through a "processed" byte of the race-status
object):

1. Read whether the race was already completed (flag 2) before this finish.
2. Best result by `GRace::Type` (gameplay-races.md): types 0, 2, 4, 9, 10 (sprint, drag, tollbooth, jumps) keep the
   smaller race time; types 1 and 3 (circuit, knockout) keep a lap-based value; type 5 (speed trap) and 7 (cash
   grab) keep the larger points value; type 6 keeps the larger integer. A first finish always stores the value.
3. Set flag 2 in career mode, flag 1 otherwise; update speed fields.
4. In career mode add the race's `CashValue` (attribute `0xD8BAA07B`, float, truncated) to the profile cash
   (career settings offset 0x0C). This happens on every finish, not only the first.
5. If it was not completed before: grant each entry of the race's reward list (attribute `0x50104D90`, request
   objects), then for every race in its unlock list (attribute `0xFC8995C8`) set flag 4, and flag 8 unless the race
   attribute `0xA4E6FCFD` is true.
6. Refresh the trigger icons.

The `Reputation` attribute (`0x477EC5AA`, e.g. 23,999 on boss race `1.5.2`) is not read by the finish hook. Its
consumer was not found. **[unconfirmed]**

### 4.3 Bin counters **[exe]**

`races won` = number of `WorldRaces` entries whose record has flag 2.
`challenges passed` = number of milestone entries with the bin's index in state 4 (completed) plus the number of
speed-trap challenge entries with the bin's index and flag 4. Both are recomputed after a milestone completes and
stored as two u16 in the bin record (offset 0x18 challenges, 0x1A races won). Boss races are **not** counted in
races won. **[exe]** + **[verified]** (a save with all milestones done shows 4 + 3 = 7 challenges for a bin with 4
milestones and 3 speed traps).

### 4.4 Milestones and speed traps **[exe]** + **[verified]**

Each bin has 3 or 4 milestone collections (`milestones/bin_NN/challenge_N_*`; types: cops damaged, cops
destroyed, pursuit length, pursuit evasion time, cost to state, roadblocks dodged, tire spikes dodged, total
infractions) and 1..3 speed traps (`speedtraps/bin_NN/speedtrapK`). The table has 55 milestones and 39 speed-trap
entries. Entering a bin activates its milestones (state 1 becomes 2) and sets flags 3 on its speed-trap entries.
A milestone moves 2, 3 (reached, waiting to be claimed), 4 (completed). On completion the bin counters are
recomputed, the milestone's `Bounty` is awarded and `ChallengeCompleted` is broadcast; the rival scripts hear it as
`NotifyChallengePassed`. Bounties: 100,000 (bin 4 cops damaged) down to 5,000 (bin 15 evasion time). **[exe]**

## 5. CurrentBin

### 5.1 Storage and values **[exe]** + **[verified]**

Career settings object at profile offset 0xA8: `+0` current car handle, `+4` flags, `+8` **CurrentBin** (u8),
`+0xC` cash (u32). In the save file CurrentBin is the byte at file offset `0x4038` and cash the u32 at `0x4039`
(the community editor's money offset), checked on both saves (values 15 and 1).

- A new career sets it to 16 on the first safe-house arrival (`DoSafeHouseIntro`).
- A debug start sets 15.
- The first safe-house screen after the prologue, with CurrentBin 16, enters bin 15 (section 5.2).

### 5.2 Entering a bin **[exe]**

The routine takes a target bin and loops `CurrentBin - 1` down to the target. For every step it writes CurrentBin,
sets milestone state 1 to 2 for that bin, sets the speed-trap flags |= 3, and, unless a flag of the manager is set,
looks up the bin record and grants each entry of its unlock list (the `unlocks/upgrade_*` collections: parts that
the shops sell from then on; section 1 table, last column) through the same request object as a race reward. It
also marks the rival's `Boss` character and finally refreshes the race-status object's current bin and its barriers.

### 5.3 What calls it (open) **[unconfirmed]**

Callers: the first-visit step above; a Lua function named `EEnterBin(n)`; the handler of an event of the same name
(the event's constructor is an empty stub). No shipped script (259 chunks) and no data in the install contains the
event hash `0x2AF7332B`. A separate screen-sequence routine runs after the bin changes and reads the new
CurrentBin (section 5.4), so the trigger is somewhere in the front-end or an unscanned data source.

### 5.4 Post-bin front-end sequence **[exe]**

A step counter drives screens after a rival is beaten: 1 `SafeHouseMarkers`, 2 `SafeHouseRegionUnlock` only when
CurrentBin is 12 or 8, 3/4 `SafeHouseRivalBio`, 5 movie `blacklist_NN` (`FEAnyMovie`), 6 either (CurrentBin 15) the
reputation overview and career flag 0x20, or (first time at CurrentBin 13, career flag 0x10 clear) the
`bounty_tutorial` movie and flag 0x10, then a final request. The movies `MOVIES\blacklist_01..15` are the rival
introductions.

## 6. Regions

The city opens in three steps, set by the barrier lists of the bins (collections `BARRIER_LOCKED_10`..`_15`):

| CurrentBin | Barriers closed | Minimap file |
|---|---|---|
| 16..13 | 11, 12, 13, 14, 15 | `MINI_MAP_UNLOCK_1` (first region) |
| 12..9 | 10, 12, 13 | `MINI_MAP_UNLOCK_2` (two regions) |
| 8..1 | none | `MINI_MAP` (full city) |

The map choice in free roam: bin < 9 full, 9..12 `_UNLOCK_2`, >= 13 `_UNLOCK_1`. **[exe]** (map) **[verified]**
(barrier lists; bins 9..12 list 10, 12, 13 in different orders).

## 7. Flags and hooks

### 7.1 Career flags (career settings offset +4) **[exe]**

| Bit | Value | Meaning |
|---|---|---|
| 0 | 0x1 | a career exists (new career when clear) |
| 4 | 0x10 | has rap sheet (set by `SetHasRapSheet` at bin 13 and by the tutorial step) |
| 5 | 0x20 | reputation overview shown at bin 15 |
| 11 | 0x800 | cleared on load; layout switch of the rap sheet main screen |
| 14 | 0x4000 | **career completed**; set when the post-race return finds race `1.8.1` completed; unlocks all rival challenges and the `GameWon.fng` screen |
| 15 | 0x8000 | race event `19.8.69` (challenge 34) finished |
| 16 | 0x10000 | every race of the challenge bin (19) except `19.8.31` and optional ones completed |
| 17 | 0x20000 | first safe-house visit processed |

Both inspected saves hold flags `0x001DF7F3`, so they carry bits 14..17 together; they look tool-made and cannot
confirm the individual bits. **[unconfirmed]** for the save-side semantics.

### 7.2 Script hooks (registry at `0x0061E750`)

| Name | Address | Behaviour |
|---|---|---|
| `IsCareerMode` | `0x00605610` | race-status mode word equals 2 |
| `UnlockRace(r)` | `0x00612140` | section 4.1 |
| `IsRaceUnlocked(r)` | `0x00605170` | record flag 8 |
| `IsRaceCompleted(r)` | `0x006051B0` | record flag 2 |
| `GetNumRacesWon(bin)` / `GetNumChallengesPassed(bin)` | `0x00604A30` / `0x00604A10` | bin record u16 at +0x1A / +0x18 |
| `GetPlayerBounty` | `0x00612300` | stable total |
| `AwardCash` / `AwardPoints` | `0x00612130` / `0x0060E030` | same function: adds the amount to the racer's in-race total (race-status racer record +0x120, clamped at 0). No profile effect |
| `SetHasRapSheet` | `0x00605670` | career flag 0x10 |
| `DoSafeHouseIntro` | `0x00605560` | stores the respawn marker, CurrentBin = 16, opens the front-end |
| `JumpToCarLot` / `JumpToSafeHouse` | `0x00605250` / `0x006052B0` | front-end requests `Car_Select.fng` / `MainMenu_Sub.fng` |
| `SkipCareerIntro` | `0x00612490` | debug flags only |
| `DoSpecialSetup` | `0x00605830` | forces the player's car for special races when career flag 20 is clear |
| `SetHasBeenWon` | `0x00604080` | writes the race-status "processed" byte |
| `IntroduceRival`, `SetRaceCompleteForFE`, `StorePursuitRepForFE`, `ShowWinningPostRaceScreen`, `ShowLosingPostRaceScreen` | | stubs (return at once) |

Race-status object (global `0x0091E000`): `+0x1958` racer count, `+0x1960` state (0 roaming, 1 racing), `+0x1964`
game mode (2 career), `+0x1968` current race parameters, `+0x196C` current bin record, `+0x4554` processed byte;
racers are 0x194 bytes each from +0x18. **[exe]**

## 8. Save data

### 8.1 Placement **[verified]**

Inside the profile buffer (starts at file 0x24, data at 0x34): the 0x4000-byte "Game" block at 0x34, then the career
settings (section 5.1) at 0x4034: `u32 car, u8 CurrentBin, u32 cash, u16 (+0x10), u32 flags, 600 bytes (150 u32,
0xFF-filled in the samples), u16, 16-byte id string`; then the stable, then the 300 race records
(1,201 dwords).

### 8.2 The "Game" block **[verified]** (both saves) **[exe]**

16-byte MD5 of the rest, `"Game"`, version 8, then counts: state tables, 8 message slots (0x20 bytes), 30 timer
records (0x10), 55 milestone records and 39 challenge records (0x14 each, matched on load by the key in word 1),
512 flag bits (64 bytes), the bin block (`u32 n`, then n x `u16 bin, u16 4, u32 (u16 challenges, u16 races won)`, 20 bins),
and two respawn marker hashes. Data starts at block offset 0x80, each part 16-byte aligned. State tables are
`key u32, size u32, bytes`, key = hash of the bin or graph name (`race_bin_NN`, `speedtraps`), first u32 of the bytes =
current state hash.

Milestone record (5 u32): `type hash`, `key = lookup2(collection name)`, `state u8 / smaller-is-better flag u8 / bin u16`,
`threshold f32`, `0`. Challenge record: `flags u8 (bit 0 and 1 set on entering the bin, bit 2 completed) + bin u16`,
`key`, `0`, `threshold f32` (speed in km/h), `0`. **[verified]** (names resolve for all 94 keys; completed samples show
state 4 and flags 5).

## 9. Entry and end of the career

- Menu strings `CLoadCareer`, `CStartNewCareer`, `CResumeCareer`; the game-flow object `CareerGame` at `0x008A8F48`.
  Starting a new career loads the preset car `M3GTRCAREERSTART`, clears the race globals, marks event `16.1.0` as
  the current engaged race and starts the game flow (section 5.1). The respawn marker falls back to the collection
  `career_start_l2ra`. **[exe]**
- After the D-Day intro (`16.2.1`) the done handler calls `JumpToCarLot`: the first real car is chosen at the lot,
  then bin 16 drives to the safe house (GPS target). **[verified]**
- The last race is `1.8.1`; its win trigger needs speed >= 180 km/h and direction cosine >= 0.8, then the NIS
  `EndingNis04` plays and the pursuit is bailed. **[verified]**
- `RapSheetMain_ENDGAME.fng` / `RapSheetLogin2_ENDGAME.fng` are rap-sheet navigation targets; the strings
  `CAREER_COMPLETED_DATA` and `GAME_COMPLETED_DATA` fill the stats page. The `CareerModeHasBeenCompletedAtLeastOnce`
  flag of the profile layout was not located separately. **[unconfirmed]**

## 10. Open questions

1. What advances CurrentBin after bin 15 (section 5.3).
2. Names and effects of attributes `0xB39ED8C3`, `0x39509746`, `0x8CB01ABF`, `0xA4E6FCFD`, `0xE10FB7A3`, `0x1823B89E`.
3. The consumer of the race `Reputation` value.
4. What the OPM bin is and when it opens.
5. Whether `CurrentBin` may differ from the count of `done` bins in a normal game.
